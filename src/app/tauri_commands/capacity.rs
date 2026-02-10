use crate::app::state::AppState;
use serde::{Deserialize, Serialize};

use super::common::emit_frontend_event;

// ==========================================
// 产能池管理相关命令
// ==========================================

/// 查询产能池列表
///
/// # 参数
/// - machine_codes: 机组代码列表 (JSON数组字符串, 如: ["H032", "H033"])
/// - date_from: 日期范围起始 (YYYY-MM-DD)
/// - date_to: 日期范围结束 (YYYY-MM-DD)
/// - version_id: 方案版本ID (可选，若未提供则使用当前激活版本)
///
/// # 返回
/// - 成功: JSON字符串, 包含产能池列表
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn get_capacity_pools(
    state: tauri::State<'_, AppState>,
    machine_codes: String,
    date_from: String,
    date_to: String,
    version_id: Option<String>,
) -> Result<String, String> {
    use chrono::NaiveDate;

    // 解析机组代码列表
    let codes: Vec<String> =
        serde_json::from_str(&machine_codes).map_err(|e| format!("机组代码格式错误: {}", e))?;

    // 解析日期
    let start_date = NaiveDate::parse_from_str(&date_from, "%Y-%m-%d")
        .map_err(|e| format!("起始日期格式错误（应为YYYY-MM-DD）: {}", e))?;
    let end_date = NaiveDate::parse_from_str(&date_to, "%Y-%m-%d")
        .map_err(|e| format!("结束日期格式错误（应为YYYY-MM-DD）: {}", e))?;

    let plan_api = state.plan_api.clone();
    let capacity_pool_repo = state.capacity_pool_repo.clone();

    let result = tauri::async_runtime::spawn_blocking(
        move || -> Result<Vec<crate::domain::capacity::CapacityPool>, String> {
            let _perf = crate::perf::PerfGuard::new("ipc.get_capacity_pools");

            // 获取版本ID（如未提供则使用当前激活版本）
            let vid = match version_id.as_ref() {
                Some(v) => v.clone(),
                None => plan_api
                    .get_latest_active_version_id()
                    .map_err(|e| format!("获取激活版本失败: {}", e))?
                    .ok_or_else(|| "当前没有激活版本".to_string())?,
            };

            // 收集所有机组的产能池
            let mut all_pools = Vec::new();
            for code in &codes {
                let pools = capacity_pool_repo
                    .find_by_date_range(&vid, code, start_date, end_date)
                    .map_err(|e| format!("查询产能池失败: {}", e))?;
                all_pools.extend(pools);
            }
            Ok(all_pools)
        },
    )
    .await
    .map_err(|e| format!("任务执行失败: {}", e))??;

    // 序列化返回
    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 更新产能池参数
///
/// # 参数
/// - machine_code: 机组代码
/// - plan_date: 计划日期 (YYYY-MM-DD)
/// - target_capacity_t: 目标产能（吨）
/// - limit_capacity_t: 上限产能（吨）
/// - reason: 修改原因
///
/// # 返回
/// - 成功: 空JSON对象
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn update_capacity_pool(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    machine_code: String,
    plan_date: String,
    target_capacity_t: f64,
    limit_capacity_t: f64,
    reason: String,
    operator: String,
    version_id: Option<String>,
) -> Result<String, String> {
    use crate::domain::action_log::ActionLog;
    use crate::domain::capacity::CapacityPool;
    use chrono::NaiveDate;

    // 解析日期
    let date = NaiveDate::parse_from_str(&plan_date, "%Y-%m-%d")
        .map_err(|e| format!("日期格式错误（应为YYYY-MM-DD）: {}", e))?;

    // 验证参数
    if target_capacity_t < 0.0 {
        return Err("目标产能不能为负数".to_string());
    }
    if limit_capacity_t < 0.0 {
        return Err("上限产能不能为负数".to_string());
    }
    if limit_capacity_t < target_capacity_t {
        return Err("上限产能不能小于目标产能".to_string());
    }

    tracing::info!(
        "[update_capacity_pool] 更新产能池: {} {} target={} limit={} reason={} operator={}",
        machine_code,
        plan_date,
        target_capacity_t,
        limit_capacity_t,
        reason,
        operator
    );

    // 获取版本ID（如未提供则使用当前激活版本）
    let vid = match version_id.as_ref() {
        Some(v) => v.clone(),
        None => state
            .plan_api
            .get_latest_active_version_id()
            .map_err(|e| format!("获取激活版本失败: {}", e))?
            .ok_or_else(|| "当前没有激活版本".to_string())?,
    };

    // 查询现有产能池
    let existing = state
        .capacity_pool_repo
        .find_by_machine_and_date(&vid, &machine_code, date)
        .map_err(|e| format!("查询产能池失败: {}", e))?;

    // 记录旧值用于审计
    let old_target = existing.as_ref().map(|p| p.target_capacity_t);
    let old_limit = existing.as_ref().map(|p| p.limit_capacity_t);

    // 构造新的产能池数据
    let pool = match existing {
        Some(mut p) => {
            // 更新现有记录
            p.target_capacity_t = target_capacity_t;
            p.limit_capacity_t = limit_capacity_t;
            p
        }
        None => {
            // 创建新记录
            CapacityPool {
                version_id: vid.clone(),
                machine_code: machine_code.clone(),
                plan_date: date,
                target_capacity_t,
                limit_capacity_t,
                used_capacity_t: 0.0,
                overflow_t: 0.0,
                frozen_capacity_t: 0.0,
                accumulated_tonnage_t: 0.0,
                roll_campaign_id: None,
            }
        }
    };

    // 保存到数据库
    state
        .capacity_pool_repo
        .upsert_single(&pool)
        .map_err(|e| format!("更新产能池失败: {}", e))?;

    let version_id_for_log = version_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("N/A")
        .to_string();

    // 记录 ActionLog（红线5：可解释性/审计追踪）
    let action_log = ActionLog {
        action_id: uuid::Uuid::new_v4().to_string(),
        version_id: Some(version_id_for_log),
        action_type: "UPDATE_CAPACITY_POOL".to_string(),
        action_ts: chrono::Local::now().naive_local(),
        actor: operator.clone(),
        payload_json: Some(serde_json::json!({
            "machine_code": machine_code,
            "plan_date": plan_date,
            "target_capacity_t": target_capacity_t,
            "limit_capacity_t": limit_capacity_t,
            "reason": reason,
        })),
        impact_summary_json: Some(serde_json::json!({
            "old_target": old_target,
            "new_target": target_capacity_t,
            "old_limit": old_limit,
            "new_limit": limit_capacity_t,
        })),
        machine_code: Some(machine_code.clone()),
        date_range_start: Some(date),
        date_range_end: Some(date),
        detail: Some(reason),
    };

    // 尝试记录ActionLog，失败时只记录警告（不影响主要操作）
    if let Err(e) = state.action_log_repo.insert(&action_log) {
        tracing::warn!(error = %e, "记录产能池更新操作日志失败");
    }

    // 可选：触发决策读模型刷新（产能池参数修改可能影响超限/瓶颈等口径）
    if let Some(version_id) = version_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if let Err(e) = state
            .plan_api
            .manual_refresh_decision(version_id, &operator)
        {
            tracing::warn!("产能池更新后触发决策刷新失败: {}", e);
        }

        emit_frontend_event(
            &app,
            "risk_snapshot_updated",
            serde_json::json!({ "version_id": version_id, "source": "update_capacity_pool" }),
        );
    }

    Ok("{}".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityPoolUpdate {
    pub machine_code: String,
    pub plan_date: String,
    pub target_capacity_t: f64,
    pub limit_capacity_t: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchUpdateCapacityPoolsResponse {
    pub requested: usize,
    pub updated: usize,
    pub skipped: usize,
    pub upserted_rows: usize,
    pub refresh: Option<crate::api::plan_api::ManualRefreshDecisionResponse>,
    pub message: String,
}

/// 批量更新产能池参数（P2-1）
///
/// # 参数
/// - updates: JSON数组字符串，元素结构同 CapacityPoolUpdate
/// - reason: 修改原因（必填）
/// - operator: 操作人
/// - version_id: 关联版本（可选；若传入则会 best-effort 触发决策刷新）
///
/// # 返回
/// - 成功: JSON字符串, BatchUpdateCapacityPoolsResponse
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn batch_update_capacity_pools(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    updates: String,
    reason: String,
    operator: String,
    version_id: Option<String>,
) -> Result<String, String> {
    use crate::domain::action_log::ActionLog;
    use crate::domain::capacity::CapacityPool;
    use chrono::NaiveDate;

    if reason.trim().is_empty() {
        return Err("请输入调整原因".to_string());
    }
    if operator.trim().is_empty() {
        return Err("操作人不能为空".to_string());
    }

    // 获取版本ID（如未提供则使用当前激活版本）
    let vid = match version_id.as_ref() {
        Some(v) => v.clone(),
        None => state
            .plan_api
            .get_latest_active_version_id()
            .map_err(|e| format!("获取激活版本失败: {}", e))?
            .ok_or_else(|| "当前没有激活版本".to_string())?,
    };

    let items: Vec<CapacityPoolUpdate> =
        serde_json::from_str(&updates).map_err(|e| format!("updates格式错误: {}", e))?;
    if items.is_empty() {
        return Err("updates不能为空".to_string());
    }

    let mut pools_to_upsert: Vec<CapacityPool> = Vec::new();
    let mut skipped = 0usize;

    let mut min_date: Option<NaiveDate> = None;
    let mut max_date: Option<NaiveDate> = None;

    // 为了避免 ActionLog payload 过大，对明细做截断（只保留 sample）
    let mut change_samples: Vec<serde_json::Value> = Vec::new();
    let max_samples: usize = 200;

    for it in &items {
        let date = NaiveDate::parse_from_str(&it.plan_date, "%Y-%m-%d")
            .map_err(|e| format!("日期格式错误（应为YYYY-MM-DD）: {}", e))?;

        // 验证参数
        if it.target_capacity_t < 0.0 {
            return Err(format!(
                "目标产能不能为负数: {} {}",
                it.machine_code, it.plan_date
            ));
        }
        if it.limit_capacity_t < 0.0 {
            return Err(format!(
                "上限产能不能为负数: {} {}",
                it.machine_code, it.plan_date
            ));
        }
        if it.limit_capacity_t < it.target_capacity_t {
            return Err(format!(
                "上限产能不能小于目标产能: {} {}",
                it.machine_code, it.plan_date
            ));
        }

        let existing = state
            .capacity_pool_repo
            .find_by_machine_and_date(&vid, &it.machine_code, date)
            .map_err(|e| format!("查询产能池失败: {}", e))?;

        // 跳过无变化项（避免无意义的 OR REPLACE + 审计噪音）
        let unchanged = existing.as_ref().is_some_and(|p| {
            (p.target_capacity_t - it.target_capacity_t).abs() < f64::EPSILON
                && (p.limit_capacity_t - it.limit_capacity_t).abs() < f64::EPSILON
        });
        if unchanged {
            skipped += 1;
            continue;
        }

        let old_target = existing.as_ref().map(|p| p.target_capacity_t);
        let old_limit = existing.as_ref().map(|p| p.limit_capacity_t);

        let pool = match existing {
            Some(mut p) => {
                p.target_capacity_t = it.target_capacity_t;
                p.limit_capacity_t = it.limit_capacity_t;
                p
            }
            None => CapacityPool {
                version_id: vid.clone(),
                machine_code: it.machine_code.clone(),
                plan_date: date,
                target_capacity_t: it.target_capacity_t,
                limit_capacity_t: it.limit_capacity_t,
                used_capacity_t: 0.0,
                overflow_t: 0.0,
                frozen_capacity_t: 0.0,
                accumulated_tonnage_t: 0.0,
                roll_campaign_id: None,
            },
        };

        pools_to_upsert.push(pool);

        if change_samples.len() < max_samples {
            change_samples.push(serde_json::json!({
                "machine_code": it.machine_code,
                "plan_date": it.plan_date,
                "old_target": old_target,
                "new_target": it.target_capacity_t,
                "old_limit": old_limit,
                "new_limit": it.limit_capacity_t,
            }));
        }

        min_date = Some(min_date.map(|d| d.min(date)).unwrap_or(date));
        max_date = Some(max_date.map(|d| d.max(date)).unwrap_or(date));
    }

    let updated = pools_to_upsert.len();
    let requested = items.len();

    let upserted_rows = if pools_to_upsert.is_empty() {
        0
    } else {
        state
            .capacity_pool_repo
            .upsert_batch(pools_to_upsert)
            .map_err(|e| format!("批量更新产能池失败: {}", e))?
    };

    let version_id_for_log = version_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("N/A")
        .to_string();

    let action_log = ActionLog {
        action_id: uuid::Uuid::new_v4().to_string(),
        version_id: Some(version_id_for_log),
        action_type: "BATCH_UPDATE_CAPACITY_POOL".to_string(),
        action_ts: chrono::Local::now().naive_local(),
        actor: operator.clone(),
        payload_json: Some(serde_json::json!({
            "requested": requested,
            "updated": updated,
            "skipped": skipped,
            "reason": reason,
            "changes_sample": change_samples,
            "sample_truncated": requested.saturating_sub(skipped) > max_samples,
        })),
        impact_summary_json: Some(serde_json::json!({
            "requested": requested,
            "updated": updated,
            "skipped": skipped,
            "upserted_rows": upserted_rows,
        })),
        machine_code: None,
        date_range_start: min_date,
        date_range_end: max_date,
        detail: Some("批量更新产能池参数".to_string()),
    };

    if let Err(e) = state.action_log_repo.insert(&action_log) {
        tracing::warn!(error = %e, "记录批量产能池更新操作日志失败");
    }

    // best-effort：若提供 version_id，则触发决策刷新，并 emit 一个事件让前端及时拉取刷新状态。
    let refresh = if updated == 0 {
        None
    } else if let Some(vid) = version_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let resp = match state.plan_api.manual_refresh_decision(vid, &operator) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("批量更新产能池后触发决策刷新失败: {}", e);
                crate::api::plan_api::ManualRefreshDecisionResponse {
                    version_id: vid.to_string(),
                    task_id: None,
                    success: false,
                    message: format!("触发决策刷新失败: {}", e),
                }
            }
        };

        emit_frontend_event(
            &app,
            "risk_snapshot_updated",
            serde_json::json!({ "version_id": vid, "source": "batch_update_capacity_pools" }),
        );

        Some(resp)
    } else {
        None
    };

    let resp = BatchUpdateCapacityPoolsResponse {
        requested,
        updated,
        skipped,
        upserted_rows,
        refresh,
        message: if updated == 0 {
            "无变更，已跳过".to_string()
        } else {
            "批量更新完成".to_string()
        },
    };

    serde_json::to_string(&resp).map_err(|e| format!("序列化失败: {}", e))
}

// ==========================================
// 机组产能配置管理相关命令
// ==========================================

/// 查询机组产能配置
///
/// # 参数
/// - version_id: 版本ID
/// - machine_codes: 可选的机组代码列表 (JSON数组字符串)，如为空则返回该版本下所有配置
///
/// # 返回
/// - 成功: JSON字符串, 包含配置列表
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn get_machine_capacity_configs(
    state: tauri::State<'_, AppState>,
    version_id: String,
    machine_codes: Option<String>,
) -> Result<String, String> {
    use crate::api::machine_config_api::MachineConfigApi;
    use crate::repository::MachineConfigRepository;

    let db_path = state.db_path.clone();
    let capacity_pool_repo = state.capacity_pool_repo.clone();
    let action_log_repo = state.action_log_repo.clone();

    let result = tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let _perf = crate::perf::PerfGuard::new("ipc.get_machine_capacity_configs");

        // 创建仓储和API实例
        let machine_config_repo = std::sync::Arc::new(
            MachineConfigRepository::new(&db_path)
                .map_err(|e| format!("创建机组配置仓储失败: {}", e))?,
        );

        let machine_config_api =
            MachineConfigApi::new(machine_config_repo, capacity_pool_repo, action_log_repo);

        // 解析 machine_codes（如果提供）
        let codes: Option<Vec<String>> = match machine_codes {
            Some(ref json_str) if !json_str.trim().is_empty() => Some(
                serde_json::from_str(json_str).map_err(|e| format!("机组代码格式错误: {}", e))?,
            ),
            _ => None,
        };

        // 调用API
        let configs = machine_config_api
            .get_machine_capacity_configs(&version_id, codes)
            .map_err(|e| format!("查询机组配置失败: {}", e))?;

        // 序列化返回
        serde_json::to_string(&configs).map_err(|e| format!("序列化失败: {}", e))
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))??;

    Ok(result)
}

/// 创建或更新机组配置
///
/// # 参数
/// - request_json: 请求数据的JSON字符串 (包含 version_id, machine_code, default_daily_target_t等)
///
/// # 返回
/// - 成功: JSON字符串, 包含创建/更新响应
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn create_or_update_machine_config(
    state: tauri::State<'_, AppState>,
    request_json: String,
) -> Result<String, String> {
    use crate::api::machine_config_api::{CreateOrUpdateMachineConfigRequest, MachineConfigApi};
    use crate::repository::MachineConfigRepository;

    let db_path = state.db_path.clone();
    let capacity_pool_repo = state.capacity_pool_repo.clone();
    let action_log_repo = state.action_log_repo.clone();

    let result = tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let _perf = crate::perf::PerfGuard::new("ipc.create_or_update_machine_config");

        // 解析请求
        let request: CreateOrUpdateMachineConfigRequest =
            serde_json::from_str(&request_json).map_err(|e| format!("请求格式错误: {}", e))?;

        // 创建仓储和API实例
        let machine_config_repo = std::sync::Arc::new(
            MachineConfigRepository::new(&db_path)
                .map_err(|e| format!("创建机组配置仓储失败: {}", e))?,
        );

        let machine_config_api =
            MachineConfigApi::new(machine_config_repo, capacity_pool_repo, action_log_repo);

        // 调用API
        let response = machine_config_api
            .create_or_update_machine_config(request)
            .map_err(|e| format!("创建/更新机组配置失败: {}", e))?;

        // 序列化返回
        serde_json::to_string(&response).map_err(|e| format!("序列化失败: {}", e))
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))??;

    Ok(result)
}

/// 批量应用机组配置到产能池日期范围
///
/// # 参数
/// - request_json: 请求数据的JSON字符串 (包含 version_id, machine_code, date_from, date_to等)
///
/// # 返回
/// - 成功: JSON字符串, 包含应用结果响应
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn apply_machine_config_to_dates(
    state: tauri::State<'_, AppState>,
    request_json: String,
) -> Result<String, String> {
    use crate::api::machine_config_api::{ApplyConfigToDateRangeRequest, MachineConfigApi};
    use crate::repository::MachineConfigRepository;

    let db_path = state.db_path.clone();
    let capacity_pool_repo = state.capacity_pool_repo.clone();
    let action_log_repo = state.action_log_repo.clone();

    let result = tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let _perf = crate::perf::PerfGuard::new("ipc.apply_machine_config_to_dates");

        // 解析请求
        let request: ApplyConfigToDateRangeRequest =
            serde_json::from_str(&request_json).map_err(|e| format!("请求格式错误: {}", e))?;

        // 创建仓储和API实例
        let machine_config_repo = std::sync::Arc::new(
            MachineConfigRepository::new(&db_path)
                .map_err(|e| format!("创建机组配置仓储失败: {}", e))?,
        );

        let machine_config_api =
            MachineConfigApi::new(machine_config_repo, capacity_pool_repo, action_log_repo);

        // 调用API
        let response = machine_config_api
            .apply_machine_config_to_dates(request)
            .map_err(|e| format!("应用机组配置失败: {}", e))?;

        // 序列化返回
        serde_json::to_string(&response).map_err(|e| format!("序列化失败: {}", e))
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))??;

    Ok(result)
}

/// 查询机组配置历史（跨版本）
///
/// # 参数
/// - machine_code: 机组代码
/// - limit: 可选的限制条数
///
/// # 返回
/// - 成功: JSON字符串, 包含历史配置列表（按创建时间倒序）
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn get_machine_config_history(
    state: tauri::State<'_, AppState>,
    machine_code: String,
    limit: Option<usize>,
) -> Result<String, String> {
    use crate::api::machine_config_api::MachineConfigApi;
    use crate::repository::MachineConfigRepository;

    let db_path = state.db_path.clone();
    let capacity_pool_repo = state.capacity_pool_repo.clone();
    let action_log_repo = state.action_log_repo.clone();

    let result = tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let _perf = crate::perf::PerfGuard::new("ipc.get_machine_config_history");

        // 创建仓储和API实例
        let machine_config_repo = std::sync::Arc::new(
            MachineConfigRepository::new(&db_path)
                .map_err(|e| format!("创建机组配置仓储失败: {}", e))?,
        );

        let machine_config_api =
            MachineConfigApi::new(machine_config_repo, capacity_pool_repo, action_log_repo);

        // 调用API
        let history = machine_config_api
            .get_machine_config_history(&machine_code, limit)
            .map_err(|e| format!("查询配置历史失败: {}", e))?;

        // 序列化返回
        serde_json::to_string(&history).map_err(|e| format!("序列化失败: {}", e))
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))??;

    Ok(result)
}

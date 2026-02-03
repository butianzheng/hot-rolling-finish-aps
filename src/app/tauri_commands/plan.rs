use crate::app::state::AppState;
use crate::engine::ScheduleStrategy;

use super::common::{emit_frontend_event, map_api_error, parse_date};

// ==========================================
// 排产方案相关命令
// ==========================================

/// 创建排产方案
#[tauri::command(rename_all = "snake_case")]
pub async fn create_plan(
    state: tauri::State<'_, AppState>,
    plan_name: String,
    created_by: String,
) -> Result<String, String> {
    let result = state
        .plan_api
        .create_plan(plan_name, created_by)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询方案列表
#[tauri::command(rename_all = "snake_case")]
pub async fn list_plans(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let result = state.plan_api.list_plans().map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询方案详情
#[tauri::command(rename_all = "snake_case")]
pub async fn get_plan_detail(
    state: tauri::State<'_, AppState>,
    plan_id: String,
) -> Result<String, String> {
    let result = state
        .plan_api
        .get_plan_detail(&plan_id)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询最近创建的激活版本ID（跨方案）
#[tauri::command(rename_all = "snake_case")]
pub async fn get_latest_active_version_id(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let result = state
        .plan_api
        .get_latest_active_version_id()
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 删除排产方案（级联删除版本与明细）
#[tauri::command(rename_all = "snake_case")]
pub async fn delete_plan(
    state: tauri::State<'_, AppState>,
    plan_id: String,
    operator: String,
) -> Result<String, String> {
    state
        .plan_api
        .delete_plan(&plan_id, &operator)
        .map_err(map_api_error)?;

    Ok("{}".to_string())
}

/// 删除版本（仅允许删除非激活版本）
#[tauri::command(rename_all = "snake_case")]
pub async fn delete_version(
    state: tauri::State<'_, AppState>,
    version_id: String,
    operator: String,
) -> Result<String, String> {
    state
        .plan_api
        .delete_version(&version_id, &operator)
        .map_err(map_api_error)?;

    Ok("{}".to_string())
}

/// 创建新版本
#[tauri::command(rename_all = "snake_case")]
pub async fn create_version(
    state: tauri::State<'_, AppState>,
    plan_id: String,
    window_days: i32,
    frozen_from_date: Option<String>,
    note: Option<String>,
    created_by: String,
) -> Result<String, String> {
    let frozen_date = frozen_from_date
        .map(|s| parse_date(&s))
        .transpose()?;

    let result = state
        .plan_api
        .create_version(plan_id, window_days, frozen_date, note, created_by)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询版本列表
#[tauri::command(rename_all = "snake_case")]
pub async fn list_versions(
    state: tauri::State<'_, AppState>,
    plan_id: String,
) -> Result<String, String> {
    let result = state
        .plan_api
        .list_versions(&plan_id)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 激活版本
#[tauri::command(rename_all = "snake_case")]
pub async fn activate_version(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    version_id: String,
    operator: String,
) -> Result<String, String> {
    state
        .plan_api
        .activate_version(&version_id, &operator)
        .map_err(map_api_error)?;

    // 版本切换后，多个页面需要联动刷新（plan items / decision read models / KPI）。
    emit_frontend_event(&app, "plan_updated", serde_json::json!({ "version_id": version_id }));
    emit_frontend_event(&app, "risk_snapshot_updated", serde_json::json!({}));

    Ok("{}".to_string()) // 返回空JSON对象表示成功
}

/// 版本回滚（激活历史版本 + 恢复配置快照）
#[tauri::command(rename_all = "snake_case")]
pub async fn rollback_version(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    plan_id: String,
    target_version_id: String,
    operator: String,
    reason: String,
) -> Result<String, String> {
    let result = state
        .plan_api
        .rollback_version(&plan_id, &target_version_id, &operator, &reason)
        .map_err(map_api_error)?;

    // 回滚后，多个页面需要联动刷新（plan items / decision read models / KPI）。
    emit_frontend_event(
        &app,
        "plan_updated",
        serde_json::json!({ "version_id": target_version_id }),
    );
    emit_frontend_event(&app, "risk_snapshot_updated", serde_json::json!({}));

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 试算接口（沙盘模式）
#[tauri::command(rename_all = "snake_case")]
pub async fn simulate_recalc(
    state: tauri::State<'_, AppState>,
    version_id: String,
    base_date: String,
    frozen_date: Option<String>,
    operator: String,
    strategy: Option<String>,
) -> Result<String, String> {
    let base_date = parse_date(&base_date)?;
    let frozen_date = frozen_date.map(|s| parse_date(&s)).transpose()?;

    let strategy = strategy
        .as_deref()
        .unwrap_or("balanced")
        .parse::<ScheduleStrategy>()
        .map_err(|e| format!("策略类型解析失败: {}", e))?;

    let result = state
        .plan_api
        .simulate_recalc_with_strategy(&version_id, base_date, frozen_date, &operator, strategy)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 一键重算
#[tauri::command(rename_all = "snake_case")]
pub async fn recalc_full(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    version_id: String,
    base_date: String,
    frozen_date: Option<String>,
    operator: String,
    strategy: Option<String>,
) -> Result<String, String> {
    let base_date = parse_date(&base_date)?;
    let frozen_date = frozen_date.map(|s| parse_date(&s)).transpose()?;

    let strategy = strategy
        .as_deref()
        .unwrap_or("balanced")
        .parse::<ScheduleStrategy>()
        .map_err(|e| format!("策略类型解析失败: {}", e))?;

    let result = state
        .plan_api
        .recalc_full_with_strategy(&version_id, base_date, frozen_date, &operator, strategy)
        .map_err(map_api_error)?;

    let version_id_for_event = result.version_id.clone();
    emit_frontend_event(
        &app,
        "plan_updated",
        serde_json::json!({ "version_id": version_id_for_event }),
    );
    emit_frontend_event(&app, "risk_snapshot_updated", serde_json::json!({}));

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 获取预设策略列表（用于策略草案对比）
#[tauri::command(rename_all = "snake_case")]
pub async fn get_strategy_presets(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let result = state
        .plan_api
        .get_strategy_presets()
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 生成多策略草案（dry-run，不落库）
#[tauri::command(rename_all = "snake_case")]
pub async fn generate_strategy_drafts(
    state: tauri::State<'_, AppState>,
    base_version_id: String,
    plan_date_from: String,
    plan_date_to: String,
    strategies: Vec<String>,
    operator: String,
) -> Result<String, String> {
    let from = parse_date(&plan_date_from)?;
    let to = parse_date(&plan_date_to)?;

    let result = state
        .plan_api
        .generate_strategy_drafts(&base_version_id, from, to, strategies, &operator)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 发布策略草案：生成正式版本（落库）
#[tauri::command(rename_all = "snake_case")]
pub async fn apply_strategy_draft(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    draft_id: String,
    operator: String,
) -> Result<String, String> {
    let result = state
        .plan_api
        .apply_strategy_draft(&draft_id, &operator)
        .map_err(map_api_error)?;

    let version_id_for_event = result.version_id.clone();
    emit_frontend_event(
        &app,
        "plan_updated",
        serde_json::json!({ "version_id": version_id_for_event }),
    );
    emit_frontend_event(&app, "risk_snapshot_updated", serde_json::json!({}));

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询策略草案变更明细（用于前端抽屉展示）
#[tauri::command(rename_all = "snake_case")]
pub async fn get_strategy_draft_detail(
    state: tauri::State<'_, AppState>,
    draft_id: String,
) -> Result<String, String> {
    let result = state
        .plan_api
        .get_strategy_draft_detail(&draft_id)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 列出策略草案（用于页面刷新/重启后的恢复）
#[tauri::command(rename_all = "snake_case")]
pub async fn list_strategy_drafts(
    state: tauri::State<'_, AppState>,
    base_version_id: String,
    plan_date_from: String,
    plan_date_to: String,
    status_filter: Option<String>,
    limit: Option<i64>,
) -> Result<String, String> {
    let from = parse_date(&plan_date_from)?;
    let to = parse_date(&plan_date_to)?;

    let result = state
        .plan_api
        .list_strategy_drafts(&base_version_id, from, to, status_filter, limit)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 清理过期草案（避免草案表无限增长）
#[tauri::command(rename_all = "snake_case")]
pub async fn cleanup_expired_strategy_drafts(
    state: tauri::State<'_, AppState>,
    keep_days: i64,
) -> Result<String, String> {
    let result = state
        .plan_api
        .cleanup_expired_strategy_drafts(keep_days)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询排产明细
#[tauri::command(rename_all = "snake_case")]
pub async fn list_plan_items(
    state: tauri::State<'_, AppState>,
    version_id: String,
    plan_date_from: Option<String>,
    plan_date_to: Option<String>,
    machine_code: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<String, String> {
    let from = plan_date_from
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_date)
        .transpose()?;

    let to = plan_date_to
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_date)
        .transpose()?;

    let machine_code = machine_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let has_filters = from.is_some()
        || to.is_some()
        || machine_code.is_some()
        || limit.is_some()
        || offset.is_some();

    let result = if has_filters {
        state
            .plan_api
            .list_plan_items_filtered(
                &version_id,
                machine_code.as_deref(),
                from,
                to,
                limit,
                offset,
            )
            .map_err(map_api_error)?
    } else {
        state
            .plan_api
            .list_plan_items(&version_id)
            .map_err(map_api_error)?
    };

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 按日期查询排产明细
#[tauri::command(rename_all = "snake_case")]
pub async fn list_items_by_date(
    state: tauri::State<'_, AppState>,
    version_id: String,
    plan_date: String,
) -> Result<String, String> {
    let date = parse_date(&plan_date)?;

    let result = state
        .plan_api
        .list_items_by_date(&version_id, date)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 版本对比
#[tauri::command(rename_all = "snake_case")]
pub async fn compare_versions(
    state: tauri::State<'_, AppState>,
    version_id_a: String,
    version_id_b: String,
) -> Result<String, String> {
    let result = state
        .plan_api
        .compare_versions(&version_id_a, &version_id_b)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 版本对比 KPI 汇总（聚合）
#[tauri::command(rename_all = "snake_case")]
pub async fn compare_versions_kpi(
    state: tauri::State<'_, AppState>,
    version_id_a: String,
    version_id_b: String,
) -> Result<String, String> {
    let result = state
        .plan_api
        .compare_versions_kpi(&version_id_a, &version_id_b)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 移动排产项
///
/// # 参数
/// - version_id: 版本ID
/// - moves: 移动项列表 (JSON字符串)
/// - mode: 校验模式 (AUTO_FIX/STRICT)
/// - operator: 操作人（写入操作日志）
/// - reason: 操作原因（可选）
#[tauri::command(rename_all = "snake_case")]
pub async fn move_items(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    version_id: String,
    moves: String,
    mode: Option<String>,
    operator: String,
    reason: Option<String>,
) -> Result<String, String> {
    use crate::api::ValidationMode;
    use crate::api::plan_api::MoveItemRequest;

    // 解析移动项列表
    let move_requests: Vec<MoveItemRequest> = serde_json::from_str(&moves)
        .map_err(|e| format!("解析移动项失败: {}", e))?;

    // 解析校验模式，默认为Strict（兼容 AutoFix 和 AUTO_FIX 两种格式）
    let validation_mode = match mode.as_deref() {
        Some("AutoFix") | Some("AUTO_FIX") => ValidationMode::AutoFix,
        _ => ValidationMode::Strict,
    };

    let result = state
        .plan_api
        .move_items(&version_id, move_requests, validation_mode, &operator, reason.as_deref())
        .map_err(map_api_error)?;

    emit_frontend_event(
        &app,
        "plan_updated",
        serde_json::json!({ "version_id": version_id, "has_violations": result.has_violations }),
    );
    emit_frontend_event(&app, "risk_snapshot_updated", serde_json::json!({}));

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}


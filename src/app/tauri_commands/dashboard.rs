use crate::app::state::AppState;

use super::common::{map_api_error, parse_date};

// ==========================================
// 驾驶舱相关命令
// ==========================================

/// 查询风险快照列表
#[tauri::command(rename_all = "snake_case")]
pub async fn list_risk_snapshots(
    state: tauri::State<'_, AppState>,
    version_id: String,
) -> Result<String, String> {
    let result = state
        .dashboard_api
        .list_risk_snapshots(&version_id)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询风险快照（按日期）
#[tauri::command(rename_all = "snake_case")]
pub async fn get_risk_snapshot(
    state: tauri::State<'_, AppState>,
    version_id: String,
    snapshot_date: String,
) -> Result<String, String> {
    let date = parse_date(&snapshot_date)?;

    let result = state
        .dashboard_api
        .get_risk_snapshot(&version_id, date)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 哪天最危险
#[tauri::command(rename_all = "snake_case")]
pub async fn get_most_risky_date(
    state: tauri::State<'_, AppState>,
    version_id: String,
) -> Result<String, String> {
    let result = state
        .dashboard_api
        .get_most_risky_date(&version_id, None, None, None, None)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 哪些紧急单无法完成
#[tauri::command(rename_all = "snake_case")]
pub async fn get_unsatisfied_urgent_materials(
    state: tauri::State<'_, AppState>,
    version_id: String,
) -> Result<String, String> {
    let result = state
        .dashboard_api
        .get_unsatisfied_urgent_materials(&version_id, None, None, None)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 哪些冷料压库（向后兼容版本，使用 version_id）
#[tauri::command(rename_all = "snake_case")]
pub async fn get_cold_stock_materials(
    state: tauri::State<'_, AppState>,
    version_id: String,
    threshold_days: Option<i32>,
) -> Result<String, String> {
    // threshold_days 参数为向后兼容保留，不再使用
    let _ = threshold_days;

    let result = state
        .dashboard_api
        .get_cold_stock_materials(&version_id, None, None, None)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 哪个机组最堵（向后兼容版本，使用 version_id）
#[tauri::command(rename_all = "snake_case")]
pub async fn get_most_congested_machine(
    state: tauri::State<'_, AppState>,
    version_id: String,
) -> Result<String, String> {
    let result = state
        .dashboard_api
        .get_most_congested_machine(&version_id, None, None, None, None, None)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 获取决策数据刷新状态（P0-2）
#[tauri::command(rename_all = "snake_case")]
pub async fn get_refresh_status(
    state: tauri::State<'_, AppState>,
    version_id: String,
) -> Result<String, String> {
    // 该查询应当“非常快”，且在重算/发布期间前端会高频轮询。
    // 若放进 spawn_blocking，可能在 blocking pool 被长任务占满时排队，从而触发前端 Timeout。
    let result = state
        .dashboard_api
        .get_refresh_status(&version_id)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 手动触发决策读模型刷新（P0-2：失败可重试）
#[tauri::command(rename_all = "snake_case")]
pub async fn manual_refresh_decision(
    state: tauri::State<'_, AppState>,
    version_id: String,
    operator: String,
) -> Result<String, String> {
    let plan_api = state.plan_api.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        plan_api.manual_refresh_decision(&version_id, &operator)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
    .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询操作日志（按时间范围）
#[tauri::command(rename_all = "snake_case")]
pub async fn list_action_logs(
    state: tauri::State<'_, AppState>,
    start_time: String,
    end_time: String,
) -> Result<String, String> {
    use chrono::NaiveDateTime;

    let start = NaiveDateTime::parse_from_str(&start_time, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| format!("开始时间格式错误: {}", e))?;

    let end = NaiveDateTime::parse_from_str(&end_time, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| format!("结束时间格式错误: {}", e))?;

    let result = state
        .dashboard_api
        .list_action_logs(start, end)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询操作日志（按材料ID + 时间范围）
#[tauri::command(rename_all = "snake_case")]
pub async fn list_action_logs_by_material(
    state: tauri::State<'_, AppState>,
    material_id: String,
    start_time: String,
    end_time: String,
    limit: Option<i32>,
) -> Result<String, String> {
    use chrono::NaiveDateTime;

    let start = NaiveDateTime::parse_from_str(&start_time, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| format!("开始时间格式错误: {}", e))?;

    let end = NaiveDateTime::parse_from_str(&end_time, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| format!("结束时间格式错误: {}", e))?;

    let limit = limit.unwrap_or(50);

    let result = state
        .dashboard_api
        .list_action_logs_by_material(&material_id, start, end, limit)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询操作日志（按版本）
#[tauri::command(rename_all = "snake_case")]
pub async fn list_action_logs_by_version(
    state: tauri::State<'_, AppState>,
    version_id: String,
) -> Result<String, String> {
    let result = state
        .dashboard_api
        .list_action_logs_by_version(&version_id)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询最近操作
#[tauri::command(rename_all = "snake_case")]
pub async fn get_recent_actions(
    state: tauri::State<'_, AppState>,
    limit: i32,
    offset: Option<i32>,
    start_time: Option<String>,
    end_time: Option<String>,
) -> Result<String, String> {
    use chrono::NaiveDateTime;

    let offset = offset.unwrap_or(0);

    let start = start_time
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .map_err(|e| format!("开始时间格式错误: {}", e))
        })
        .transpose()?;

    let end = end_time
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .map_err(|e| format!("结束时间格式错误: {}", e))
        })
        .transpose()?;

    let result = if offset != 0 || start.is_some() || end.is_some() {
        state
            .dashboard_api
            .get_recent_actions_filtered(limit, offset, start, end)
            .map_err(map_api_error)?
    } else {
        state
            .dashboard_api
            .get_recent_actions(limit)
            .map_err(map_api_error)?
    };

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

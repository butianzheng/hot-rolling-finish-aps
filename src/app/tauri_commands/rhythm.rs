use crate::app::state::AppState;
use crate::engine::{ScheduleEvent, ScheduleEventType};

use super::common::map_api_error;

// ==========================================
// 每日生产节奏（品种大类）相关命令
// ==========================================

/// 查询节奏预设模板
#[tauri::command(rename_all = "snake_case")]
pub async fn list_rhythm_presets(
    state: tauri::State<'_, AppState>,
    dimension: Option<String>,
    active_only: Option<bool>,
) -> Result<String, String> {
    let result = state
        .rhythm_api
        .list_presets_with_active(dimension.as_deref(), active_only.unwrap_or(true))
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 新增/更新节奏预设模板（供设置中心维护）
#[tauri::command(rename_all = "snake_case")]
pub async fn upsert_rhythm_preset(
    state: tauri::State<'_, AppState>,
    preset_id: Option<String>,
    preset_name: String,
    dimension: String,
    target_json: String,
    is_active: Option<bool>,
    operator: String,
    reason: String,
) -> Result<String, String> {
    let result = state
        .rhythm_api
        .upsert_preset(
            preset_id.as_deref(),
            &preset_name,
            &dimension,
            &target_json,
            is_active,
            &operator,
            &reason,
        )
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 启用/停用节奏预设模板（软删除）
#[tauri::command(rename_all = "snake_case")]
pub async fn set_rhythm_preset_active(
    state: tauri::State<'_, AppState>,
    preset_id: String,
    is_active: bool,
    operator: String,
    reason: String,
) -> Result<String, String> {
    let result = state
        .rhythm_api
        .set_preset_active(&preset_id, is_active, &operator, &reason)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询节奏目标（按版本×机组×日期）
#[tauri::command(rename_all = "snake_case")]
pub async fn list_rhythm_targets(
    state: tauri::State<'_, AppState>,
    version_id: String,
    dimension: String,
    machine_codes: Option<String>, // JSON数组字符串
    date_from: Option<String>,
    date_to: Option<String>,
) -> Result<String, String> {
    let machine_codes = if let Some(raw) = machine_codes {
        let list: Vec<String> = serde_json::from_str(&raw).map_err(|e| format!("machine_codes 解析失败: {}", e))?;
        Some(list)
    } else {
        None
    };

    let result = state
        .rhythm_api
        .list_targets(
            &version_id,
            &dimension,
            machine_codes.as_deref(),
            date_from.as_deref(),
            date_to.as_deref(),
        )
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 更新单日节奏目标（一天一套）
#[tauri::command(rename_all = "snake_case")]
pub async fn upsert_rhythm_target(
    state: tauri::State<'_, AppState>,
    version_id: String,
    machine_code: String,
    plan_date: String, // YYYY-MM-DD
    dimension: String,
    target_json: String, // JSON对象
    preset_id: Option<String>,
    operator: String,
    reason: String,
) -> Result<String, String> {
    state
        .rhythm_api
        .upsert_target(
            &version_id,
            &machine_code,
            &plan_date,
            &dimension,
            &target_json,
            preset_id.as_deref(),
            &operator,
            &reason,
        )
        .map_err(map_api_error)?;

    // 发布事件触发决策读模型刷新（D4 等）
    if let Some(ref publisher) = state.event_publisher {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(plan_date.trim(), "%Y-%m-%d") {
            let event = ScheduleEvent::incremental(
                version_id.clone(),
                ScheduleEventType::RhythmTargetChanged,
                Some(format!("upsert_rhythm_target: {}", machine_code)),
                Some(vec![machine_code.clone()]),
                Some((d, d)),
            );
            if let Err(e) = publisher.publish(event) {
                tracing::warn!("发布 RhythmTargetChanged 事件失败: {}", e);
            }
        }
    }

    Ok("{}".to_string())
}

/// 批量应用节奏模板到指定机组/日期范围
#[tauri::command(rename_all = "snake_case")]
pub async fn apply_rhythm_preset(
    state: tauri::State<'_, AppState>,
    version_id: String,
    dimension: String,
    preset_id: String,
    machine_codes: String, // JSON数组字符串
    date_from: String,
    date_to: String,
    overwrite: Option<bool>,
    operator: String,
    reason: String,
) -> Result<String, String> {
    let machines: Vec<String> =
        serde_json::from_str(&machine_codes).map_err(|e| format!("machine_codes 解析失败: {}", e))?;

    let applied = state
        .rhythm_api
        .apply_preset(
            &version_id,
            &dimension,
            &preset_id,
            &machines,
            &date_from,
            &date_to,
            overwrite.unwrap_or(true),
            &operator,
            &reason,
        )
        .map_err(map_api_error)?;

    if let Some(ref publisher) = state.event_publisher {
        let start = chrono::NaiveDate::parse_from_str(date_from.trim(), "%Y-%m-%d").ok();
        let end = chrono::NaiveDate::parse_from_str(date_to.trim(), "%Y-%m-%d").ok();
        let event = ScheduleEvent::incremental(
            version_id.clone(),
            ScheduleEventType::RhythmTargetChanged,
            Some(format!("apply_rhythm_preset: {}", preset_id)),
            Some(machines),
            match (start, end) {
                (Some(s), Some(e)) => Some((s, e)),
                _ => None,
            },
        );
        if let Err(e) = publisher.publish(event) {
            tracing::warn!("发布 RhythmTargetChanged 事件失败: {}", e);
        }
    }

    serde_json::to_string(&serde_json::json!({ "applied": applied }))
        .map_err(|e| format!("序列化失败: {}", e))
}

/// 查询单日节奏画像（目标 vs 实际）
#[tauri::command(rename_all = "snake_case")]
pub async fn get_daily_rhythm_profile(
    state: tauri::State<'_, AppState>,
    version_id: String,
    machine_code: String,
    plan_date: String,
) -> Result<String, String> {
    let result = state
        .rhythm_api
        .get_daily_profile(&version_id, &machine_code, &plan_date)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}


use crate::app::state::AppState;
use crate::engine::{ScheduleEvent, ScheduleEventType};

use super::common::{map_api_error, parse_date};

// ==========================================
// 换辊管理相关命令
// ==========================================

/// 查询版本的所有换辊窗口
#[tauri::command(rename_all = "snake_case")]
pub async fn list_roll_campaigns(
    state: tauri::State<'_, AppState>,
    version_id: String,
) -> Result<String, String> {
    let result = state
        .roller_api
        .list_campaigns(&version_id)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询版本的换辊时间监控计划（用于微调）
#[tauri::command(rename_all = "snake_case")]
pub async fn list_roll_campaign_plans(
    state: tauri::State<'_, AppState>,
    version_id: String,
) -> Result<String, String> {
    let result = state
        .roller_api
        .list_campaign_plans(&version_id)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 更新换辊时间监控计划（周期起点/计划换辊时刻/停机时长）
#[tauri::command(rename_all = "snake_case")]
pub async fn upsert_roll_campaign_plan(
    state: tauri::State<'_, AppState>,
    version_id: String,
    machine_code: String,
    initial_start_at: String,
    next_change_at: Option<String>,
    downtime_minutes: Option<i32>,
    operator: String,
    reason: String,
) -> Result<String, String> {
    state
        .roller_api
        .upsert_campaign_plan(
            &version_id,
            &machine_code,
            &initial_start_at,
            next_change_at.as_deref(),
            downtime_minutes,
            &operator,
            &reason,
        )
        .map_err(map_api_error)?;

    // 发布 ScheduleEvent 触发决策读模型刷新（D5 等）
    if let Some(ref publisher) = state.event_publisher {
        let event = ScheduleEvent::incremental(
            version_id.clone(),
            ScheduleEventType::RollCampaignChanged,
            Some(format!("upsert_roll_campaign_plan: {}", machine_code)),
            Some(vec![machine_code.clone()]),
            None,
        );
        if let Err(e) = publisher.publish(event) {
            tracing::warn!("发布 RollCampaignChanged 事件失败: {}", e);
        }
    }

    Ok("{}".to_string())
}

/// 查询机组当前进行中的换辊窗口
#[tauri::command(rename_all = "snake_case")]
pub async fn get_active_roll_campaign(
    state: tauri::State<'_, AppState>,
    version_id: String,
    machine_code: String,
) -> Result<String, String> {
    let result = state
        .roller_api
        .get_active_campaign(&version_id, &machine_code)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询需要换辊的机组列表
#[tauri::command(rename_all = "snake_case")]
pub async fn list_needs_roll_change(
    state: tauri::State<'_, AppState>,
    version_id: String,
) -> Result<String, String> {
    let result = state
        .roller_api
        .list_needs_roll_change(&version_id)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 创建新的换辊窗口
#[tauri::command(rename_all = "snake_case")]
pub async fn create_roll_campaign(
    state: tauri::State<'_, AppState>,
    version_id: String,
    machine_code: String,
    campaign_no: i32,
    start_date: String,
    suggest_threshold_t: Option<f64>,
    hard_limit_t: Option<f64>,
    operator: String,
    reason: String,
) -> Result<String, String> {
    let start_date = parse_date(&start_date)?;

    state
        .roller_api
        .create_campaign(
            &version_id,
            &machine_code,
            campaign_no,
            start_date,
            suggest_threshold_t,
            hard_limit_t,
            &operator,
            &reason,
        )
        .map_err(map_api_error)?;

    // 发布 ScheduleEvent 触发决策读模型刷新（D5 等）
    if let Some(ref publisher) = state.event_publisher {
        let event = ScheduleEvent::incremental(
            version_id.clone(),
            ScheduleEventType::RollCampaignChanged,
            Some(format!(
                "create_roll_campaign: {}#{}",
                machine_code, campaign_no
            )),
            Some(vec![machine_code.clone()]),
            None,
        );
        if let Err(e) = publisher.publish(event) {
            tracing::warn!("发布 RollCampaignChanged 事件失败: {}", e);
        }
    }

    Ok("{}".to_string())
}

/// 结束换辊窗口
#[tauri::command(rename_all = "snake_case")]
pub async fn close_roll_campaign(
    state: tauri::State<'_, AppState>,
    version_id: String,
    machine_code: String,
    campaign_no: i32,
    end_date: String,
    operator: String,
    reason: String,
) -> Result<String, String> {
    let end_date = parse_date(&end_date)?;

    state
        .roller_api
        .close_campaign(
            &version_id,
            &machine_code,
            campaign_no,
            end_date,
            &operator,
            &reason,
        )
        .map_err(map_api_error)?;

    // 发布 ScheduleEvent 触发决策读模型刷新（D5 等）
    if let Some(ref publisher) = state.event_publisher {
        let event = ScheduleEvent::incremental(
            version_id.clone(),
            ScheduleEventType::RollCampaignChanged,
            Some(format!(
                "close_roll_campaign: {}#{}",
                machine_code, campaign_no
            )),
            Some(vec![machine_code.clone()]),
            None,
        );
        if let Err(e) = publisher.publish(event) {
            tracing::warn!("发布 RollCampaignChanged 事件失败: {}", e);
        }
    }

    Ok("{}".to_string())
}

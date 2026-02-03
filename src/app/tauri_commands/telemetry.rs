use crate::app::state::AppState;
use crate::domain::action_log::ActionLog;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::common::map_api_error;

// ==========================================
// 前端遥测/错误上报
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportFrontendEventResponse {
    pub success: bool,
    pub message: String,
}

/// 前端日志/错误上报：写入 action_log（便于复用现有查询界面）
///
/// 约定：
/// - action_type: FRONTEND_ERROR / FRONTEND_WARN / FRONTEND_INFO / FRONTEND_DEBUG / FRONTEND_EVENT
/// - payload_json: 由前端组织，后端仅做落库
#[tauri::command(rename_all = "snake_case")]
pub async fn report_frontend_event(
    state: tauri::State<'_, AppState>,
    version_id: Option<String>,
    actor: Option<String>,
    level: String,
    message: String,
    payload_json: serde_json::Value,
) -> Result<String, String> {
    // 尽量关联到一个“可追溯”的版本：
    // - 优先使用前端传入的 version_id（通常是当前激活版本）
    // - 否则尝试回填“最近激活版本”
    let mut resolved_version_id = version_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if resolved_version_id.is_none() {
        resolved_version_id = state
            .plan_api
            .get_latest_active_version_id()
            .map_err(map_api_error)?
            .filter(|s| !s.trim().is_empty());
    }

    // 若仍无法关联版本，则跳过写入（best-effort，不影响前端流程）
    let Some(version_id) = resolved_version_id else {
        let resp = ReportFrontendEventResponse {
            success: true,
            message: "未找到可关联的激活版本，已跳过写入".to_string(),
        };
        return serde_json::to_string(&resp).map_err(|e| format!("序列化失败: {}", e));
    };

    let actor = actor
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string();

    let level_norm = level.trim().to_lowercase();
    let action_type = match level_norm.as_str() {
        "error" => "FRONTEND_ERROR",
        "warn" | "warning" => "FRONTEND_WARN",
        "info" => "FRONTEND_INFO",
        "debug" => "FRONTEND_DEBUG",
        _ => "FRONTEND_EVENT",
    };

    let log = ActionLog {
        action_id: Uuid::new_v4().to_string(),
        version_id: Some(version_id),
        action_type: action_type.to_string(),
        action_ts: chrono::Local::now().naive_local(),
        actor,
        payload_json: Some(serde_json::json!({
            "level": level_norm,
            "message": message,
            "payload": payload_json,
        })),
        impact_summary_json: None,
        machine_code: None,
        date_range_start: None,
        date_range_end: None,
        detail: Some(message),
    };

    // best-effort: 上报失败不应影响前端流程
    if let Err(e) = state.action_log_repo.insert(&log) {
        tracing::warn!("report_frontend_event insert failed: {}", e);
    }

    let resp = ReportFrontendEventResponse {
        success: true,
        message: "OK".to_string(),
    };
    serde_json::to_string(&resp).map_err(|e| format!("序列化失败: {}", e))
}

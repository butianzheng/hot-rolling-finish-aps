use crate::api::error::ApiError;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tauri::Manager;

// ==========================================
// 公共工具：错误映射、日期解析、事件发送
// ==========================================

/// 错误响应（返回给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ErrorResponse {
    /// 错误代码
    pub code: String,

    /// 错误消息
    pub message: String,

    /// 详细信息（可选）
    pub details: Option<serde_json::Value>,
}

/// 将ApiError转换为JSON字符串（Tauri要求）
pub(super) fn map_api_error(err: ApiError) -> String {
    let error_response = ErrorResponse {
        code: match &err {
            ApiError::FrozenZoneProtection(_) => "FROZEN_ZONE_PROTECTION",
            ApiError::MaturityConstraintViolation { .. } => "MATURITY_CONSTRAINT_VIOLATION",
            ApiError::CapacityConstraintViolation { .. } => "CAPACITY_CONSTRAINT_VIOLATION",
            ApiError::RedLineViolation(_) => "RED_LINE_VIOLATION",
            ApiError::InvalidInput(_) => "INVALID_INPUT",
            ApiError::NotFound(_) => "NOT_FOUND",
            ApiError::BusinessRuleViolation(_) => "BUSINESS_RULE_VIOLATION",
            ApiError::InvalidStateTransition { .. } => "INVALID_STATE_TRANSITION",
            ApiError::OptimisticLockFailure(_) => "OPTIMISTIC_LOCK_FAILURE",
            ApiError::VersionConflict(_) => "VERSION_CONFLICT",
            ApiError::StalePlanRevision { .. } => "STALE_PLAN_REV",
            ApiError::DatabaseError(_) => "DATABASE_ERROR",
            ApiError::DatabaseConnectionError(_) => "DATABASE_CONNECTION_ERROR",
            ApiError::DatabaseTransactionError(_) => "DATABASE_TRANSACTION_ERROR",
            ApiError::ImportError(_) => "IMPORT_ERROR",
            ApiError::ValidationError(_) => "VALIDATION_ERROR",
            ApiError::ManualOperationValidationError { .. } => "MANUAL_OPERATION_VALIDATION_ERROR",
            ApiError::InternalError(_) => "INTERNAL_ERROR",
            ApiError::Other(_) => "OTHER_ERROR",
        }
        .to_string(),
        message: err.to_string(),
        details: match &err {
            ApiError::ManualOperationValidationError { violations, .. } => {
                Some(serde_json::json!({ "violations": violations }))
            }
            ApiError::StalePlanRevision {
                version_id,
                expected_plan_rev,
                actual_plan_rev,
            } => Some(serde_json::json!({
                "version_id": version_id,
                "expected_plan_rev": expected_plan_rev,
                "actual_plan_rev": actual_plan_rev,
            })),
            _ => None,
        },
    };

    serde_json::to_string(&error_response).unwrap_or_else(|_| err.to_string())
}

/// 解析日期字符串
pub(super) fn parse_date(date_str: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|e| format!("日期格式错误（应为YYYY-MM-DD）: {}", e))
}

/// best-effort: emit a frontend event; do not fail the command if emitting fails.
pub(super) fn emit_frontend_event(app: &tauri::AppHandle, event: &str, payload: serde_json::Value) {
    if let Err(e) = app.emit_all(event, payload) {
        tracing::warn!("emit_all failed: event={}, error={}", event, e);
    }
}

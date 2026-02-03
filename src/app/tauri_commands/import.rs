use crate::app::state::AppState;
use crate::engine::{ScheduleEvent, ScheduleEventType};

use super::common::{emit_frontend_event, map_api_error};

// ==========================================
// 材料导入相关命令
// ==========================================

/// 导入材料数据
#[tauri::command(rename_all = "snake_case")]
pub async fn import_materials(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    file_path: String,
    source_batch_id: String,
    mapping_profile_id: Option<String>,
) -> Result<String, String> {
    // 调试日志
    tracing::info!("[import_materials] 收到请求:");
    tracing::info!("  file_path: {}", file_path);
    tracing::info!("  source_batch_id: {}", source_batch_id);
    tracing::info!("  mapping_profile_id: {:?}", mapping_profile_id);

    let result = state
        .import_api
        .import_materials(
            &file_path,
            &source_batch_id,
            mapping_profile_id.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!("[import_materials] 导入失败: {:?}", e);
            map_api_error(e)
        })?;

    tracing::info!("[import_materials] 导入成功: {:?}", result);

    // 发布 ScheduleEvent 触发决策读模型刷新
    if let Some(ref publisher) = state.event_publisher {
        if let Ok(Some(version_id)) = state.plan_api.get_latest_active_version_id() {
            let event = ScheduleEvent::full_scope(
                version_id,
                ScheduleEventType::MaterialStateChanged,
                Some("import_materials".to_string()),
            );
            if let Err(e) = publisher.publish(event) {
                tracing::warn!("发布 MaterialStateChanged 事件失败: {}", e);
            }
        }
    }

    emit_frontend_event(
        &app,
        "material_state_changed",
        serde_json::json!({ "source_batch_id": source_batch_id }),
    );
    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 列出导入冲突
#[tauri::command(rename_all = "snake_case")]
pub async fn list_import_conflicts(
    state: tauri::State<'_, AppState>,
    status: Option<String>,
    limit: i32,
    offset: i32,
    batch_id: Option<String>,
) -> Result<String, String> {
    let result = state
        .import_api
        .list_import_conflicts(status.as_deref(), limit, offset, batch_id.as_deref())
        .await
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 解决导入冲突
///
/// # 参数
/// - conflict_id: 冲突记录ID
/// - action: 解决动作 (KEEP_EXISTING/OVERWRITE/MERGE)
/// - note: 解决备注 (可选)
/// - operator: 操作人 (可选，默认 "system")
///
/// # 返回
/// - 成功: 空JSON对象
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn resolve_import_conflict(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    conflict_id: String,
    action: String,
    note: Option<String>,
    operator: Option<String>,
) -> Result<String, String> {
    use crate::domain::action_log::ActionLog;

    let operator = operator.unwrap_or_else(|| "system".to_string());

    // 解决冲突并获取冲突详情
    let conflict = state
        .import_api
        .resolve_import_conflict(&conflict_id, &action, note.as_deref())
        .await
        .map_err(map_api_error)?;

    // 记录 ActionLog（红线5：可解释性/审计追踪）
    let action_log = ActionLog {
        action_id: uuid::Uuid::new_v4().to_string(),
        version_id: None,
        action_type: "RESOLVE_IMPORT_CONFLICT".to_string(),
        action_ts: chrono::Local::now().naive_local(),
        actor: operator,
        payload_json: Some(serde_json::json!({
            "conflict_id": conflict_id,
            "action": action,
            "note": note,
        })),
        impact_summary_json: Some(serde_json::json!({
            "material_id": conflict.material_id,
            "conflict_type": format!("{:?}", conflict.conflict_type),
            "batch_id": conflict.batch_id,
            "resolution_action": action,
        })),
        machine_code: None,
        date_range_start: None,
        date_range_end: None,
        detail: note,
    };

    // 尝试记录 ActionLog，失败时只记录警告
    if let Err(e) = state.action_log_repo.insert(&action_log) {
        tracing::warn!(error = %e, "记录冲突解决操作日志失败");
    }

    emit_frontend_event(&app, "material_state_changed", serde_json::json!({}));

    Ok("{}".to_string())
}

/// 批量处理导入冲突
///
/// # 参数
/// - conflict_ids: 冲突ID列表
/// - action: 处理动作 (KEEP_EXISTING, OVERWRITE, MERGE)
/// - note: 处理备注（可选）
/// - operator: 操作人（可选，默认为 "system"）
///
/// # 返回
/// - 成功: BatchResolveConflictsResponse JSON
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn batch_resolve_import_conflicts(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    conflict_ids: Vec<String>,
    action: String,
    note: Option<String>,
    operator: Option<String>,
) -> Result<String, String> {
    use crate::domain::action_log::ActionLog;

    let operator = operator.unwrap_or_else(|| "system".to_string());

    // 批量处理冲突
    let result = state
        .import_api
        .batch_resolve_import_conflicts(&conflict_ids, &action, note.as_deref(), &operator)
        .await
        .map_err(map_api_error)?;

    // 记录 ActionLog（红线5：可解释性/审计追踪）
    let action_log = ActionLog {
        action_id: uuid::Uuid::new_v4().to_string(),
        version_id: None,
        action_type: "BATCH_RESOLVE_IMPORT_CONFLICT".to_string(),
        action_ts: chrono::Local::now().naive_local(),
        actor: operator.clone(),
        payload_json: Some(serde_json::json!({
            "conflict_count": conflict_ids.len(),
            "action": action,
            "conflict_ids": conflict_ids,
        })),
        impact_summary_json: Some(serde_json::json!({
            "success_count": result.success_count,
            "fail_count": result.fail_count,
            "all_resolved": result.all_resolved,
        })),
        machine_code: None,
        date_range_start: None,
        date_range_end: None,
        detail: note,
    };

    // 尝试记录 ActionLog，失败时只记录警告
    if let Err(e) = state.action_log_repo.insert(&action_log) {
        tracing::warn!(error = %e, "记录批量冲突解决操作日志失败");
    }

    emit_frontend_event(&app, "material_state_changed", serde_json::json!({}));

    // 返回结果JSON
    let response_json = serde_json::to_string(&result)
        .map_err(|e| format!("序列化批量处理结果失败: {}", e))?;

    Ok(response_json)
}

/// 取消导入批次
///
/// # 参数
/// - batch_id: 批次ID
/// - operator: 操作人（可选，默认为 "system"）
///
/// # 返回
/// - 成功: CancelImportBatchResponse JSON
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn cancel_import_batch(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    batch_id: String,
    operator: Option<String>,
) -> Result<String, String> {
    use crate::domain::action_log::ActionLog;

    let operator = operator.unwrap_or_else(|| "system".to_string());

    // 取消导入批次
    let result = state
        .import_api
        .cancel_import_batch(&batch_id)
        .await
        .map_err(map_api_error)?;

    // 记录 ActionLog（红线5：可解释性/审计追踪）
    let action_log = ActionLog {
        action_id: uuid::Uuid::new_v4().to_string(),
        version_id: None,
        action_type: "CANCEL_IMPORT_BATCH".to_string(),
        action_ts: chrono::Local::now().naive_local(),
        actor: operator.clone(),
        payload_json: Some(serde_json::json!({
            "batch_id": batch_id,
        })),
        impact_summary_json: Some(serde_json::json!({
            "deleted_conflicts": result.deleted_conflicts,
            "deleted_materials": result.deleted_materials,
        })),
        machine_code: None,
        date_range_start: None,
        date_range_end: None,
        detail: Some(format!("取消导入批次: {}", batch_id)),
    };

    // 尝试记录 ActionLog，失败时只记录警告
    if let Err(e) = state.action_log_repo.insert(&action_log) {
        tracing::warn!(error = %e, "记录取消导入批次操作日志失败");
    }

    emit_frontend_event(&app, "material_state_changed", serde_json::json!({}));

    // 返回结果JSON
    let response_json = serde_json::to_string(&result)
        .map_err(|e| format!("序列化取消导入结果失败: {}", e))?;

    Ok(response_json)
}


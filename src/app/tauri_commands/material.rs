use crate::app::state::AppState;
use crate::engine::{ScheduleEvent, ScheduleEventType};

use super::common::{emit_frontend_event, map_api_error};

// ==========================================
// 材料相关命令
// ==========================================

/// 查询材料列表
#[tauri::command(rename_all = "snake_case")]
pub async fn list_materials(
    state: tauri::State<'_, AppState>,
    machine_code: Option<String>,
    steel_grade: Option<String>,
    sched_state: Option<String>,
    urgent_level: Option<String>,
    lock_status: Option<String>,
    query_text: Option<String>,
    limit: i32,
    offset: i32,
) -> Result<String, String> {
    let material_api = state.material_api.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _perf = crate::perf::PerfGuard::new("ipc.list_materials");
        material_api.list_materials(
            machine_code,
            steel_grade,
            sched_state,
            urgent_level,
            lock_status,
            query_text,
            limit,
            offset,
        )
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
    .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 物料池树形汇总（机组 × 状态）
#[tauri::command(rename_all = "snake_case")]
pub async fn get_material_pool_summary(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let material_api = state.material_api.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _perf = crate::perf::PerfGuard::new("ipc.get_material_pool_summary");
        material_api.get_material_pool_summary()
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
    .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询材料详情
#[tauri::command(rename_all = "snake_case")]
pub async fn get_material_detail(
    state: tauri::State<'_, AppState>,
    material_id: String,
) -> Result<String, String> {
    let result = state
        .material_api
        .get_material_detail(&material_id)
        .map_err(map_api_error)?;

    // 将 Option<(master, state)> 元组转换为 {master, state} 对象，
    // 以匹配前端 MaterialDetailResponseSchema 期望的对象结构
    let response = match result {
        Some((master, mat_state)) => serde_json::json!({
            "master": master,
            "state": mat_state,
        }),
        None => serde_json::json!(null),
    };

    serde_json::to_string(&response).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询适温待排材料
#[tauri::command(rename_all = "snake_case")]
pub async fn list_ready_materials(
    state: tauri::State<'_, AppState>,
    machine_code: Option<String>,
) -> Result<String, String> {
    let result = state
        .material_api
        .list_ready_materials(machine_code)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 批量锁定/解锁材料
#[tauri::command(rename_all = "snake_case")]
pub async fn batch_lock_materials(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    material_ids: Vec<String>,
    lock_flag: bool,
    operator: String,
    reason: String,
    mode: Option<String>,
) -> Result<String, String> {
    use crate::api::ValidationMode;

    let material_count = material_ids.len();

    // 解析校验模式，默认为Strict
    let validation_mode = match mode.as_deref() {
        Some("AutoFix") => ValidationMode::AutoFix,
        _ => ValidationMode::Strict,
    };

    let result = state
        .material_api
        .batch_lock_materials(material_ids, lock_flag, &operator, &reason, validation_mode)
        .map_err(map_api_error)?;

    // 发布 ScheduleEvent 触发决策读模型刷新
    if let Some(ref publisher) = state.event_publisher {
        if let Ok(Some(version_id)) = state.plan_api.get_latest_active_version_id() {
            let event = ScheduleEvent::full_scope(
                version_id,
                ScheduleEventType::MaterialStateChanged,
                Some("batch_lock_materials".to_string()),
            );
            if let Err(e) = publisher.publish(event) {
                tracing::warn!("发布 MaterialStateChanged 事件失败: {}", e);
            }
        }
    }

    emit_frontend_event(
        &app,
        "material_state_changed",
        serde_json::json!({ "count": material_count, "lock_flag": lock_flag }),
    );

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 批量强制放行材料
#[tauri::command(rename_all = "snake_case")]
pub async fn batch_force_release(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    material_ids: Vec<String>,
    operator: String,
    reason: String,
    mode: Option<String>,
) -> Result<String, String> {
    use crate::api::ValidationMode;

    let material_count = material_ids.len();

    // 解析校验模式，默认为Strict
    let validation_mode = match mode.as_deref() {
        Some("AutoFix") => ValidationMode::AutoFix,
        _ => ValidationMode::Strict,
    };

    let result = state
        .material_api
        .batch_force_release(material_ids, &operator, &reason, validation_mode)
        .map_err(map_api_error)?;

    // 发布 ScheduleEvent 触发决策读模型刷新
    if let Some(ref publisher) = state.event_publisher {
        if let Ok(Some(version_id)) = state.plan_api.get_latest_active_version_id() {
            let event = ScheduleEvent::full_scope(
                version_id,
                ScheduleEventType::MaterialStateChanged,
                Some("batch_force_release".to_string()),
            );
            if let Err(e) = publisher.publish(event) {
                tracing::warn!("发布 MaterialStateChanged 事件失败: {}", e);
            }
        }
    }

    emit_frontend_event(&app, "material_state_changed", serde_json::json!({ "count": material_count }));

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 批量设置紧急标志
#[tauri::command(rename_all = "snake_case")]
pub async fn batch_set_urgent(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    material_ids: Vec<String>,
    manual_urgent_flag: bool,
    operator: String,
    reason: String,
) -> Result<String, String> {
    let material_count = material_ids.len();

    let result = state
        .material_api
        .batch_set_urgent(material_ids, manual_urgent_flag, &operator, &reason)
        .map_err(map_api_error)?;

    // 发布 ScheduleEvent 触发决策读模型刷新
    if let Some(ref publisher) = state.event_publisher {
        if let Ok(Some(version_id)) = state.plan_api.get_latest_active_version_id() {
            let event = ScheduleEvent::full_scope(
                version_id,
                ScheduleEventType::MaterialStateChanged,
                Some("batch_set_urgent".to_string()),
            );
            if let Err(e) = publisher.publish(event) {
                tracing::warn!("发布 MaterialStateChanged 事件失败: {}", e);
            }
        }
    }

    emit_frontend_event(
        &app,
        "material_state_changed",
        serde_json::json!({ "count": material_count, "manual_urgent_flag": manual_urgent_flag }),
    );

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 按紧急等级查询材料
#[tauri::command(rename_all = "snake_case")]
pub async fn list_materials_by_urgent_level(
    state: tauri::State<'_, AppState>,
    urgent_level: String,
    machine_code: Option<String>,
) -> Result<String, String> {
    use crate::domain::types::UrgentLevel;

    let level = match urgent_level.as_str() {
        "L0" => UrgentLevel::L0,
        "L1" => UrgentLevel::L1,
        "L2" => UrgentLevel::L2,
        "L3" => UrgentLevel::L3,
        _ => return Err(format!("无效的紧急等级: {}", urgent_level)),
    };

    let result = state
        .material_api
        .list_materials_by_urgent_level(level, machine_code)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

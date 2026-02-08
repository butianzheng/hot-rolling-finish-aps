use crate::app::state::AppState;
use crate::engine::{ScheduleEvent, ScheduleEventType};

use super::common::map_api_error;

// ==========================================
// 配置管理相关命令
// ==========================================

/// 查询所有配置
#[tauri::command(rename_all = "snake_case")]
pub async fn list_configs(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let result = state.config_api.list_configs().map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询单个配置
#[tauri::command(rename_all = "snake_case")]
pub async fn get_config(
    state: tauri::State<'_, AppState>,
    scope_id: String,
    key: String,
) -> Result<String, String> {
    let result = state
        .config_api
        .get_config(&scope_id, &key)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 更新配置
#[tauri::command(rename_all = "snake_case")]
pub async fn update_config(
    state: tauri::State<'_, AppState>,
    scope_id: String,
    key: String,
    value: String,
    operator: String,
    reason: String,
) -> Result<String, String> {
    state
        .config_api
        .update_config(&scope_id, &key, &value, &operator, &reason)
        .map_err(map_api_error)?;

    // 发布 ManualTrigger 事件（配置变更可能影响多个决策口径）
    if let Some(ref publisher) = state.event_publisher {
        if let Ok(Some(version_id)) = state.plan_api.get_latest_active_version_id() {
            let event = ScheduleEvent::full_scope(
                version_id,
                ScheduleEventType::ManualTrigger,
                Some(format!("update_config: {}.{}", scope_id, key)),
            );
            if let Err(e) = publisher.publish(event) {
                tracing::warn!("发布 ManualTrigger 事件失败: {}", e);
            }
        }
    }

    Ok("{}".to_string())
}

/// 批量更新配置
#[tauri::command(rename_all = "snake_case")]
pub async fn batch_update_configs(
    state: tauri::State<'_, AppState>,
    configs: String,
    operator: String,
    reason: String,
) -> Result<String, String> {
    use crate::api::config_api::ConfigItem;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct LooseConfigItem {
        scope_id: String,
        #[serde(default)]
        scope_type: String,
        key: String,
        value: String,
        #[serde(default)]
        updated_at: Option<String>,
    }

    let loose_items: Vec<LooseConfigItem> =
        serde_json::from_str(&configs).map_err(|e| format!("解析配置列表失败: {}", e))?;
    let configs: Vec<ConfigItem> = loose_items
        .into_iter()
        .map(|item| ConfigItem {
            scope_id: item.scope_id,
            scope_type: item.scope_type,
            key: item.key,
            value: item.value,
            updated_at: item.updated_at,
        })
        .collect();

    let count = state
        .config_api
        .batch_update_configs(configs, &operator, &reason)
        .map_err(map_api_error)?;

    // 发布 ManualTrigger 事件（批量配置变更可能影响多个决策口径）
    if let Some(ref publisher) = state.event_publisher {
        if let Ok(Some(version_id)) = state.plan_api.get_latest_active_version_id() {
            let event = ScheduleEvent::full_scope(
                version_id,
                ScheduleEventType::ManualTrigger,
                Some("batch_update_configs".to_string()),
            );
            if let Err(e) = publisher.publish(event) {
                tracing::warn!("发布 ManualTrigger 事件失败: {}", e);
            }
        }
    }

    serde_json::to_string(&serde_json::json!({ "updated_count": count }))
        .map_err(|e| format!("序列化失败: {}", e))
}

/// 获取配置快照
#[tauri::command(rename_all = "snake_case")]
pub async fn get_config_snapshot(state: tauri::State<'_, AppState>) -> Result<String, String> {
    state
        .config_api
        .get_config_snapshot()
        .map_err(map_api_error)
}

/// 从快照恢复配置
#[tauri::command(rename_all = "snake_case")]
pub async fn restore_config_from_snapshot(
    state: tauri::State<'_, AppState>,
    snapshot_json: String,
    operator: String,
    reason: String,
) -> Result<String, String> {
    let count = state
        .config_api
        .restore_from_snapshot(&snapshot_json, &operator, &reason)
        .map_err(map_api_error)?;

    serde_json::to_string(&serde_json::json!({ "restored_count": count }))
        .map_err(|e| format!("序列化失败: {}", e))
}

/// 保存自定义策略（持久化到 config_kv）
#[tauri::command(rename_all = "snake_case")]
pub async fn save_custom_strategy(
    state: tauri::State<'_, AppState>,
    strategy_json: String,
    operator: String,
    reason: String,
) -> Result<String, String> {
    use crate::config::strategy_profile::CustomStrategyProfile;

    let profile: CustomStrategyProfile =
        serde_json::from_str(&strategy_json).map_err(|e| format!("解析自定义策略失败: {}", e))?;

    let resp = state
        .config_api
        .save_custom_strategy(profile, &operator, &reason)
        .map_err(map_api_error)?;

    serde_json::to_string(&resp).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询所有自定义策略
#[tauri::command(rename_all = "snake_case")]
pub async fn list_custom_strategies(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let result = state
        .config_api
        .list_custom_strategies()
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

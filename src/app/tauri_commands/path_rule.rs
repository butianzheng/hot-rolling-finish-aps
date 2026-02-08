use crate::app::state::AppState;

use super::common::{map_api_error, parse_date};

// ==========================================
// 宽厚路径规则相关命令（v0.6）
// ==========================================

/// 查询路径规则配置
#[tauri::command(rename_all = "snake_case")]
pub async fn get_path_rule_config(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let result = state
        .path_rule_api
        .get_path_rule_config()
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 更新路径规则配置
#[tauri::command(rename_all = "snake_case")]
pub async fn update_path_rule_config(
    state: tauri::State<'_, AppState>,
    config_json: String,
    operator: String,
    reason: String,
) -> Result<String, String> {
    use crate::api::path_rule_api::PathRuleConfigDto;

    let config: PathRuleConfigDto =
        serde_json::from_str(&config_json).map_err(|e| format!("解析配置失败: {}", e))?;

    state
        .path_rule_api
        .update_path_rule_config(config, &operator, &reason)
        .map_err(map_api_error)?;

    Ok("{}".to_string())
}

/// 查询待人工确认的路径违规材料（OVERRIDE_REQUIRED）
#[tauri::command(rename_all = "snake_case")]
pub async fn list_path_override_pending(
    state: tauri::State<'_, AppState>,
    version_id: String,
    machine_code: String,
    plan_date: String,
) -> Result<String, String> {
    let date = parse_date(&plan_date)?;
    let result = state
        .path_rule_api
        .list_path_override_pending(&version_id, &machine_code, date)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询路径规则待确认汇总（跨日期/跨机组）
#[tauri::command(rename_all = "snake_case")]
pub async fn list_path_override_pending_summary(
    state: tauri::State<'_, AppState>,
    version_id: String,
    plan_date_from: String,
    plan_date_to: String,
    machine_codes: Option<String>, // JSON数组字符串（可选）
) -> Result<String, String> {
    let from = parse_date(&plan_date_from)?;
    let to = parse_date(&plan_date_to)?;

    let machine_codes = if let Some(raw) = machine_codes {
        let list: Vec<String> =
            serde_json::from_str(&raw).map_err(|e| format!("machine_codes 解析失败: {}", e))?;
        Some(list)
    } else {
        None
    };

    let result = state
        .path_rule_api
        .list_path_override_pending_summary(&version_id, from, to, machine_codes.as_deref())
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 单个材料人工确认路径突破
#[tauri::command(rename_all = "snake_case")]
pub async fn confirm_path_override(
    state: tauri::State<'_, AppState>,
    version_id: String,
    material_id: String,
    confirmed_by: String,
    reason: String,
) -> Result<String, String> {
    state
        .path_rule_api
        .confirm_path_override(&version_id, &material_id, &confirmed_by, &reason)
        .map_err(map_api_error)?;

    Ok("{}".to_string())
}

/// 批量人工确认路径突破
#[tauri::command(rename_all = "snake_case")]
pub async fn batch_confirm_path_override(
    state: tauri::State<'_, AppState>,
    version_id: String,
    material_ids: String,
    confirmed_by: String,
    reason: String,
) -> Result<String, String> {
    let ids: Vec<String> =
        serde_json::from_str(&material_ids).map_err(|e| format!("解析材料ID列表失败: {}", e))?;

    let result = state
        .path_rule_api
        .batch_confirm_path_override(&version_id, &ids, &confirmed_by, &reason)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 按日期范围/机组范围批量人工确认路径突破
#[tauri::command(rename_all = "snake_case")]
pub async fn batch_confirm_path_override_by_range(
    state: tauri::State<'_, AppState>,
    version_id: String,
    plan_date_from: String,
    plan_date_to: String,
    machine_codes: Option<String>, // JSON数组字符串（可选）
    confirmed_by: String,
    reason: String,
) -> Result<String, String> {
    let from = parse_date(&plan_date_from)?;
    let to = parse_date(&plan_date_to)?;

    let machine_codes = if let Some(raw) = machine_codes {
        let list: Vec<String> =
            serde_json::from_str(&raw).map_err(|e| format!("machine_codes 解析失败: {}", e))?;
        Some(list)
    } else {
        None
    };

    let result = state
        .path_rule_api
        .batch_confirm_path_override_by_range(
            &version_id,
            from,
            to,
            machine_codes.as_deref(),
            &confirmed_by,
            &reason,
        )
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 单个材料人工拒绝路径突破
#[tauri::command(rename_all = "snake_case")]
pub async fn reject_path_override(
    state: tauri::State<'_, AppState>,
    version_id: String,
    material_id: String,
    rejected_by: String,
    reason: String,
) -> Result<String, String> {
    state
        .path_rule_api
        .reject_path_override(&version_id, &material_id, &rejected_by, &reason)
        .map_err(map_api_error)?;

    Ok("{}".to_string())
}

/// 批量人工拒绝路径突破
#[tauri::command(rename_all = "snake_case")]
pub async fn batch_reject_path_override(
    state: tauri::State<'_, AppState>,
    version_id: String,
    material_ids: String,
    rejected_by: String,
    reason: String,
) -> Result<String, String> {
    let ids: Vec<String> =
        serde_json::from_str(&material_ids).map_err(|e| format!("解析材料ID列表失败: {}", e))?;

    let result = state
        .path_rule_api
        .batch_reject_path_override(&version_id, &ids, &rejected_by, &reason)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 按日期范围/机组范围批量人工拒绝路径突破
#[tauri::command(rename_all = "snake_case")]
pub async fn batch_reject_path_override_by_range(
    state: tauri::State<'_, AppState>,
    version_id: String,
    plan_date_from: String,
    plan_date_to: String,
    machine_codes: Option<String>, // JSON数组字符串（可选）
    rejected_by: String,
    reason: String,
) -> Result<String, String> {
    let from = parse_date(&plan_date_from)?;
    let to = parse_date(&plan_date_to)?;

    let machine_codes = if let Some(raw) = machine_codes {
        let list: Vec<String> =
            serde_json::from_str(&raw).map_err(|e| format!("machine_codes 解析失败: {}", e))?;
        Some(list)
    } else {
        None
    };

    let result = state
        .path_rule_api
        .batch_reject_path_override_by_range(
            &version_id,
            from,
            to,
            machine_codes.as_deref(),
            &rejected_by,
            &reason,
        )
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 查询当前活跃换辊周期的路径锚点
#[tauri::command(rename_all = "snake_case")]
pub async fn get_roll_cycle_anchor(
    state: tauri::State<'_, AppState>,
    version_id: String,
    machine_code: String,
) -> Result<String, String> {
    let result = state
        .path_rule_api
        .get_roll_cycle_anchor(&version_id, &machine_code)
        .map_err(map_api_error)?;

    serde_json::to_string(&result).map_err(|e| format!("序列化失败: {}", e))
}

/// 手动重置换辊周期（锚点/累计状态清零，campaign_no+1）
#[tauri::command(rename_all = "snake_case")]
pub async fn reset_roll_cycle(
    state: tauri::State<'_, AppState>,
    version_id: String,
    machine_code: String,
    actor: String,
    reason: String,
) -> Result<String, String> {
    state
        .path_rule_api
        .reset_roll_cycle(&version_id, &machine_code, &actor, &reason)
        .map_err(map_api_error)?;

    Ok("{}".to_string())
}

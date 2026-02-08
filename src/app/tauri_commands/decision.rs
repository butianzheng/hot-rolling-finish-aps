use crate::app::state::AppState;
use crate::api::error::ApiError;
use super::common::map_api_error;

fn validate_expected_plan_rev(
    state: &tauri::State<'_, AppState>,
    version_id: &str,
    expected_plan_rev: Option<i32>,
) -> Result<(), String> {
    let expected = match expected_plan_rev {
        Some(v) => v,
        None => return Ok(()),
    };

    let version = state
        .plan_api
        .get_version_detail(version_id)
        .map_err(map_api_error)?;

    let actual = version.revision;
    if actual != expected {
        return Err(map_api_error(ApiError::StalePlanRevision {
            version_id: version_id.to_string(),
            expected_plan_rev: expected,
            actual_plan_rev: actual,
        }));
    }

    Ok(())
}

// ==========================================
// 决策支持相关命令
// ==========================================

/// D1: 查询日期风险摘要 - "哪天最危险"
///
/// # 参数
/// - version_id: 方案版本ID
/// - date_from: 日期范围起始 (YYYY-MM-DD)
/// - date_to: 日期范围结束 (YYYY-MM-DD)
/// - risk_level_filter: 风险等级过滤 (可选, JSON数组字符串, 如: ["HIGH", "CRITICAL"])
/// - limit: 返回条数限制 (可选, 默认10)
/// - sort_by: 排序方式 (可选, "risk_score" | "plan_date" | "capacity_util_pct")
///
/// # 返回
/// - 成功: JSON字符串, 包含日期风险摘要列表
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn get_decision_day_summary(
    state: tauri::State<'_, AppState>,
    version_id: String,
    expected_plan_rev: Option<i32>,
    date_from: String,
    date_to: String,
    risk_level_filter: Option<String>,
    limit: Option<u32>,
    sort_by: Option<String>,
) -> Result<String, String> {
    use crate::decision::api::{DecisionApi, GetDecisionDaySummaryRequest};

    validate_expected_plan_rev(&state, &version_id, expected_plan_rev)?;

    // 解析风险等级过滤器
    let risk_level_filter = if let Some(filter_str) = risk_level_filter {
        let parsed: Vec<String> = serde_json::from_str(&filter_str)
            .map_err(|e| format!("风险等级过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 构建请求
    let request = GetDecisionDaySummaryRequest {
        version_id,
        date_from,
        date_to,
        risk_level_filter,
        limit,
        sort_by,
    };

    // 调用 DecisionApi
    let response = state.decision_api.get_decision_day_summary(request)?;

    // 序列化返回
    serde_json::to_string(&response).map_err(|e| format!("序列化失败: {}", e))
}

/// D4: 查询机组堵塞概况 - "哪个机组最堵"
///
/// # 参数
/// - version_id: 方案版本ID
/// - date_from: 日期范围起始 (YYYY-MM-DD)
/// - date_to: 日期范围结束 (YYYY-MM-DD)
/// - machine_codes: 机组代码过滤 (可选, JSON数组字符串, 如: ["H032", "H033"])
/// - bottleneck_level_filter: 堵塞等级过滤 (可选, JSON数组字符串, 如: ["HIGH", "CRITICAL"])
/// - bottleneck_type_filter: 堵塞类型过滤 (可选, JSON数组字符串, 如: ["Capacity", "Structure"])
/// - limit: 返回条数限制 (可选, 默认50)
///
/// # 返回
/// - 成功: JSON字符串, 包含机组堵塞点列表和热力图统计
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn get_machine_bottleneck_profile(
    state: tauri::State<'_, AppState>,
    version_id: String,
    expected_plan_rev: Option<i32>,
    date_from: String,
    date_to: String,
    machine_codes: Option<String>,
    bottleneck_level_filter: Option<String>,
    bottleneck_type_filter: Option<String>,
    limit: Option<u32>,
) -> Result<String, String> {
    use crate::decision::api::{DecisionApi, GetMachineBottleneckProfileRequest};

    validate_expected_plan_rev(&state, &version_id, expected_plan_rev)?;

    // 解析机组代码过滤器
    let machine_codes = if let Some(codes_str) = machine_codes {
        let parsed: Vec<String> = serde_json::from_str(&codes_str)
            .map_err(|e| format!("机组代码过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 解析堵塞等级过滤器
    let bottleneck_level_filter = if let Some(filter_str) = bottleneck_level_filter {
        let parsed: Vec<String> = serde_json::from_str(&filter_str)
            .map_err(|e| format!("堵塞等级过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 解析堵塞类型过滤器
    let bottleneck_type_filter = if let Some(filter_str) = bottleneck_type_filter {
        let parsed: Vec<String> = serde_json::from_str(&filter_str)
            .map_err(|e| format!("堵塞类型过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 构建请求
    let request = GetMachineBottleneckProfileRequest {
        version_id,
        date_from,
        date_to,
        machine_codes,
        bottleneck_level_filter,
        bottleneck_type_filter,
        limit,
    };

    // 调用 DecisionApi
    let response = state.decision_api.get_machine_bottleneck_profile(request)?;

    // 序列化返回
    serde_json::to_string(&response).map_err(|e| format!("序列化失败: {}", e))
}

/// D2: 查询紧急订单失败集合 - "哪些紧急单无法完成"
///
/// # 参数
/// - version_id: 方案版本ID
/// - fail_type_filter: 失败类型过滤 (可选, JSON数组字符串, 如: ["Overdue", "CapacityShortage"])
/// - urgency_level_filter: 紧急等级过滤 (可选, JSON数组字符串, 如: ["L2", "L3"])
/// - machine_codes: 机组代码过滤 (可选, JSON数组字符串, 如: ["H032", "H033"])
/// - due_date_from: 交货日期范围起始 (可选, YYYY-MM-DD)
/// - due_date_to: 交货日期范围结束 (可选, YYYY-MM-DD)
/// - completion_rate_threshold: 完成率阈值 (可选, 0-100)
/// - limit: 返回条数限制 (可选, 默认50)
/// - offset: 分页偏移 (可选, 默认0)
///
/// # 返回
/// - 成功: JSON字符串, 包含订单失败列表和统计摘要
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn list_order_failure_set(
    state: tauri::State<'_, AppState>,
    version_id: String,
    expected_plan_rev: Option<i32>,
    fail_type_filter: Option<String>,
    urgency_level_filter: Option<String>,
    machine_codes: Option<String>,
    due_date_from: Option<String>,
    due_date_to: Option<String>,
    completion_rate_threshold: Option<f64>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<String, String> {
    use crate::decision::api::{DecisionApi, ListOrderFailureSetRequest};

    validate_expected_plan_rev(&state, &version_id, expected_plan_rev)?;

    // 解析失败类型过滤器
    let fail_type_filter = if let Some(filter_str) = fail_type_filter {
        let parsed: Vec<String> = serde_json::from_str(&filter_str)
            .map_err(|e| format!("失败类型过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 解析紧急等级过滤器
    let urgency_level_filter = if let Some(filter_str) = urgency_level_filter {
        let parsed: Vec<String> = serde_json::from_str(&filter_str)
            .map_err(|e| format!("紧急等级过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 解析机组代码过滤器
    let machine_codes = if let Some(codes_str) = machine_codes {
        let parsed: Vec<String> = serde_json::from_str(&codes_str)
            .map_err(|e| format!("机组代码过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 构建请求
    let request = ListOrderFailureSetRequest {
        version_id,
        fail_type_filter,
        urgency_level_filter,
        machine_codes,
        due_date_from,
        due_date_to,
        completion_rate_threshold,
        limit,
        offset,
    };

    // 调用 DecisionApi
    let response = state.decision_api.list_order_failure_set(request)?;

    // 序列化返回
    serde_json::to_string(&response).map_err(|e| format!("序列化失败: {}", e))
}

/// D3: 查询冷料压库概况 - "哪些冷料压库"
///
/// # 参数
/// - version_id: 方案版本ID
/// - machine_codes: 机组代码过滤 (可选, JSON数组字符串, 如: ["H032", "H033"])
/// - pressure_level_filter: 压库等级过滤 (可选, JSON数组字符串, 如: ["HIGH", "CRITICAL"])
/// - age_bin_filter: 库龄分组过滤 (可选, JSON数组字符串, 如: ["30-60", "60-90", "90+"])
/// - limit: 返回条数限制 (可选, 默认50)
///
/// # 返回
/// - 成功: JSON字符串, 包含冷料压库概况列表和统计摘要
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn get_cold_stock_profile(
    state: tauri::State<'_, AppState>,
    version_id: String,
    expected_plan_rev: Option<i32>,
    machine_codes: Option<String>,
    pressure_level_filter: Option<String>,
    age_bin_filter: Option<String>,
    limit: Option<u32>,
) -> Result<String, String> {
    use crate::decision::api::{DecisionApi, GetColdStockProfileRequest};

    validate_expected_plan_rev(&state, &version_id, expected_plan_rev)?;

    // 解析机组代码过滤器
    let machine_codes = if let Some(codes_str) = machine_codes {
        let parsed: Vec<String> = serde_json::from_str(&codes_str)
            .map_err(|e| format!("机组代码过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 解析压库等级过滤器
    let pressure_level_filter = if let Some(filter_str) = pressure_level_filter {
        let parsed: Vec<String> = serde_json::from_str(&filter_str)
            .map_err(|e| format!("压库等级过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 解析库龄分组过滤器
    let age_bin_filter = if let Some(filter_str) = age_bin_filter {
        let parsed: Vec<String> = serde_json::from_str(&filter_str)
            .map_err(|e| format!("库龄分组过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 构建请求
    let request = GetColdStockProfileRequest {
        version_id,
        machine_codes,
        pressure_level_filter,
        age_bin_filter,
        limit,
    };

    // 调用 DecisionApi
    let response = state.decision_api.get_cold_stock_profile(request)?;

    // 序列化返回
    serde_json::to_string(&response).map_err(|e| format!("序列化失败: {}", e))
}

/// D5: 查询轧制活动警报 - "换辊是否异常"
///
/// # 参数
/// - version_id: 方案版本ID
/// - machine_codes: 机组代码过滤 (可选, JSON数组字符串, 如: ["H032", "H033"])
/// - alert_level_filter: 警报等级过滤 (可选, JSON数组字符串, 如: ["WARNING", "CRITICAL"])
/// - alert_type_filter: 警报类型过滤 (可选, JSON数组字符串)
/// - date_from: 日期范围起始 (可选, YYYY-MM-DD)
/// - date_to: 日期范围结束 (可选, YYYY-MM-DD)
/// - limit: 返回条数限制 (可选)
///
/// # 返回
/// - 成功: JSON字符串, 包含换辊警报列表和统计摘要
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn get_roll_campaign_alert(
    state: tauri::State<'_, AppState>,
    version_id: String,
    expected_plan_rev: Option<i32>,
    machine_codes: Option<String>,
    alert_level_filter: Option<String>,
    alert_type_filter: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<u32>,
) -> Result<String, String> {
    use crate::decision::api::{DecisionApi, ListRollCampaignAlertsRequest};

    validate_expected_plan_rev(&state, &version_id, expected_plan_rev)?;

    // 解析机组代码过滤器
    let machine_codes = if let Some(codes_str) = machine_codes {
        let parsed: Vec<String> = serde_json::from_str(&codes_str)
            .map_err(|e| format!("机组代码过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 解析警报等级过滤器
    let alert_level_filter = if let Some(filter_str) = alert_level_filter {
        let parsed: Vec<String> = serde_json::from_str(&filter_str)
            .map_err(|e| format!("警报等级过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 解析警报类型过滤器
    let alert_type_filter = if let Some(filter_str) = alert_type_filter {
        let parsed: Vec<String> = serde_json::from_str(&filter_str)
            .map_err(|e| format!("警报类型过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 构建请求
    let request = ListRollCampaignAlertsRequest {
        version_id,
        machine_codes,
        alert_level_filter,
        alert_type_filter,
        date_from,
        date_to,
        limit,
    };

    // 调用 DecisionApi
    let response = state.decision_api.list_roll_campaign_alerts(request)?;

    // 序列化返回
    serde_json::to_string(&response).map_err(|e| format!("序列化失败: {}", e))
}

/// D6: 查询容量优化机会 - "是否存在产能优化空间"
///
/// # 参数
/// - version_id: 方案版本ID
/// - date_from: 日期范围起始 (可选, YYYY-MM-DD)
/// - date_to: 日期范围结束 (可选, YYYY-MM-DD)
/// - machine_codes: 机组代码过滤 (可选, JSON数组字符串, 如: ["H032", "H033"])
/// - opportunity_type_filter: 机会类型过滤 (可选, JSON数组字符串)
/// - min_opportunity_t: 最小机会吨位阈值 (可选, 默认10.0)
/// - limit: 返回条数限制 (可选)
///
/// # 返回
/// - 成功: JSON字符串, 包含容量优化机会列表和统计摘要
/// - 失败: 错误消息
#[tauri::command(rename_all = "snake_case")]
pub async fn get_capacity_opportunity(
    state: tauri::State<'_, AppState>,
    version_id: String,
    expected_plan_rev: Option<i32>,
    date_from: Option<String>,
    date_to: Option<String>,
    machine_codes: Option<String>,
    opportunity_type_filter: Option<String>,
    min_opportunity_t: Option<f64>,
    limit: Option<u32>,
) -> Result<String, String> {
    use crate::decision::api::{DecisionApi, GetCapacityOpportunityRequest};

    validate_expected_plan_rev(&state, &version_id, expected_plan_rev)?;

    // 解析机组代码过滤器
    let machine_codes = if let Some(codes_str) = machine_codes {
        let parsed: Vec<String> = serde_json::from_str(&codes_str)
            .map_err(|e| format!("机组代码过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 解析机会类型过滤器
    let opportunity_type_filter = if let Some(filter_str) = opportunity_type_filter {
        let parsed: Vec<String> = serde_json::from_str(&filter_str)
            .map_err(|e| format!("机会类型过滤器格式错误: {}", e))?;
        Some(parsed)
    } else {
        None
    };

    // 构建请求
    let request = GetCapacityOpportunityRequest {
        version_id,
        machine_codes,
        date_from,
        date_to,
        opportunity_type_filter,
        min_opportunity_t,
        limit,
    };

    // 调用 DecisionApi
    let response = state.decision_api.get_capacity_opportunity(request)?;

    // 序列化返回
    serde_json::to_string(&response).map_err(|e| format!("序列化失败: {}", e))
}

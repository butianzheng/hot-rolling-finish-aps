// ==========================================
// 热轧精整排产系统 - DecisionApi DTO 定义
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md
// 职责: 定义 DecisionApi 的请求和响应结构
// ==========================================

use serde::{Deserialize, Serialize};

// ==========================================
// D1: get_decision_day_summary - 哪天最危险
// ==========================================

/// D1 请求: 查询日期风险摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDecisionDaySummaryRequest {
    /// 方案版本 ID（必填）
    pub version_id: String,

    /// 日期范围起始（必填，ISO DATE: YYYY-MM-DD）
    pub date_from: String,

    /// 日期范围结束（必填，ISO DATE: YYYY-MM-DD）
    pub date_to: String,

    /// 风险等级过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_level_filter: Option<Vec<String>>, // ["LOW", "MEDIUM", "HIGH", "CRITICAL"]

    /// 返回条数限制（可选，默认 10）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// 排序方式（可选，默认按风险分数降序）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>, // "risk_score" | "plan_date" | "capacity_util_pct"
}

/// D1 响应: 日期风险摘要列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionDaySummaryResponse {
    /// 方案版本 ID
    pub version_id: String,

    /// 查询时间戳 (ISO 8601)
    pub as_of: String,

    /// 日期摘要列表
    pub items: Vec<DaySummaryDto>,

    /// 总记录数
    pub total_count: u32,
}

/// 日期摘要 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySummaryDto {
    /// 计划日期 (YYYY-MM-DD)
    pub plan_date: String,

    /// 风险分数 (0-100)
    pub risk_score: f64,

    /// 风险等级 ("LOW" | "MEDIUM" | "HIGH" | "CRITICAL")
    pub risk_level: String,

    /// 产能利用率 (%)
    pub capacity_util_pct: f64,

    /// 超载吨数 (t)
    pub overload_weight_t: f64,

    /// 紧急单失败数量
    pub urgent_failure_count: u32,

    /// 主要风险原因（按权重降序）
    pub top_reasons: Vec<ReasonItemDto>,

    /// 涉及的机组列表
    pub involved_machines: Vec<String>,
}

/// 原因项 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasonItemDto {
    /// 原因代码
    pub code: String,

    /// 原因描述
    pub msg: String,

    /// 权重 (0-1)
    pub weight: f64,

    /// 影响的材料数量（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affected_count: Option<u32>,
}

// ==========================================
// D4: get_machine_bottleneck_profile - 哪个机组最堵
// ==========================================

/// D4 请求: 查询机组堵塞概况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMachineBottleneckProfileRequest {
    /// 方案版本 ID（必填）
    pub version_id: String,

    /// 日期范围起始（必填，ISO DATE）
    pub date_from: String,

    /// 日期范围结束（必填，ISO DATE）
    pub date_to: String,

    /// 机组代码过滤（可选，为空表示所有机组）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_codes: Option<Vec<String>>,

    /// 堵塞等级过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottleneck_level_filter: Option<Vec<String>>, // ["LOW", "MEDIUM", "HIGH", "CRITICAL"]

    /// 堵塞类型过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottleneck_type_filter: Option<Vec<String>>, // ["Capacity", "Structure", "RollChange", "ColdStock", "Mixed"]

    /// 返回条数限制（可选，默认 50）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// D4 响应: 机组堵塞概况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineBottleneckProfileResponse {
    /// 方案版本 ID
    pub version_id: String,

    /// 查询时间戳
    pub as_of: String,

    /// 堵塞点列表
    pub items: Vec<BottleneckPointDto>,

    /// 总记录数
    pub total_count: u32,

    /// 热力图统计（可选，用于前端渲染）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heatmap_stats: Option<HeatmapStatsDto>,
}

/// 堵塞点 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleneckPointDto {
    /// 机组代码
    pub machine_code: String,

    /// 计划日期 (YYYY-MM-DD)
    pub plan_date: String,

    /// 堵塞分数 (0-100)
    pub bottleneck_score: f64,

    /// 堵塞等级 ("LOW" | "MEDIUM" | "HIGH" | "CRITICAL")
    pub bottleneck_level: String,

    /// 堵塞类型列表
    pub bottleneck_types: Vec<String>,

    /// 产能利用率 (%)
    pub capacity_util_pct: f64,

    /// 待排产材料数量
    pub pending_material_count: u32,

    /// 待排产重量 (t)
    pub pending_weight_t: f64,

    /// 已排产材料数量
    pub scheduled_material_count: u32,

    /// 已排产重量 (t)
    pub scheduled_weight_t: f64,

    /// 堵塞原因（按影响降序）
    pub reasons: Vec<ReasonItemDto>,

    /// 推荐操作（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_actions: Option<Vec<String>>,
}

/// 热力图统计 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapStatsDto {
    /// 平均堵塞分数
    pub avg_score: f64,

    /// 最大堵塞分数
    pub max_score: f64,

    /// 堵塞天数（分数 > 50）
    pub bottleneck_days_count: u32,

    /// 按机组的统计
    pub by_machine: Vec<MachineStatsDto>,
}

/// 机组统计 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineStatsDto {
    pub machine_code: String,
    pub avg_score: f64,
    pub max_score: f64,
    pub bottleneck_days: u32,
}

// ==========================================
// D2: list_order_failure_set - 哪些紧急单无法完成
// ==========================================

/// D2 请求: 查询紧急订单失败集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListOrderFailureSetRequest {
    /// 方案版本 ID（必填）
    pub version_id: String,

    /// 失败类型过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_type_filter: Option<Vec<String>>,

    /// 紧急等级过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urgency_level_filter: Option<Vec<String>>,

    /// 机组代码过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_codes: Option<Vec<String>>,

    /// 交货日期范围起始（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date_from: Option<String>,

    /// 交货日期范围结束（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date_to: Option<String>,

    /// 完成率阈值过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_rate_threshold: Option<f64>,

    /// 分页：限制条数（可选，默认 50）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// 分页：偏移量（可选，默认 0）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
}

/// D2 响应: 订单失败集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFailureSetResponse {
    pub version_id: String,
    pub as_of: String,
    pub items: Vec<OrderFailureDto>,
    pub total_count: u32,
    pub summary: OrderFailureSummaryDto,
}

/// 订单失败 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFailureDto {
    pub contract_no: String,
    pub due_date: String,
    pub days_to_due: i32,
    pub urgency_level: String,
    pub fail_type: String,
    pub completion_rate: f64,
    pub total_weight_t: f64,
    pub scheduled_weight_t: f64,
    pub unscheduled_weight_t: f64,
    pub machine_code: String,
    pub blocking_factors: Vec<BlockingFactorDto>,
    pub failure_reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_actions: Option<Vec<String>>,
}

/// 阻塞因素 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockingFactorDto {
    pub factor_type: String,
    pub description: String,
    pub impact: f64,
    pub affected_material_count: u32,
}

/// 订单失败摘要 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFailureSummaryDto {
    pub total_failures: u32,
    pub by_fail_type: Vec<TypeCountDto>,
    pub by_urgency: Vec<TypeCountDto>,
    pub total_unscheduled_weight_t: f64,
}

// ==========================================
// D3: get_cold_stock_profile - 哪些冷料压库
// ==========================================

/// D3 请求: 查询冷料压库概况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetColdStockProfileRequest {
    pub version_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_codes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pressure_level_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_bin_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// D3 响应: 冷料压库概况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColdStockProfileResponse {
    pub version_id: String,
    pub as_of: String,
    pub items: Vec<ColdStockBucketDto>,
    pub total_count: u32,
    pub summary: ColdStockSummaryDto,
}

/// 冷料分桶 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColdStockBucketDto {
    pub machine_code: String,
    pub age_bin: String,
    pub count: u32,
    pub weight_t: f64,
    pub pressure_score: f64,
    pub pressure_level: String,
    pub avg_age_days: f64,
    pub max_age_days: i32,
    pub structure_gap: String,
    pub reasons: Vec<ReasonItemDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend: Option<ColdStockTrendDto>,
}

/// 冷料趋势 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColdStockTrendDto {
    pub direction: String, // "RISING" | "STABLE" | "FALLING"
    pub change_rate_pct: f64,
    pub baseline_days: i32,
}

/// 冷料摘要 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColdStockSummaryDto {
    pub total_cold_stock_count: u32,
    pub total_cold_stock_weight_t: f64,
    pub avg_age_days: f64,
    pub high_pressure_count: u32,
    pub by_machine: Vec<MachineStockStatsDto>,
    pub by_age_bin: Vec<AgeBinStatsDto>,
}

/// 机组库存统计 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineStockStatsDto {
    pub machine_code: String,
    pub count: u32,
    pub weight_t: f64,
    pub avg_pressure_score: f64,
}

/// 年龄分桶统计 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgeBinStatsDto {
    pub age_bin: String,
    pub count: u32,
    pub weight_t: f64,
}

// ==========================================
// D5: list_roll_campaign_alerts - 换辊是否异常
// ==========================================

/// D5 请求: 查询换辊预警列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRollCampaignAlertsRequest {
    pub version_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_codes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_level_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_type_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// D5 响应: 换辊预警列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollCampaignAlertsResponse {
    pub version_id: String,
    pub as_of: String,
    pub items: Vec<RollAlertDto>,
    pub total_count: u32,
    pub summary: RollAlertSummaryDto,
}

/// 换辊预警 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollAlertDto {
    pub machine_code: String,
    pub campaign_id: String,
    pub campaign_start_date: String,
    pub current_tonnage_t: f64,
    pub soft_limit_t: f64,
    pub hard_limit_t: f64,
    pub remaining_tonnage_t: f64,
    pub alert_level: String,
    pub alert_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_hard_stop_date: Option<String>,
    pub alert_message: String,
    pub impact_description: String,
    pub recommended_actions: Vec<String>,
}

/// 换辊预警摘要 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollAlertSummaryDto {
    pub total_alerts: u32,
    pub by_level: Vec<TypeCountDto>,
    pub by_type: Vec<TypeCountDto>,
    pub near_hard_stop_count: u32,
}

// ==========================================
// D6: get_capacity_opportunity - 是否存在产能优化空间
// ==========================================

/// D6 请求: 查询产能优化机会
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCapacityOpportunityRequest {
    pub version_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_codes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opportunity_type_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_opportunity_t: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// D6 响应: 产能优化机会
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityOpportunityResponse {
    pub version_id: String,
    pub as_of: String,
    pub items: Vec<CapacityOpportunityDto>,
    pub total_count: u32,
    pub summary: CapacityOpportunitySummaryDto,
}

/// 产能优化机会 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityOpportunityDto {
    pub machine_code: String,
    pub plan_date: String,
    pub opportunity_type: String,
    pub current_util_pct: f64,
    pub target_capacity_t: f64,
    pub used_capacity_t: f64,
    pub opportunity_space_t: f64,
    pub optimized_util_pct: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitivity: Option<SensitivityAnalysisDto>,
    pub description: String,
    pub recommended_actions: Vec<String>,
    pub potential_benefits: Vec<String>,
}

/// 敏感性分析 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityAnalysisDto {
    pub scenarios: Vec<ScenarioDto>,
    pub best_scenario_index: usize,
}

/// 场景 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDto {
    pub name: String,
    pub adjustment: String,
    pub util_pct: f64,
    pub risk_score: f64,
    pub affected_material_count: u32,
}

/// 产能优化机会摘要 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityOpportunitySummaryDto {
    pub total_opportunities: u32,
    pub total_opportunity_space_t: f64,
    pub by_type: Vec<TypeCountDto>,
    pub avg_current_util_pct: f64,
    pub avg_optimized_util_pct: f64,
}

// ==========================================
// 通用 DTO
// ==========================================

/// 类型统计 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCountDto {
    pub type_name: String,
    pub count: u32,
    pub weight_t: f64,
}

// ==========================================
// 错误响应 DTO
// ==========================================

/// 错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: Some(details),
        }
    }
}

use serde::{Deserialize, Serialize};

/// 自定义策略（持久化对象）
///
/// 存储位置：config_kv（scope_id='global'，key='custom_strategy/{strategy_id}'）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomStrategyProfile {
    /// 自定义策略 ID（用于选择/引用）
    pub strategy_id: String,

    /// 显示名称（中文）
    pub title: String,

    /// 说明（可选）
    #[serde(default)]
    pub description: Option<String>,

    /// 基于哪个预设策略（balanced/urgent_first/capacity_first/cold_stock_first）
    pub base_strategy: String,

    /// 参数（权重/阈值等，后续可扩展）
    #[serde(default)]
    pub parameters: CustomStrategyParameters,
}

/// 自定义策略参数（轻量版：先覆盖“无需查库”的策略微调维度）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomStrategyParameters {
    /// 紧急优先权重（越大越强调 urgent_level）
    #[serde(default)]
    pub urgent_weight: Option<f64>,

    /// 产能优先权重（越大越强调 weight_t）
    #[serde(default)]
    pub capacity_weight: Option<f64>,

    /// 冷坨优先权重（越大越强调 stock_age_days）
    #[serde(default)]
    pub cold_stock_weight: Option<f64>,

    /// 交期优先权重（越大越强调 due_date）
    #[serde(default)]
    pub due_date_weight: Option<f64>,

    /// 轧制产出时间优先权重（越大越强调 rolling_output_age_days）
    #[serde(default)]
    pub rolling_output_age_weight: Option<f64>,

    /// 冷坨账龄阈值（天）
    #[serde(default)]
    pub cold_stock_age_threshold_days: Option<i32>,

    /// 允许溢出比例（0~1），用于“策略建议/草案筛选”等场景（不直接改产能池硬约束）
    #[serde(default)]
    pub overflow_tolerance_pct: Option<f64>,
}

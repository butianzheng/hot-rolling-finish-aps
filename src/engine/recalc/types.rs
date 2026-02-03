use crate::config::strategy_profile::CustomStrategyParameters;
use crate::domain::plan::PlanItem;
use crate::engine::strategy::ScheduleStrategy;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ==========================================
// RecalcResult - 重算结果
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecalcResult {
    pub version_id: String,    // 新版本ID
    pub version_no: i32,       // 版本号
    pub total_items: usize,    // 总计划项数
    pub mature_count: usize,   // 适温材料数
    pub immature_count: usize, // 未适温材料数
    pub frozen_items: usize,   // 冻结材料数
    pub recalc_items: usize,   // 重算材料数
    pub elapsed_ms: i64,       // 耗时(毫秒)
}

// ==========================================
// RescheduleResult - 重排产结果（内部使用）
// ==========================================
/// 重排产结果
/// 职责: execute_reschedule的返回值，包含排产明细和统计信息
#[derive(Debug, Clone)]
pub struct RescheduleResult {
    /// 排产的计划项
    pub plan_items: Vec<PlanItem>,
    /// 成熟材料数（READY/LOCKED/FORCE_RELEASE）
    pub mature_count: usize,
    /// 未成熟材料数（PENDING_MATURE）
    pub immature_count: usize,
    /// 总已用产能（吨）
    pub total_capacity_used: f64,
    /// 超限天数
    pub overflow_days: usize,
}

// ==========================================
// RecalcConfig - 重算配置
// ==========================================
#[derive(Debug, Clone)]
pub struct RecalcConfig {
    pub default_window_days: i32,      // 默认计算窗口: 30天
    pub default_cascade_days: i32,     // 默认联动窗口: 7天
    pub frozen_days_before_today: i32, // 冻结区天数: 2天
    pub auto_activate: bool,           // 是否自动激活: false
}

impl Default for RecalcConfig {
    fn default() -> Self {
        Self {
            default_window_days: 30,
            default_cascade_days: 7,
            frozen_days_before_today: 2,
            auto_activate: false,
        }
    }
}

/// 解析后的“执行策略”（用于把自定义策略参数下沉到引擎层）
#[derive(Debug, Clone)]
pub struct ResolvedStrategyProfile {
    /// 对外展示/审计使用（例如 balanced / custom:my_strategy_1）
    pub strategy_key: String,
    /// 实际执行使用的基础策略（预设策略枚举）
    pub base_strategy: ScheduleStrategy,
    /// 版本中文命名使用的策略标题
    pub title_cn: String,
    /// 可选：策略参数（自定义策略才有）
    pub parameters: Option<CustomStrategyParameters>,
}

impl ResolvedStrategyProfile {
    pub fn parameters_json(&self) -> JsonValue {
        match &self.parameters {
            Some(p) => serde_json::to_value(p).unwrap_or(JsonValue::Null),
            None => JsonValue::Null,
        }
    }
}


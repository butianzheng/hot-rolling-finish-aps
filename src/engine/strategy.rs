// ==========================================
// 热轧精整排产系统 - 策略定义
// ==========================================
// 用途：
// - Strategy Drafts（多策略草案）在不落库的前提下，使用不同策略做试算；
// - Publish（生成正式版本）时复用相同策略参数，保证结果可复现。

use serde::{Deserialize, Serialize};

/// 排程策略（用于草案对比/一键重算的策略化入口）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleStrategy {
    Balanced,
    UrgentFirst,
    CapacityFirst,
    ColdStockFirst,
}

impl ScheduleStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScheduleStrategy::Balanced => "balanced",
            ScheduleStrategy::UrgentFirst => "urgent_first",
            ScheduleStrategy::CapacityFirst => "capacity_first",
            ScheduleStrategy::ColdStockFirst => "cold_stock_first",
        }
    }

    pub fn title_cn(&self) -> &'static str {
        match self {
            ScheduleStrategy::Balanced => "均衡方案",
            ScheduleStrategy::UrgentFirst => "紧急优先",
            ScheduleStrategy::CapacityFirst => "产能优先",
            ScheduleStrategy::ColdStockFirst => "冷坨消化",
        }
    }
}

impl Default for ScheduleStrategy {
    fn default() -> Self {
        ScheduleStrategy::Balanced
    }
}

impl std::str::FromStr for ScheduleStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "balanced" => Ok(ScheduleStrategy::Balanced),
            "urgent_first" | "urgent-first" => Ok(ScheduleStrategy::UrgentFirst),
            "capacity_first" | "capacity-first" => Ok(ScheduleStrategy::CapacityFirst),
            "cold_stock_first" | "cold-stock-first" => Ok(ScheduleStrategy::ColdStockFirst),
            other => Err(format!("未知策略类型: {}", other)),
        }
    }
}

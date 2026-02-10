// ==========================================
// 热轧精整排产系统 - 决策用例模块
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 5 节
// 职责: 定义 6 个核心决策问题的标准输出和刷新触发
// ==========================================

pub mod d1_most_risky_day;
pub mod d2_order_failure;
pub mod d3_cold_stock;
pub mod d4_machine_bottleneck;
pub mod d5_roll_campaign_alert;
pub mod d6_capacity_opportunity;

// 用例实现
pub mod impls;

// 重导出用例接口
pub use d1_most_risky_day::{DaySummary, MostRiskyDayUseCase, ReasonItem};
pub use d2_order_failure::{
    BlockingFactor, FailureStats, FailureType, OrderFailure, OrderFailureUseCase,
};
pub use d3_cold_stock::{ColdStockProfile, ColdStockSummary, ColdStockUseCase};
pub use d4_machine_bottleneck::{
    BottleneckHeatmap, MachineBottleneckProfile, MachineBottleneckUseCase,
};
pub use d5_roll_campaign_alert::{RollAlert, RollAlertSummary, RollCampaignAlertUseCase};
pub use d6_capacity_opportunity::{
    CapacityOpportunity, CapacityOpportunityUseCase, OptimizationSummary,
};

// ==========================================
// 通用类型定义
// ==========================================

use serde::{Deserialize, Serialize};

/// 决策用例查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionQuery {
    /// 版本 ID
    pub version_id: String,
    /// 日期范围（可选）
    pub date_range: Option<DateRange>,
    /// 机组代码（可选）
    pub machine_code: Option<String>,
}

/// 日期范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    /// 开始日期 (YYYY-MM-DD)
    pub start_date: String,
    /// 结束日期 (YYYY-MM-DD)
    pub end_date: String,
}

/// 决策用例响应基础结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResponse<T> {
    /// 版本 ID
    pub version_id: String,
    /// 快照时间 (ISO 8601)
    pub as_of: String,
    /// 数据项
    pub items: Vec<T>,
}

impl<T> DecisionResponse<T> {
    /// 创建新的决策响应
    pub fn new(version_id: String, items: Vec<T>) -> Self {
        Self {
            version_id,
            as_of: chrono::Utc::now().to_rfc3339(),
            items,
        }
    }
}

/// 刷新触发器类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefreshTrigger {
    /// 排产项变更
    PlanItemChanged,
    /// 风险快照更新
    RiskSnapshotUpdated,
    /// 材料状态变更
    MaterialStateChanged,
    /// 产能池变更
    CapacityPoolChanged,
    /// 换辊活动变更
    RollCampaignChanged,
    /// 版本创建
    VersionCreated,
    /// 手动刷新
    ManualRefresh,
}

/// 刷新范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshScope {
    /// 版本 ID
    pub version_id: String,
    /// 影响的机组（None 表示全部）
    pub machine_codes: Option<Vec<String>>,
    /// 影响的日期范围（None 表示全部）
    pub date_range: Option<DateRange>,
}

impl RefreshScope {
    /// 创建全量刷新范围
    pub fn full(version_id: String) -> Self {
        Self {
            version_id,
            machine_codes: None,
            date_range: None,
        }
    }

    /// 创建增量刷新范围
    pub fn incremental(
        version_id: String,
        machine_codes: Vec<String>,
        date_range: DateRange,
    ) -> Self {
        Self {
            version_id,
            machine_codes: Some(machine_codes),
            date_range: Some(date_range),
        }
    }

    /// 判断是否为全量刷新
    pub fn is_full(&self) -> bool {
        self.machine_codes.is_none() && self.date_range.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_response_creation() {
        let response = DecisionResponse::new("V001".to_string(), vec![1, 2, 3]);
        assert_eq!(response.version_id, "V001");
        assert_eq!(response.items.len(), 3);
        assert!(!response.as_of.is_empty());
    }

    #[test]
    fn test_refresh_scope_full() {
        let scope = RefreshScope::full("V001".to_string());
        assert!(scope.is_full());
        assert!(scope.machine_codes.is_none());
        assert!(scope.date_range.is_none());
    }

    #[test]
    fn test_refresh_scope_incremental() {
        let scope = RefreshScope::incremental(
            "V001".to_string(),
            vec!["H032".to_string()],
            DateRange {
                start_date: "2026-01-23".to_string(),
                end_date: "2026-01-25".to_string(),
            },
        );
        assert!(!scope.is_full());
        assert!(scope.machine_codes.is_some());
        assert!(scope.date_range.is_some());
    }
}

// ==========================================
// 热轧精整排产系统 - 决策用例实现模块
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md
// 职责: 决策用例的具体实现
// ==========================================

// D1: 哪天最危险 - 用例实现
pub mod d1_most_risky_day_impl;

// D4: 哪个机组最堵 - 用例实现
pub mod d4_machine_bottleneck_impl;

// D2: 哪些紧急单无法完成 - 用例实现
pub mod d2_order_failure_impl;

// D3: 哪些冷料压库 - 用例实现
pub mod d3_cold_stock_impl;

// D5: 换辊是否异常 - 用例实现
pub mod d5_roll_campaign_alert_impl;

// D6: 是否存在产能优化空间 - 用例实现
pub mod d6_capacity_opportunity_impl;

// 重导出用例实现
pub use d1_most_risky_day_impl::MostRiskyDayUseCaseImpl;
pub use d4_machine_bottleneck_impl::MachineBottleneckUseCaseImpl;
pub use d2_order_failure_impl::OrderFailureUseCaseImpl;
pub use d3_cold_stock_impl::ColdStockUseCaseImpl;
pub use d5_roll_campaign_alert_impl::RollCampaignAlertUseCaseImpl;
pub use d6_capacity_opportunity_impl::CapacityOpportunityUseCaseImpl;

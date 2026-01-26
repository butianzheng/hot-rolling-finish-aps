// ==========================================
// 热轧精整排产系统 - 决策层仓储模块
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md
// 职责: 决策层数据访问接口
// ==========================================

// D1: 日期风险摘要仓储
pub mod day_summary_repo;

// D4: 机组堵塞仓储
pub mod bottleneck_repo;

// D2: 订单失败仓储
pub mod order_failure_repo;

// D3: 冷料压库仓储
pub mod cold_stock_repo;

// D5: 换辊预警仓储
pub mod roll_alert_repo;

// D6: 产能优化机会仓储
pub mod capacity_opportunity_repo;

// 重导出仓储
pub use day_summary_repo::DaySummaryRepository;
pub use bottleneck_repo::BottleneckRepository;
pub use order_failure_repo::OrderFailureRepository;
pub use cold_stock_repo::ColdStockRepository;
pub use roll_alert_repo::RollAlertRepository;
pub use capacity_opportunity_repo::CapacityOpportunityRepository;

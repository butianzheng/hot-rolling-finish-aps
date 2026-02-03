// ==========================================
// 热轧精整排产系统 - 核心库
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - 系统宪法
// 技术栈: Tauri + Rust + SQLite
// 系统定位: 决策支持系统 (人工最终控制权)
// ==========================================

// 初始化国际化系统
rust_i18n::i18n!("locales", fallback = "zh-CN");

// ==========================================
// 模块声明
// ==========================================

// 领域层 - 实体与类型
pub mod domain;

// 数据仓储层 - 数据访问
pub mod repository;

// 引擎层 - 业务规则
pub mod engine;

// 导入层 - 外部数据
pub mod importer;

// 配置层 - 系统配置
pub mod config;

// 数据库基础设施（连接初始化/PRAGMA 统一）
pub mod db;

// 日志系统
pub mod logging;

// 国际化
pub mod i18n;

// API 层 - 业务接口
pub mod api;

// 应用层 - Tauri 集成
pub mod app;

// 决策层 - 决策结构层 (Decision Structure Layer)
pub mod decision;

// ==========================================
// 重导出核心类型
// ==========================================

// 领域类型
pub use domain::types::{
    RiskLevel, RollStatus, RushLevel, SchedState, Season, SeasonMode, UrgentLevel,
};

// 领域实体
pub use domain::{
    ActionLog, ActionType, CapacityPool, ImpactSummary, MaterialMaster, MaterialState, Plan,
    PlanItem, PlanVersion, RiskSnapshot, RollerCampaign,
};

// 引擎
pub use engine::{
    CapacityFiller, EligibilityEngine, ImpactSummaryEngine, PrioritySorter, RecalcEngine,
    RiskEngine, RollCampaignEngine, ScheduleOrchestrator, StructureCorrector, UrgencyEngine,
};

// API
pub use api::{DashboardApi, MaterialApi, PlanApi};

// 决策对象
pub use decision::{
    BottleneckPoint, BottleneckReason, BottleneckType,
    CapacityConstraint, CapacitySlice,
    ColdStockBucket,
    CommitmentUnit,
    MachineDay,
    MaterialCandidate,
    PlanningDay,
    RiskFactor, RiskSnapshotView,
};

// ==========================================
// 库初始化
// ==========================================

// TODO: 初始化函数
// pub async fn initialize() -> Result<(), Box<dyn std::error::Error>> {
//     // 1. 初始化数据库连接池
//     // 2. 运行数据库迁移
//     // 3. 加载配置
//     // 4. 初始化日志系统
//     // Ok(())
// }

// ==========================================
// 错误类型
// ==========================================

// TODO: 定义统一错误类型
// pub enum AppError {
//     Database(String),
//     Engine(String),
//     Validation(String),
//     NotFound(String),
//     Internal(String),
// }

// ==========================================
// 常量定义
// ==========================================

// 系统版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// 系统名称
pub const APP_NAME: &str = "热轧精整排产系统";

// 数据库版本
pub const DB_VERSION: &str = "v0.1";

// TODO: 添加更多系统常量

// ==========================================
// 预编译检查
// ==========================================

// 确保编译时所有模块可见
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    // TODO: 添加更多测试
}

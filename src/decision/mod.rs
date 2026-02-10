// ==========================================
// 热轧精整排产系统 - 决策层模块
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx
// 职责: 决策结构层（Decision Structure Layer）
// ==========================================
// 系统定位: 决策支持系统 (人工最终控制权)
// 架构原则:
// - 决策层对外只输出"决策视图/快照/解释"，不暴露底层表结构
// - 决策层统一定义口径：危险/堵塞/压库/违约等指标的阈值、算法版本与可解释字段
// - 决策层统一刷新：任何会影响决策口径的写操作，必须触发对应读模型刷新
// - 引擎层保持"可组合、可单测、可替换"；读模型层保持"面向查询、面向驾驶舱"
// ==========================================

// 决策对象模型
pub mod models;

// 决策用例
pub mod use_cases;

// 决策仓储
pub mod repository;

// 决策服务（刷新等）
pub mod services;

// 决策 API
pub mod api;

// 公共工具模块（P4: 重构重复逻辑下沉）
pub mod common;

// 重导出核心类型
pub use models::{
    BottleneckPoint, BottleneckReason, BottleneckType, CapacityConstraint, CapacitySlice,
    ColdStockBucket, CommitmentUnit, MachineDay, MaterialCandidate, PlanningDay, RiskFactor,
    RiskSnapshotView,
};

// 重导出用例类型
pub use use_cases::{
    BlockingFactor,
    BottleneckHeatmap,
    // D6: 是否存在产能优化空间
    CapacityOpportunity,
    CapacityOpportunityUseCase,
    // D3: 哪些冷料压库
    ColdStockProfile,
    ColdStockSummary,
    ColdStockUseCase,
    DateRange,
    // D1: 哪天最危险
    DaySummary,
    // 通用类型
    DecisionQuery,
    DecisionResponse,
    FailureStats,
    FailureType,
    // D4: 哪个机组最堵
    MachineBottleneckProfile,
    MachineBottleneckUseCase,
    MostRiskyDayUseCase,
    OptimizationSummary,
    // D2: 哪些紧急单无法完成
    OrderFailure,
    OrderFailureUseCase,
    ReasonItem,
    RefreshScope,
    RefreshTrigger,
    // D5: 换辊是否异常
    RollAlert,
    RollAlertSummary,
    RollCampaignAlertUseCase,
};

// 重导出 API DTO
pub use api::*;

// 重导出仓储
pub use repository::*;

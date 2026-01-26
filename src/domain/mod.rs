// ==========================================
// 热轧精整排产系统 - 领域模型层
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART C 数据与状态体系
// 依据: Engine_Specs_v0.3_Integrated.md - 主实体定义
// ==========================================
// 职责: 定义领域实体、类型、业务规则接口
// 红线: 不含数据访问逻辑,不含引擎逻辑
// ==========================================

pub mod action_log;
pub mod capacity;
pub mod material;
pub mod plan;
pub mod risk;
pub mod roller;
pub mod types;

// 重导出核心类型
pub use action_log::{ActionLog, ActionType, CapacityChange, ImpactSummary, MaterialChange, RiskChange};
pub use capacity::{CapacityConstraint, CapacityPool};
pub use material::{
    ConflictType, DqLevel, DqReport, DqSummary, DqViolation, ImportBatch, ImportConflict,
    ImportResult, MaterialEligibility, MaterialMaster, MaterialState, MaterialUrgency,
    RawMaterialRecord,
};
pub use plan::{Plan, PlanItem, PlanVersion, PlanVersionManagement};
pub use risk::{RiskAssessment, RiskSnapshot};
pub use roller::{RollerCampaign, RollerCampaignMonitor};
pub use types::{
    RiskLevel, RollStatus, RushLevel, SchedState, Season, SeasonMode, UrgentLevel,
};

// TODO: 添加领域服务模块 (domain services)
// TODO: 添加值对象模块 (value objects)
// TODO: 添加领域事件模块 (domain events)

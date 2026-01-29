// ==========================================
// 热轧精整排产系统 - 引擎层
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART D 引擎体系
// 依据: Engine_Specs_v0.3_Integrated.md - 1.2 模块拆分
// ==========================================
// 职责: 实现业务规则引擎,不拼 SQL
// 红线: Engine 不拼 SQL, 所有规则必须输出 reason
// ==========================================

pub mod capacity_filler;
pub mod eligibility;
pub mod eligibility_core;
pub mod events;
pub mod impact_summary;
pub mod importer;
pub mod material_state_derivation;
pub mod orchestrator;
pub mod priority;
pub mod recalc;
pub mod repositories;
pub mod risk;
pub mod roll_campaign;
pub mod structure;
pub mod strategy;
pub mod urgency;

// 重导出核心引擎
pub use capacity_filler::CapacityFiller;
pub use eligibility::EligibilityEngine;
pub use eligibility_core::EligibilityCore;
pub use events::{
    NoOpEventPublisher, OptionalEventPublisher, ScheduleEvent, ScheduleEventPublisher,
    ScheduleEventType,
};
pub use impact_summary::ImpactSummaryEngine;
pub use importer::MaterialImporter;
pub use material_state_derivation::MaterialStateDerivationService;
pub use orchestrator::{ScheduleOrchestrator, ScheduleResult};
pub use priority::PrioritySorter;
pub use recalc::{RecalcConfig, RecalcEngine, RecalcResult};
pub use repositories::ScheduleRepositories;
pub use risk::RiskEngine;
pub use roll_campaign::RollCampaignEngine;
pub use structure::{StructureCorrector, StructureViolationReport};
pub use strategy::ScheduleStrategy;
pub use urgency::UrgencyEngine;

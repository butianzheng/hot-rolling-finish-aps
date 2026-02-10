// ==========================================
// 热轧精整排产系统 - 重算/联动引擎
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 6. Recalc Engine
// 依据: docs/recalc_engine_design.md - 设计规格
// 红线: 冻结区材料不可被系统调整
// ==========================================
// 职责: 一键重算 / 局部重排 / 联动窗口
// 输入: 排产版本 + 窗口天数 + 冻结区范围
// 输出: 新版本 + 重算后的 plan_item
// ==========================================

mod core;
mod ops;
mod refresh;
mod reschedule;
mod risk;
mod types;
mod versioning;

pub use types::{RecalcConfig, RecalcResult, RescheduleResult, ResolvedStrategyProfile};

use crate::config::ConfigManager;
use crate::engine::events::OptionalEventPublisher;
use crate::engine::RiskEngine;
use crate::engine::{CapacityFiller, EligibilityEngine, PrioritySorter, UrgencyEngine};
use crate::repository::{
    ActionLogRepository, CapacityPoolRepository, MaterialMasterRepository, MaterialStateRepository,
    PathOverridePendingRepository, PlanItemRepository, PlanVersionRepository,
    RiskSnapshotRepository, RollerCampaignRepository,
};
use std::sync::Arc;

// ==========================================
// RecalcEngine - 重算/联动引擎
// ==========================================
pub struct RecalcEngine {
    // 仓储依赖
    version_repo: Arc<PlanVersionRepository>,
    item_repo: Arc<PlanItemRepository>,
    material_state_repo: Arc<MaterialStateRepository>,
    material_master_repo: Arc<MaterialMasterRepository>,
    capacity_repo: Arc<CapacityPoolRepository>,
    action_log_repo: Arc<ActionLogRepository>,
    risk_snapshot_repo: Arc<RiskSnapshotRepository>,
    roller_campaign_repo: Arc<RollerCampaignRepository>,
    path_override_pending_repo: Arc<PathOverridePendingRepository>,

    // 引擎依赖
    eligibility_engine: Arc<EligibilityEngine<ConfigManager>>,
    urgency_engine: Arc<UrgencyEngine>,
    priority_sorter: Arc<PrioritySorter>,
    capacity_filler: Arc<CapacityFiller>,
    risk_engine: Arc<RiskEngine>,

    // 事件发布器 (依赖倒置: Engine 定义 trait, Decision 实现)
    event_publisher: OptionalEventPublisher,

    // 配置
    config: RecalcConfig,
    config_manager: Arc<ConfigManager>,
}

// ==========================================
// 热轧精整排产系统 - 排产方案 API
// ==========================================
// 职责: 排产方案管理、版本管理、明细查询
// 红线合规: 红线1-5全覆盖
// 依据: 实施计划 Phase 3
// ==========================================

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::error::{ApiError, ApiResult};
use crate::config::ConfigManager;
use crate::domain::plan::{Plan, PlanVersion, PlanItem};
use crate::domain::types::PlanVersionStatus;
use crate::domain::action_log::ActionLog;
use crate::repository::plan_repo::{PlanItemRepository, PlanItemVersionAgg, PlanRepository, PlanVersionRepository};
use crate::repository::action_log_repo::ActionLogRepository;
use crate::repository::material_repo::{MaterialMasterRepository, MaterialStateRepository, MaterialStateSnapshotLite};
use crate::repository::risk_repo::RiskSnapshotRepository;
use crate::repository::capacity_repo::CapacityPoolRepository;
use crate::repository::{StrategyDraftEntity, StrategyDraftRepository, StrategyDraftStatus};
use crate::engine::recalc::{RecalcEngine, ResolvedStrategyProfile};
use crate::engine::risk::RiskEngine;
use crate::engine::ScheduleStrategy;
use crate::engine::events::{OptionalEventPublisher, ScheduleEvent, ScheduleEventPublisher, ScheduleEventType};

// ==========================================
// PlanApi - 排产方案 API
// ==========================================

/// 排产方案API
///
/// 职责：
/// 1. 方案管理（创建、查询、删除）
/// 2. 版本管理（创建版本、激活版本、查询版本）
/// 3. 排产计算（一键重算、部分重算）
/// 4. 版本对比和回滚
/// 5. 工业红线合规性验证
pub struct PlanApi {
    plan_repo: Arc<PlanRepository>,
    plan_version_repo: Arc<PlanVersionRepository>,
    plan_item_repo: Arc<PlanItemRepository>,
    material_state_repo: Arc<MaterialStateRepository>,
    material_master_repo: Arc<MaterialMasterRepository>,
    capacity_repo: Arc<CapacityPoolRepository>,
    strategy_draft_repo: Arc<StrategyDraftRepository>,
    action_log_repo: Arc<ActionLogRepository>,
    risk_snapshot_repo: Arc<RiskSnapshotRepository>,
    config_manager: Arc<ConfigManager>,
    recalc_engine: Arc<RecalcEngine>,
    risk_engine: Arc<RiskEngine>,
    // 事件发布器（依赖倒置：不再直接依赖 Decision 层的 RefreshQueue）
    event_publisher: OptionalEventPublisher,
}

impl PlanApi {
    /// 创建新的PlanApi实例
    pub fn new(
        plan_repo: Arc<PlanRepository>,
        plan_version_repo: Arc<PlanVersionRepository>,
        plan_item_repo: Arc<PlanItemRepository>,
        material_state_repo: Arc<MaterialStateRepository>,
        material_master_repo: Arc<MaterialMasterRepository>,
        capacity_repo: Arc<CapacityPoolRepository>,
        strategy_draft_repo: Arc<StrategyDraftRepository>,
        action_log_repo: Arc<ActionLogRepository>,
        risk_snapshot_repo: Arc<RiskSnapshotRepository>,
        config_manager: Arc<ConfigManager>,
        recalc_engine: Arc<RecalcEngine>,
        risk_engine: Arc<RiskEngine>,
        event_publisher: Option<Arc<dyn ScheduleEventPublisher>>,
    ) -> Self {
        let event_publisher = match event_publisher {
            Some(p) => OptionalEventPublisher::with_publisher(p),
            None => OptionalEventPublisher::none(),
        };

        Self {
            plan_repo,
            plan_version_repo,
            plan_item_repo,
            material_state_repo,
            material_master_repo,
            capacity_repo,
            strategy_draft_repo,
            action_log_repo,
            risk_snapshot_repo,
            config_manager,
            recalc_engine,
            risk_engine,
            event_publisher,
        }
    }
}

mod plan_management;
mod version_management;
mod recalc;
mod strategy_drafts;
mod items_query;
mod version_comparison;
mod operations;

// ==========================================
// DTO 类型定义
// ==========================================

/// 重算响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecalcResponse {
    /// 重算运行ID
    pub run_id: String,

    /// 版本ID
    pub version_id: String,

    /// 版本修订号（plan_rev）
    pub plan_rev: i32,

    /// 排产材料数量
    pub plan_items_count: usize,

    /// 冻结区材料数量
    pub frozen_items_count: usize,

    /// 是否成功
    pub success: bool,

    /// 消息
    pub message: String,
}

/// 版本对比结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionComparisonResult {
    /// 版本A ID
    pub version_id_a: String,

    /// 版本B ID
    pub version_id_b: String,

    /// 移动材料数量（日期或机组变化）
    pub moved_count: usize,

    /// 新增材料数量（只在B中）
    pub added_count: usize,

    /// 删除材料数量（只在A中）
    pub removed_count: usize,

    /// 被挤出材料数量（在A中排产，在B中未排产）
    pub squeezed_out_count: usize,

    /// 风险变化（按日期）
    pub risk_delta: Option<Vec<RiskDelta>>,

    /// 产能变化（按机组和日期）
    pub capacity_delta: Option<Vec<CapacityDelta>>,

    /// 配置变化
    pub config_changes: Option<Vec<ConfigChange>>,

    /// 消息
    pub message: String,
}

/// 版本对比 KPI 汇总结果（聚合）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionComparisonKpiResult {
    pub version_id_a: String,
    pub version_id_b: String,
    pub kpi_a: VersionKpiSummary,
    pub kpi_b: VersionKpiSummary,
    pub diff_counts: VersionDiffCounts,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiffCounts {
    pub moved_count: usize,
    pub added_count: usize,
    pub removed_count: usize,
    pub squeezed_out_count: usize,
}

/// 单版本 KPI 汇总（尽量使用现有表聚合，避免额外“明细级”查库）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionKpiSummary {
    pub plan_items_count: usize,
    pub total_weight_t: f64,
    pub locked_in_plan_count: usize,
    pub force_release_in_plan_count: usize,
    pub plan_date_from: Option<NaiveDate>,
    pub plan_date_to: Option<NaiveDate>,

    // ===== risk_snapshot 聚合（若缺失则为 None）=====
    pub overflow_days: Option<usize>,
    pub overflow_t: Option<f64>,
    pub capacity_used_t: Option<f64>,
    pub capacity_target_t: Option<f64>,
    pub capacity_limit_t: Option<f64>,
    pub capacity_util_pct: Option<f64>,
    pub mature_backlog_t: Option<f64>,
    pub immature_backlog_t: Option<f64>,
    pub urgent_total_t: Option<f64>,
    pub snapshot_date_from: Option<NaiveDate>,
    pub snapshot_date_to: Option<NaiveDate>,
}

impl VersionKpiSummary {
    fn from_aggs(plan: PlanItemVersionAgg, risk: VersionRiskKpi) -> Self {
        let has_risk = !risk.is_empty();
        Self {
            plan_items_count: plan.plan_items_count,
            total_weight_t: plan.total_weight_t,
            locked_in_plan_count: plan.locked_in_plan_count,
            force_release_in_plan_count: plan.force_release_in_plan_count,
            plan_date_from: plan.plan_date_from,
            plan_date_to: plan.plan_date_to,

            overflow_days: has_risk.then_some(risk.overflow_days),
            overflow_t: has_risk.then_some(risk.overflow_t),
            capacity_used_t: has_risk.then_some(risk.used_capacity_t),
            capacity_target_t: has_risk.then_some(risk.target_capacity_t),
            capacity_limit_t: has_risk.then_some(risk.limit_capacity_t),
            capacity_util_pct: has_risk.then_some(risk.capacity_util_pct),
            mature_backlog_t: has_risk.then_some(risk.mature_backlog_t),
            immature_backlog_t: has_risk.then_some(risk.immature_backlog_t),
            urgent_total_t: has_risk.then_some(risk.urgent_total_t),
            snapshot_date_from: if has_risk { risk.snapshot_date_from } else { None },
            snapshot_date_to: if has_risk { risk.snapshot_date_to } else { None },
        }
    }
}

#[derive(Debug, Clone)]
struct VersionRiskKpi {
    overflow_days: usize,
    overflow_t: f64,
    used_capacity_t: f64,
    target_capacity_t: f64,
    limit_capacity_t: f64,
    capacity_util_pct: f64,
    mature_backlog_t: f64,
    immature_backlog_t: f64,
    urgent_total_t: f64,
    snapshot_date_from: Option<NaiveDate>,
    snapshot_date_to: Option<NaiveDate>,
}

impl VersionRiskKpi {
    fn empty() -> Self {
        Self {
            overflow_days: 0,
            overflow_t: 0.0,
            used_capacity_t: 0.0,
            target_capacity_t: 0.0,
            limit_capacity_t: 0.0,
            capacity_util_pct: 0.0,
            mature_backlog_t: 0.0,
            immature_backlog_t: 0.0,
            urgent_total_t: 0.0,
            snapshot_date_from: None,
            snapshot_date_to: None,
        }
    }

    fn is_empty(&self) -> bool {
        self.snapshot_date_from.is_none()
    }
}

/// 风险变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskDelta {
    /// 日期
    pub date: String,

    /// 版本A的风险分数
    pub risk_score_a: Option<f64>,

    /// 版本B的风险分数
    pub risk_score_b: Option<f64>,

    /// 风险分数变化
    pub risk_score_delta: f64,
}

/// 产能变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityDelta {
    /// 机组代码
    pub machine_code: String,

    /// 日期
    pub date: String,

    /// 版本A的已用产能
    pub used_capacity_a: Option<f64>,

    /// 版本B的已用产能
    pub used_capacity_b: Option<f64>,

    /// 产能变化
    pub capacity_delta: f64,
}

/// 配置变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    /// 配置键
    pub key: String,

    /// 版本A的值
    pub value_a: Option<String>,

    /// 版本B的值
    pub value_b: Option<String>,
}

/// 移动项请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveItemRequest {
    /// 材料ID
    pub material_id: String,

    /// 目标日期
    pub to_date: String,

    /// 目标序号
    pub to_seq: i32,

    /// 目标机组
    pub to_machine: String,
}

/// 移动项结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveItemResult {
    /// 材料ID
    pub material_id: String,

    /// 是否成功
    pub success: bool,

    /// 原日期
    pub from_date: Option<String>,

    /// 原机组
    pub from_machine: Option<String>,

    /// 目标日期
    pub to_date: String,

    /// 目标机组
    pub to_machine: String,

    /// 错误消息（如果失败）
    pub error: Option<String>,

    /// 违规类型（如果有）
    pub violation_type: Option<String>,
}

/// 移动项响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveItemsResponse {
    /// 版本ID
    pub version_id: String,

    /// 移动结果列表
    pub results: Vec<MoveItemResult>,

    /// 成功数量
    pub success_count: usize,

    /// 失败数量
    pub failed_count: usize,

    /// 是否有违规
    pub has_violations: bool,

    /// 消息
    pub message: String,
}

// ==========================================
// Strategy Drafts DTO
// ==========================================

/// 策略预设（后端提供默认策略列表，前端可按需扩展）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPreset {
    pub strategy: ScheduleStrategy,
    pub title: String,
    pub description: String,
    pub default_parameters: Value,
}

/// 排产明细日期边界（用于 Workbench AUTO 日期范围）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanItemDateBoundsResponse {
    pub version_id: String,
    pub machine_code: Option<String>,
    pub min_plan_date: Option<NaiveDate>,
    pub max_plan_date: Option<NaiveDate>,
    pub total_count: i64,
}

/// 单个策略草案摘要（用于并排对比）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDraftSummary {
    pub draft_id: String,
    pub base_version_id: String,
    pub strategy: String,
    pub plan_items_count: usize,
    pub frozen_items_count: usize,
    pub calc_items_count: usize,
    pub mature_count: usize,
    pub immature_count: usize,
    pub total_capacity_used_t: f64,
    pub overflow_days: usize,
    pub moved_count: usize,
    pub added_count: usize,
    pub removed_count: usize,
    pub squeezed_out_count: usize,
    pub message: String,
}

/// 策略草案变更明细项（用于解释对比）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDraftDiffItem {
    pub material_id: String,
    /// 变更类型：MOVED / ADDED / SQUEEZED_OUT
    pub change_type: String,

    pub from_plan_date: Option<NaiveDate>,
    pub from_machine_code: Option<String>,
    pub from_seq_no: Option<i32>,

    pub to_plan_date: Option<NaiveDate>,
    pub to_machine_code: Option<String>,
    pub to_seq_no: Option<i32>,

    /// 草案侧落位原因（来自引擎产出 plan_item.assign_reason；冻结/旧快照可能为空）
    pub to_assign_reason: Option<String>,
    /// 草案侧紧急等级快照
    pub to_urgent_level: Option<String>,
    /// 草案侧排产状态快照
    pub to_sched_state: Option<String>,

    /// material_state 快照（用于解释“挤出”等现象；避免前端逐条查库）
    pub material_state_snapshot: Option<MaterialStateSnapshotLite>,
}

/// 查询草案变更明细响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStrategyDraftDetailResponse {
    pub draft_id: String,
    pub base_version_id: String,
    pub plan_date_from: NaiveDate,
    pub plan_date_to: NaiveDate,
    pub strategy: String,
    pub diff_items: Vec<StrategyDraftDiffItem>,
    pub diff_items_total: usize,
    pub diff_items_truncated: bool,
    pub message: String,
}

/// 生成多策略草案响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateStrategyDraftsResponse {
    pub base_version_id: String,
    pub plan_date_from: NaiveDate,
    pub plan_date_to: NaiveDate,
    pub drafts: Vec<StrategyDraftSummary>,
    pub message: String,
}

/// 列出策略草案响应（用于页面刷新/重启后的恢复）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListStrategyDraftsResponse {
    pub base_version_id: String,
    pub plan_date_from: NaiveDate,
    pub plan_date_to: NaiveDate,
    pub drafts: Vec<StrategyDraftSummary>,
    pub message: String,
}

/// 发布策略草案响应（生成正式版本）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyStrategyDraftResponse {
    pub version_id: String,
    pub success: bool,
    pub message: String,
}

/// 手动触发决策读模型刷新响应（P0-2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualRefreshDecisionResponse {
    pub version_id: String,
    pub task_id: Option<String>,
    pub success: bool,
    pub message: String,
}

/// 版本回滚响应（P1-2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackVersionResponse {
    pub plan_id: String,
    pub from_version_id: Option<String>,
    pub to_version_id: String,
    pub restored_config_count: Option<usize>,
    pub config_restore_skipped: Option<String>,
    pub message: String,
}

/// 清理草案响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupStrategyDraftsResponse {
    pub deleted_count: usize,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_api_structure() {
        // 这个测试只是验证结构是否正确定义
        // 实际的集成测试在 tests/ 目录
    }
}

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
use crate::domain::capacity::CapacityPool;
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

    // ==========================================
    // 方案管理接口
    // ==========================================

    /// 创建排产方案
    ///
    /// # 参数
    /// - plan_name: 方案名称
    /// - created_by: 创建人
    ///
    /// # 返回
    /// - Ok(String): 方案ID
    /// - Err(ApiError): API错误
    pub fn create_plan(&self, plan_name: String, created_by: String) -> ApiResult<String> {
        // 参数验证
        if plan_name.trim().is_empty() {
            return Err(ApiError::InvalidInput("方案名称不能为空".to_string()));
        }
        if created_by.trim().is_empty() {
            return Err(ApiError::InvalidInput("创建人不能为空".to_string()));
        }

        // 创建Plan实例
        let plan = Plan {
            plan_id: uuid::Uuid::new_v4().to_string(),
            plan_name,
            plan_type: "BASELINE".to_string(),
            base_plan_id: None,
            created_by: created_by.clone(),
            created_at: chrono::Local::now().naive_local(),
            updated_at: chrono::Local::now().naive_local(),
        };

        // 保存到数据库
        self.plan_repo
            .create(&plan)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None,
            action_type: "CREATE_PLAN".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: created_by,
            payload_json: Some(serde_json::json!({
                "plan_id": plan.plan_id,
                "plan_name": plan.plan_name,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("创建方案: {}", plan.plan_name)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(plan.plan_id)
    }

    /// 查询方案列表
    ///
    /// # 返回
    /// - Ok(Vec<Plan>): 方案列表
    /// - Err(ApiError): API错误
    pub fn list_plans(&self) -> ApiResult<Vec<Plan>> {
        self.plan_repo
            .list_all()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 查询方案详情
    ///
    /// # 参数
    /// - plan_id: 方案ID
    ///
    /// # 返回
    /// - Ok(Some(Plan)): 方案详情
    /// - Ok(None): 方案不存在
    /// - Err(ApiError): API错误
    pub fn get_plan_detail(&self, plan_id: &str) -> ApiResult<Option<Plan>> {
        if plan_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("方案ID不能为空".to_string()));
        }

        self.plan_repo
            .find_by_id(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 查询最近创建的激活版本ID（跨方案）
    ///
    /// 用途：前端启动时自动回填工作版本，避免“已有激活版本但界面提示未选择”。
    pub fn get_latest_active_version_id(&self) -> ApiResult<Option<String>> {
        self.plan_version_repo
            .find_latest_active_version_id()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 删除排产方案（同时删除其版本与明细）
    ///
    /// 注意：该操作为破坏性操作，仅建议在开发/测试数据管理中使用。
    pub fn delete_plan(&self, plan_id: &str, operator: &str) -> ApiResult<()> {
        if plan_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("方案ID不能为空".to_string()));
        }
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }

        // 校验存在性并取名称用于审计记录
        let plan = self
            .plan_repo
            .find_by_id(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("方案{}不存在", plan_id)))?;

        // 显式删除关联数据（避免依赖 SQLite foreign_keys 配置）
        let versions = self
            .plan_version_repo
            .find_by_plan_id(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let mut deleted_items = 0usize;
        let mut deleted_risks = 0usize;
        let mut deleted_drafts = 0usize;
        let mut detached_action_logs = 0usize;

        for v in &versions {
            deleted_drafts += self
                .strategy_draft_repo
                .delete_by_version(&v.version_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

            deleted_items += self
                .plan_item_repo
                .delete_by_version(&v.version_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
            deleted_risks += self
                .risk_snapshot_repo
                .delete_by_version(&v.version_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

            detached_action_logs += self
                .action_log_repo
                .detach_version(&v.version_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

            self.plan_version_repo
                .delete(&v.version_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        }

        self.plan_repo
            .delete(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None,
            action_type: "DELETE_PLAN".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "plan_id": plan.plan_id,
                "plan_name": plan.plan_name,
                "deleted_versions": versions.len(),
                "deleted_plan_items": deleted_items,
                "deleted_risk_snapshots": deleted_risks,
                "deleted_strategy_drafts": deleted_drafts,
                "detached_action_logs": detached_action_logs,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("删除方案: {}", plan.plan_name)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    // ==========================================
    // 版本管理接口
    // ==========================================

    /// 创建新版本
    ///
    /// # 参数
    /// - plan_id: 方案ID
    /// - window_days: 窗口天数
    /// - frozen_from_date: 冻结区起始日期（可选）
    /// - note: 备注（可选）
    /// - created_by: 创建人
    ///
    /// # 返回
    /// - Ok(String): 版本ID
    /// - Err(ApiError): API错误
    pub fn create_version(
        &self,
        plan_id: String,
        window_days: i32,
        frozen_from_date: Option<NaiveDate>,
        note: Option<String>,
        created_by: String,
    ) -> ApiResult<String> {
        // 参数验证
        if plan_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("方案ID不能为空".to_string()));
        }
        if !(1..=60).contains(&window_days) {
            return Err(ApiError::InvalidInput(
                "窗口天数必须在1-60之间".to_string(),
            ));
        }

        // 检查Plan是否存在
        let plan = self
            .plan_repo
            .find_by_id(&plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        if plan.is_none() {
            return Err(ApiError::NotFound(format!("方案{}不存在", plan_id)));
        }

        // 创建配置快照JSON（用于版本回滚/对比口径）
        // 注意：元信息（例如中文命名/备注）统一写入 __meta_*，避免污染“配置差异”与回滚恢复。
        let config_snapshot_json = Some(
            self.config_manager
                .get_config_snapshot()
                .map_err(|e| ApiError::InternalError(e.to_string()))?,
        );

        // 创建PlanVersion实例（version_no 由仓储层在事务内分配，避免并发冲突）
        let mut version = PlanVersion {
            version_id: uuid::Uuid::new_v4().to_string(),
            plan_id: plan_id.clone(),
            version_no: 0,
            status: PlanVersionStatus::Draft,
            frozen_from_date,
            recalc_window_days: Some(window_days),
            config_snapshot_json,
            created_by: Some(created_by.clone()),
            created_at: chrono::Local::now().naive_local(),
            revision: 1,
        };

        // 保存到数据库
        self.plan_version_repo
            .create_with_next_version_no(&mut version)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 写入备注/命名等元信息（不改变表结构，写到 config_snapshot_json.__meta_*）
        if let Some(note_text) = note.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
            // best-effort: meta 更新失败不影响版本创建，但会影响“命名显示/回滚可解释性”
            if let Ok(mut map) =
                serde_json::from_str::<std::collections::HashMap<String, String>>(
                    version.config_snapshot_json.as_deref().unwrap_or("{}"),
                )
            {
                map.insert("__meta_version_name_cn".to_string(), note_text.to_string());
                map.insert("__meta_note".to_string(), note_text.to_string());
                map.insert(
                    "__meta_note_created_at".to_string(),
                    chrono::Local::now().to_rfc3339(),
                );

                if let Ok(next_json) = serde_json::to_string(&map) {
                    version.config_snapshot_json = Some(next_json);
                    if let Err(e) = self.plan_version_repo.update(&version) {
                        tracing::warn!("创建版本后写入 meta 失败: {}", e);
                    }
                }
            }
        }

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version.version_id.clone()),
            action_type: "CREATE_VERSION".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: created_by,
            payload_json: Some(serde_json::json!({
                "plan_id": plan_id,
                "version_no": version.version_no,
                "window_days": window_days,
                "frozen_from_date": frozen_from_date.map(|d| d.to_string()),
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("创建版本: V{}", version.version_no)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(version.version_id)
    }

    /// 查询版本列表
    ///
    /// # 参数
    /// - plan_id: 方案ID
    ///
    /// # 返回
    /// - Ok(Vec<PlanVersion>): 版本列表
    /// - Err(ApiError): API错误
    pub fn list_versions(&self, plan_id: &str) -> ApiResult<Vec<PlanVersion>> {
        if plan_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("方案ID不能为空".to_string()));
        }

        self.plan_version_repo
            .find_by_plan_id(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 删除版本（仅允许删除非激活版本）
    pub fn delete_version(&self, version_id: &str, operator: &str) -> ApiResult<()> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }

        let version = self
            .plan_version_repo
            .find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        if version.status == PlanVersionStatus::Active {
            return Err(ApiError::BusinessRuleViolation(
                "不能删除激活版本，请先激活其他版本或将其归档".to_string(),
            ));
        }

        // 显式删除关联数据（避免依赖 SQLite foreign_keys 配置）
        // 0. 删除策略草稿（strategy_draft 表有外键引用）
        let deleted_drafts = self
            .strategy_draft_repo
            .delete_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 1. 在删除 plan_item 之前，查询受影响的产能池
        let affected_capacity_keys = self
            .get_affected_capacity_keys_for_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 2. 删除 plan_item
        let deleted_items = self
            .plan_item_repo
            .delete_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 3. 同步重置受影响的产能池
        if !affected_capacity_keys.is_empty() {
            tracing::info!(
                "版本删除后开始重置产能池: version_id={}, 涉及 {} 个(机组,日期)组合",
                version_id,
                affected_capacity_keys.len()
            );
            if let Err(e) = self.reset_capacity_pools(version_id, &affected_capacity_keys) {
                tracing::warn!("产能池重置失败: {}, 继续执行", e);
            }
        }

        // 4. 删除风险快照
        let deleted_risks = self
            .risk_snapshot_repo
            .delete_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 4.5 解绑 action_log（action_log.version_id 有外键约束，删除 plan_version 前必须置空）
        let detached_action_logs = self
            .action_log_repo
            .detach_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        self.plan_version_repo
            .delete(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            // 版本已删除，action_log.version_id 不能再引用已不存在的 plan_version（否则会触发外键约束失败）
            version_id: None,
            action_type: "DELETE_VERSION".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "version_id": version_id,
                "plan_id": version.plan_id,
                "version_no": version.version_no,
                "deleted_plan_items": deleted_items,
                "deleted_risk_snapshots": deleted_risks,
                "deleted_strategy_drafts": deleted_drafts,
                "detached_action_logs": detached_action_logs,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("删除版本: {}", version_id)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// 激活版本
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - operator: 操作人
    ///
    /// # 返回
    /// - Ok(()): 成功
    /// - Err(ApiError): API错误
    pub fn activate_version(&self, version_id: &str, operator: &str) -> ApiResult<()> {
        // 参数验证
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        // 检查版本是否存在
        let version = self
            .plan_version_repo
            .find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        // 同一方案只能有一个激活版本：仓储层在事务中完成归档+激活
        self.plan_version_repo
            .activate_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version_id.to_string()),
            action_type: "ACTIVATE_VERSION".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "version_id": version_id,
                "plan_id": version.plan_id,
                "version_no": version.version_no,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("激活版本: {}", version_id)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 同步刷新产能池以匹配该版本的 plan_item
        tracing::info!("版本激活后开始同步刷新产能池: version_id={}", version_id);
        if let Err(e) = self.recalculate_capacity_pool_for_version(version_id) {
            tracing::warn!("产能池同步刷新失败: {}, 继续执行", e);
            // 不阻断流程，仅记录警告
        }

        // 触发决策视图全量刷新
        let event = ScheduleEvent::full_scope(
            version_id.to_string(),
            ScheduleEventType::ManualTrigger, // 版本激活属于手动触发
            Some(format!("Version activated by {}", operator)),
        );

        match self.event_publisher.publish(event) {
            Ok(task_id) => {
                if !task_id.is_empty() {
                    tracing::info!("版本激活后决策视图刷新事件已发布: task_id={}, version_id={}", task_id, version_id);
                }
            }
            Err(e) => {
                tracing::warn!("版本激活后决策视图刷新事件发布失败: {}", e);
            }
        }

        Ok(())
    }

    /// 根据版本的 plan_item 重新计算产能池
    ///
    /// # 说明
    /// 当版本切换时，需要同步刷新产能池数据，确保：
    /// - used_capacity_t = 该版本 plan_item 的实际 weight_t 总和
    /// - overflow_t = max(0, used_capacity_t - limit_capacity_t)
    ///
    /// # 参数
    /// - version_id: 版本ID
    ///
    /// # 返回
    /// - Ok(()): 成功
    /// - Err(ApiError): API错误
    fn recalculate_capacity_pool_for_version(&self, version_id: &str) -> ApiResult<()> {
        // 1. 获取版本的日期窗口
        let version = self.plan_version_repo.find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        let window_days = version.recalc_window_days.unwrap_or(30);
        let frozen_date = version.frozen_from_date
            .unwrap_or_else(|| chrono::Local::now().date_naive());
        let end_date = frozen_date + chrono::Duration::days(window_days as i64);

        // 2. 先清零窗口内所有 capacity_pool 的 used_capacity_t 和 overflow_t
        // 这确保不会有残留值（避免"利用率高但已排为0"的异常显示）
        self.capacity_repo.reset_used_in_date_range(version_id, frozen_date, end_date)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        tracing::info!(
            "已清零产能池 used_capacity_t: version_id={}, date_range=[{}, {}]",
            version_id, frozen_date, end_date
        );

        // 3. 查询该版本的所有 plan_item 并按 (machine_code, plan_date) 聚合
        let items = self.plan_item_repo.find_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 4. 按 (machine_code, plan_date) 聚合计算 used_capacity_t
        let mut capacity_updates: HashMap<(String, String), f64> = HashMap::new();
        for item in items {
            let key = (item.machine_code.clone(), item.plan_date.to_string());
            *capacity_updates.entry(key).or_insert(0.0) += item.weight_t;
        }

        tracing::info!("产能池同步：version_id={}, 涉及 {} 个(机组,日期)组合",
            version_id, capacity_updates.len());

        // 5. 查询现有产能池，更新 used_capacity_t 和 overflow_t
        for ((machine_code, plan_date_str), used_weight) in capacity_updates {
            let plan_date = NaiveDate::parse_from_str(&plan_date_str, "%Y-%m-%d")
                .map_err(|_| ApiError::InvalidInput(format!("日期格式错误: {}", plan_date_str)))?;

            // 查询现有产能池
            let mut capacity_pool = self.capacity_repo
                .find_by_machine_and_date(version_id, &machine_code, plan_date)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
                .unwrap_or_else(|| {
                    // 如果不存在，创建默认产能池
                    tracing::debug!("产能池不存在，创建默认值: version_id={}, machine={}, date={}",
                        version_id, machine_code, plan_date);
                    CapacityPool {
                        version_id: version_id.to_string(),
                        machine_code: machine_code.clone(),
                        plan_date,
                        target_capacity_t: 0.0,
                        limit_capacity_t: 0.0,
                        used_capacity_t: 0.0,
                        overflow_t: 0.0,
                        frozen_capacity_t: 0.0,
                        accumulated_tonnage_t: 0.0,
                        roll_campaign_id: None,
                    }
                });

            // 更新 used_capacity_t
            capacity_pool.used_capacity_t = used_weight;

            // 重新计算 overflow_t
            capacity_pool.overflow_t = if used_weight > capacity_pool.limit_capacity_t {
                used_weight - capacity_pool.limit_capacity_t
            } else {
                0.0
            };

            // 持久化
            self.capacity_repo.upsert_single(&capacity_pool)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    /// 查询版本中所有 plan_item 的 (machine_code, plan_date) 组合
    ///
    /// # 说明
    /// 用于在删除 plan_item 之前，获取受影响的产能池位置，以便后续重置
    fn get_affected_capacity_keys_for_version(
        &self,
        version_id: &str,
    ) -> ApiResult<Vec<(String, NaiveDate)>> {
        let items = self
            .plan_item_repo
            .find_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let mut keys = std::collections::HashSet::new();
        for item in items {
            keys.insert((item.machine_code, item.plan_date));
        }

        Ok(keys.into_iter().collect())
    }

    /// 重置产能池（将 used_capacity_t 和 overflow_t 清零）
    ///
    /// # 说明
    /// 当删除 plan_item 后，需要同步重置产能池数据，确保数据一致性
    ///
    /// # 参数
    /// - version_id: 版本ID (P1-1: 版本化改造)
    /// - keys: (machine_code, plan_date) 列表
    fn reset_capacity_pools(&self, version_id: &str, keys: &[(String, NaiveDate)]) -> ApiResult<()> {
        for (machine_code, plan_date) in keys {
            // 查询产能池（如果不存在则跳过）
            if let Some(mut capacity_pool) = self
                .capacity_repo
                .find_by_machine_and_date(version_id, machine_code, *plan_date)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            {
                // 重置 used_capacity_t 和 overflow_t
                capacity_pool.used_capacity_t = 0.0;
                capacity_pool.overflow_t = 0.0;

                // 持久化
                self.capacity_repo
                    .upsert_single(&capacity_pool)
                    .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// 版本回滚（激活历史版本，并按需恢复该版本记录的配置快照）
    ///
    /// 规则：
    /// - 仅允许回滚到同一 plan 的历史版本
    /// - 写入 ActionLog（包含 from/to/version_no/恢复配置数量/原因）
    /// - 发布刷新事件（触发决策读模型刷新）
    pub fn rollback_version(
        &self,
        plan_id: &str,
        target_version_id: &str,
        operator: &str,
        reason: &str,
    ) -> ApiResult<RollbackVersionResponse> {
        if plan_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("方案ID不能为空".to_string()));
        }
        if target_version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("目标版本ID不能为空".to_string()));
        }
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("回滚原因不能为空".to_string()));
        }

        // 校验 plan 存在（便于输出可读信息）
        let plan = self
            .plan_repo
            .find_by_id(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("方案{}不存在", plan_id)))?;

        // 校验目标版本存在且属于该 plan
        let target = self
            .plan_version_repo
            .find_by_id(target_version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", target_version_id)))?;

        if target.plan_id != plan_id {
            return Err(ApiError::BusinessRuleViolation(format!(
                "目标版本不属于该方案：plan_id={}, target.plan_id={}",
                plan_id, target.plan_id
            )));
        }

        let current_active = self
            .plan_version_repo
            .find_active_version(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let from_version_id = current_active.as_ref().map(|v| v.version_id.clone());
        let from_version_no = current_active.as_ref().map(|v| v.version_no);

        // 1) 尝试恢复配置（优先：保证后续重算/解释口径一致）
        let mut restored_config_count: Option<usize> = None;
        let mut config_restore_skipped: Option<String> = None;

        match target
            .config_snapshot_json
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            None => {
                config_restore_skipped = Some("目标版本缺少 config_snapshot_json，已跳过配置恢复".to_string());
            }
            Some(snapshot_json) => {
                // 防御：识别“备注型快照”（旧实现可能把 note 写入 config_snapshot_json）
                // - 若对象内非 __meta_* 键过少且包含 note，则认为不是 config_kv 快照，避免把 note 写入 config_kv。
                let mut should_skip = None::<String>;
                match serde_json::from_str::<serde_json::Value>(snapshot_json) {
                    Ok(serde_json::Value::Object(obj)) => {
                        let non_meta_key_count = obj
                            .keys()
                            .filter(|k| !k.starts_with("__meta_"))
                            .count();
                        if non_meta_key_count == 0 {
                            should_skip = Some("目标版本配置快照为空对象，已跳过配置恢复".to_string());
                        } else if non_meta_key_count <= 2 && obj.contains_key("note") {
                            should_skip = Some(
                                "目标版本 config_snapshot_json 更像备注信息（含 note），已跳过配置恢复".to_string(),
                            );
                        }
                    }
                    Ok(_) => {
                        should_skip =
                            Some("目标版本 config_snapshot_json 不是 JSON 对象，已跳过配置恢复".to_string());
                    }
                    Err(e) => {
                        should_skip = Some(format!(
                            "目标版本 config_snapshot_json 解析失败（{}），已跳过配置恢复",
                            e
                        ));
                    }
                }

                if let Some(msg) = should_skip {
                    config_restore_skipped = Some(msg);
                } else {
                    let count = self
                        .config_manager
                        .restore_config_from_snapshot(snapshot_json)
                        .map_err(|e| ApiError::InternalError(format!("恢复配置失败: {}", e)))?;
                    restored_config_count = Some(count);
                }
            }
        }

        // 2) 激活目标版本（事务内归档其他 ACTIVE）
        self.plan_version_repo
            .activate_version(target_version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 3) 写入审计日志
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(target_version_id.to_string()),
            action_type: "ROLLBACK_VERSION".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "plan_id": plan_id,
                "plan_name": plan.plan_name,
                "from_version_id": from_version_id,
                "from_version_no": from_version_no,
                "to_version_id": target_version_id,
                "to_version_no": target.version_no,
                "restored_config_count": restored_config_count,
                "config_restore_skipped": config_restore_skipped,
                "reason": reason,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!(
                "版本回滚: {:?} -> V{} | {}",
                from_version_no,
                target.version_no,
                reason.trim()
            )),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 4) 触发决策视图全量刷新（回滚属于手动触发）
        let event = ScheduleEvent::full_scope(
            target_version_id.to_string(),
            ScheduleEventType::ManualTrigger,
            Some(format!("rollback_version by {} | {}", operator, reason.trim())),
        );

        if let Err(e) = self.event_publisher.publish(event) {
            tracing::warn!("版本回滚后决策刷新事件发布失败: {}", e);
        }

        Ok(RollbackVersionResponse {
            plan_id: plan_id.to_string(),
            from_version_id,
            to_version_id: target_version_id.to_string(),
            restored_config_count,
            config_restore_skipped,
            message: "回滚完成".to_string(),
        })
    }

    /// 手动触发决策读模型刷新（P0-2）
    ///
    /// 说明：
    /// - 这是“可重试”的兜底入口：当决策数据刷新失败或用户怀疑数据过期时，可手动触发一次全量刷新。
    /// - 实际执行依赖 event_publisher（默认由 Decision 层 RefreshQueueAdapter 提供）。
    pub fn manual_refresh_decision(
        &self,
        version_id: &str,
        operator: &str,
    ) -> ApiResult<ManualRefreshDecisionResponse> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }

        // 校验版本存在（避免写入无效 action_log）
        let _ = self
            .plan_version_repo
            .find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        let event = ScheduleEvent::full_scope(
            version_id.to_string(),
            ScheduleEventType::ManualTrigger,
            Some(format!("manual_refresh_decision by {}", operator)),
        );

        let mut task_id: Option<String> = None;
        let mut message = String::new();
        let mut success = true;

        match self.event_publisher.publish(event) {
            Ok(id) => {
                if id.trim().is_empty() {
                    success = false;
                    message = "已收到刷新请求，但当前未配置决策刷新组件（可能不会执行）".to_string();
                } else {
                    task_id = Some(id.clone());
                    message = format!("已触发决策刷新: task_id={}", id);
                }
            }
            Err(e) => {
                success = false;
                message = format!("触发决策刷新失败: {}", e);
            }
        }

        // 记录 ActionLog（best-effort）
        let log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version_id.to_string()),
            action_type: "MANUAL_REFRESH_DECISION".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "version_id": version_id,
                "task_id": task_id,
                "success": success,
                "message": message,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some("手动触发决策读模型刷新".to_string()),
        };

        if let Err(e) = self.action_log_repo.insert(&log) {
            tracing::warn!("记录操作日志失败: {}", e);
        }

        Ok(ManualRefreshDecisionResponse {
            version_id: version_id.to_string(),
            task_id,
            success,
            message,
        })
    }

    // ==========================================
    // 排产计算接口
    // ==========================================

    /// 试算接口（沙盘模式）
    ///
    /// # 参数
    /// - version_id: 版本ID（作为基准版本）
    /// - base_date: 基准日期（从哪天开始排产）
    /// - _frozen_date: 冻结日期（保留参数，实际由RecalcEngine内部计算）
    /// - operator: 操作人
    ///
    /// # 返回
    /// - Ok(RecalcResponse): 试算结果（不保存到数据库）
    /// - Err(ApiError): API错误
    ///
    /// # 说明
    /// - 使用RecalcEngine的dry-run模式
    /// - 不写入plan_item表
    /// - 不写入risk_snapshot表
    /// - 不记录ActionLog
    /// - 返回内存中的结果供前端预览
    pub fn simulate_recalc(
        &self,
        version_id: &str,
        base_date: NaiveDate,
        _frozen_date: Option<NaiveDate>,
        operator: &str,
    ) -> ApiResult<RecalcResponse> {
        self.simulate_recalc_with_strategy(
            version_id,
            base_date,
            _frozen_date,
            operator,
            ScheduleStrategy::Balanced,
        )
    }

    /// 试算接口（沙盘模式）- 指定策略
    pub fn simulate_recalc_with_strategy(
        &self,
        version_id: &str,
        base_date: NaiveDate,
        _frozen_date: Option<NaiveDate>,
        operator: &str,
        strategy: ScheduleStrategy,
    ) -> ApiResult<RecalcResponse> {
        // 参数验证
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        // 加载版本信息
        let version = self
            .plan_version_repo
            .find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        // 获取窗口天数
        let window_days = version.recalc_window_days.unwrap_or(30);

        // 调用RecalcEngine执行试算（dry-run模式）
        let result = self
            .recalc_engine
            .recalc_full(
                &version.plan_id,
                base_date,
                window_days,
                operator,
                true,
                strategy,
            )
            .map_err(|e| ApiError::InternalError(format!("试算失败: {}", e)))?;

        // 返回结果（不记录ActionLog）
        Ok(RecalcResponse {
            version_id: result.version_id,
            plan_items_count: result.total_items,
            frozen_items_count: result.frozen_items,
            success: true,
            message: format!(
                "试算完成（{}），共排产{}个材料（冻结{}个，重算{}个）",
                strategy.as_str(),
                result.total_items, result.frozen_items, result.recalc_items
            ),
        })
    }

    /// 一键重算（核心方法）
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - base_date: 基准日期（从哪天开始排产）
    /// - frozen_date: 冻结日期（该日期之前的材料为冻结区）
    /// - operator: 操作人
    ///
    /// # 返回
    /// - Ok(RecalcResponse): 重算结果
    /// - Err(ApiError): API错误
    ///
    /// # 红线合规
    /// - 红线1: 加载冻结区材料，不调整其排产结果
    /// - 红线2: 调用EligibilityEngine验证适温状态
    /// - 红线3: 调用UrgencyEngine计算紧急等级
    /// - 红线4: 调用CapacityFiller填充，不超产能
    /// - 红线5: 记录ActionLog，包含窗口天数、冻结日期等
    pub fn recalc_full(
        &self,
        version_id: &str,
        base_date: NaiveDate,
        frozen_date: Option<NaiveDate>,
        operator: &str,
    ) -> ApiResult<RecalcResponse> {
        self.recalc_full_with_strategy(
            version_id,
            base_date,
            frozen_date,
            operator,
            ScheduleStrategy::Balanced,
        )
    }

    /// 一键重算（核心方法）- 指定策略
    pub fn recalc_full_with_strategy(
        &self,
        version_id: &str,
        base_date: NaiveDate,
        frozen_date: Option<NaiveDate>,
        operator: &str,
        strategy: ScheduleStrategy,
    ) -> ApiResult<RecalcResponse> {
        // 参数验证
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        // 加载版本信息
        let version = self
            .plan_version_repo
            .find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        // 红线1预检: 设置冻结日期
        let _frozen_from_date = frozen_date.or(version.frozen_from_date);

        // 获取窗口天数（如果版本没有设置，使用默认值30天）
        let window_days = version.recalc_window_days.unwrap_or(30);

        // 调用RecalcEngine执行重算
        tracing::info!(
            "开始重算 version_id={}, plan_id={}, base_date={}, window_days={}",
            version_id,
            version.plan_id,
            base_date,
            window_days
        );

        // 调用 RecalcEngine 执行实际重算
        let recalc_result = self
            .recalc_engine
            .recalc_full(
                &version.plan_id,
                base_date,
                window_days,
                operator,
                false,
                strategy,
            )
            .map_err(|e| ApiError::InternalError(format!("重算失败: {}", e)))?;

        let plan_items_count = recalc_result.total_items;
        let frozen_items_count = recalc_result.frozen_items;

        // 记录ActionLog（红线5: 可解释性）
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(recalc_result.version_id.clone()),
            action_type: "RECALC_FULL".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "base_date": base_date.to_string(),
                "window_days": window_days,
                "frozen_from_date": frozen_date.map(|d| d.to_string()),
                "strategy": strategy.as_str(),
            })),
            impact_summary_json: Some(serde_json::json!({
                "plan_items_count": plan_items_count,
                "frozen_items_count": frozen_items_count,
                "mature_count": recalc_result.mature_count,
                "immature_count": recalc_result.immature_count,
                "elapsed_ms": recalc_result.elapsed_ms,
            })),
            machine_code: None,
            date_range_start: Some(base_date),
            date_range_end: Some(
                base_date + chrono::Duration::days(window_days as i64),
            ),
            detail: Some("一键重算".to_string()),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 返回结果
        Ok(RecalcResponse {
            version_id: recalc_result.version_id,
            plan_items_count,
            frozen_items_count,
            success: true,
            message: format!("重算完成（{}），共排产{}个材料", strategy.as_str(), plan_items_count),
        })
    }

    // ==========================================
    // 策略草案接口（多策略对比）
    // ==========================================

    /// 生成多策略草案（dry-run 试算，草案落库持久化）
    ///
    /// # 说明
    /// - 排产计算采用 dry-run 模式：不写 plan_item / risk_snapshot / capacity_pool / material_state；
    /// - 草案本身会写入 decision_strategy_draft（避免刷新/重启丢失；支持并发/审计）；
    /// - 草案发布时必须再走一次生产模式重算（生成正式版本），保证审计与可追溯。
    pub fn generate_strategy_drafts(
        &self,
        base_version_id: &str,
        plan_date_from: NaiveDate,
        plan_date_to: NaiveDate,
        strategies: Vec<String>,
        operator: &str,
    ) -> ApiResult<GenerateStrategyDraftsResponse> {
        if base_version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("基准版本ID不能为空".to_string()));
        }
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }
        if strategies.is_empty() {
            return Err(ApiError::InvalidInput("策略列表不能为空".to_string()));
        }

        let (from, to) = if plan_date_to < plan_date_from {
            (plan_date_to, plan_date_from)
        } else {
            (plan_date_from, plan_date_to)
        };

        let range_days = (to - from).num_days();
        if range_days > 60 {
            return Err(ApiError::InvalidInput("时间跨度过大，最多支持60天".to_string()));
        }

        // 校验基准版本存在
        let base_version = self
            .plan_version_repo
            .find_by_id(base_version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", base_version_id)))?;

        // 仅允许针对当前激活版本生成草案，避免“草案发布”时基准漂移导致不可复现。
        let active_version = self
            .plan_version_repo
            .find_active_version(&base_version.plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::InvalidInput("当前方案没有激活版本，无法生成草案".to_string()))?;

        if active_version.version_id != base_version_id {
            return Err(ApiError::VersionConflict(format!(
                "基准版本已变更：草案基于 {}，当前激活版本为 {}。请刷新后重新生成草案。",
                base_version_id, active_version.version_id
            )));
        }

        // 基准版本在时间范围内的快照（用于 diff）
        let base_items_in_range = self
            .plan_item_repo
            .find_by_date_range(base_version_id, from, to)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 冻结项（locked_in_plan=1）在范围内需要计入草案快照，否则会被误判为“挤出”
        let frozen_items_in_range: Vec<PlanItem> = self
            .plan_item_repo
            .find_frozen_items(base_version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .into_iter()
            .filter(|item| item.plan_date >= from && item.plan_date <= to)
            .collect();

        // 与 RecalcEngine 默认一致：固定三条机组（后续可改为从配置/机组表动态加载）
        let machine_codes = vec!["H032".to_string(), "H033".to_string(), "H034".to_string()];

        let now = chrono::Local::now().naive_local();
        let expires_at = now + chrono::Duration::hours(72);
        let mut summaries = Vec::new();

        let mut seen: HashSet<String> = HashSet::new();
        for raw_strategy_key in strategies {
            let raw_strategy_key = raw_strategy_key.trim().to_string();
            if raw_strategy_key.is_empty() {
                continue;
            }
            if !seen.insert(raw_strategy_key.clone()) {
                continue;
            }

            let profile = self
                .recalc_engine
                .resolve_strategy_profile(&raw_strategy_key)
                .map_err(|e| ApiError::InvalidInput(format!("策略解析失败（{}）: {}", raw_strategy_key, e)))?;

            let draft_id = uuid::Uuid::new_v4().to_string();

            let reschedule = self
                .recalc_engine
                .execute_reschedule(
                    base_version_id,
                    (from, to),
                    &machine_codes,
                    true,
                    profile.base_strategy,
                    profile.parameters.clone(),
                )
                .map_err(|e| ApiError::InternalError(format!("生成草案失败: {}", e)))?;

            let mature_count = reschedule.mature_count;
            let immature_count = reschedule.immature_count;
            let total_capacity_used_t = reschedule.total_capacity_used;
            let overflow_days = reschedule.overflow_days;
            let reschedule_items = reschedule.plan_items;

            let mut draft_items_in_range: Vec<PlanItem> = Vec::with_capacity(
                frozen_items_in_range.len() + reschedule_items.len(),
            );

            for mut item in frozen_items_in_range.clone() {
                item.version_id = draft_id.clone();
                draft_items_in_range.push(item);
            }

            let frozen_items_count = frozen_items_in_range.len();
            let mut calc_items_count = 0usize;

            for mut item in reschedule_items.into_iter() {
                if item.plan_date < from || item.plan_date > to {
                    continue;
                }
                item.version_id = draft_id.clone();
                draft_items_in_range.push(item);
                calc_items_count += 1;
            }

            let (
                moved_count,
                added_count,
                removed_count,
                squeezed_out_count,
                diff_items,
                diff_items_total,
                diff_items_truncated,
            ) = Self::diff_plan_items_detail(&base_items_in_range, &draft_items_in_range);

            let summary = StrategyDraftSummary {
                draft_id: draft_id.clone(),
                base_version_id: base_version_id.to_string(),
                strategy: profile.strategy_key.clone(),
                plan_items_count: draft_items_in_range.len(),
                frozen_items_count,
                calc_items_count,
                mature_count,
                immature_count,
                total_capacity_used_t,
                overflow_days,
                moved_count,
                added_count,
                removed_count,
                squeezed_out_count,
                message: format!(
                    "{} | 排产{}(冻结{}+新排{}) | 成熟{} 未成熟{} | 预计产量{:.1}t | 超限机组日{} | 移动{} 新增{} 挤出{}",
                    profile.title_cn.as_str(),
                    draft_items_in_range.len(),
                    frozen_items_count,
                    calc_items_count,
                    mature_count,
                    immature_count,
                    total_capacity_used_t,
                    overflow_days,
                    moved_count,
                    added_count,
                    squeezed_out_count
                ),
            };

            let params_json = profile.parameters_json();
            let params_json = if params_json.is_null() {
                None
            } else {
                Some(params_json.to_string())
            };

            let summary_json = serde_json::to_string(&summary)
                .map_err(|e| ApiError::InternalError(format!("序列化草案摘要失败: {}", e)))?;
            let diff_items_json = serde_json::to_string(&diff_items)
                .map_err(|e| ApiError::InternalError(format!("序列化草案变更明细失败: {}", e)))?;

            let entity = StrategyDraftEntity {
                draft_id: draft_id.clone(),
                base_version_id: base_version_id.to_string(),
                plan_date_from: from,
                plan_date_to: to,
                strategy_key: profile.strategy_key.clone(),
                strategy_base: profile.base_strategy.as_str().to_string(),
                strategy_title_cn: profile.title_cn.clone(),
                strategy_params_json: params_json,
                status: StrategyDraftStatus::Draft,
                created_by: operator.to_string(),
                created_at: now,
                expires_at,
                published_as_version_id: None,
                published_by: None,
                published_at: None,
                locked_by: None,
                locked_at: None,
                summary_json,
                diff_items_json,
                diff_items_total: diff_items_total as i64,
                diff_items_truncated,
            };

            self.strategy_draft_repo
                .insert(&entity)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

            summaries.push(summary);
        }

        let draft_count = summaries.len();

        Ok(GenerateStrategyDraftsResponse {
            base_version_id: base_version_id.to_string(),
            plan_date_from: from,
            plan_date_to: to,
            drafts: summaries,
            message: format!("已生成{}个策略草案", draft_count),
        })
    }

    /// 发布策略草案：生成正式版本（落库）
    pub fn apply_strategy_draft(
        &self,
        draft_id: &str,
        operator: &str,
    ) -> ApiResult<ApplyStrategyDraftResponse> {
        if draft_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("草案ID不能为空".to_string()));
        }
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }

        // best-effort: 先尝试将过期草案标记为 EXPIRED，避免误发布
        if let Err(e) = self.strategy_draft_repo.expire_if_needed(draft_id) {
            tracing::warn!("expire_if_needed failed: {}", e);
        }

        let record = self
            .strategy_draft_repo
            .find_by_id(draft_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("草案{}不存在或已过期", draft_id)))?;

        if record.status != StrategyDraftStatus::Draft {
            return Err(ApiError::InvalidInput(format!(
                "草案状态不允许发布: {}",
                record.status.as_str()
            )));
        }

        // 并发保护：发布前加锁（best-effort）
        let lock_rows = self
            .strategy_draft_repo
            .try_lock_for_publish(draft_id, operator)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        if lock_rows == 0 {
            return Err(ApiError::VersionConflict(
                "草案已被其他用户锁定、已过期或状态已变更，请刷新后重试".to_string(),
            ));
        }

        // 校验基准版本仍为激活版本，避免基准漂移导致“发布结果不可复现”
        let base_version = self
            .plan_version_repo
            .find_by_id(&record.base_version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", record.base_version_id)))?;

        let active_version = self
            .plan_version_repo
            .find_active_version(&base_version.plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::InvalidInput("当前方案没有激活版本，无法发布草案".to_string()))?;

        if active_version.version_id != record.base_version_id {
            return Err(ApiError::VersionConflict(format!(
                "基准版本已变更：草案基于 {}，当前激活版本为 {}。请重新生成草案后再发布。",
                record.base_version_id, active_version.version_id
            )));
        }

        let window_days_i64 = (record.plan_date_to - record.plan_date_from).num_days();
        if window_days_i64 < 0 {
            return Err(ApiError::InvalidInput("草案日期范围非法".to_string()));
        }
        if window_days_i64 > 60 {
            return Err(ApiError::InvalidInput("草案时间跨度过大，最多支持60天".to_string()));
        }
        let window_days = window_days_i64 as i32;

        // 从草案快照中恢复策略 profile（避免发布时策略漂移导致不可复现）
        let base_strategy = record
            .strategy_base
            .parse::<ScheduleStrategy>()
            .map_err(|e| ApiError::InvalidInput(format!("草案策略解析失败: {}", e)))?;
        let parameters = match record.strategy_params_json.as_deref() {
            Some(raw) if !raw.trim().is_empty() && raw.trim() != "null" => {
                Some(serde_json::from_str(raw).map_err(|e| {
                    ApiError::InvalidInput(format!("草案参数解析失败: {}", e))
                })?)
            }
            _ => None,
        };
        let profile = ResolvedStrategyProfile {
            strategy_key: record.strategy_key.clone(),
            base_strategy,
            title_cn: record.strategy_title_cn.clone(),
            parameters,
        };

        let result = match self.recalc_engine.recalc_full_with_profile(
            &base_version.plan_id,
            record.plan_date_from,
            window_days,
            operator,
            false,
            profile.clone(),
        ) {
            Ok(v) => v,
            Err(e) => {
                // best-effort: 释放锁，避免草案长期处于 locked 状态
                if let Err(unlock_err) =
                    self.strategy_draft_repo.unlock(draft_id, operator)
                {
                    tracing::warn!("unlock draft failed: {}", unlock_err);
                }
                return Err(ApiError::InternalError(format!("发布草案失败: {}", e)));
            }
        };

        // 标记草案已发布（best-effort：版本已生成，失败也不应阻塞主流程）
        let published_at = chrono::Local::now().naive_local();
        if let Err(e) = self.strategy_draft_repo.mark_published(
            draft_id,
            &result.version_id,
            operator,
            published_at,
        ) {
            tracing::warn!("mark_published failed: {}", e);
        }

        // 审计记录：发布草案属于“决策行为”，需要落 ActionLog
        let log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(result.version_id.clone()),
            action_type: "APPLY_STRATEGY_DRAFT".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "draft_id": draft_id,
                "base_version_id": record.base_version_id,
                "plan_date_from": record.plan_date_from.to_string(),
                "plan_date_to": record.plan_date_to.to_string(),
                "window_days": window_days,
                "strategy": profile.strategy_key,
                "strategy_base": profile.base_strategy.as_str(),
                "strategy_title_cn": profile.title_cn,
                "parameters": profile.parameters_json(),
            })),
            impact_summary_json: Some(serde_json::json!({
                "plan_items_count": result.total_items,
                "frozen_items_count": result.frozen_items,
                "mature_count": result.mature_count,
                "immature_count": result.immature_count,
                "elapsed_ms": result.elapsed_ms,
            })),
            machine_code: None,
            date_range_start: Some(record.plan_date_from),
            date_range_end: Some(record.plan_date_to),
            detail: Some(format!("发布策略草案: {} ({})", record.strategy_title_cn.as_str(), draft_id)),
        };

        self.action_log_repo
            .insert(&log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(ApplyStrategyDraftResponse {
            version_id: result.version_id,
            success: true,
            message: "草案已发布，已生成正式版本".to_string(),
        })
    }

    /// 查询策略草案变更明细（用于前端解释对比）
    pub fn get_strategy_draft_detail(
        &self,
        draft_id: &str,
    ) -> ApiResult<GetStrategyDraftDetailResponse> {
        if draft_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("草案ID不能为空".to_string()));
        }

        // best-effort: 先尝试将过期草案标记为 EXPIRED
        if let Err(e) = self.strategy_draft_repo.expire_if_needed(draft_id) {
            tracing::warn!("expire_if_needed failed: {}", e);
        }

        let record = self
            .strategy_draft_repo
            .find_by_id(draft_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("草案{}不存在或已过期", draft_id)))?;

        let mut diff_items: Vec<StrategyDraftDiffItem> = serde_json::from_str(&record.diff_items_json)
            .map_err(|e| ApiError::InternalError(format!("解析草案变更明细失败: {}", e)))?;

        let mut message = if record.diff_items_truncated {
            format!(
                "变更明细过多，已截断展示 {}/{} 条",
                diff_items.len(),
                record.diff_items_total
            )
        } else {
            "OK".to_string()
        };

        // best-effort: 为“挤出”项补充 material_state 快照，减少前端逐条查库
        let squeezed_ids: Vec<String> = diff_items
            .iter()
            .filter(|it| it.change_type == "SQUEEZED_OUT")
            .map(|it| it.material_id.clone())
            .collect();
        if !squeezed_ids.is_empty() {
            match self
                .material_state_repo
                .find_snapshots_by_material_ids(&squeezed_ids)
            {
                Ok(list) => {
                    let map: HashMap<String, MaterialStateSnapshotLite> = list
                        .into_iter()
                        .map(|s| (s.material_id.clone(), s))
                        .collect();
                    for it in diff_items.iter_mut() {
                        if it.change_type != "SQUEEZED_OUT" {
                            continue;
                        }
                        it.material_state_snapshot = map.get(&it.material_id).cloned();
                    }
                }
                Err(e) => {
                    message = format!("{}（material_state 快照加载失败：{}）", message, e);
                }
            }
        }

        Ok(GetStrategyDraftDetailResponse {
            draft_id: draft_id.to_string(),
            base_version_id: record.base_version_id,
            plan_date_from: record.plan_date_from,
            plan_date_to: record.plan_date_to,
            strategy: record.strategy_key,
            diff_items,
            diff_items_total: record.diff_items_total as usize,
            diff_items_truncated: record.diff_items_truncated,
            message,
        })
    }

    /// 列出并恢复指定基准版本 + 日期范围内的草案（默认：每个策略仅返回最新一条）
    pub fn list_strategy_drafts(
        &self,
        base_version_id: &str,
        plan_date_from: NaiveDate,
        plan_date_to: NaiveDate,
        status_filter: Option<String>,
        limit: Option<i64>,
    ) -> ApiResult<ListStrategyDraftsResponse> {
        if base_version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("基准版本ID不能为空".to_string()));
        }

        let (from, to) = if plan_date_to < plan_date_from {
            (plan_date_to, plan_date_from)
        } else {
            (plan_date_from, plan_date_to)
        };

        let range_days = (to - from).num_days();
        if range_days > 60 {
            return Err(ApiError::InvalidInput("时间跨度过大，最多支持60天".to_string()));
        }

        let status = status_filter
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(StrategyDraftStatus::parse);

        let rows = self
            .strategy_draft_repo
            .list_by_base_version_and_range(base_version_id, from, to, status, limit.unwrap_or(200))
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let now = chrono::Local::now().naive_local();
        let mut seen: HashSet<String> = HashSet::new();
        let mut drafts: Vec<StrategyDraftSummary> = Vec::new();
        let mut expired_count = 0usize;
        let mut parse_failed = 0usize;

        for record in rows.into_iter() {
            // best-effort: 避免把已过期但未标记的草案返回给前端
            if record.status == StrategyDraftStatus::Draft && record.expires_at <= now {
                expired_count += 1;
                let _ = self.strategy_draft_repo.expire_if_needed(&record.draft_id);
                continue;
            }

            // 每个策略只取最新一条（query 已按 created_at DESC 排序）
            if !seen.insert(record.strategy_key.clone()) {
                continue;
            }

            match serde_json::from_str::<StrategyDraftSummary>(&record.summary_json) {
                Ok(mut summary) => {
                    // 防御：以 DB 为准覆盖关键字段，避免历史数据格式漂移
                    summary.draft_id = record.draft_id;
                    summary.base_version_id = record.base_version_id;
                    summary.strategy = record.strategy_key;
                    drafts.push(summary);
                }
                Err(e) => {
                    parse_failed += 1;
                    tracing::warn!(
                        "failed to parse decision_strategy_draft.summary_json: draft_id={}, err={}",
                        record.draft_id,
                        e
                    );
                }
            }
        }

        let mut message = format!("已找到{}个草案", drafts.len());
        if expired_count > 0 {
            message = format!("{}（{}个已过期）", message, expired_count);
        }
        if parse_failed > 0 {
            message = format!("{}（{}个解析失败）", message, parse_failed);
        }

        Ok(ListStrategyDraftsResponse {
            base_version_id: base_version_id.to_string(),
            plan_date_from: from,
            plan_date_to: to,
            drafts,
            message,
        })
    }

    /// 清理过期草案（默认保留 7 天，最大 90 天）
    pub fn cleanup_expired_strategy_drafts(
        &self,
        keep_days: i64,
    ) -> ApiResult<CleanupStrategyDraftsResponse> {
        let deleted_count = self
            .strategy_draft_repo
            .cleanup_expired(keep_days)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(CleanupStrategyDraftsResponse {
            deleted_count,
            message: format!("已清理{}条过期草案", deleted_count),
        })
    }

    /// 获取预设策略列表（用于前端展示/默认对比）
    pub fn get_strategy_presets(&self) -> ApiResult<Vec<StrategyPreset>> {
        Ok(vec![
            StrategyPreset {
                strategy: ScheduleStrategy::Balanced,
                title: "均衡方案".to_string(),
                description: "在交付/产能/库存之间保持均衡".to_string(),
                default_parameters: serde_json::json!({}),
            },
            StrategyPreset {
                strategy: ScheduleStrategy::UrgentFirst,
                title: "紧急优先".to_string(),
                description: "优先保障 L3/L2 紧急订单".to_string(),
                default_parameters: serde_json::json!({}),
            },
            StrategyPreset {
                strategy: ScheduleStrategy::CapacityFirst,
                title: "产能优先".to_string(),
                description: "优先提升产能利用率，减少溢出".to_string(),
                default_parameters: serde_json::json!({}),
            },
            StrategyPreset {
                strategy: ScheduleStrategy::ColdStockFirst,
                title: "冷坨消化".to_string(),
                description: "优先消化冷坨/压库物料".to_string(),
                default_parameters: serde_json::json!({}),
            },
        ])
    }

    fn diff_plan_items(
        items_a: &[PlanItem],
        items_b: &[PlanItem],
    ) -> (usize, usize, usize, usize) {
        let (moved_count, added_count, removed_count, squeezed_out_count, _, _, _) =
            Self::diff_plan_items_detail(items_a, items_b);
        (moved_count, added_count, removed_count, squeezed_out_count)
    }

    fn diff_plan_items_detail(
        items_a: &[PlanItem],
        items_b: &[PlanItem],
    ) -> (
        usize,
        usize,
        usize,
        usize,
        Vec<StrategyDraftDiffItem>,
        usize,
        bool,
    ) {
        const MAX_DIFF_ITEMS: usize = 5000;

        // 逻辑保持与 compare_versions 一致：只统计 moved/added/removed/squeezed_out
        let map_a: HashMap<String, &PlanItem> = items_a
            .iter()
            .map(|item| (item.material_id.clone(), item))
            .collect();
        let map_b: HashMap<String, &PlanItem> = items_b
            .iter()
            .map(|item| (item.material_id.clone(), item))
            .collect();

        let mut moved_count = 0usize;
        let mut added_count = 0usize;
        let mut squeezed_out_count = 0usize;
        let mut diff_items: Vec<StrategyDraftDiffItem> = Vec::new();

        for (material_id, item_a) in map_a.iter() {
            if let Some(item_b) = map_b.get(material_id) {
                if item_a.plan_date != item_b.plan_date || item_a.machine_code != item_b.machine_code
                {
                    moved_count += 1;
                    diff_items.push(StrategyDraftDiffItem {
                        material_id: material_id.clone(),
                        change_type: "MOVED".to_string(),
                        from_plan_date: Some(item_a.plan_date),
                        from_machine_code: Some(item_a.machine_code.clone()),
                        from_seq_no: Some(item_a.seq_no),
                        to_plan_date: Some(item_b.plan_date),
                        to_machine_code: Some(item_b.machine_code.clone()),
                        to_seq_no: Some(item_b.seq_no),
                        to_assign_reason: item_b.assign_reason.clone(),
                        to_urgent_level: item_b.urgent_level.clone(),
                        to_sched_state: item_b.sched_state.clone(),
                        material_state_snapshot: None,
                    });
                }
            } else {
                squeezed_out_count += 1;
                diff_items.push(StrategyDraftDiffItem {
                    material_id: material_id.clone(),
                    change_type: "SQUEEZED_OUT".to_string(),
                    from_plan_date: Some(item_a.plan_date),
                    from_machine_code: Some(item_a.machine_code.clone()),
                    from_seq_no: Some(item_a.seq_no),
                    to_plan_date: None,
                    to_machine_code: None,
                    to_seq_no: None,
                    to_assign_reason: None,
                    to_urgent_level: None,
                    to_sched_state: None,
                    material_state_snapshot: None,
                });
            }
        }

        for (material_id, item_b) in map_b.iter() {
            if !map_a.contains_key(material_id) {
                added_count += 1;
                diff_items.push(StrategyDraftDiffItem {
                    material_id: material_id.clone(),
                    change_type: "ADDED".to_string(),
                    from_plan_date: None,
                    from_machine_code: None,
                    from_seq_no: None,
                    to_plan_date: Some(item_b.plan_date),
                    to_machine_code: Some(item_b.machine_code.clone()),
                    to_seq_no: Some(item_b.seq_no),
                    to_assign_reason: item_b.assign_reason.clone(),
                    to_urgent_level: item_b.urgent_level.clone(),
                    to_sched_state: item_b.sched_state.clone(),
                    material_state_snapshot: None,
                });
            }
        }

        // 固定排序：变更类型 -> 日期 -> 机组 -> material_id
        let type_rank = |t: &str| match t {
            "MOVED" => 0i32,
            "ADDED" => 1i32,
            "SQUEEZED_OUT" => 2i32,
            _ => 9i32,
        };

        diff_items.sort_by(|a, b| {
            let ra = type_rank(&a.change_type);
            let rb = type_rank(&b.change_type);
            if ra != rb {
                return ra.cmp(&rb);
            }

            let da = a.to_plan_date.or(a.from_plan_date);
            let db = b.to_plan_date.or(b.from_plan_date);
            if da != db {
                return da.cmp(&db);
            }

            let ma = a
                .to_machine_code
                .as_deref()
                .or(a.from_machine_code.as_deref())
                .unwrap_or("");
            let mb = b
                .to_machine_code
                .as_deref()
                .or(b.from_machine_code.as_deref())
                .unwrap_or("");
            if ma != mb {
                return ma.cmp(mb);
            }

            a.material_id.cmp(&b.material_id)
        });

        let diff_items_total = diff_items.len();
        let diff_items_truncated = diff_items_total > MAX_DIFF_ITEMS;
        if diff_items_truncated {
            diff_items.truncate(MAX_DIFF_ITEMS);
        }

        let removed_count = squeezed_out_count;
        (
            moved_count,
            added_count,
            removed_count,
            squeezed_out_count,
            diff_items,
            diff_items_total,
            diff_items_truncated,
        )
    }

    // ==========================================
    // 明细查询接口
    // ==========================================

    /// 查询排产明细（按版本）
    ///
    /// # 参数
    /// - version_id: 版本ID
    ///
    /// # 返回
    /// - Ok(Vec<PlanItem>): 排产明细列表
    /// - Err(ApiError): API错误
    pub fn list_plan_items(&self, version_id: &str) -> ApiResult<Vec<PlanItem>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        let mut items = self.plan_item_repo
            .find_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        self.enrich_plan_items(&mut items);
        Ok(items)
    }

    /// 查询排产明细（可选过滤 + 分页）
    ///
    /// 说明：
    /// - 该接口用于“增量加载/按时间窗加载”，避免前端一次性拉取全量 plan_item；
    /// - 不改变旧接口 `list_plan_items` 的语义，便于逐步迁移。
    pub fn list_plan_items_filtered(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        plan_date_from: Option<NaiveDate>,
        plan_date_to: Option<NaiveDate>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> ApiResult<Vec<PlanItem>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        if let Some(limit) = limit {
            if limit <= 0 || limit > 20_000 {
                return Err(ApiError::InvalidInput(
                    "limit必须在1-20000之间".to_string(),
                ));
            }
        }
        if let Some(offset) = offset {
            if offset < 0 {
                return Err(ApiError::InvalidInput("offset不能为负数".to_string()));
            }
        }

        self.plan_item_repo
            .find_by_filters_paged(
                version_id,
                machine_code,
                plan_date_from,
                plan_date_to,
                limit,
                offset,
            )
            .map(|mut items| {
                self.enrich_plan_items(&mut items);
                items
            })
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 查询排产明细（按日期）
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - plan_date: 排产日期
    ///
    /// # 返回
    /// - Ok(Vec<PlanItem>): 排产明细列表
    /// - Err(ApiError): API错误
    pub fn list_items_by_date(
        &self,
        version_id: &str,
        plan_date: NaiveDate,
    ) -> ApiResult<Vec<PlanItem>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        self.plan_item_repo
            .find_by_date(version_id, plan_date)
            .map(|mut items| {
                self.enrich_plan_items(&mut items);
                items
            })
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 从 material_state 和 material_master 批量补充排产明细的快照字段
    ///
    /// 补充字段：urgent_level, sched_state (来自 material_state)，steel_grade (来自 material_master.steel_mark)
    fn enrich_plan_items(&self, items: &mut [PlanItem]) {
        if items.is_empty() {
            return;
        }

        let material_ids: Vec<String> = items.iter().map(|it| it.material_id.clone()).collect();

        // 1. 从 material_state 获取 urgent_level, sched_state
        if let Ok(snapshots) = self.material_state_repo.find_snapshots_by_material_ids(&material_ids) {
            let state_map: HashMap<String, MaterialStateSnapshotLite> = snapshots
                .into_iter()
                .map(|s| (s.material_id.clone(), s))
                .collect();

            for item in items.iter_mut() {
                if let Some(snap) = state_map.get(&item.material_id) {
                    if item.urgent_level.is_none() {
                        item.urgent_level = snap.urgent_level.clone();
                    }
                    if item.sched_state.is_none() {
                        item.sched_state = snap.sched_state.clone();
                    }
                }
            }
        }

        // 2. 从 material_master 获取 steel_mark → steel_grade
        if let Ok(steel_map) = self.material_master_repo.find_steel_marks_by_ids(&material_ids) {
            for item in items.iter_mut() {
                if item.steel_grade.is_none() {
                    if let Some(mark) = steel_map.get(&item.material_id) {
                        item.steel_grade = Some(mark.clone());
                    }
                }
            }
        }
    }

    // ==========================================
    // 版本对比接口
    // ==========================================

    /// 版本对比
    ///
    /// # 参数
    /// - version_id_a: 版本A ID
    /// - version_id_b: 版本B ID
    ///
    /// # 返回
    /// - Ok(VersionComparisonResult): 对比结果
    /// - Err(ApiError): API错误
    pub fn compare_versions(
        &self,
        version_id_a: &str,
        version_id_b: &str,
    ) -> ApiResult<VersionComparisonResult> {
        // 参数验证
        if version_id_a.trim().is_empty() || version_id_b.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        // 1. 加载两个版本的排产明细
        let items_a = self.plan_item_repo.find_by_version(version_id_a)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        let items_b = self.plan_item_repo.find_by_version(version_id_b)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 2. 构建材料ID到PlanItem的映射
        use std::collections::HashMap;
        let map_a: HashMap<String, &crate::domain::plan::PlanItem> = items_a
            .iter()
            .map(|item| (item.material_id.clone(), item))
            .collect();
        let map_b: HashMap<String, &crate::domain::plan::PlanItem> = items_b
            .iter()
            .map(|item| (item.material_id.clone(), item))
            .collect();

        // 3. 计算差异
        let mut moved_count = 0;
        let mut added_count = 0;
        let mut squeezed_out_count = 0;

        // 遍历版本A的材料
        for (material_id, item_a) in map_a.iter() {
            if let Some(item_b) = map_b.get(material_id) {
                // 材料在两个版本中都存在
                if item_a.plan_date != item_b.plan_date || item_a.machine_code != item_b.machine_code {
                    // 日期或机组变化 = 移动
                    moved_count += 1;
                }
            } else {
                // 材料只在A中，不在B中 = 被挤出
                squeezed_out_count += 1;
            }
        }

        // 遍历版本B的材料
        for (material_id, _item_b) in map_b.iter() {
            if !map_a.contains_key(material_id) {
                // 材料只在B中，不在A中 = 新增
                added_count += 1;
            }
        }

        // 删除数量 = 被挤出数量
        let removed_count = squeezed_out_count;

        // 4. 加载两个版本信息（用于配置对比）
        let version_a = self.plan_version_repo.find_by_id(version_id_a)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id_a)))?;
        let version_b = self.plan_version_repo.find_by_id(version_id_b)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id_b)))?;

        // 5. 对比配置快照
        let config_changes = self.compare_config_snapshots(
            version_a.config_snapshot_json.as_deref(),
            version_b.config_snapshot_json.as_deref(),
        )?;

        // 6. 对比风险快照（TODO: 需要RiskSnapshotRepository支持）
        let risk_delta = None; // 暂时返回None，待RiskEngine实现后补充

        // 7. 对比产能变化（TODO: 需要CapacityPoolRepository支持）
        let capacity_delta = None; // 暂时返回None，待实现后补充

        Ok(VersionComparisonResult {
            version_id_a: version_id_a.to_string(),
            version_id_b: version_id_b.to_string(),
            moved_count,
            added_count,
            removed_count,
            squeezed_out_count,
            risk_delta,
            capacity_delta,
            config_changes,
            message: format!(
                "版本对比完成: 移动{}个, 新增{}个, 删除{}个, 挤出{}个",
                moved_count, added_count, removed_count, squeezed_out_count
            ),
        })
    }

    /// 版本对比 KPI 汇总（聚合接口，避免前端全量拉取 plan_item 再本地计算）
    ///
    /// 说明：
    /// - plan_item 侧：使用 SQL 聚合（count/sum/min/max + diff counts）
    /// - risk_snapshot 侧：基于既有读模型聚合（mature/immature、overflow_days/overflow_t 等）
    pub fn compare_versions_kpi(
        &self,
        version_id_a: &str,
        version_id_b: &str,
    ) -> ApiResult<VersionComparisonKpiResult> {
        if version_id_a.trim().is_empty() || version_id_b.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        // 版本存在性校验（避免 silent 0）
        let _version_a = self
            .plan_version_repo
            .find_by_id(version_id_a)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id_a)))?;

        let _version_b = self
            .plan_version_repo
            .find_by_id(version_id_b)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id_b)))?;

        let agg_a = self
            .plan_item_repo
            .get_version_agg(version_id_a)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        let agg_b = self
            .plan_item_repo
            .get_version_agg(version_id_b)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let diff_counts = self
            .plan_item_repo
            .get_versions_diff_counts(version_id_a, version_id_b)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let build_risk_kpi = |version_id: &str| -> ApiResult<VersionRiskKpi> {
            let snapshots = self
                .risk_snapshot_repo
                .find_by_version_id(version_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

            if snapshots.is_empty() {
                return Ok(VersionRiskKpi::empty());
            }

            let mut snapshot_date_from: Option<NaiveDate> = None;
            let mut snapshot_date_to: Option<NaiveDate> = None;
            let mut overflow_dates: HashSet<NaiveDate> = HashSet::new();

            let mut overflow_t = 0.0;
            let mut used_capacity_t = 0.0;
            let mut target_capacity_t = 0.0;
            let mut limit_capacity_t = 0.0;
            let mut mature_backlog_t = 0.0;
            let mut immature_backlog_t = 0.0;
            let mut urgent_total_t = 0.0;

            for s in snapshots.iter() {
                snapshot_date_from = match snapshot_date_from {
                    Some(d) => Some(std::cmp::min(d, s.snapshot_date)),
                    None => Some(s.snapshot_date),
                };
                snapshot_date_to = match snapshot_date_to {
                    Some(d) => Some(std::cmp::max(d, s.snapshot_date)),
                    None => Some(s.snapshot_date),
                };

                if s.overflow_t > 0.0 {
                    overflow_dates.insert(s.snapshot_date);
                }

                overflow_t += s.overflow_t;
                used_capacity_t += s.used_capacity_t;
                target_capacity_t += s.target_capacity_t;
                limit_capacity_t += s.limit_capacity_t;
                mature_backlog_t += s.mature_backlog_t;
                immature_backlog_t += s.immature_backlog_t;
                urgent_total_t += s.urgent_total_t;
            }

            let capacity_util_pct = if target_capacity_t > 0.0 {
                (used_capacity_t / target_capacity_t) * 100.0
            } else {
                0.0
            };

            Ok(VersionRiskKpi {
                overflow_days: overflow_dates.len(),
                overflow_t,
                used_capacity_t,
                target_capacity_t,
                limit_capacity_t,
                capacity_util_pct,
                mature_backlog_t,
                immature_backlog_t,
                urgent_total_t,
                snapshot_date_from,
                snapshot_date_to,
            })
        };

        let risk_a = build_risk_kpi(version_id_a)?;
        let risk_b = build_risk_kpi(version_id_b)?;

        let missing_risk_snapshot = risk_a.is_empty() || risk_b.is_empty();
        let message = if missing_risk_snapshot {
            "KPI 汇总完成（部分版本缺少 risk_snapshot，相关指标将返回 null）".to_string()
        } else {
            "KPI 汇总完成".to_string()
        };

        Ok(VersionComparisonKpiResult {
            version_id_a: version_id_a.to_string(),
            version_id_b: version_id_b.to_string(),
            kpi_a: VersionKpiSummary::from_aggs(agg_a, risk_a),
            kpi_b: VersionKpiSummary::from_aggs(agg_b, risk_b),
            diff_counts: VersionDiffCounts {
                moved_count: diff_counts.moved_count,
                added_count: diff_counts.added_count,
                removed_count: diff_counts.removed_count,
                squeezed_out_count: diff_counts.squeezed_out_count,
            },
            message,
        })
    }

    /// 对比配置快照
    ///
    /// # 参数
    /// - snapshot_a: 版本A的配置快照JSON
    /// - snapshot_b: 版本B的配置快照JSON
    ///
    /// # 返回
    /// - Ok(Option<Vec<ConfigChange>>): 配置变化列表
    /// - Err(ApiError): 解析失败
    fn compare_config_snapshots(
        &self,
        snapshot_a: Option<&str>,
        snapshot_b: Option<&str>,
    ) -> ApiResult<Option<Vec<ConfigChange>>> {
        use std::collections::HashMap;

        // 如果两个快照都不存在，返回None
        if snapshot_a.is_none() && snapshot_b.is_none() {
            return Ok(None);
        }

        // 解析快照A
        let config_a: HashMap<String, String> = if let Some(json) = snapshot_a {
            serde_json::from_str(json)
                .map_err(|e| ApiError::InvalidInput(format!("解析配置快照A失败: {}", e)))?
        } else {
            HashMap::new()
        };

        // 解析快照B
        let config_b: HashMap<String, String> = if let Some(json) = snapshot_b {
            serde_json::from_str(json)
                .map_err(|e| ApiError::InvalidInput(format!("解析配置快照B失败: {}", e)))?
        } else {
            HashMap::new()
        };

        // 过滤元信息字段（例如版本中文命名），避免污染“配置差异”视图。
        let mut config_a = config_a;
        let mut config_b = config_b;
        config_a.retain(|k, _| !k.starts_with("__meta_"));
        config_b.retain(|k, _| !k.starts_with("__meta_"));

        // 收集所有配置键
        let mut all_keys: std::collections::HashSet<String> = config_a.keys().cloned().collect();
        all_keys.extend(config_b.keys().cloned());

        // 对比配置
        let mut changes = Vec::new();
        for key in all_keys {
            let value_a = config_a.get(&key).cloned();
            let value_b = config_b.get(&key).cloned();

            // 只记录有变化的配置
            if value_a != value_b {
                changes.push(ConfigChange {
                    key: key.clone(),
                    value_a,
                    value_b,
                });
            }
        }

        if changes.is_empty() {
            Ok(None)
        } else {
            Ok(Some(changes))
        }
    }

    // ==========================================
    // 排产操作接口
    // ==========================================

    /// 移动排产项
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - moves: 移动项列表
    /// - mode: 校验模式 (STRICT/AUTO_FIX)
    ///
    /// # 返回
    /// - Ok(MoveItemsResponse): 移动结果
    /// - Err(ApiError): API错误
    ///
    /// # 红线合规
    /// - 红线1: 冻结区材料不可移动
    /// - 红线2: 非适温材料不可移动到当日
    pub fn move_items(
        &self,
        version_id: &str,
        moves: Vec<MoveItemRequest>,
        mode: crate::api::ValidationMode,
        operator: &str,
        reason: Option<&str>,
    ) -> ApiResult<MoveItemsResponse> {
        use std::collections::HashMap;

        // 参数验证
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if moves.is_empty() {
            return Err(ApiError::InvalidInput("移动项列表不能为空".to_string()));
        }

        // 1. 验证版本存在且为草稿状态
        let version = self.plan_version_repo.find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        if !version.is_draft() && !version.is_active() {
            return Err(ApiError::BusinessRuleViolation(
                "只能修改草稿或激活状态的版本".to_string()
            ));
        }

        // 2. 加载当前版本的所有排产明细
        let current_items = self.plan_item_repo.find_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 构建材料ID到PlanItem的映射
        let item_map: HashMap<String, PlanItem> = current_items
            .into_iter()
            .map(|item| (item.material_id.clone(), item))
            .collect();

        // 3. 处理每个移动请求
        let mut results = Vec::new();
        let mut success_count = 0;
        let mut failed_count = 0;
        let mut has_violations = false;
        let mut items_to_update = Vec::new();

        for move_req in moves {
            // 解析目标日期
            let to_date = match NaiveDate::parse_from_str(&move_req.to_date, "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => {
                    failed_count += 1;
                    results.push(MoveItemResult {
                        material_id: move_req.material_id.clone(),
                        success: false,
                        from_date: None,
                        from_machine: None,
                        to_date: move_req.to_date.clone(),
                        to_machine: move_req.to_machine.clone(),
                        error: Some("日期格式错误，应为YYYY-MM-DD".to_string()),
                        violation_type: None,
                    });
                    continue;
                }
            };

            // 查找原排产项
            let original_item = match item_map.get(&move_req.material_id) {
                Some(item) => item,
                None => {
                    failed_count += 1;
                    results.push(MoveItemResult {
                        material_id: move_req.material_id.clone(),
                        success: false,
                        from_date: None,
                        from_machine: None,
                        to_date: move_req.to_date.clone(),
                        to_machine: move_req.to_machine.clone(),
                        error: Some(format!("材料{}在版本中不存在", move_req.material_id)),
                        violation_type: None,
                    });
                    continue;
                }
            };

            // 红线1: 检查冻结区保护
            if original_item.locked_in_plan {
                has_violations = true;
                match mode {
                    crate::api::ValidationMode::Strict => {
                        failed_count += 1;
                        results.push(MoveItemResult {
                            material_id: move_req.material_id.clone(),
                            success: false,
                            from_date: Some(original_item.plan_date.format("%Y-%m-%d").to_string()),
                            from_machine: Some(original_item.machine_code.clone()),
                            to_date: move_req.to_date.clone(),
                            to_machine: move_req.to_machine.clone(),
                            error: Some("冻结区材料不可移动".to_string()),
                            violation_type: Some("FROZEN_ZONE".to_string()),
                        });
                        continue;
                    }
                    crate::api::ValidationMode::AutoFix => {
                        // AutoFix 模式跳过冻结材料
                        results.push(MoveItemResult {
                            material_id: move_req.material_id.clone(),
                            success: false,
                            from_date: Some(original_item.plan_date.format("%Y-%m-%d").to_string()),
                            from_machine: Some(original_item.machine_code.clone()),
                            to_date: move_req.to_date.clone(),
                            to_machine: move_req.to_machine.clone(),
                            error: Some("冻结区材料已跳过".to_string()),
                            violation_type: Some("FROZEN_ZONE_SKIPPED".to_string()),
                        });
                        continue;
                    }
                }
            }

            // 创建更新后的排产项
            let updated_item = PlanItem {
                version_id: version_id.to_string(),
                material_id: move_req.material_id.clone(),
                machine_code: move_req.to_machine.clone(),
                plan_date: to_date,
                seq_no: move_req.to_seq,
                weight_t: original_item.weight_t,
                source_type: "MANUAL".to_string(), // 手动移动
                locked_in_plan: original_item.locked_in_plan,
                force_release_in_plan: original_item.force_release_in_plan,
                violation_flags: original_item.violation_flags.clone(),
                urgent_level: original_item.urgent_level.clone(),
                sched_state: original_item.sched_state.clone(),
                assign_reason: Some("MANUAL_MOVE".to_string()),
                steel_grade: original_item.steel_grade.clone(),
            };

            items_to_update.push(updated_item);
            success_count += 1;
            results.push(MoveItemResult {
                material_id: move_req.material_id.clone(),
                success: true,
                from_date: Some(original_item.plan_date.format("%Y-%m-%d").to_string()),
                from_machine: Some(original_item.machine_code.clone()),
                to_date: move_req.to_date.clone(),
                to_machine: move_req.to_machine.clone(),
                error: None,
                violation_type: None,
            });
        }

        // 4. 批量更新排产项
        if !items_to_update.is_empty() {
            self.plan_item_repo.batch_upsert(&items_to_update)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

            // 5. 记录操作日志
            let actor = if operator.trim().is_empty() { "system" } else { operator };
            let detail = match reason {
                Some(r) if !r.trim().is_empty() => format!("移动{}个排产项 | {}", success_count, r.trim()),
                _ => format!("移动{}个排产项", success_count),
            };

            let log = ActionLog {
                action_id: uuid::Uuid::new_v4().to_string(),
                version_id: Some(version_id.to_string()),
                action_type: "MOVE_ITEMS".to_string(),
                action_ts: chrono::Local::now().naive_local(),
                actor: actor.to_string(),
                payload_json: Some(serde_json::json!({
                    "success_count": success_count,
                    "failed_count": failed_count,
                    "has_violations": has_violations,
                    "reason": reason,
                    "moved_materials": items_to_update.iter().map(|i| &i.material_id).collect::<Vec<_>>(),
                })),
                impact_summary_json: None,
                machine_code: None,
                date_range_start: None,
                date_range_end: None,
                detail: Some(detail),
            };

            if let Err(e) = self.action_log_repo.insert(&log) {
                tracing::warn!("记录操作日志失败: {}", e);
            }

            // 6. 触发刷新事件
            let event = ScheduleEvent::full_scope(
                version_id.to_string(),
                ScheduleEventType::PlanItemChanged,
                Some("move_items".to_string()),
            );

            if let Err(e) = self.event_publisher.publish(event) {
                tracing::warn!("发布刷新事件失败: {}", e);
            }
        }

        Ok(MoveItemsResponse {
            version_id: version_id.to_string(),
            results,
            success_count,
            failed_count,
            has_violations,
            message: format!(
                "移动完成: 成功{}个, 失败{}个{}",
                success_count,
                failed_count,
                if has_violations { ", 存在违规" } else { "" }
            ),
        })
    }
}

// ==========================================
// DTO 类型定义
// ==========================================

/// 重算响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecalcResponse {
    /// 版本ID
    pub version_id: String,

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

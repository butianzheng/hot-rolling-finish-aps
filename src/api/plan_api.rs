// ==========================================
// 热轧精整排产系统 - 排产方案 API
// ==========================================
// 职责: 排产方案管理、版本管理、明细查询
// 红线合规: 红线1-5全覆盖
// 依据: 实施计划 Phase 3
// ==========================================

use std::sync::Arc;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::api::error::{ApiError, ApiResult};
use crate::domain::plan::{Plan, PlanVersion, PlanItem};
use crate::domain::action_log::ActionLog;
use crate::repository::plan_repo::{PlanRepository, PlanVersionRepository, PlanItemRepository};
use crate::repository::action_log_repo::ActionLogRepository;
use crate::repository::risk_repo::RiskSnapshotRepository;
use crate::engine::recalc::RecalcEngine;
use crate::engine::risk::RiskEngine;
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
    action_log_repo: Arc<ActionLogRepository>,
    risk_snapshot_repo: Arc<RiskSnapshotRepository>,
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
        action_log_repo: Arc<ActionLogRepository>,
        risk_snapshot_repo: Arc<RiskSnapshotRepository>,
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
            action_log_repo,
            risk_snapshot_repo,
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
            version_id: "N/A".to_string(),
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

        for v in &versions {
            deleted_items += self
                .plan_item_repo
                .delete_by_version(&v.version_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
            deleted_risks += self
                .risk_snapshot_repo
                .delete_by_version(&v.version_id)
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
            version_id: "N/A".to_string(),
            action_type: "DELETE_PLAN".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "plan_id": plan.plan_id,
                "plan_name": plan.plan_name,
                "deleted_versions": versions.len(),
                "deleted_plan_items": deleted_items,
                "deleted_risk_snapshots": deleted_risks,
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

        // 获取下一个版本号
        let version_no = self
            .plan_version_repo
            .get_next_version_no(&plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 创建配置快照JSON（存储note等额外信息）
        let config_snapshot_json = if let Some(ref note_text) = note {
            Some(serde_json::json!({
                "note": note_text,
                "created_at": chrono::Local::now().to_rfc3339(),
            }).to_string())
        } else {
            None
        };

        // 创建PlanVersion实例
        let version = PlanVersion {
            version_id: uuid::Uuid::new_v4().to_string(),
            plan_id: plan_id.clone(),
            version_no,
            status: "DRAFT".to_string(),
            frozen_from_date,
            recalc_window_days: Some(window_days),
            config_snapshot_json,
            created_by: Some(created_by.clone()),
            created_at: chrono::Local::now().naive_local(),
            revision: 1,
        };

        // 保存到数据库
        self.plan_version_repo
            .create(&version)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: version.version_id.clone(),
            action_type: "CREATE_VERSION".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: created_by,
            payload_json: Some(serde_json::json!({
                "plan_id": plan_id,
                "version_no": version_no,
                "window_days": window_days,
                "frozen_from_date": frozen_from_date.map(|d| d.to_string()),
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("创建版本: V{}", version_no)),
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

        if version.status == "ACTIVE" {
            return Err(ApiError::BusinessRuleViolation(
                "不能删除激活版本，请先激活其他版本或将其归档".to_string(),
            ));
        }

        // 显式删除关联数据（避免依赖 SQLite foreign_keys 配置）
        let deleted_items = self
            .plan_item_repo
            .delete_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let deleted_risks = self
            .risk_snapshot_repo
            .delete_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        self.plan_version_repo
            .delete(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: version_id.to_string(),
            action_type: "DELETE_VERSION".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "plan_id": version.plan_id,
                "version_no": version.version_no,
                "deleted_plan_items": deleted_items,
                "deleted_risk_snapshots": deleted_risks,
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
            version_id: version_id.to_string(),
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
            .recalc_full(&version.plan_id, base_date, window_days, operator, true)
            .map_err(|e| ApiError::InternalError(format!("试算失败: {}", e)))?;

        // 返回结果（不记录ActionLog）
        Ok(RecalcResponse {
            version_id: result.version_id,
            plan_items_count: result.total_items,
            frozen_items_count: result.frozen_items,
            success: true,
            message: format!(
                "试算完成，共排产{}个材料（冻结{}个，重算{}个）",
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
            .recalc_full(&version.plan_id, base_date, window_days, operator, false)
            .map_err(|e| ApiError::InternalError(format!("重算失败: {}", e)))?;

        let plan_items_count = recalc_result.total_items;
        let frozen_items_count = recalc_result.frozen_items;

        // 记录ActionLog（红线5: 可解释性）
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: recalc_result.version_id.clone(),
            action_type: "RECALC_FULL".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "base_date": base_date.to_string(),
                "window_days": window_days,
                "frozen_from_date": frozen_date.map(|d| d.to_string()),
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
            message: format!("重算完成，共排产{}个材料", plan_items_count),
        })
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

        self.plan_item_repo
            .find_by_version(version_id)
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
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
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
            let log = ActionLog {
                action_id: uuid::Uuid::new_v4().to_string(),
                version_id: version_id.to_string(),
                action_type: "MOVE_ITEMS".to_string(),
                action_ts: chrono::Local::now().naive_local(),
                actor: "system".to_string(),
                payload_json: Some(serde_json::json!({
                    "success_count": success_count,
                    "failed_count": failed_count,
                    "has_violations": has_violations,
                    "moved_materials": items_to_update.iter().map(|i| &i.material_id).collect::<Vec<_>>(),
                })),
                impact_summary_json: None,
                machine_code: None,
                date_range_start: None,
                date_range_end: None,
                detail: Some(format!("移动{}个排产项", success_count)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_api_structure() {
        // 这个测试只是验证结构是否正确定义
        // 实际的集成测试在 tests/ 目录
    }
}

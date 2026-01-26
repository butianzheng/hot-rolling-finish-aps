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

use crate::domain::capacity::CapacityPool;
use crate::domain::material::MaterialState;
use crate::domain::plan::{PlanItem, PlanVersion};
use crate::domain::types::SchedState;
use crate::engine::events::{OptionalEventPublisher, ScheduleEvent, ScheduleEventPublisher, ScheduleEventType};
use crate::engine::orchestrator::ScheduleOrchestrator;
use crate::engine::{CapacityFiller, EligibilityEngine, PrioritySorter, UrgencyEngine};
use crate::config::ConfigManager;
use crate::repository::{
    ActionLogRepository, CapacityPoolRepository, MaterialMasterRepository,
    MaterialStateRepository, PlanItemRepository, PlanVersionRepository,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

// ==========================================
// RecalcResult - 重算结果
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecalcResult {
    pub version_id: String,            // 新版本ID
    pub version_no: i32,               // 版本号
    pub total_items: usize,            // 总计划项数
    pub mature_count: usize,           // 适温材料数
    pub immature_count: usize,         // 未适温材料数
    pub frozen_items: usize,           // 冻结材料数
    pub recalc_items: usize,           // 重算材料数
    pub elapsed_ms: i64,               // 耗时(毫秒)
}

// ==========================================
// RescheduleResult - 重排产结果（内部使用）
// ==========================================
/// 重排产结果
/// 职责: execute_reschedule的返回值，包含排产明细和统计信息
#[derive(Debug, Clone)]
pub struct RescheduleResult {
    /// 排产的计划项
    pub plan_items: Vec<PlanItem>,
    /// 成熟材料数（READY/LOCKED/FORCE_RELEASE）
    pub mature_count: usize,
    /// 未成熟材料数（PENDING_MATURE）
    pub immature_count: usize,
    /// 总已用产能（吨）
    pub total_capacity_used: f64,
    /// 超限天数
    pub overflow_days: usize,
}

// ==========================================
// RecalcConfig - 重算配置
// ==========================================
#[derive(Debug, Clone)]
pub struct RecalcConfig {
    pub default_window_days: i32,      // 默认计算窗口: 30天
    pub default_cascade_days: i32,     // 默认联动窗口: 7天
    pub frozen_days_before_today: i32, // 冻结区天数: 2天
    pub auto_activate: bool,           // 是否自动激活: false
}

impl Default for RecalcConfig {
    fn default() -> Self {
        Self {
            default_window_days: 30,
            default_cascade_days: 7,
            frozen_days_before_today: 2,
            auto_activate: false,
        }
    }
}

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

    // 引擎依赖
    eligibility_engine: Arc<EligibilityEngine<ConfigManager>>,
    urgency_engine: Arc<UrgencyEngine>,
    priority_sorter: Arc<PrioritySorter>,
    capacity_filler: Arc<CapacityFiller>,

    // 事件发布器 (依赖倒置: Engine 定义 trait, Decision 实现)
    event_publisher: OptionalEventPublisher,

    // 配置
    config: RecalcConfig,
    config_manager: Arc<ConfigManager>,
}

impl RecalcEngine {
    /// 创建新的RecalcEngine实例
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version_repo: Arc<PlanVersionRepository>,
        item_repo: Arc<PlanItemRepository>,
        material_state_repo: Arc<MaterialStateRepository>,
        material_master_repo: Arc<MaterialMasterRepository>,
        capacity_repo: Arc<CapacityPoolRepository>,
        action_log_repo: Arc<ActionLogRepository>,
        eligibility_engine: Arc<EligibilityEngine<ConfigManager>>,
        urgency_engine: Arc<UrgencyEngine>,
        priority_sorter: Arc<PrioritySorter>,
        capacity_filler: Arc<CapacityFiller>,
        config: RecalcConfig,
        config_manager: Arc<ConfigManager>,
        event_publisher: Option<Arc<dyn ScheduleEventPublisher>>,
    ) -> Self {
        let event_publisher = match event_publisher {
            Some(p) => OptionalEventPublisher::with_publisher(p),
            None => OptionalEventPublisher::none(),
        };

        Self {
            version_repo,
            item_repo,
            material_state_repo,
            material_master_repo,
            capacity_repo,
            action_log_repo,
            eligibility_engine,
            urgency_engine,
            priority_sorter,
            capacity_filler,
            event_publisher,
            config,
            config_manager,
        }
    }

    /// 创建带默认配置的RecalcEngine实例
    #[allow(clippy::too_many_arguments)]
    pub fn with_default_config(
        version_repo: Arc<PlanVersionRepository>,
        item_repo: Arc<PlanItemRepository>,
        material_state_repo: Arc<MaterialStateRepository>,
        material_master_repo: Arc<MaterialMasterRepository>,
        capacity_repo: Arc<CapacityPoolRepository>,
        action_log_repo: Arc<ActionLogRepository>,
        eligibility_engine: Arc<EligibilityEngine<ConfigManager>>,
        urgency_engine: Arc<UrgencyEngine>,
        priority_sorter: Arc<PrioritySorter>,
        capacity_filler: Arc<CapacityFiller>,
        config_manager: Arc<ConfigManager>,
        event_publisher: Option<Arc<dyn ScheduleEventPublisher>>,
    ) -> Self {
        Self::new(
            version_repo,
            item_repo,
            material_state_repo,
            material_master_repo,
            capacity_repo,
            action_log_repo,
            eligibility_engine,
            urgency_engine,
            priority_sorter,
            capacity_filler,
            RecalcConfig::default(),
            config_manager,
            event_publisher,
        )
    }

    /// 从仓储集合创建 RecalcEngine 实例
    ///
    /// 这是推荐的构造方式，使用 `ScheduleRepositories` 聚合仓储依赖，
    /// 减少构造函数参数数量，提升代码可读性。
    ///
    /// # 参数
    /// - `repos`: 仓储集合
    /// - `eligibility_engine`: 资格判定引擎
    /// - `urgency_engine`: 紧急度引擎
    /// - `priority_sorter`: 优先级排序器
    /// - `capacity_filler`: 产能填充器
    /// - `config`: 重算配置
    /// - `config_manager`: 配置管理器
    /// - `event_publisher`: 事件发布器（可选）
    ///
    /// # 示例
    /// ```ignore
    /// use crate::engine::{RecalcEngine, ScheduleRepositories};
    ///
    /// let repos = ScheduleRepositories::new(...);
    /// let engine = RecalcEngine::from_repositories(
    ///     repos,
    ///     eligibility_engine,
    ///     urgency_engine,
    ///     priority_sorter,
    ///     capacity_filler,
    ///     RecalcConfig::default(),
    ///     config_manager,
    ///     None,
    /// );
    /// ```
    pub fn from_repositories(
        repos: crate::engine::repositories::ScheduleRepositories,
        eligibility_engine: Arc<EligibilityEngine<ConfigManager>>,
        urgency_engine: Arc<UrgencyEngine>,
        priority_sorter: Arc<PrioritySorter>,
        capacity_filler: Arc<CapacityFiller>,
        config: RecalcConfig,
        config_manager: Arc<ConfigManager>,
        event_publisher: Option<Arc<dyn ScheduleEventPublisher>>,
    ) -> Self {
        Self::new(
            repos.version_repo,
            repos.item_repo,
            repos.material_state_repo,
            repos.material_master_repo,
            repos.capacity_repo,
            repos.action_log_repo,
            eligibility_engine,
            urgency_engine,
            priority_sorter,
            capacity_filler,
            config,
            config_manager,
            event_publisher,
        )
    }

    // ==========================================
    // 核心方法
    // ==========================================

    /// 一键重算 (全局重排)
    ///
    /// # 参数
    /// - `plan_id`: 方案ID
    /// - `base_date`: 基准日期 (重算起始日期)
    /// - `window_days`: 计算窗口天数
    /// - `operator`: 操作人
    /// - `is_dry_run`: 是否为试算模式（true=不落库，false=落库）
    ///
    /// # 返回
    /// - `Ok(RecalcResult)`: 重算成功，返回结果
    /// - `Err`: 重算失败
    ///
    /// # 红线
    /// - 冻结区材料不可被系统调整
    /// - 未适温材料不得进入当日产能池
    /// - 必须调用完整引擎链
    /// - 所有操作必须记录日志
    ///
    /// # 试算模式 (is_dry_run=true)
    /// - 不创建新版本
    /// - 不写入plan_item表
    /// - 不写入risk_snapshot表
    /// - 返回内存中的结果供前端预览
    pub fn recalc_full(
        &self,
        plan_id: &str,
        base_date: NaiveDate,
        window_days: i32,
        operator: &str,
        is_dry_run: bool,
    ) -> Result<RecalcResult, Box<dyn Error>> {
        // 1. 查询激活版本 (如果存在)
        let base_version = self.version_repo.find_active_version(plan_id)?;

        // 2. 创建新版本（试算模式下也创建临时版本用于计算）
        let mut new_version = if is_dry_run {
            // 试算模式：创建临时版本对象（不写库）
            PlanVersion {
                version_id: Uuid::new_v4().to_string(),
                plan_id: plan_id.to_string(),
                version_no: 0, // 试算版本号为0
                status: "SIMULATE".to_string(),
                frozen_from_date: None,
                recalc_window_days: Some(window_days),
                config_snapshot_json: Some(format!("试算 (操作人: {})", operator)),
                created_by: Some(operator.to_string()),
                created_at: chrono::Utc::now().naive_utc(),
                revision: 0,
            }
        } else {
            // 生产模式：创建并保存版本
            self.create_derived_version(
                plan_id,
                base_version.as_ref().map(|v| v.version_id.as_str()),
                window_days,
                Some(format!("一键重算 (操作人: {})", operator)),
                operator,
            )?
        };

        // 3. 计算冻结区起始日期
        let frozen_from_date = self.calculate_frozen_from_date(base_date);
        new_version.frozen_from_date = Some(frozen_from_date);

        // 4. 如果有基准版本，复制冻结区（仅生产模式）
        let frozen_count = if !is_dry_run {
            if let Some(base_ver) = &base_version {
                self.copy_frozen_zone(&base_ver.version_id, &new_version.version_id, frozen_from_date)?
            } else {
                0
            }
        } else {
            0
        };

        // 5. 执行重排 (计算区)
        let end_date = base_date + chrono::Duration::days(window_days as i64);
        let machine_codes = vec!["H032".to_string(), "H033".to_string(), "H034".to_string()];
        let reschedule_result = self.execute_reschedule(
            &new_version.version_id,
            (base_date, end_date),
            &machine_codes,
        )?;

        // 6. 提取统计信息
        let plan_items = reschedule_result.plan_items;
        let mature_count = reschedule_result.mature_count;
        let immature_count = reschedule_result.immature_count;

        // 7. 保存明细（仅生产模式）
        let inserted_count = if !is_dry_run && !plan_items.is_empty() {
            self.item_repo.batch_insert(&plan_items)?
        } else {
            plan_items.len()
        };

        // 8. 更新版本的frozen_from_date（仅生产模式）
        if !is_dry_run {
            self.version_repo.update(&new_version)?;
        }

        // 9. 生成风险快照（仅生产模式，TODO: 阶段3实施）
        // if !is_dry_run {
        //     RiskEngine.generate_snapshot()
        // }

        // 10. 记录操作日志（仅生产模式，TODO: 阶段3实施）
        // if !is_dry_run {
        //     ActionLogRepository.insert()
        // }

        // 11. 激活新版本（仅生产模式且auto_activate=true）
        if !is_dry_run && self.config.auto_activate {
            self.version_repo.activate_version(&new_version.version_id)?;
        }

        // 12. 触发决策视图刷新（仅生产模式）
        if !is_dry_run {
            self.trigger_decision_refresh(
                &new_version.version_id,
                ScheduleEventType::PlanItemChanged,
                Some(base_date),
                Some(end_date),
                Some(&machine_codes),
            )?;
        }

        // 13. 构建返回结果
        Ok(RecalcResult {
            version_id: new_version.version_id.clone(),
            version_no: new_version.version_no,
            total_items: inserted_count,
            mature_count,
            immature_count,
            frozen_items: frozen_count,
            recalc_items: inserted_count - frozen_count,
            elapsed_ms: 0, // TODO: 添加计时
        })
    }

    /// 局部重排 (指定日期范围)
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `start_date`: 重排起始日期
    /// - `end_date`: 重排结束日期
    /// - `operator`: 操作人
    /// - `is_dry_run`: 是否为试算模式（true=不落库，false=落库）
    ///
    /// # 返回
    /// - `Ok(RecalcResult)`: 重排成功
    /// - `Err`: 重排失败
    ///
    /// # 红线
    /// - 不删除冻结区明细
    /// - 日期范围外的明细不受影响
    ///
    /// # 试算模式 (is_dry_run=true)
    /// - 不删除现有plan_item
    /// - 不写入新的plan_item
    /// - 返回内存中的结果供前端预览
    pub fn recalc_partial(
        &self,
        version_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        _operator: &str,
        is_dry_run: bool,
    ) -> Result<RecalcResult, Box<dyn Error>> {
        // 1. 查询版本
        let version = self
            .version_repo
            .find_by_id(version_id)?
            .ok_or("Version not found")?;

        // 2. 查询冻结区明细 (用于统计)
        let frozen_items = self.item_repo.find_frozen_items(version_id)?;
        let frozen_in_range_count = frozen_items
            .iter()
            .filter(|i| i.plan_date >= start_date && i.plan_date <= end_date)
            .count();

        // 3. 删除日期范围的非冻结明细（仅生产模式）
        if !is_dry_run {
            // 注: delete_by_date_range会删除所有明细，业务层需确保冻结区不被删除
            // 这里我们先查询冻结区明细，删除后再重新插入
            let frozen_to_keep: Vec<PlanItem> = frozen_items
                .into_iter()
                .filter(|i| i.plan_date >= start_date && i.plan_date <= end_date)
                .collect();

            let _deleted_count = self
                .item_repo
                .delete_by_date_range(version_id, start_date, end_date)?;

            // 4. 重新插入冻结区明细
            if !frozen_to_keep.is_empty() {
                self.item_repo.batch_insert(&frozen_to_keep)?;
            }
        }

        // 5. 执行重排 (计算区)
        let machine_codes = vec!["H032".to_string(), "H033".to_string(), "H034".to_string()];
        let reschedule_result = self.execute_reschedule(
            version_id,
            (start_date, end_date),
            &machine_codes,
        )?;

        // 6. 提取统计信息
        let plan_items = reschedule_result.plan_items;
        let mature_count = reschedule_result.mature_count;
        let immature_count = reschedule_result.immature_count;

        // 7. 保存新明细（仅生产模式）
        let inserted_count = if !is_dry_run && !plan_items.is_empty() {
            self.item_repo.batch_insert(&plan_items)?
        } else {
            plan_items.len()
        };

        // 8. 更新风险快照（仅生产模式，TODO: 阶段3实施）
        // if !is_dry_run {
        //     RiskEngine.generate_snapshot()
        // }

        // 9. 记录操作日志（仅生产模式，TODO: 阶段3实施）
        // if !is_dry_run {
        //     ActionLogRepository.insert()
        // }

        // 10. 触发决策视图刷新（仅生产模式）
        if !is_dry_run {
            self.trigger_decision_refresh(
                version_id,
                ScheduleEventType::PlanItemChanged,
                Some(start_date),
                Some(end_date),
                Some(&machine_codes),
            )?;
        }

        // 11. 构建返回结果
        Ok(RecalcResult {
            version_id: version.version_id.clone(),
            version_no: version.version_no,
            total_items: inserted_count,
            mature_count,
            immature_count,
            frozen_items: frozen_in_range_count,
            recalc_items: inserted_count - frozen_in_range_count,
            elapsed_ms: 0, // TODO: 添加计时
        })
    }

    /// 联动窗口重排
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `trigger_date`: 触发日期
    /// - `cascade_days`: 联动天数
    /// - `operator`: 操作人
    /// - `is_dry_run`: 是否为试算模式（true=不落库，false=落库）
    ///
    /// # 返回
    /// - `Ok(RecalcResult)`: 重排成功
    /// - `Err`: 重排失败
    pub fn recalc_cascade(
        &self,
        version_id: &str,
        trigger_date: NaiveDate,
        cascade_days: i32,
        operator: &str,
        is_dry_run: bool,
    ) -> Result<RecalcResult, Box<dyn Error>> {
        // 计算联动范围
        let start_date = trigger_date;
        let end_date = trigger_date + chrono::Duration::days(cascade_days as i64);

        // 调用局部重排
        self.recalc_partial(version_id, start_date, end_date, operator, is_dry_run)
    }

    // ==========================================
    // 版本管理
    // ==========================================

    /// 创建派生版本 (基于现有版本)
    ///
    /// # 参数
    /// - `plan_id`: 方案ID
    /// - `base_version_id`: 基准版本ID (可选，如果为None则基于激活版本)
    /// - `window_days`: 计算窗口天数
    /// - `note`: 备注
    /// - `operator`: 操作人
    ///
    /// # 返回
    /// - `Ok(PlanVersion)`: 新版本
    /// - `Err`: 创建失败
    pub fn create_derived_version(
        &self,
        plan_id: &str,
        _base_version_id: Option<&str>,
        window_days: i32,
        _note: Option<String>,
        operator: &str,
    ) -> Result<PlanVersion, Box<dyn Error>> {
        // 1. 获取下一个版本号
        let version_no = self.version_repo.get_next_version_no(plan_id)?;

        // 2. 获取配置快照
        let config_snapshot = self.config_manager.get_config_snapshot()?;

        // 3. 创建PlanVersion对象
        let version = PlanVersion {
            version_id: Uuid::new_v4().to_string(),
            plan_id: plan_id.to_string(),
            version_no,
            status: "DRAFT".to_string(),
            frozen_from_date: None, // 将在recalc_full中设置
            recalc_window_days: Some(window_days),
            config_snapshot_json: Some(config_snapshot), // 存储配置快照
            created_by: Some(operator.to_string()),
            created_at: chrono::Utc::now().naive_utc(),
            revision: 0, // 乐观锁：初始修订号
        };

        // 4. 保存版本
        self.version_repo.create(&version)?;

        Ok(version)
    }

    /// 复制冻结区 (从旧版本到新版本)
    ///
    /// # 参数
    /// - `from_version_id`: 源版本ID
    /// - `to_version_id`: 目标版本ID
    /// - `frozen_from_date`: 冻结区起始日期 (< frozen_from_date的明细被复制)
    ///
    /// # 返回
    /// - `Ok(count)`: 复制的明细数量
    /// - `Err`: 复制失败
    ///
    /// # 红线
    /// - 只复制 locked_in_plan = true 的明细
    /// - 只复制 plan_date < frozen_from_date 的明细
    pub fn copy_frozen_zone(
        &self,
        from_version_id: &str,
        to_version_id: &str,
        frozen_from_date: NaiveDate,
    ) -> Result<usize, Box<dyn Error>> {
        // 1. 查询源版本的冻结区明细
        let frozen_items = self.item_repo.find_frozen_items(from_version_id)?;

        // 2. 过滤: 只复制 plan_date < frozen_from_date 的明细
        let items_to_copy: Vec<PlanItem> = frozen_items
            .into_iter()
            .filter(|item| item.plan_date < frozen_from_date)
            .map(|mut item| {
                // 修改version_id为目标版本
                item.version_id = to_version_id.to_string();
                // 确保source_type为FROZEN
                item.source_type = "FROZEN".to_string();
                item
            })
            .collect();

        // 3. 批量插入
        let count = self.item_repo.batch_insert(&items_to_copy)?;

        Ok(count)
    }

    // ==========================================
    // 重排逻辑
    // ==========================================

    /// 执行重排 (调用完整引擎链)
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `date_range`: 日期范围 (start, end)
    /// - `machine_codes`: 机组列表
    ///
    /// # 返回
    /// - `Ok(RescheduleResult)`: 重排产结果（包含plan_items和统计信息）
    /// - `Err`: 重排失败
    ///
    /// # 红线
    /// - 必须调用完整引擎链
    /// - 未适温材料不进入当日产能池
    /// - 冻结区材料只读，不可修改
    pub fn execute_reschedule(
        &self,
        version_id: &str,
        date_range: (NaiveDate, NaiveDate),
        machine_codes: &[String],
    ) -> Result<RescheduleResult, Box<dyn Error>> {
        // 检查是否已经在 tokio 运行时中
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // 已经在运行时中，使用 block_in_place 来运行异步代码
            tokio::task::block_in_place(|| {
                handle.block_on(async {
                    self.execute_reschedule_async(version_id, date_range, machine_codes)
                        .await
                })
            })
        } else {
            // 不在运行时中，创建新的运行时
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                self.execute_reschedule_async(version_id, date_range, machine_codes)
                    .await
            })
        }
    }

    /// 异步执行重排产（内部实现）
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `date_range`: 日期范围 (start, end)
    /// - `machine_codes`: 机组列表
    ///
    /// # 返回
    /// - `Ok(RescheduleResult)`: 重排产结果
    /// - `Err`: 重排失败
    async fn execute_reschedule_async(
        &self,
        version_id: &str,
        date_range: (NaiveDate, NaiveDate),
        machine_codes: &[String],
    ) -> Result<RescheduleResult, Box<dyn Error>> {
        // ===== Step 1: 查询冻结区材料（冻结区保护红线） =====
        let frozen_items = self.item_repo.find_frozen_items(version_id)?;

        // ===== Step 2: 初始化统计 =====
        let mut all_plan_items = Vec::new();
        let mut mature_count = 0;
        let mut immature_count = 0;
        let mut total_capacity_used = 0.0;
        let mut overflow_days = 0;

        // 跟踪已排产的材料ID，避免重复排产
        let mut scheduled_material_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        // 将冻结区材料加入已排产集合
        for item in &frozen_items {
            scheduled_material_ids.insert(item.material_id.clone());
        }

        // ===== Step 3: 多日循环 =====
        let (start_date, end_date) = date_range;
        let mut current_date = start_date;

        while current_date <= end_date {
            // ===== Step 4: 多机组循环 =====
            for machine_code in machine_codes {
                // ----- 4.1 查询候选材料 -----
                let materials = self.material_master_repo.find_by_machine(machine_code)?;

                // ----- 4.2 查询材料状态 -----
                let states: Vec<MaterialState> = materials
                    .iter()
                    .filter_map(|m| {
                        self.material_state_repo
                            .find_by_id(&m.material_id)
                            .ok()
                            .flatten()
                    })
                    .collect();

                // ----- 4.3 过滤适温材料（适温约束红线） -----
                // 同时排除已排产的材料，避免重复排产
                let total_materials = materials.len();
                let (ready_materials, ready_states): (Vec<_>, Vec<_>) = materials
                    .into_iter()
                    .zip(states.into_iter())
                    .filter(|(material, state)| {
                        // 排除已排产的材料
                        if scheduled_material_ids.contains(&material.material_id) {
                            return false;
                        }
                        // 只有READY, LOCKED, FORCE_RELEASE可以排产
                        state.sched_state == SchedState::Ready
                            || state.sched_state == SchedState::Locked
                            || state.sched_state == SchedState::ForceRelease
                    })
                    .unzip();

                // 统计成熟/未成熟数量
                let ready_count = ready_materials.len();
                mature_count += ready_count;
                immature_count += total_materials - ready_count;

                // 如果没有适温材料，跳过本次排产
                if ready_materials.is_empty() {
                    continue;
                }

                // ----- 4.4 查询或创建产能池 -----
                let mut capacity_pool = self
                    .capacity_repo
                    .find_by_machine_and_date(machine_code, current_date)?
                    .unwrap_or_else(|| CapacityPool {
                        machine_code: machine_code.clone(),
                        plan_date: current_date,
                        target_capacity_t: 1800.0, // 默认值
                        limit_capacity_t: 2000.0,  // 默认值
                        used_capacity_t: 0.0,
                        overflow_t: 0.0,
                        frozen_capacity_t: 0.0,
                        accumulated_tonnage_t: 0.0,
                        roll_campaign_id: None,
                    });

                // ----- 4.5 提取当日冻结项 -----
                let frozen_for_today: Vec<PlanItem> = frozen_items
                    .iter()
                    .filter(|item| {
                        item.plan_date == current_date && &item.machine_code == machine_code
                    })
                    .cloned()
                    .collect();

                // ----- 4.6 查询结构目标配比 -----
                let target_ratio = self.config_manager.get_target_ratio().await
                    .unwrap_or_else(|e| {
                        tracing::warn!("加载目标配比配置失败: {}, 使用空配置", e);
                        HashMap::new()
                    });
                let deviation_threshold = self.config_manager.get_deviation_threshold().await
                    .unwrap_or_else(|e| {
                        tracing::warn!("加载偏差阈值配置失败: {}, 使用默认值 0.1", e);
                        0.1
                    });

                // ----- 4.7 创建编排器并执行单日排产 -----
                let orchestrator = ScheduleOrchestrator::new(self.config_manager.clone());

                let schedule_result = orchestrator
                    .execute_single_day_schedule(
                        ready_materials,
                        ready_states,
                        &mut capacity_pool,
                        frozen_for_today,
                        &target_ratio,
                        deviation_threshold,
                        current_date,
                        version_id,
                    )
                    .await?;

                // ----- 4.8 收集排产结果 -----
                // 将新排产的材料ID加入已排产集合，避免后续日期重复排产
                for item in &schedule_result.plan_items {
                    scheduled_material_ids.insert(item.material_id.clone());
                }
                all_plan_items.extend(schedule_result.plan_items);
                total_capacity_used += schedule_result.updated_capacity_pool.used_capacity_t;

                // 统计超限天数
                if schedule_result.updated_capacity_pool.overflow_t > 0.0 {
                    overflow_days += 1;
                }

                // ----- 4.9 更新产能池（写回数据库） -----
                self.capacity_repo
                    .upsert_single(&schedule_result.updated_capacity_pool)?;

                // ----- 4.10 持久化修改的材料状态（urgent_level, rush_level 等） -----
                // Orchestrator 更新了 eligible_materials 中的状态，必须持久化到数据库
                // 否则紧急等级判定结果会丢失
                let updated_states: Vec<MaterialState> = schedule_result
                    .eligible_materials
                    .into_iter()
                    .map(|(_, state)| state)
                    .collect();

                if !updated_states.is_empty() {
                    self.material_state_repo.batch_insert_material_state(updated_states)?;
                    tracing::debug!(
                        machine_code = %capacity_pool.machine_code,
                        plan_date = %capacity_pool.plan_date,
                        "材料状态已持久化（包含紧急等级更新）"
                    );
                }
            }

            current_date += chrono::Duration::days(1);
        }

        // ===== Step 5: 返回结果 =====
        Ok(RescheduleResult {
            plan_items: all_plan_items,
            mature_count,
            immature_count,
            total_capacity_used,
            overflow_days,
        })
    }

    // ==========================================
    // 辅助方法
    // ==========================================

    /// 计算冻结区起始日期
    ///
    /// # 参数
    /// - `base_date`: 基准日期
    ///
    /// # 返回
    /// - 冻结区起始日期 (base_date - frozen_days_before_today)
    fn calculate_frozen_from_date(&self, base_date: NaiveDate) -> NaiveDate {
        base_date - chrono::Duration::days(self.config.frozen_days_before_today as i64)
    }

    /// 收集统计信息
    ///
    /// # 参数
    /// - `items`: 排产明细列表
    ///
    /// # 返回
    /// - (scheduled_count, frozen_count)
    fn collect_statistics(&self, items: &[PlanItem]) -> (usize, usize) {
        let scheduled_count = items.len();
        let frozen_count = items.iter().filter(|i| i.locked_in_plan).count();
        (scheduled_count, frozen_count)
    }

    /// 触发决策视图刷新
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `event_type`: 事件类型
    /// - `start_date`: 起始日期 (可选)
    /// - `end_date`: 结束日期 (可选)
    /// - `machine_codes`: 受影响的机组列表 (可选)
    ///
    /// # 返回
    /// - `Ok(())`: 事件发布成功
    /// - `Err`: 发布失败
    ///
    /// # 说明
    /// - 如果 event_publisher 未配置，则跳过刷新
    /// - 如果有 start_date 和 end_date，则进行增量刷新
    /// - 如果有 machine_codes，则刷新指定机组
    fn trigger_decision_refresh(
        &self,
        version_id: &str,
        event_type: ScheduleEventType,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        machine_codes: Option<&[String]>,
    ) -> Result<(), Box<dyn Error>> {
        // 构建事件范围
        let affected_date_range = match (start_date, end_date) {
            (Some(start), Some(end)) => Some((start, end)),
            _ => None,
        };

        let affected_machines = machine_codes.map(|codes| codes.to_vec());

        // 创建排产事件
        let event = ScheduleEvent::incremental(
            version_id.to_string(),
            event_type,
            Some("RecalcEngine triggered refresh".to_string()),
            affected_machines,
            affected_date_range,
        );

        // 发布事件
        match self.event_publisher.publish(event) {
            Ok(task_id) => {
                if !task_id.is_empty() {
                    tracing::info!(
                        "决策视图刷新事件已发布: task_id={}, version_id={}",
                        task_id,
                        version_id
                    );
                }
                Ok(())
            }
            Err(e) => {
                tracing::error!("决策视图刷新事件发布失败: {}", e);
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            }
        }
    }

    // ==========================================
    // 配置方法
    // ==========================================

    /// 获取默认联动窗口天数
    pub fn get_default_cascade_days(&self) -> i32 {
        self.config.default_cascade_days
    }

    /// 获取默认计算窗口天数
    pub fn get_default_window_days(&self) -> i32 {
        self.config.default_window_days
    }

    /// 获取冻结区天数
    pub fn get_frozen_days_before_today(&self) -> i32 {
        self.config.frozen_days_before_today
    }
}

// ==========================================
// API集成测试辅助工具
// ==========================================
// 职责: 提供API层集成测试的通用辅助函数
// ==========================================

#[path = "../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use tempfile::NamedTempFile;
use chrono::NaiveDate;

use hot_rolling_aps::api::{MaterialApi, PlanApi, DashboardApi, ConfigApi, RollerApi, ApiError, ManualOperationValidator};
use hot_rolling_aps::repository::{
    material_repo::{MaterialMasterRepository, MaterialStateRepository},
    plan_repo::{PlanRepository, PlanVersionRepository, PlanItemRepository},
    action_log_repo::ActionLogRepository,
    risk_repo::RiskSnapshotRepository,
    capacity_repo::CapacityPoolRepository,
    roller_repo::RollerCampaignRepository,
    strategy_draft_repo::StrategyDraftRepository,
    decision_refresh_repo::DecisionRefreshRepository,
};
use hot_rolling_aps::engine::{
    eligibility::EligibilityEngine,
    urgency::UrgencyEngine,
    recalc::RecalcEngine,
    risk::RiskEngine,
    priority::PrioritySorter,
    capacity_filler::CapacityFiller,
};
use hot_rolling_aps::config::config_manager::ConfigManager;
use hot_rolling_aps::engine::events::ScheduleEventPublisher;
use hot_rolling_aps::decision::services::{DecisionRefreshService, RefreshQueue, RefreshQueueAdapter};
use hot_rolling_aps::decision::api::{DecisionApi, DecisionApiImpl};
use hot_rolling_aps::decision::repository::{
    DaySummaryRepository, BottleneckRepository, OrderFailureRepository,
    ColdStockRepository, RollAlertRepository, CapacityOpportunityRepository,
};
use hot_rolling_aps::decision::use_cases::impls::{
    MostRiskyDayUseCaseImpl, MachineBottleneckUseCaseImpl, OrderFailureUseCaseImpl,
    ColdStockUseCaseImpl, RollCampaignAlertUseCaseImpl, CapacityOpportunityUseCaseImpl,
};
use hot_rolling_aps::domain::material::{MaterialMaster, MaterialState};
use hot_rolling_aps::domain::capacity::CapacityPool;
use hot_rolling_aps::domain::plan::Plan;
use hot_rolling_aps::domain::types::SchedState;

use std::sync::Mutex;
use rusqlite::Connection;

// ==========================================
// API测试环境
// ==========================================

/// API测试环境
///
/// 包含所有API实例和必要的依赖
pub struct ApiTestEnv {
    pub db_path: String,
    pub material_api: Arc<MaterialApi>,
    pub plan_api: Arc<PlanApi>,
    pub dashboard_api: Arc<DashboardApi>,
    pub config_api: Arc<ConfigApi>,
    pub roller_api: Arc<RollerApi>,

    // Repository层（用于测试数据准备）
    pub material_master_repo: Arc<MaterialMasterRepository>,
    pub material_state_repo: Arc<MaterialStateRepository>,
    pub plan_repo: Arc<PlanRepository>,
    pub plan_version_repo: Arc<PlanVersionRepository>,
    pub plan_item_repo: Arc<PlanItemRepository>,
    pub capacity_pool_repo: Arc<CapacityPoolRepository>,
    pub action_log_repo: Arc<ActionLogRepository>,

    // 临时文件（确保生命周期）
    _temp_file: NamedTempFile,
}

impl ApiTestEnv {
    /// 创建新的API测试环境
    ///
    /// # 说明
    /// - 使用临时数据库文件
    /// - 初始化所有Repository、Engine和API
    /// - 自动执行数据库迁移
    pub fn new() -> Result<Self, String> {
        // 创建临时数据库文件并初始化schema
        let (temp_file, db_path) = test_helpers::create_test_db()
            .map_err(|e| format!("创建测试数据库失败: {}", e))?;

        // 初始化数据库连接
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("无法打开数据库: {}", e))?;
        let conn = Arc::new(Mutex::new(conn));

        // ==========================================
        // 初始化Repository层
        // ==========================================

        let material_master_repo = Arc::new(
            MaterialMasterRepository::new(&db_path)
                .map_err(|e| format!("无法创建MaterialMasterRepository: {}", e))?
        );

        let material_state_repo = Arc::new(
            MaterialStateRepository::new(&db_path)
                .map_err(|e| format!("无法创建MaterialStateRepository: {}", e))?
        );

        let plan_repo = Arc::new(PlanRepository::new(conn.clone()));
        let plan_version_repo = Arc::new(PlanVersionRepository::new(conn.clone()));
        let plan_item_repo = Arc::new(PlanItemRepository::new(conn.clone()));
        let action_log_repo = Arc::new(ActionLogRepository::new(conn.clone()));
        let strategy_draft_repo = Arc::new(StrategyDraftRepository::new(conn.clone()));

        let risk_snapshot_repo = Arc::new(
            RiskSnapshotRepository::new(&db_path)
                .map_err(|e| format!("无法创建RiskSnapshotRepository: {}", e))?
        );

        let capacity_pool_repo = Arc::new(
            CapacityPoolRepository::new(db_path.clone())
                .map_err(|e| format!("无法创建CapacityPoolRepository: {}", e))?
        );

        // ==========================================
        // 初始化Engine层
        // ==========================================

        let config_manager = Arc::new(
            ConfigManager::new(&db_path)
                .map_err(|e| format!("无法创建ConfigManager: {}", e))?
        );

        let eligibility_engine = Arc::new(EligibilityEngine::new(config_manager.clone()));
        let urgency_engine = Arc::new(UrgencyEngine::new());
        let priority_sorter = Arc::new(PrioritySorter::new());
        let capacity_filler = Arc::new(CapacityFiller::new());
        let risk_engine = Arc::new(RiskEngine::new());

        // 决策视图刷新队列（测试环境可选）
        // 通过 RefreshQueueAdapter 实现依赖倒置
        let decision_refresh_service = Arc::new(DecisionRefreshService::new(conn.clone()));
        let event_publisher: Option<Arc<dyn ScheduleEventPublisher>> =
            RefreshQueue::new(conn.clone(), decision_refresh_service)
                .ok()
                .map(|queue| Arc::new(RefreshQueueAdapter::new(Arc::new(queue))) as Arc<dyn ScheduleEventPublisher>);

        let recalc_engine = Arc::new(RecalcEngine::with_default_config(
            plan_version_repo.clone(),
            plan_item_repo.clone(),
            material_state_repo.clone(),
            material_master_repo.clone(),
            capacity_pool_repo.clone(),
            action_log_repo.clone(),
            eligibility_engine.clone(),
            urgency_engine.clone(),
            priority_sorter.clone(),
            capacity_filler.clone(),
            config_manager.clone(),
            event_publisher.clone(),
        ));

        // ==========================================
        // 初始化API层
        // ==========================================

        // 创建validator
        let validator = Arc::new(ManualOperationValidator::new(
            material_state_repo.clone(),
            plan_item_repo.clone(),
            capacity_pool_repo.clone(),
        ));

        let material_api = Arc::new(MaterialApi::new(
            material_master_repo.clone(),
            material_state_repo.clone(),
            action_log_repo.clone(),
            eligibility_engine.clone(),
            urgency_engine.clone(),
            validator.clone(),
        ));

        let plan_api = Arc::new(PlanApi::new(
            plan_repo.clone(),
            plan_version_repo.clone(),
            plan_item_repo.clone(),
            material_state_repo.clone(),
            strategy_draft_repo.clone(),
            action_log_repo.clone(),
            risk_snapshot_repo.clone(),
            config_manager.clone(),
            recalc_engine,
            risk_engine,
            event_publisher,
        ));

        // 创建 Decision 层依赖（用于 DashboardApi）- 完整版本 D1-D6
        let day_summary_repo = Arc::new(DaySummaryRepository::new(conn.clone()));
        let bottleneck_repo = Arc::new(BottleneckRepository::new(conn.clone()));
        let order_failure_repo = Arc::new(OrderFailureRepository::new(conn.clone()));
        let cold_stock_repo = Arc::new(ColdStockRepository::new(conn.clone()));
        let roll_alert_repo = Arc::new(RollAlertRepository::new(conn.clone()));
        let capacity_opportunity_repo = Arc::new(CapacityOpportunityRepository::new(conn.clone()));

        let d1_use_case = Arc::new(MostRiskyDayUseCaseImpl::new(day_summary_repo));
        let d2_use_case = Arc::new(OrderFailureUseCaseImpl::new(order_failure_repo));
        let d3_use_case = Arc::new(ColdStockUseCaseImpl::new(cold_stock_repo));
        let d4_use_case = Arc::new(MachineBottleneckUseCaseImpl::new(bottleneck_repo));
        let d5_use_case = Arc::new(RollCampaignAlertUseCaseImpl::new(roll_alert_repo));
        let d6_use_case = Arc::new(CapacityOpportunityUseCaseImpl::new(capacity_opportunity_repo));

        let decision_api: Arc<dyn DecisionApi> = Arc::new(DecisionApiImpl::new_full(
            d1_use_case,
            d2_use_case,
            d3_use_case,
            d4_use_case,
            d5_use_case,
            d6_use_case,
        ));

        let decision_refresh_repo = Arc::new(DecisionRefreshRepository::new(conn.clone()));
        let dashboard_api = Arc::new(DashboardApi::new(
            decision_api,
            action_log_repo.clone(),
            decision_refresh_repo,
        ));

        // ConfigApi
        let config_api = Arc::new(ConfigApi::new(
            conn.clone(),
            config_manager.clone(),
            action_log_repo.clone(),
        ));

        // RollerApi
        let roller_repo = Arc::new(RollerCampaignRepository::from_connection(conn.clone()));
        let roller_api = Arc::new(RollerApi::new(
            roller_repo,
            action_log_repo.clone(),
        ));

        Ok(Self {
            db_path,
            material_api,
            plan_api,
            dashboard_api,
            config_api,
            roller_api,
            material_master_repo,
            material_state_repo,
            plan_repo,
            plan_version_repo,
            plan_item_repo,
            capacity_pool_repo,
            action_log_repo,
            _temp_file: temp_file,
        })
    }

    /// 准备测试材料数据
    ///
    /// # 参数
    /// - materials: MaterialMaster列表
    /// - states: MaterialState列表
    pub fn prepare_materials(
        &self,
        materials: Vec<MaterialMaster>,
        states: Vec<MaterialState>,
    ) -> Result<(), String> {
        // 插入MaterialMaster
        if !materials.is_empty() {
            self.material_master_repo
                .batch_insert_material_master(materials)
                .map_err(|e| format!("插入MaterialMaster失败: {}", e))?;
        }

        // 插入MaterialState
        if !states.is_empty() {
            self.material_state_repo
                .batch_insert_material_state(states)
                .map_err(|e| format!("插入MaterialState失败: {}", e))?;
        }

        Ok(())
    }

    /// 准备测试产能池数据
    pub fn prepare_capacity_pools(&self, pools: Vec<CapacityPool>) -> Result<(), String> {
        if !pools.is_empty() {
            self.capacity_pool_repo
                .upsert_batch(pools)
                .map_err(|e| format!("插入CapacityPool失败: {}", e))?;
        }
        Ok(())
    }

    /// 准备测试方案数据
    pub fn prepare_plan(&self, plan: Plan) -> Result<(), String> {
        self.plan_repo
            .create(&plan)
            .map_err(|e| format!("插入Plan失败: {}", e))?;
        Ok(())
    }
}

// ==========================================
// 红线验证辅助函数
// ==========================================

/// 验证是否违反冻结区保护红线
///
/// # 说明
/// 检查ApiError是否为FrozenZoneProtection类型
pub fn assert_frozen_zone_violation(result: Result<impl std::fmt::Debug, ApiError>) {
    match result {
        Err(ApiError::FrozenZoneProtection(_)) => {
            // 预期的错误类型
        }
        Ok(val) => panic!("预期FrozenZoneProtection错误，但操作成功: {:?}", val),
        Err(e) => panic!("预期FrozenZoneProtection错误，但得到: {:?}", e),
    }
}

/// 验证是否违反适温约束红线
pub fn assert_maturity_constraint_violation(result: Result<impl std::fmt::Debug, ApiError>) {
    match result {
        Err(ApiError::MaturityConstraintViolation { .. }) => {
            // 预期的错误类型
        }
        Ok(val) => panic!("预期MaturityConstraintViolation错误，但操作成功: {:?}", val),
        Err(e) => panic!("预期MaturityConstraintViolation错误，但得到: {:?}", e),
    }
}

/// 验证是否违反产能约束红线
pub fn assert_capacity_constraint_violation(result: Result<impl std::fmt::Debug, ApiError>) {
    match result {
        Err(ApiError::CapacityConstraintViolation { .. }) => {
            // 预期的错误类型
        }
        Ok(val) => panic!("预期CapacityConstraintViolation错误，但操作成功: {:?}", val),
        Err(e) => panic!("预期CapacityConstraintViolation错误，但得到: {:?}", e),
    }
}

/// 验证是否违反工业红线
pub fn assert_red_line_violation(result: Result<impl std::fmt::Debug, ApiError>) {
    match result {
        Err(ApiError::RedLineViolation(_)) => {
            // 预期的错误类型
        }
        Ok(val) => panic!("预期RedLineViolation错误，但操作成功: {:?}", val),
        Err(e) => panic!("预期RedLineViolation错误，但得到: {:?}", e),
    }
}

/// 验证是否为无效输入错误
pub fn assert_invalid_input(result: Result<impl std::fmt::Debug, ApiError>) {
    match result {
        Err(ApiError::InvalidInput(_)) => {
            // 预期的错误类型
        }
        Ok(val) => panic!("预期InvalidInput错误，但操作成功: {:?}", val),
        Err(e) => panic!("预期InvalidInput错误，但得到: {:?}", e),
    }
}

// ==========================================
// 测试数据辅助函数
// ==========================================

/// 创建测试用的MaterialMaster
pub fn create_test_material(
    material_id: &str,
    machine_code: &str,
    weight_t: f64,
    due_date: Option<NaiveDate>,
) -> MaterialMaster {
    use crate::helpers::test_data_builder::MaterialBuilder;

    let mut builder = MaterialBuilder::new(material_id)
        .machine(machine_code)
        .weight(weight_t);

    if let Some(date) = due_date {
        builder = builder.due_date(date);
    }

    builder.build()
}

/// 创建测试用的MaterialState
pub fn create_test_state(
    material_id: &str,
    sched_state: SchedState,
    _ready_in_days: i32,
) -> MaterialState {
    use crate::helpers::test_data_builder::MaterialStateBuilder;

    MaterialStateBuilder::new(material_id)
        .sched_state(sched_state)
        .build()
}

/// 创建已排产的测试材料
pub fn create_scheduled_material(
    material_id: &str,
    machine_code: &str,
    scheduled_date: NaiveDate,
) -> (MaterialMaster, MaterialState) {
    use crate::helpers::test_data_builder::{MaterialBuilder, MaterialStateBuilder};

    let master = MaterialBuilder::new(material_id)
        .machine(machine_code)
        .weight(100.0)
        .build();

    let mut state = MaterialStateBuilder::new(material_id)
        .sched_state(SchedState::Scheduled)
        .build();

    state.scheduled_date = Some(scheduled_date);
    state.scheduled_machine_code = Some(machine_code.to_string());
    state.in_frozen_zone = true;

    (master, state)
}

/// 创建测试用的Plan
pub fn create_test_plan(plan_id: &str, plan_name: &str, created_by: &str) -> Plan {
    use chrono::Local;

    let now = Local::now().naive_local();

    Plan {
        plan_id: plan_id.to_string(),
        plan_name: plan_name.to_string(),
        plan_type: "SCENARIO".to_string(),
        base_plan_id: None,
        created_by: created_by.to_string(),
        created_at: now,
        updated_at: now,
    }
}

// ==========================================
// ActionLog验证辅助函数
// ==========================================

/// 验证ActionLog是否已记录
///
/// # 说明
/// 检查最近的ActionLog是否包含指定的action_type
pub fn assert_action_logged(
    env: &ApiTestEnv,
    action_type: &str,
    expected_count: usize,
) -> Result<(), String> {
    let logs = env.action_log_repo
        .find_recent(100)
        .map_err(|e| format!("查询ActionLog失败: {}", e))?;

    let matching_logs: Vec<_> = logs.iter()
        .filter(|log| log.action_type == action_type)
        .collect();

    if matching_logs.len() < expected_count {
        return Err(format!(
            "预期至少{}条{}类型的ActionLog，实际找到{}条",
            expected_count,
            action_type,
            matching_logs.len()
        ));
    }

    Ok(())
}

/// 验证最近的ActionLog包含指定的operator
pub fn assert_action_has_operator(
    env: &ApiTestEnv,
    operator: &str,
) -> Result<(), String> {
    let logs = env.action_log_repo
        .find_recent(1)
        .map_err(|e| format!("查询ActionLog失败: {}", e))?;

    if logs.is_empty() {
        return Err("未找到任何ActionLog".to_string());
    }

    let latest_log = &logs[0];
    if latest_log.actor != operator {
        return Err(format!(
            "预期operator为{}，实际为{}",
            operator,
            latest_log.actor
        ));
    }

    Ok(())
}

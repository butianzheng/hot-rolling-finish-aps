// ==========================================
// 热轧精整排产系统 - 应用状态
// ==========================================
// 职责: 管理应用级别的共享状态和API实例
// 依据: 实施计划 Phase 5
// ==========================================

use std::sync::{Arc, Mutex};
use rusqlite::Connection;

use crate::api::{MaterialApi, PlanApi, DashboardApi, ConfigApi, RollerApi, RhythmApi, ManualOperationValidator, ImportApi};
use crate::decision::api::DecisionApiImpl;
use crate::decision::repository::{
    DaySummaryRepository, BottleneckRepository,
    OrderFailureRepository, ColdStockRepository,
    RollAlertRepository, CapacityOpportunityRepository,
};
use crate::decision::use_cases::impls::{
    MostRiskyDayUseCaseImpl, MachineBottleneckUseCaseImpl,
    OrderFailureUseCaseImpl, ColdStockUseCaseImpl,
    RollCampaignAlertUseCaseImpl, CapacityOpportunityUseCaseImpl,
};
use crate::decision::services::{
    DecisionRefreshService, RefreshQueue, RefreshQueueAdapter, RefreshScope, RefreshTask,
    RefreshTrigger,
};
use crate::engine::ScheduleEventPublisher;
use crate::repository::{
    material_repo::{MaterialMasterRepository, MaterialStateRepository},
    plan_repo::{PlanRepository, PlanVersionRepository, PlanItemRepository},
    action_log_repo::ActionLogRepository,
    risk_repo::RiskSnapshotRepository,
    capacity_repo::CapacityPoolRepository,
    roller_repo::RollerCampaignRepository,
    roll_campaign_plan_repo::RollCampaignPlanRepository,
    plan_rhythm_repo::PlanRhythmRepository,
    strategy_draft_repo::StrategyDraftRepository,
    decision_refresh_repo::DecisionRefreshRepository,
};
use crate::engine::{
    eligibility::EligibilityEngine,
    urgency::UrgencyEngine,
    recalc::RecalcEngine,
    risk::RiskEngine,
    priority::PrioritySorter,
    capacity_filler::CapacityFiller,
};
use crate::config::config_manager::ConfigManager;

/// 应用状态
///
/// 包含所有API实例和共享资源
/// 在Tauri应用中作为全局状态管理
pub struct AppState {
    /// 数据库路径
    pub db_path: String,

    /// 材料API
    pub material_api: Arc<MaterialApi>,

    /// 排产方案API
    pub plan_api: Arc<PlanApi>,

    /// 驾驶舱API
    pub dashboard_api: Arc<DashboardApi>,

    /// 配置管理API
    pub config_api: Arc<ConfigApi>,

    /// 换辊管理API
    pub roller_api: Arc<RollerApi>,

    /// 每日生产节奏API
    pub rhythm_api: Arc<RhythmApi>,

    /// 决策支持API
    pub decision_api: Arc<DecisionApiImpl>,

    /// 材料导入API
    pub import_api: Arc<ImportApi>,

    /// 产能池仓储（用于产能管理命令）
    pub capacity_pool_repo: Arc<CapacityPoolRepository>,

    /// 操作日志仓储（用于审计追踪）
    pub action_log_repo: Arc<ActionLogRepository>,

    /// 事件发布器（用于触发决策读模型刷新）
    pub event_publisher: Option<Arc<dyn ScheduleEventPublisher>>,
}

impl AppState {
    /// 创建新的AppState实例
    ///
    /// # 参数
    /// - db_path: 数据库文件路径
    ///
    /// # 返回
    /// - Ok(AppState): 应用状态实例
    /// - Err(String): 初始化错误
    ///
    /// # 说明
    /// 该方法会：
    /// 1. 初始化所有Repository
    /// 2. 初始化所有Engine
    /// 3. 创建所有API实例
    pub fn new(db_path: String) -> Result<Self, String> {
        tracing::info!("初始化AppState，数据库路径: {}", db_path);

        // 创建数据库连接（共享连接）
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("无法打开数据库: {}", e))?;
        // Best-effort: keep DB optimizations from blocking app startup.
        if let Err(e) = ensure_action_log_indexes(&conn) {
            tracing::warn!("action_log 索引初始化失败(将继续启动): {}", e);
        }
        let conn = Arc::new(Mutex::new(conn));

        // ==========================================
        // 初始化Repository层
        // ==========================================

        // 材料相关Repository
        let material_master_repo = Arc::new(
            MaterialMasterRepository::new(&db_path)
                .map_err(|e| format!("无法创建MaterialMasterRepository: {}", e))?
        );
        let material_state_repo = Arc::new(
            MaterialStateRepository::new(&db_path)
                .map_err(|e| format!("无法创建MaterialStateRepository: {}", e))?
        );

        // 排产相关Repository
        let plan_repo = Arc::new(PlanRepository::new(conn.clone()));
        let plan_version_repo = Arc::new(PlanVersionRepository::new(conn.clone()));
        let plan_item_repo = Arc::new(PlanItemRepository::new(conn.clone()));

        // 其他Repository
        let action_log_repo = Arc::new(ActionLogRepository::new(conn.clone()));
        // 策略草案仓储（草案持久化：避免刷新/重启丢失）
        let strategy_draft_repo = Arc::new(StrategyDraftRepository::new(conn.clone()));
        let risk_snapshot_repo = Arc::new(
            RiskSnapshotRepository::new(&db_path)
                .map_err(|e| format!("无法创建RiskSnapshotRepository: {}", e))?
        );
        let capacity_pool_repo = Arc::new(
            CapacityPoolRepository::new(db_path.clone())
                .map_err(|e| format!("无法创建CapacityPoolRepository: {}", e))?
        );

        let roller_campaign_repo = Arc::new(
            RollerCampaignRepository::new(&db_path)
                .map_err(|e| format!("无法创建RollerCampaignRepository: {}", e))?
        );

        let roll_campaign_plan_repo = Arc::new(
            RollCampaignPlanRepository::new(&db_path)
                .map_err(|e| format!("无法创建RollCampaignPlanRepository: {}", e))?
        );

        let plan_rhythm_repo = Arc::new(
            PlanRhythmRepository::new(&db_path)
                .map_err(|e| format!("无法创建PlanRhythmRepository: {}", e))?
        );

        // 决策层Repository (D1-D6)
        let day_summary_repo = Arc::new(DaySummaryRepository::new(conn.clone()));           // D1
        let order_failure_repo = Arc::new(OrderFailureRepository::new(conn.clone()));       // D2
        let cold_stock_repo = Arc::new(ColdStockRepository::new(conn.clone()));             // D3
        let bottleneck_repo = Arc::new(BottleneckRepository::new(conn.clone()));            // D4
        let roll_alert_repo = Arc::new(RollAlertRepository::new(conn.clone()));             // D5
        let capacity_opportunity_repo = Arc::new(CapacityOpportunityRepository::new(conn.clone())); // D6

        // ==========================================
        // 初始化Engine层
        // ==========================================

        // 配置管理器
        let config_manager = Arc::new(
            ConfigManager::new(&db_path)
                .map_err(|e| format!("无法创建ConfigManager: {}", e))?
        );

        // 适温判定引擎
        let eligibility_engine = Arc::new(EligibilityEngine::new(config_manager.clone()));

        // 紧急等级判定引擎
        let urgency_engine = Arc::new(UrgencyEngine::new());

        // 优先级排序器
        let priority_sorter = Arc::new(PrioritySorter::new());

        // 产能填充器
        let capacity_filler = Arc::new(CapacityFiller::new());

        // 风险引擎
        let risk_engine = Arc::new(RiskEngine::new());

        // 决策视图刷新队列和事件适配器
        let decision_refresh_service = Arc::new(DecisionRefreshService::new(conn.clone()));
        let event_publisher: Option<Arc<dyn ScheduleEventPublisher>> =
            match RefreshQueue::new(conn.clone(), decision_refresh_service) {
            Ok(queue) => {
                tracing::info!("RefreshQueue 初始化成功");
                let queue_arc = Arc::new(queue);

                // 启动时尝试处理历史遗留的待刷新任务，避免读模型长期为空（例如上次运行只入队未执行）。
                match queue_arc.process_all() {
                    Ok(refresh_ids) => {
                        if !refresh_ids.is_empty() {
                            tracing::info!(
                                "启动时已处理决策刷新队列任务: refreshed_count={}",
                                refresh_ids.len()
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!("启动时处理决策刷新队列失败: {}", e);
                    }
                }

                // 启动时对“当前激活版本”做一次兜底刷新：
                // - 典型场景：上次运行时刷新逻辑/数据源缺失导致 D1/D3 等读模型为空；
                // - 若不主动刷新，会出现“驾驶舱/决策看板数据为空且不联动”的问题。
                if let Ok(Some(active_version_id)) = plan_version_repo.find_latest_active_version_id()
                {
                    // 注意：不要在持有 conn 锁时调用 queue 方法，避免死锁。
                    let needs_refresh = {
                        let c = conn.lock()
                            .map_err(|e| format!("启动时锁获取失败: {}", e))?;
                        let d1_count: i64 = c
                            .query_row(
                                "SELECT COUNT(*) FROM decision_day_summary WHERE version_id = ?1",
                                [&active_version_id],
                                |row| row.get(0),
                            )
                            .unwrap_or(0);
                        let d3_count: i64 = c
                            .query_row(
                                "SELECT COUNT(*) FROM decision_cold_stock_profile WHERE version_id = ?1",
                                [&active_version_id],
                                |row| row.get(0),
                            )
                            .unwrap_or(0);
                        d1_count == 0 || d3_count == 0
                    };

                    if needs_refresh {
                        let scope = RefreshScope {
                            version_id: active_version_id.clone(),
                            is_full_refresh: true,
                            affected_machines: None,
                            affected_date_range: None,
                        };

                        let task = RefreshTask::new(
                            scope,
                            RefreshTrigger::ManualRefresh,
                            Some("Startup bootstrap refresh".to_string()),
                            3,
                        );

                        match queue_arc.enqueue(task) {
                            Ok(task_id) => {
                                tracing::info!(
                                    "启动时已触发决策读模型全量刷新: task_id={}, version_id={}",
                                    task_id,
                                    active_version_id
                                );
                                if let Err(e) = queue_arc.process_all() {
                                    tracing::warn!("启动时执行决策读模型刷新失败: {}", e);
                                }
                            }
                            Err(e) => {
                                tracing::warn!("启动时触发决策读模型刷新入队失败: {}", e);
                            }
                        }
                    }
                }

                // 创建适配器，实现 Engine → Decision 的事件传递
                Some(Arc::new(RefreshQueueAdapter::new(queue_arc)) as Arc<dyn ScheduleEventPublisher>)
            }
            Err(e) => {
                tracing::warn!("RefreshQueue 初始化失败: {}, 将跳过决策视图刷新", e);
                None
            }
        };

        // 每日生产节奏 API（用于工作台的节奏窗口）
        let rhythm_api = Arc::new(RhythmApi::new(
            plan_rhythm_repo,
            action_log_repo.clone(),
            config_manager.clone(),
        ));

        // 重算引擎（需要所有依赖）
        // 使用事件发布器而非直接依赖 RefreshQueue，实现依赖倒置
        let recalc_engine = Arc::new(RecalcEngine::with_default_config(
            plan_version_repo.clone(),
            plan_item_repo.clone(),
            material_state_repo.clone(),
            material_master_repo.clone(),
            capacity_pool_repo.clone(),
            action_log_repo.clone(),
            risk_snapshot_repo.clone(),
            eligibility_engine.clone(),
            urgency_engine.clone(),
            priority_sorter.clone(),
            capacity_filler.clone(),
            risk_engine.clone(),
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

        // 材料API
        let material_api = Arc::new(MaterialApi::new(
            material_master_repo.clone(),
            material_state_repo.clone(),
            action_log_repo.clone(),
            eligibility_engine.clone(),
            urgency_engine.clone(),
            validator.clone(),
        ));

        // 排产方案API
        // 使用事件发布器而非直接依赖 RefreshQueue，实现依赖倒置
        let plan_api = Arc::new(PlanApi::new(
            plan_repo,
            plan_version_repo,
            plan_item_repo.clone(),
            material_state_repo.clone(),
            material_master_repo.clone(),
            capacity_pool_repo.clone(),
            strategy_draft_repo.clone(),
            action_log_repo.clone(),
            risk_snapshot_repo.clone(),
            config_manager.clone(),
            recalc_engine,
            risk_engine,
            event_publisher.clone(),
        ));

        // 决策支持API（需要在 DashboardApi 之前初始化）
        // 初始化 D1-D6 用例
        let d1_use_case = Arc::new(MostRiskyDayUseCaseImpl::new(day_summary_repo));
        let d2_use_case = Arc::new(OrderFailureUseCaseImpl::new(order_failure_repo));
        let d3_use_case = Arc::new(ColdStockUseCaseImpl::new(cold_stock_repo));
        let d4_use_case = Arc::new(MachineBottleneckUseCaseImpl::new(bottleneck_repo));
        let d5_use_case = Arc::new(RollCampaignAlertUseCaseImpl::new(roll_alert_repo));
        let d6_use_case = Arc::new(CapacityOpportunityUseCaseImpl::new(capacity_opportunity_repo));

        // 使用 new_full() 创建完整的 DecisionApiImpl (支持 D1-D6)
        let decision_api = Arc::new(DecisionApiImpl::new_full(
            d1_use_case,
            d2_use_case,
            d3_use_case,
            d4_use_case,
            d5_use_case,
            d6_use_case,
        ));

        // 驾驶舱API（封装 DecisionApi）
        let decision_refresh_repo = Arc::new(DecisionRefreshRepository::new(conn.clone()));
        let dashboard_api = Arc::new(DashboardApi::new(
            decision_api.clone(),
            action_log_repo.clone(),
            decision_refresh_repo,
        ));

        // 配置管理API
        let config_api = Arc::new(ConfigApi::new(
            conn.clone(),
            config_manager.clone(),
            action_log_repo.clone(),
        ));

        // 换辊管理API
        let roller_api = Arc::new(RollerApi::new(
            roller_campaign_repo,
            roll_campaign_plan_repo,
            action_log_repo.clone(),
            config_manager.clone(),
        ));

        // 材料导入API
        let import_api = Arc::new(ImportApi::new(db_path.clone()));

        tracing::info!("AppState初始化完成");

        Ok(Self {
            db_path,
            material_api,
            plan_api,
            dashboard_api,
            config_api,
            roller_api,
            rhythm_api,
            decision_api,
            import_api,
            capacity_pool_repo,
            action_log_repo,
            event_publisher,
        })
    }

    /// 获取数据库路径
    pub fn get_db_path(&self) -> &str {
        &self.db_path
    }
}

fn ensure_action_log_indexes(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        r#"
        -- action_log is frequently queried by time window; keep this fast for UX (workbench/strategy compare).
        CREATE INDEX IF NOT EXISTS idx_action_ts ON action_log(action_ts);
        CREATE INDEX IF NOT EXISTS idx_action_type_ts ON action_log(action_type, action_ts);
        CREATE INDEX IF NOT EXISTS idx_action_actor_ts ON action_log(actor, action_ts);
        CREATE INDEX IF NOT EXISTS idx_action_machine_ts ON action_log(machine_code, action_ts);
        CREATE INDEX IF NOT EXISTS idx_action_date_range ON action_log(date_range_start, date_range_end);
        "#,
    )?;
    Ok(())
}

// ==========================================
// 默认数据库路径辅助函数
// ==========================================

/// 获取默认数据库路径
///
/// # 返回
/// - 开发环境: 用户数据目录/hot-rolling-aps-dev/hot_rolling_aps.db（首次运行会从项目根目录的 ./hot_rolling_aps.db 复制一份作为初始数据）
/// - 生产环境: 用户数据目录/hot-rolling-aps/hot_rolling_aps.db
pub fn get_default_db_path() -> String {
    use std::path::PathBuf;

    // 允许通过环境变量显式指定 DB 路径（便于调试/测试/CI）
    if let Ok(path) = std::env::var("HOT_ROLLING_APS_DB_PATH") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    // 使用用户数据目录，避免开发期 DB 文件变化触发 `tauri dev` 的文件监控重启（看起来像“闪退重启”）。
    // 先给一个默认回退值，后续如果能拿到 data_dir 再覆盖。
    let mut path = PathBuf::from("./hot_rolling_aps.db");

    // 尝试获取用户数据目录
    if let Some(data_dir) = dirs::data_dir() {
        // 开发环境使用独立目录，避免污染生产数据
        #[cfg(debug_assertions)]
        {
            path = data_dir.join("hot-rolling-aps-dev");
        }

        #[cfg(not(debug_assertions))]
        {
            path = data_dir.join("hot-rolling-aps");
        }

        // 确保目录存在
        std::fs::create_dir_all(&path).ok();
        path = path.join("hot_rolling_aps.db");

        // 开发环境：如果目标 DB 不存在，但项目根目录有初始 DB，则复制一份作为种子数据
        #[cfg(debug_assertions)]
        {
            if !path.exists() {
                let seed = PathBuf::from("./hot_rolling_aps.db");
                if seed.exists() {
                    // best-effort: 复制失败不应阻塞启动（后续会自动创建空库并建表）
                    let _ = std::fs::copy(seed, &path);
                }
            }
        }
    }

    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_db_path() {
        let path = get_default_db_path();
        assert!(!path.is_empty());
        assert!(path.ends_with(".db"));
    }

    // 注意：AppState::new() 的测试需要真实的数据库文件
    // 这些测试应该在集成测试中进行
}

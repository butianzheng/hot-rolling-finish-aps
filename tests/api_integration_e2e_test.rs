// ==========================================
// API层集成端到端测试
// ==========================================
// 目标: 验证MaterialApi → PlanApi → DashboardApi的完整集成
// 简化版本：只使用API层，不涉及内部实现细节
// ==========================================

#[path = "test_helpers.rs"]
mod test_helpers;

#[cfg(test)]
mod api_integration_e2e_test {
    use crate::test_helpers;
    use crate::test_helpers::create_test_db;
    use chrono::Duration;
    use hot_rolling_aps::api::{DashboardApi, MaterialApi, PlanApi, ValidationMode};
    use hot_rolling_aps::config::config_manager::ConfigManager;
    use hot_rolling_aps::decision::api::{DecisionApi, DecisionApiImpl};
    use hot_rolling_aps::decision::repository::{BottleneckRepository, DaySummaryRepository};
    use hot_rolling_aps::decision::use_cases::impls::{
        MachineBottleneckUseCaseImpl, MostRiskyDayUseCaseImpl,
    };
    use hot_rolling_aps::engine::{
        CapacityFiller, EligibilityEngine, MaterialStateDerivationService, PrioritySorter,
        RecalcEngine, RiskEngine, UrgencyEngine,
    };
    use hot_rolling_aps::importer::{
        ConflictHandlerImpl, CsvParser, DataCleanerImpl, DerivationServiceImpl, DqValidatorImpl,
        FieldMapperImpl, MaterialImporter, MaterialImporterImpl,
    };
    use hot_rolling_aps::repository::MaterialImportRepositoryImpl;
    use hot_rolling_aps::repository::{
        action_log_repo::ActionLogRepository,
        capacity_repo::CapacityPoolRepository,
        decision_refresh_repo::DecisionRefreshRepository,
        material_repo::{MaterialMasterRepository, MaterialStateRepository},
        path_override_pending_repo::PathOverridePendingRepository,
        plan_repo::{PlanItemRepository, PlanRepository, PlanVersionRepository},
        risk_repo::RiskSnapshotRepository,
        roller_repo::RollerCampaignRepository,
        strategy_draft_repo::StrategyDraftRepository,
    };
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};
    use tempfile::NamedTempFile;

    // 定义具体的 MaterialImporter 类型
    type TestMaterialImporter = MaterialImporterImpl<MaterialImportRepositoryImpl, ConfigManager>;

    /// 创建完整测试环境
    fn setup_test_env() -> (
        NamedTempFile,
        String,
        TestMaterialImporter,
        Arc<PlanApi>,
        Arc<MaterialApi>,
        Arc<DashboardApi>,
        Arc<CapacityPoolRepository>,
    ) {
        let (temp_file, db_path) = create_test_db().unwrap();
        let conn = Arc::new(Mutex::new(
            test_helpers::open_test_connection(&db_path).unwrap(),
        ));

        // Repositories
        let material_master_repo = Arc::new(MaterialMasterRepository::new(&db_path).unwrap());
        let material_state_repo = Arc::new(MaterialStateRepository::new(&db_path).unwrap());
        let plan_repo = Arc::new(PlanRepository::new(conn.clone()));
        let plan_version_repo = Arc::new(PlanVersionRepository::new(conn.clone()));
        let plan_item_repo = Arc::new(PlanItemRepository::new(conn.clone()));
        let action_log_repo = Arc::new(ActionLogRepository::new(conn.clone()));
        let strategy_draft_repo = Arc::new(StrategyDraftRepository::new(conn.clone()));
        let risk_snapshot_repo = Arc::new(RiskSnapshotRepository::new(&db_path).unwrap());
        let capacity_pool_repo = Arc::new(CapacityPoolRepository::new(db_path.clone()).unwrap());
        let roller_campaign_repo = Arc::new(RollerCampaignRepository::new(&db_path).unwrap());
        let path_override_pending_repo = Arc::new(PathOverridePendingRepository::new(conn.clone()));

        // Engines
        let config_manager = Arc::new(ConfigManager::new(&db_path).unwrap());
        let eligibility_engine = Arc::new(EligibilityEngine::new(config_manager.clone()));
        let urgency_engine = Arc::new(UrgencyEngine::new());
        let priority_sorter = Arc::new(PrioritySorter::new());
        let capacity_filler = Arc::new(CapacityFiller::new());
        let risk_engine = Arc::new(RiskEngine::new());

        let recalc_engine = Arc::new(RecalcEngine::with_default_config(
            plan_version_repo.clone(),
            plan_item_repo.clone(),
            material_state_repo.clone(),
            material_master_repo.clone(),
            capacity_pool_repo.clone(),
            action_log_repo.clone(),
            risk_snapshot_repo.clone(),
            roller_campaign_repo.clone(),
            path_override_pending_repo.clone(),
            eligibility_engine.clone(),
            urgency_engine.clone(),
            priority_sorter.clone(),
            capacity_filler.clone(),
            risk_engine.clone(),
            config_manager.clone(),
            None,
        ));

        // MaterialImporter - 使用新的泛型 API（需要单独的 ConfigManager 实例）
        let import_config = ConfigManager::new(&db_path).unwrap();
        let import_repo = MaterialImportRepositoryImpl::new(&db_path).unwrap();
        let file_parser = Box::new(CsvParser);
        let field_mapper = Box::new(FieldMapperImpl);
        let data_cleaner = Box::new(DataCleanerImpl);
        let derivation_service = Box::new(DerivationServiceImpl);
        let dq_validator = Box::new(DqValidatorImpl::new(100.0));
        let conflict_handler = Box::new(ConflictHandlerImpl);
        let state_derivation_service = MaterialStateDerivationService::new();

        let material_importer = MaterialImporterImpl::new(
            import_repo,
            import_config,
            file_parser,
            field_mapper,
            data_cleaner,
            derivation_service,
            dq_validator,
            conflict_handler,
            state_derivation_service,
        );

        // Validator
        let validator = Arc::new(hot_rolling_aps::api::ManualOperationValidator::new(
            material_state_repo.clone(),
            plan_item_repo.clone(),
            capacity_pool_repo.clone(),
        ));

        // APIs
        let material_api = Arc::new(MaterialApi::new(
            material_master_repo.clone(),
            material_state_repo.clone(),
            action_log_repo.clone(),
            eligibility_engine,
            urgency_engine,
            validator,
        ));

        let plan_api = Arc::new(PlanApi::new(
            plan_repo,
            plan_version_repo,
            plan_item_repo,
            material_state_repo,
            material_master_repo,
            capacity_pool_repo.clone(),
            strategy_draft_repo,
            action_log_repo.clone(),
            risk_snapshot_repo,
            config_manager.clone(),
            recalc_engine,
            risk_engine,
            None,
        ));

        // Decision Layer (for DashboardApi) - P1 版本：仅 D1 + D4
        let day_summary_repo = Arc::new(DaySummaryRepository::new(conn.clone()));
        let bottleneck_repo = Arc::new(BottleneckRepository::new(conn.clone()));

        let decision_api: Arc<dyn DecisionApi> = Arc::new(DecisionApiImpl::new(
            Arc::new(MostRiskyDayUseCaseImpl::new(day_summary_repo)),
            Arc::new(MachineBottleneckUseCaseImpl::new(bottleneck_repo)),
        ));

        let decision_refresh_repo = Arc::new(DecisionRefreshRepository::new(conn.clone()));
        let dashboard_api = Arc::new(DashboardApi::new(
            decision_api,
            action_log_repo,
            decision_refresh_repo,
        ));

        (
            temp_file,
            db_path,
            material_importer,
            plan_api,
            material_api,
            dashboard_api,
            capacity_pool_repo,
        )
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_api_integration_import_to_dashboard() {
        println!("\n=== API集成测试：材料导入 → 排产 → 驾驶舱 ===\n");

        let (
            _temp_file,
            _db_path,
            material_importer,
            plan_api,
            _material_api,
            _dashboard_api,
            capacity_repo,
        ) = setup_test_env();

        // 1. 导入材料
        let import_result = material_importer
            .import_from_csv("tests/fixtures/datasets/01_normal_data.csv")
            .await
            .expect("材料导入失败");
        assert!(import_result.summary.success > 0);
        assert_eq!(import_result.summary.blocked, 0);
        println!(
            "✓ 步骤 1: 材料导入（成功: {}条）",
            import_result.summary.success
        );

        // 2. 创建方案和版本
        let plan_id = plan_api
            .create_plan("API集成测试".to_string(), "test_op".to_string())
            .expect("创建方案失败");
        let version_id = plan_api
            .create_version(plan_id, 7, None, None, "test_op".to_string())
            .expect("创建版本失败");
        println!("✓ 步骤 2: 创建方案和版本");

        // 3. 准备产能池（使用简单的upsert_one）
        use hot_rolling_aps::domain::capacity::CapacityPool;
        let base_date = chrono::Local::now().date_naive();
        for day_offset in 0..7 {
            for machine in &["H032", "H033", "H034"] {
                let pool = CapacityPool {
                    version_id: version_id.clone(),
                    machine_code: machine.to_string(),
                    plan_date: base_date + Duration::days(day_offset),
                    target_capacity_t: 800.0,
                    limit_capacity_t: 900.0,
                    used_capacity_t: 0.0,
                    accumulated_tonnage_t: 0.0,
                    frozen_capacity_t: 0.0,
                    overflow_t: 0.0,
                    roll_campaign_id: None,
                };
                capacity_repo.upsert_single(&pool).unwrap();
            }
        }
        println!("✓ 步骤 3: 准备产能池（3机组 × 7天）");

        // 4. 执行重算
        let recalc_result = plan_api
            .recalc_full(&version_id, base_date, None, "test_op")
            .expect("重算失败");
        assert!(recalc_result.success);
        println!(
            "✓ 步骤 4: 重算完成（排产明细: {}条）",
            recalc_result.plan_items_count
        );

        // 使用重算返回的新版本ID（RecalcEngine会创建新版本）
        let new_version_id = &recalc_result.version_id;

        // 5. 查询排产明细
        let plan_items = plan_api
            .list_plan_items(new_version_id)
            .expect("查询明细失败");
        assert!(!plan_items.is_empty());
        println!("✓ 步骤 5: 查询排产明细（{}条）", plan_items.len());

        // 6. 查询操作日志
        let logs = _dashboard_api
            .list_action_logs_by_version(new_version_id)
            .expect("查询日志失败");
        assert!(!logs.is_empty());
        println!("✓ 步骤 6: 操作日志记录（{}条）", logs.len());

        println!("\n=== API集成测试通过 ✅ ===");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_material_lock_integration() {
        println!("\n=== API集成测试：材料锁定与重算 ===\n");

        let (
            _temp_file,
            _db_path,
            material_importer,
            plan_api,
            material_api,
            _dashboard_api,
            capacity_repo,
        ) = setup_test_env();

        // 1. 导入材料
        let import_result = material_importer
            .import_from_csv("tests/fixtures/datasets/01_normal_data.csv")
            .await
            .expect("材料导入失败");
        println!("✓ 步骤 1: 材料导入（{}条）", import_result.summary.success);

        // 2. 创建方案
        let plan_id = plan_api
            .create_plan("锁定测试".to_string(), "test_op".to_string())
            .unwrap();
        let version_id = plan_api
            .create_version(plan_id, 7, None, None, "test_op".to_string())
            .unwrap();

        // 3. 准备产能池
        use hot_rolling_aps::domain::capacity::CapacityPool;
        let base_date = chrono::Local::now().date_naive();
        for day_offset in 0..7 {
            for machine in &["H032", "H033"] {
                let pool = CapacityPool {
                    version_id: version_id.clone(),
                    machine_code: machine.to_string(),
                    plan_date: base_date + Duration::days(day_offset),
                    target_capacity_t: 800.0,
                    limit_capacity_t: 900.0,
                    used_capacity_t: 0.0,
                    accumulated_tonnage_t: 0.0,
                    frozen_capacity_t: 0.0,
                    overflow_t: 0.0,
                    roll_campaign_id: None,
                };
                capacity_repo.upsert_single(&pool).unwrap();
            }
        }
        println!("✓ 步骤 2: 产能池已准备");

        // 4. 第一次重算
        plan_api
            .recalc_full(&version_id, base_date, None, "test_op")
            .unwrap();
        let items_before = plan_api.list_plan_items(&version_id).unwrap();
        println!("✓ 步骤 3: 第一次重算（{}条）", items_before.len());

        // 5. 锁定前3条材料
        if items_before.len() >= 3 {
            let material_ids: Vec<String> = items_before
                .iter()
                .take(3)
                .map(|item| item.material_id.clone())
                .collect();

            let lock_result = material_api
                .batch_lock_materials(
                    material_ids,
                    true,
                    "test_op",
                    "测试锁定",
                    ValidationMode::Strict,
                )
                .unwrap();
            println!("✓ 步骤 4: 锁定材料（{}条）", lock_result.success_count);

            // 6. 第二次重算
            plan_api
                .recalc_full(&version_id, base_date, None, "test_op")
                .unwrap();
            let items_after = plan_api.list_plan_items(&version_id).unwrap();

            // 验证冻结材料数量
            let locked_count = items_after
                .iter()
                .filter(|item| item.locked_in_plan)
                .count();
            assert_eq!(locked_count, 3, "应该有3条冻结材料");
            println!("✓ 步骤 5: 冻结区保护验证通过（{}条冻结）", locked_count);
        }

        println!("\n=== 材料锁定集成测试通过 ✅ ===");
    }
}

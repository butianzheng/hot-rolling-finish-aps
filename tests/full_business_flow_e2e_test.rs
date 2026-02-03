// ==========================================
// 完整业务流程端到端集成测试
// ==========================================
// 目标: 验证从材料导入到驾驶舱展示的完整业务流程
// 覆盖: MaterialImporter → RecalcEngine → DecisionApi → DashboardApi
// ==========================================

#[path = "test_helpers.rs"]
mod test_helpers;

#[cfg(test)]
mod full_business_flow_e2e_test {
    use crate::test_helpers::create_test_db;
    use chrono::{Duration, NaiveDate};
    use hot_rolling_aps::api::{DashboardApi, MaterialApi, PlanApi};
    use hot_rolling_aps::config::config_manager::ConfigManager;
    use hot_rolling_aps::decision::api::{DecisionApi, DecisionApiImpl};
    use hot_rolling_aps::decision::repository::{DaySummaryRepository, BottleneckRepository};
    use hot_rolling_aps::decision::services::DecisionRefreshService;
    use hot_rolling_aps::decision::use_cases::impls::{MostRiskyDayUseCaseImpl, MachineBottleneckUseCaseImpl};
    use hot_rolling_aps::domain::capacity::CapacityPool;
    use hot_rolling_aps::engine::{
        CapacityFiller, EligibilityEngine, MaterialStateDerivationService, PrioritySorter, RecalcEngine, RiskEngine,
        UrgencyEngine,
    };
    use hot_rolling_aps::importer::{
        MaterialImporter, MaterialImporterImpl, CsvParser, FieldMapperImpl, DataCleanerImpl,
        DerivationServiceImpl, DqValidatorImpl, ConflictHandlerImpl,
    };
    use hot_rolling_aps::repository::MaterialImportRepositoryImpl;
    use hot_rolling_aps::repository::{
        action_log_repo::ActionLogRepository,
        capacity_repo::CapacityPoolRepository,
        material_repo::{MaterialMasterRepository, MaterialStateRepository},
        path_override_pending_repo::PathOverridePendingRepository,
        plan_repo::{PlanItemRepository, PlanRepository, PlanVersionRepository},
        roller_repo::RollerCampaignRepository,
        risk_repo::RiskSnapshotRepository,
        strategy_draft_repo::StrategyDraftRepository,
        decision_refresh_repo::DecisionRefreshRepository,
    };
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};
    use tempfile::NamedTempFile;

    // ==========================================
    // 测试辅助函数
    // ==========================================

    // 定义具体的 MaterialImporter 类型
    type TestMaterialImporter = MaterialImporterImpl<MaterialImportRepositoryImpl, ConfigManager>;

    /// 创建完整测试环境（包含所有模块）
    fn setup_full_test_env() -> (
        NamedTempFile,
        String,
        TestMaterialImporter,
        Arc<PlanApi>,
        Arc<MaterialApi>,
        Arc<DashboardApi>,
        Arc<DecisionRefreshService>,
        Arc<CapacityPoolRepository>,
        Arc<MaterialStateRepository>,
        Arc<PlanItemRepository>,
    ) {
        // 创建临时数据库
        let (temp_file, db_path) = create_test_db().unwrap();
        let conn = Arc::new(Mutex::new(test_helpers::open_test_connection(&db_path).unwrap()));

        // === Repository 层 ===
        let material_master_repo =
            Arc::new(MaterialMasterRepository::new(&db_path).unwrap());
        let material_state_repo =
            Arc::new(MaterialStateRepository::new(&db_path).unwrap());
        let plan_repo = Arc::new(PlanRepository::new(conn.clone()));
        let plan_version_repo = Arc::new(PlanVersionRepository::new(conn.clone()));
        let plan_item_repo = Arc::new(PlanItemRepository::new(conn.clone()));
        let action_log_repo = Arc::new(ActionLogRepository::new(conn.clone()));
        let strategy_draft_repo = Arc::new(StrategyDraftRepository::new(conn.clone()));
        let risk_snapshot_repo =
            Arc::new(RiskSnapshotRepository::new(&db_path).unwrap());
        let capacity_pool_repo =
            Arc::new(CapacityPoolRepository::new(db_path.clone()).unwrap());
        let roller_campaign_repo =
            Arc::new(RollerCampaignRepository::new(&db_path).unwrap());
        let path_override_pending_repo = Arc::new(PathOverridePendingRepository::new(conn.clone()));

        // === Engine 层 ===
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
            None, // refresh_queue (not needed in tests)
        ));

        // === MaterialImporter - 使用新的泛型 API ===
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

        // === Validator ===
        let validator = Arc::new(hot_rolling_aps::api::ManualOperationValidator::new(
            material_state_repo.clone(),
            plan_item_repo.clone(),
            capacity_pool_repo.clone(),
        ));

        // === API 层 ===
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
            plan_item_repo.clone(),
            material_state_repo.clone(),
            material_master_repo,
            capacity_pool_repo.clone(),
            strategy_draft_repo,
            action_log_repo.clone(),
            risk_snapshot_repo,
            config_manager.clone(),
            recalc_engine,
            risk_engine,
            None, // refresh_queue (not needed in tests)
        ));

        // === Decision 层 (P1 版本：仅 D1 + D4) ===
        let day_summary_repo = Arc::new(DaySummaryRepository::new(conn.clone()));
        let bottleneck_repo = Arc::new(BottleneckRepository::new(conn.clone()));

        let d1_use_case = Arc::new(MostRiskyDayUseCaseImpl::new(day_summary_repo));
        let d4_use_case = Arc::new(MachineBottleneckUseCaseImpl::new(bottleneck_repo));

        let decision_api: Arc<dyn DecisionApi> = Arc::new(DecisionApiImpl::new(
            d1_use_case,
            d4_use_case,
        ));

        let decision_refresh_repo = Arc::new(DecisionRefreshRepository::new(conn.clone()));
        let dashboard_api = Arc::new(DashboardApi::new(
            decision_api,
            action_log_repo,
            decision_refresh_repo,
        ));

        // === Decision Refresh Service ===
        let refresh_service =
            Arc::new(DecisionRefreshService::new(conn.clone()));

        (
            temp_file,
            db_path,
            material_importer,
            plan_api,
            material_api,
            dashboard_api,
            refresh_service,
            capacity_pool_repo,
            material_state_repo,
            plan_item_repo,
        )
    }

    /// 准备测试产能池数据
    fn prepare_capacity_pools(
        capacity_repo: &CapacityPoolRepository,
        version_id: &str,
        machine_codes: Vec<&str>,
        base_date: NaiveDate,
        days: i64,
    ) -> Result<(), String> {
        for machine in machine_codes {
            for day_offset in 0..days {
                let plan_date = base_date + Duration::days(day_offset);
                let pool = CapacityPool {
                    version_id: version_id.to_string(),
                    machine_code: machine.to_string(),
                    plan_date,
                    target_capacity_t: 800.0,
                    limit_capacity_t: 900.0,
                    used_capacity_t: 0.0,
                    accumulated_tonnage_t: 0.0,
                    frozen_capacity_t: 0.0,
                    overflow_t: 0.0,
                    roll_campaign_id: None,
                };
                capacity_repo
                    .upsert_single(&pool)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    // ==========================================
    // 测试场景1: 完整业务流程（材料导入 → 排产 → 驾驶舱）
    // ==========================================

    #[tokio::test(flavor = "multi_thread")]
    async fn test_full_business_flow_import_to_dashboard() {
        println!("\n=== 端到端集成测试：完整业务流程 ===\n");

        // 1. 初始化测试环境
        let (
            _temp_file,
            _db_path,
            material_importer,
            plan_api,
            _material_api,
            dashboard_api,
            refresh_service,
            capacity_repo,
            material_state_repo,
            plan_item_repo,
        ) = setup_full_test_env();
        println!("✓ 步骤 1: 测试环境已初始化");

        // 2. 导入材料数据
        let import_result = material_importer
            .import_from_csv("tests/fixtures/datasets/01_normal_data.csv")
            .await
            .expect("材料导入失败");
        assert!(
            import_result.summary.success > 0,
            "应该有成功导入的材料"
        );
        assert_eq!(
            import_result.summary.blocked, 0,
            "不应该有阻塞记录"
        );
        println!(
            "✓ 步骤 2: 材料导入完成（成功: {}, 总计: {}）",
            import_result.summary.success, import_result.summary.total_rows
        );

        // 3. 验证材料状态派生（通过计数验证）
        let ready_states = material_state_repo
            .find_ready_materials(None)
            .expect("查询Ready材料失败");
        let pending_states = material_state_repo
            .find_immature_materials(None)
            .expect("查询PendingMature材料失败");
        let ready_count = ready_states.len();
        let pending_count = pending_states.len();
        println!(
            "✓ 步骤 3: 材料状态派生完成（Ready: {}, PendingMature: {}）",
            ready_count, pending_count
        );

        // 4. 创建排产方案和版本
        let plan_id = plan_api
            .create_plan(
                "完整业务流程测试方案".to_string(),
                "test_operator".to_string(),
            )
            .expect("创建方案失败");
        let version_id = plan_api
            .create_version(
                plan_id.clone(),
                7,  // 窗口天数
                None,
                Some("测试版本".to_string()),
                "test_operator".to_string(),
            )
            .expect("创建版本失败");
        println!(
            "✓ 步骤 4: 排产方案和版本已创建（Plan: {}, Version: {}）",
            plan_id, version_id
        );

        // 5. 准备产能池
        let base_date = chrono::Local::now().date_naive();
        prepare_capacity_pools(
            &capacity_repo,
            &version_id,
            vec!["H032", "H033", "H034"],
            base_date,
            7,
        )
        .expect("准备产能池失败");
        println!("✓ 步骤 5: 产能池已准备（3个机组 × 7天）");

        // 6. 执行一键重算
        let recalc_result = plan_api
            .recalc_full(
                &version_id,
                base_date,
                None,
                "test_operator",
            )
            .expect("一键重算失败");
        assert!(recalc_result.success, "重算应该成功");
        println!(
            "✓ 步骤 6: 一键重算完成（排产明细数: {}）",
            recalc_result.plan_items_count
        );

        // 注意：RecalcEngine 会创建派生版本并(默认)自动激活，新明细写入 recalc_result.version_id
        let version_id = recalc_result.version_id.clone();

        // 7. 验证排产明细
        let plan_items = plan_item_repo
            .find_by_version(&version_id)
            .expect("查询排产明细失败");
        assert!(
            !plan_items.is_empty(),
            "应该有排产明细"
        );
        println!(
            "✓ 步骤 7: 排产明细已生成（总数: {}）",
            plan_items.len()
        );

        // 8. 刷新决策视图
        let refresh_scope = hot_rolling_aps::decision::services::RefreshScope {
            version_id: version_id.clone(),
            is_full_refresh: true,
            affected_machines: None,
            affected_date_range: None,
        };
        refresh_service
            .refresh_all(
                refresh_scope,
                hot_rolling_aps::decision::services::RefreshTrigger::ManualRefresh,
                Some("test_operator".to_string()),
            )
            .expect("决策视图刷新失败");
        println!("✓ 步骤 8: 决策视图已刷新（D1-D6）");

        // 9. 查询驾驶舱数据（D1: 最危险日期）
        let risky_days_response = dashboard_api
            .get_most_risky_date(
                &version_id,
                Some(&base_date.to_string()),
                Some(&(base_date + Duration::days(6)).to_string()),
                None,
                Some(5),
            )
            .expect("查询最危险日期失败");
        println!(
            "✓ 步骤 9: 驾驶舱数据查询成功（高风险日期数: {}）",
            risky_days_response.items.len()
        );

        // 10. 验证完整流程
        assert!(
            import_result.summary.success > 0,
            "材料导入应该成功"
        );
        assert!(recalc_result.success, "重算应该成功");
        assert!(!plan_items.is_empty(), "应该生成排产明细");

        println!("\n=== 完整业务流程测试通过 ✅ ===");
        println!("  - 材料导入: {} 条", import_result.summary.success);
        println!("  - 排产明细: {} 条", plan_items.len());
        println!("  - 高风险日期: {} 个", risky_days_response.items.len());
    }

    // ==========================================
    // 测试场景2: 材料锁定 → 重算 → 冻结区保护验证
    // ==========================================

    #[tokio::test(flavor = "multi_thread")]
    async fn test_material_lock_and_frozen_zone_protection() {
        println!("\n=== 端到端集成测试：材料锁定与冻结区保护 ===\n");

        let (
            _temp_file,
            _db_path,
            material_importer,
            plan_api,
            material_api,
            _dashboard_api,
            _refresh_service,
            capacity_repo,
            material_state_repo,
            plan_item_repo,
        ) = setup_full_test_env();
        println!("✓ 步骤 1: 测试环境已初始化");

        // 2. 导入材料
        let import_result = material_importer
            .import_from_csv("tests/fixtures/datasets/01_normal_data.csv")
            .await
            .expect("材料导入失败");
        println!(
            "✓ 步骤 2: 材料导入完成（{}条）",
            import_result.summary.success
        );

        // 3. 创建方案并准备产能池
        let plan_id = plan_api
            .create_plan(
                "冻结区测试方案".to_string(),
                "test_operator".to_string(),
            )
            .expect("创建方案失败");
        let version_id = plan_api
            .create_version(
                plan_id,
                7,
                None,
                None,
                "test_operator".to_string(),
            )
            .expect("创建版本失败");

        let base_date = chrono::Local::now().date_naive();
        prepare_capacity_pools(&capacity_repo, &version_id, vec!["H032", "H033"], base_date, 7)
            .expect("准备产能池失败");
        println!("✓ 步骤 3: 方案和产能池已准备");

        // 4. 第一次重算
        let recalc_result_1 = plan_api
            .recalc_full(&version_id, base_date, None, "test_operator")
            .expect("第一次重算失败");
        let plan_items_before = plan_item_repo
            .find_by_version(&version_id)
            .expect("查询排产明细失败");
        println!(
            "✓ 步骤 4: 第一次重算完成（排产明细: {}条）",
            plan_items_before.len()
        );

        // 5. 锁定前3条材料
        if plan_items_before.len() >= 3 {
            let material_ids_to_lock: Vec<String> = plan_items_before
                .iter()
                .take(3)
                .map(|item| item.material_id.clone())
                .collect();

            let lock_result = material_api
                .batch_lock_materials(
                    material_ids_to_lock.clone(),
                    true,
                    "test_operator",
                    "测试冻结区保护",
                    hot_rolling_aps::api::ValidationMode::Strict,
                )
                .expect("锁定材料失败");
            println!(
                "✓ 步骤 5: 材料锁定完成（成功: {}, 材料: {:?}）",
                lock_result.success_count, material_ids_to_lock
            );

            // 6. 验证材料状态已更新为Locked
            for material_id in &material_ids_to_lock {
                let state = material_state_repo
                    .find_by_id(material_id)
                    .expect("查询材料状态失败")
                    .expect("材料状态不存在");
                assert_eq!(
                    state.sched_state,
                    hot_rolling_aps::domain::types::SchedState::Locked,
                    "材料{}应该是Locked状态",
                    material_id
                );
            }
            println!("✓ 步骤 6: 材料状态已验证为Locked");

            // 7. 第二次重算（应该保护冻结区材料）
            let recalc_result_2 = plan_api
                .recalc_full(&version_id, base_date, None, "test_operator")
                .expect("第二次重算失败");
            let plan_items_after = plan_item_repo
                .find_by_version(&version_id)
                .expect("查询排产明细失败");
            println!(
                "✓ 步骤 7: 第二次重算完成（排产明细: {}条）",
                plan_items_after.len()
            );

            // 8. 验证冻结材料仍然在原位置（locked_in_plan=1）
            let locked_items: Vec<_> = plan_items_after
                .iter()
                .filter(|item| item.locked_in_plan)
                .collect();
            assert_eq!(
                locked_items.len(),
                3,
                "应该有3条冻结材料"
            );
            println!(
                "✓ 步骤 8: 冻结区保护验证通过（{}条材料保持冻结）",
                locked_items.len()
            );

            println!("\n=== 冻结区保护测试通过 ✅ ===");
        } else {
            println!("⚠️  排产明细不足3条，跳过冻结区保护测试");
        }
    }
}

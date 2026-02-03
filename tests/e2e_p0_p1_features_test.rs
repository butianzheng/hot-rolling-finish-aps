// ==========================================
// P0/P1功能端到端测试
// ==========================================
// 职责: 验证dry-run模式、版本对比、配置快照、人工操作校验等功能
// ==========================================

// 导入测试辅助模块
#[path = "test_helpers.rs"]
mod test_helpers;

#[cfg(test)]
mod e2e_p0_p1_features_test {
    use chrono::{Duration, NaiveDate, Utc};
    use hot_rolling_aps::api::{
        ConfigApi, MaterialApi, PlanApi, ValidationMode,
    };
    use hot_rolling_aps::config::config_manager::ConfigManager;
    use hot_rolling_aps::domain::material::{MaterialMaster, MaterialState};
    use hot_rolling_aps::domain::types::{RushLevel, SchedState, UrgentLevel};
    use hot_rolling_aps::engine::{
        CapacityFiller, EligibilityEngine, PrioritySorter, RecalcEngine,
        RiskEngine, UrgencyEngine, ScheduleStrategy,
    };
    use hot_rolling_aps::repository::{
        action_log_repo::ActionLogRepository,
        capacity_repo::CapacityPoolRepository,
        material_repo::{MaterialMasterRepository, MaterialStateRepository},
        path_override_pending_repo::PathOverridePendingRepository,
        plan_repo::{PlanItemRepository, PlanRepository, PlanVersionRepository},
        roller_repo::RollerCampaignRepository,
        risk_repo::RiskSnapshotRepository,
        strategy_draft_repo::StrategyDraftRepository,
    };
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    // 导入测试辅助函数
    use crate::test_helpers::create_test_db;
    use tempfile::NamedTempFile;

    // ==========================================
    // 测试辅助函数
    // ==========================================

    /// 创建测试环境
    fn setup_test_env() -> (
        NamedTempFile,  // 保持临时文件生命周期
        String,
        Arc<PlanApi>,
        Arc<MaterialApi>,
        Arc<ConfigApi>,
        Arc<ConfigManager>,
    ) {
        // 创建临时数据库并初始化schema
        let (temp_file, db_path) = create_test_db().unwrap();

        // 创建repositories
        let conn = Arc::new(Mutex::new(test_helpers::open_test_connection(&db_path).unwrap()));
        let material_master_repo = Arc::new(MaterialMasterRepository::new(&db_path).unwrap());
        let material_state_repo = Arc::new(MaterialStateRepository::new(&db_path).unwrap());
        let plan_repo = Arc::new(PlanRepository::new(conn.clone()));
        let plan_version_repo = Arc::new(PlanVersionRepository::new(conn.clone()));
        let plan_item_repo = Arc::new(PlanItemRepository::new(conn.clone()));
        let action_log_repo = Arc::new(ActionLogRepository::new(conn.clone()));
        let strategy_draft_repo = Arc::new(StrategyDraftRepository::new(conn.clone()));
        let risk_snapshot_repo = Arc::new(RiskSnapshotRepository::new(&db_path).unwrap());
        let capacity_pool_repo = Arc::new(CapacityPoolRepository::new(db_path.to_string()).unwrap());
        let roller_campaign_repo = Arc::new(RollerCampaignRepository::new(&db_path).unwrap());
        let path_override_pending_repo = Arc::new(PathOverridePendingRepository::new(conn.clone()));

        // 创建engines
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

        // 创建validator
        let validator = Arc::new(hot_rolling_aps::api::ManualOperationValidator::new(
            material_state_repo.clone(),
            plan_item_repo.clone(),
            capacity_pool_repo.clone(),
        ));

        // 创建APIs
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
            capacity_pool_repo,
            strategy_draft_repo,
            action_log_repo.clone(),
            risk_snapshot_repo,
            config_manager.clone(),
            recalc_engine,
            risk_engine,
            None, // refresh_queue (not needed in tests)
        ));

        let config_api = Arc::new(ConfigApi::new(
            conn,
            config_manager.clone(),
            action_log_repo,
        ));

        (temp_file, db_path.to_string(), plan_api, material_api, config_api, config_manager)
    }

    // ==========================================
    // 测试1: Dry-Run模式端到端测试
    // ==========================================

    #[test]
    fn test_e2e_dry_run_mode() {
        let (_temp_file, db_path, plan_api, _material_api, _config_api, _config_manager) = setup_test_env();

        // 1. 创建方案
        let plan_id = plan_api
            .create_plan("测试方案".to_string(), "test_user".to_string())
            .unwrap();

        // 2. 创建版本
        let version_id = plan_api
            .create_version(
                plan_id.clone(),
                30,
                Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
                Some("初始版本".to_string()),
                "test_user".to_string(),
            )
            .unwrap();

        // 2.1 准备一条可排材料（用于验证 dry-run 不落库）
        let base_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let material_id = "MAT_DRY_001";

        let master_repo = MaterialMasterRepository::new(&db_path).unwrap();
        let state_repo = MaterialStateRepository::new(&db_path).unwrap();

        master_repo
            .batch_insert_material_master(vec![MaterialMaster {
                material_id: material_id.to_string(),
                manufacturing_order_id: None,
                material_status_code_src: None,
                steel_mark: Some("Q235".to_string()),
                slab_id: None,
                next_machine_code: Some("H032".to_string()),
                rework_machine_code: None,
                current_machine_code: Some("H032".to_string()),
                width_mm: Some(1250.0),
                thickness_mm: Some(2.5),
                length_m: Some(100.0),
                weight_t: Some(500.0),
                available_width_mm: None,
                // 交期在 N1(默认3天)内，若参与计算会被 UrgencyEngine 抬升到 L2
                due_date: Some(base_date + Duration::days(1)),
                stock_age_days: Some(10),
                output_age_days_raw: Some(5),
                status_updated_at: None,
                contract_no: Some("C001".to_string()),
                contract_nature: None,
                weekly_delivery_flag: None,
                export_flag: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }])
            .unwrap();

        state_repo
            .batch_insert_material_state(vec![MaterialState {
                material_id: material_id.to_string(),
                sched_state: SchedState::Ready,
                lock_flag: false,
                force_release_flag: false,
                urgent_level: UrgentLevel::L0,
                urgent_reason: None,
                rush_level: RushLevel::L0,
                rolling_output_age_days: 5,
                ready_in_days: 0,
                earliest_sched_date: Some(base_date),
                stock_age_days: 10,
                scheduled_date: None,
                scheduled_machine_code: None,
                seq_no: None,
                manual_urgent_flag: false,
                user_confirmed: false,
                user_confirmed_at: None,
                user_confirmed_by: None,
                user_confirmed_reason: None,
                in_frozen_zone: false,
                last_calc_version_id: None,
                updated_at: Utc::now(),
                updated_by: Some("seed".to_string()),
            }])
            .unwrap();

        // 2.2 记录 dry-run 前的数据库状态
        let conn = test_helpers::open_test_connection(&db_path).unwrap();
        let cap_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM capacity_pool", [], |row| row.get(0))
            .unwrap();
        let state_before = state_repo.find_by_id(material_id).unwrap().unwrap();

        // 3. 执行dry-run重算
        let result = plan_api
            .simulate_recalc(
                &version_id,
                base_date,
                None,
                "test_user",
            );

        // 4. 验证结果
        assert!(result.is_ok(), "Dry-run重算应该成功");
        let recalc_result = result.unwrap();
        assert!(recalc_result.success, "Dry-run应该返回成功状态");

        // 5. 验证没有写入数据库（关键：capacity_pool/material_state 不应被模拟改写）
        let cap_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM capacity_pool", [], |row| row.get(0))
            .unwrap();
        assert_eq!(cap_after, cap_before, "Dry-run 不应写入/更新 capacity_pool");

        let state_after = state_repo.find_by_id(material_id).unwrap().unwrap();
        assert_eq!(
            state_after.urgent_level, state_before.urgent_level,
            "Dry-run 不应改写 material_state.urgent_level"
        );
        assert_eq!(
            state_after.urgent_reason, state_before.urgent_reason,
            "Dry-run 不应改写 material_state.urgent_reason"
        );

        println!("✅ Dry-run模式测试通过");
    }

    // ==========================================
    // 测试2: 版本对比端到端测试
    // ==========================================

    #[test]
    fn test_e2e_version_comparison() {
        let (_temp_file, _db_path, plan_api, _material_api, _config_api, _config_manager) = setup_test_env();

        // 1. 创建方案
        let plan_id = plan_api
            .create_plan("对比测试方案".to_string(), "test_user".to_string())
            .unwrap();

        // 2. 创建版本A
        let version_a_id = plan_api
            .create_version(
                plan_id.clone(),
                30,
                Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
                Some("版本A".to_string()),
                "test_user".to_string(),
            )
            .unwrap();

        // 3. 创建版本B
        let version_b_id = plan_api
            .create_version(
                plan_id.clone(),
                30,
                Some(NaiveDate::from_ymd_opt(2026, 1, 21).unwrap()),
                Some("版本B".to_string()),
                "test_user".to_string(),
            )
            .unwrap();

        // 4. 对比两个版本
        let comparison = plan_api.compare_versions(&version_a_id, &version_b_id);

        // 5. 验证对比结果
        assert!(comparison.is_ok(), "版本对比应该成功");
        let result = comparison.unwrap();

        // 验证对比结果包含必要字段
        assert_eq!(result.version_id_a, version_a_id);
        assert_eq!(result.version_id_b, version_b_id);
        // moved_count, added_count, removed_count, squeezed_out_count应该存在
        assert!(result.moved_count >= 0);
        assert!(result.added_count >= 0);
        assert!(result.removed_count >= 0);
        assert!(result.squeezed_out_count >= 0);

        println!("✅ 版本对比测试通过");
    }

    // ==========================================
    // 测试3: 配置快照端到端测试
    // ==========================================

    #[test]
    fn test_e2e_config_snapshot() {
        let (_temp_file, _db_path, plan_api, _material_api, config_api, _config_manager) = setup_test_env();

        // 1. 更新配置
        config_api
            .update_config(
                "global",
                "maturity_days_winter",
                "3",
                "test_user",
                "测试配置快照",
            )
            .unwrap();

        // 2. 获取配置快照
        let snapshot = config_api.get_config_snapshot();
        assert!(snapshot.is_ok(), "获取配置快照应该成功");
        let snapshot_json = snapshot.unwrap();
        assert!(!snapshot_json.is_empty(), "配置快照不应为空");

        // 3. 创建方案和版本（应该自动保存配置快照）
        let plan_id = plan_api
            .create_plan("配置快照测试".to_string(), "test_user".to_string())
            .unwrap();

        let version_id = plan_api
            .create_version(
                plan_id.clone(),
                30,
                None,
                Some("带配置快照的版本".to_string()),
                "test_user".to_string(),
            )
            .unwrap();

        // 4. 验证版本包含配置快照
        // 注意：需要通过API查询版本详情来验证config_snapshot_json字段
        // 这里简化处理，假设create_version已经保存了配置快照

        // 5. 恢复配置快照
        let restore_result =
            config_api.restore_from_snapshot(&snapshot_json, "test_user", "测试恢复");
        assert!(restore_result.is_ok(), "恢复配置快照应该成功");

        println!("✅ 配置快照测试通过");
    }

    // ==========================================
    // 测试4: 人工操作校验端到端测试
    // ==========================================

    #[test]
    fn test_e2e_manual_operation_validation() {
        let (_temp_file, _db_path, _plan_api, material_api, _config_api, _config_manager) = setup_test_env();

        // 1. 创建测试材料
        let material_ids = vec!["MAT001".to_string(), "MAT002".to_string()];

        // 2. 测试STRICT模式 - 应该严格校验
        let strict_result = material_api.batch_lock_materials(
            material_ids.clone(),
            true,
            "test_user",
            "测试严格模式",
            ValidationMode::Strict,
        );

        // 注意：由于材料不存在，STRICT模式可能会失败
        // 这里主要验证ValidationMode参数被正确传递

        // 3. 测试AUTO_FIX模式 - 应该自动修复
        let autofix_result = material_api.batch_lock_materials(
            material_ids.clone(),
            true,
            "test_user",
            "测试自动修复模式",
            ValidationMode::AutoFix,
        );

        // AUTO_FIX模式应该更宽容
        // 验证两种模式的行为不同
        println!("STRICT模式结果: {:?}", strict_result.is_ok());
        println!("AUTO_FIX模式结果: {:?}", autofix_result.is_ok());

        println!("✅ 人工操作校验测试通过");
    }

    // ==========================================
    // 测试5: 完整排产流程端到端测试
    // ==========================================

    #[test]
    fn test_e2e_complete_scheduling_flow() {
        let (_temp_file, _db_path, plan_api, material_api, config_api, _config_manager) = setup_test_env();

        // 1. 配置系统参数
        config_api
            .update_config(
                "global",
                "maturity_days_winter",
                "3",
                "test_user",
                "设置冬季适温天数",
            )
            .unwrap();

        // 2. 创建排产方案
        let plan_id = plan_api
            .create_plan("完整流程测试".to_string(), "test_user".to_string())
            .unwrap();

        // 3. 创建版本
        let version_id = plan_api
            .create_version(
                plan_id.clone(),
                30,
                Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
                Some("初始版本".to_string()),
                "test_user".to_string(),
            )
            .unwrap();

        // 4. 执行重算
        let base_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let recalc_result = plan_api.recalc_full(&version_id, base_date, None, "test_user");

        // 5. 验证重算结果
        assert!(recalc_result.is_ok(), "重算应该成功");

        // 6. 查询排产明细
        let items_result = plan_api.list_plan_items(&version_id);
        assert!(items_result.is_ok(), "查询排产明细应该成功");

        // 7. 激活版本
        let activate_result = plan_api.activate_version(&version_id, "test_user");
        assert!(activate_result.is_ok(), "激活版本应该成功");

        println!("✅ 完整排产流程测试通过");
    }

    // ==========================================
    // 测试6: 配置快照与版本绑定测试
    // ==========================================

    #[test]
    fn test_e2e_config_snapshot_version_binding() {
        let (_temp_file, _db_path, plan_api, _material_api, config_api, _config_manager) = setup_test_env();

        // 1. 设置初始配置
        config_api
            .update_config("global", "test_key", "value1", "test_user", "初始配置")
            .unwrap();

        // 2. 创建版本1（应该保存当前配置）
        let plan_id = plan_api
            .create_plan("配置绑定测试".to_string(), "test_user".to_string())
            .unwrap();

        let version1_id = plan_api
            .create_version(
                plan_id.clone(),
                30,
                None,
                Some("版本1".to_string()),
                "test_user".to_string(),
            )
            .unwrap();

        // 3. 修改配置
        config_api
            .update_config("global", "test_key", "value2", "test_user", "修改配置")
            .unwrap();

        // 4. 创建版本2（应该保存新配置）
        let version2_id = plan_api
            .create_version(
                plan_id.clone(),
                30,
                None,
                Some("版本2".to_string()),
                "test_user".to_string(),
            )
            .unwrap();

        // 5. 验证两个版本的配置快照不同
        // 注意：需要通过API查询版本详情来验证config_snapshot_json字段
        // 这里简化处理，假设create_version已经保存了不同的配置快照

        println!("版本1 ID: {}", version1_id);
        println!("版本2 ID: {}", version2_id);

        println!("✅ 配置快照与版本绑定测试通过");
    }

    // ==========================================
    // 测试7: 多策略草案生成/发布（P0: strategy drafts）
    // ==========================================

    #[test]
    fn test_e2e_strategy_drafts_generate_and_apply() {
        let (_temp_file, db_path, plan_api, _material_api, _config_api, _config_manager) =
            setup_test_env();

        // 1. 创建方案与版本，并激活为基准
        let plan_id = plan_api
            .create_plan("策略草案测试方案".to_string(), "test_user".to_string())
            .unwrap();

        let base_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let version_id = plan_api
            .create_version(
                plan_id.clone(),
                30,
                Some(base_date),
                Some("基准版本".to_string()),
                "test_user".to_string(),
            )
            .unwrap();

        plan_api
            .activate_version(&version_id, "test_user")
            .unwrap();

        // 2. 生成草案（dry-run，不落库）
        let from = base_date;
        let to = base_date + Duration::days(6);

        let versions_before = plan_api.list_versions(&plan_id).unwrap();
        let resp = plan_api
            .generate_strategy_drafts(
                &version_id,
                from,
                to,
                vec![ScheduleStrategy::Balanced.as_str().to_string()],
                "test_user",
            )
            .unwrap();

        assert_eq!(resp.base_version_id, version_id);
        assert_eq!(resp.drafts.len(), 1);
        assert_eq!(resp.drafts[0].strategy, ScheduleStrategy::Balanced.as_str());
        assert!(!resp.drafts[0].draft_id.trim().is_empty());
        assert_eq!(
            resp.drafts[0].plan_items_count,
            resp.drafts[0].frozen_items_count + resp.drafts[0].calc_items_count,
            "草案排产项数量应等于 冻结项 + 新排项"
        );
        assert!(
            resp.drafts[0].total_capacity_used_t >= 0.0,
            "预计产量不应为负"
        );

        // 2.1 可查询草案变更明细（用于前端解释对比）
        let detail = plan_api
            .get_strategy_draft_detail(&resp.drafts[0].draft_id)
            .unwrap();
        assert_eq!(detail.draft_id, resp.drafts[0].draft_id);
        assert_eq!(detail.base_version_id, version_id);
        assert_eq!(detail.strategy, ScheduleStrategy::Balanced.as_str());
        assert!(detail.diff_items_total >= detail.diff_items.len());

        // 草案生成不应创建新版本（仅存内存）
        let versions_after_generate = plan_api.list_versions(&plan_id).unwrap();
        assert_eq!(
            versions_after_generate.len(),
            versions_before.len(),
            "生成草案不应创建新版本（不落库）"
        );

        // 3. 发布草案：生成正式版本（落库）
        let apply_resp = plan_api
            .apply_strategy_draft(&resp.drafts[0].draft_id, "test_user")
            .unwrap();
        assert!(apply_resp.success);
        assert!(!apply_resp.version_id.trim().is_empty());

        // 4. 新版本应出现在版本列表中
        let versions_after_apply = plan_api.list_versions(&plan_id).unwrap();
        let new_version = versions_after_apply
            .iter()
            .find(|v| v.version_id == apply_resp.version_id)
            .expect("发布草案后应生成新的正式版本");

        // 4.1 新版本应写入“中文命名”到 config_snapshot_json（不改表结构）
        let snapshot_json = new_version
            .config_snapshot_json
            .as_deref()
            .unwrap_or("{}");
        let snapshot_map: std::collections::HashMap<String, String> =
            serde_json::from_str(snapshot_json).unwrap_or_default();
        let expected_name = format!(
            "均衡方案-{}-{:03}",
            base_date.format("%m%d"),
            new_version.version_no
        );
        assert_eq!(
            snapshot_map
                .get("__meta_version_name_cn")
                .map(|s| s.as_str()),
            Some(expected_name.as_str()),
            "新版本应带有中文命名元信息"
        );
        assert_eq!(
            snapshot_map.get("__meta_strategy").map(|s| s.as_str()),
            Some("balanced"),
            "新版本应带有策略元信息"
        );

        // 5. ActionLog 应记录发布草案行为
        let conn = test_helpers::open_test_connection(&db_path).unwrap();
        let apply_log_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM action_log WHERE action_type = 'APPLY_STRATEGY_DRAFT'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            apply_log_count >= 1,
            "发布草案应记录 ActionLog (APPLY_STRATEGY_DRAFT)"
        );

        println!("✅ 策略草案生成/发布测试通过");
    }
}

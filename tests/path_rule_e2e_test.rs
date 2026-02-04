// ==========================================
// PathRule v0.6 端到端测试（后端闭环）
// ==========================================
// 目标:
// - 运行一次重算：生成锚点后，违规材料被 PATH_OVERRIDE_REQUIRED 拦截（不入 plan_item）
// - 查询待确认列表：能定位该违规材料
// - 人工确认：写 material_state.user_confirmed* + action_log
// - 再次重算：已确认材料可正常入池
// ==========================================

#[path = "test_helpers.rs"]
mod test_helpers;

mod helpers;

#[cfg(test)]
mod path_rule_e2e_test {
    use crate::helpers::test_data_builder::{MaterialBuilder, MaterialStateBuilder};
    use crate::test_helpers;
    use crate::test_helpers::create_test_db;
    use chrono::{Duration, NaiveDate};
    use hot_rolling_aps::api::{PathRuleApi, PlanApi};
    use hot_rolling_aps::config::config_manager::ConfigManager;
    use hot_rolling_aps::engine::{
        CapacityFiller, EligibilityEngine, PrioritySorter, RecalcEngine, RiskEngine, UrgencyEngine,
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
    use tempfile::NamedTempFile;

    fn setup_env() -> (
        NamedTempFile,
        String,
        Arc<PlanApi>,
        Arc<PathRuleApi>,
        Arc<PlanItemRepository>,
        Arc<ActionLogRepository>,
        Arc<MaterialMasterRepository>,
        Arc<MaterialStateRepository>,
    ) {
        let (temp_file, db_path) = create_test_db().expect("create_test_db failed");
        let conn = Arc::new(Mutex::new(test_helpers::open_test_connection(&db_path).expect("open db failed")));

        // === Repository ===
        let material_master_repo =
            Arc::new(MaterialMasterRepository::new(&db_path).expect("MaterialMasterRepository init failed"));
        let material_state_repo =
            Arc::new(MaterialStateRepository::new(&db_path).expect("MaterialStateRepository init failed"));
        let plan_repo = Arc::new(PlanRepository::new(conn.clone()));
        let plan_version_repo = Arc::new(PlanVersionRepository::new(conn.clone()));
        let plan_item_repo = Arc::new(PlanItemRepository::new(conn.clone()));
        let action_log_repo = Arc::new(ActionLogRepository::new(conn.clone()));
        let strategy_draft_repo = Arc::new(StrategyDraftRepository::new(conn.clone()));
        let risk_snapshot_repo =
            Arc::new(RiskSnapshotRepository::new(&db_path).expect("RiskSnapshotRepository init failed"));
        let capacity_pool_repo =
            Arc::new(CapacityPoolRepository::new(db_path.clone()).expect("CapacityPoolRepository init failed"));
        let roller_campaign_repo =
            Arc::new(RollerCampaignRepository::new(&db_path).expect("RollerCampaignRepository init failed"));
        let path_override_pending_repo = Arc::new(PathOverridePendingRepository::new(conn.clone()));

        // === Engine ===
        let config_manager = Arc::new(ConfigManager::new(&db_path).expect("ConfigManager init failed"));
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

        // === API ===
        let plan_api = Arc::new(PlanApi::new(
            plan_repo,
            plan_version_repo,
            plan_item_repo.clone(),
            material_state_repo.clone(),
            material_master_repo.clone(),
            capacity_pool_repo,
            strategy_draft_repo,
            action_log_repo.clone(),
            risk_snapshot_repo,
            config_manager.clone(),
            recalc_engine,
            risk_engine,
            None,
        ));

        let path_rule_api = Arc::new(PathRuleApi::new(
            conn,
            config_manager,
            plan_item_repo.clone(),
            material_master_repo.clone(),
            material_state_repo.clone(),
            roller_campaign_repo,
            action_log_repo.clone(),
            path_override_pending_repo,
        ));

        (
            temp_file,
            db_path,
            plan_api,
            path_rule_api,
            plan_item_repo,
            action_log_repo,
            material_master_repo,
            material_state_repo,
        )
    }

    #[test]
    fn test_path_rule_e2e_pending_confirm_and_reschedule() {
        let (
            _temp_file,
            _db_path,
            plan_api,
            path_rule_api,
            plan_item_repo,
            action_log_repo,
            material_master_repo,
            material_state_repo,
        ) = setup_env();

        let base_date = NaiveDate::from_ymd_opt(2026, 2, 2).unwrap();
        let machine_code = "H032";
        let anchor_id = "MAT_ANCHOR";
        let candidate_id = "MAT_WIDE";

        // 1) 写入两条材料：先排一块“窄薄”作为日内锚点，再来一块“宽厚”触发待确认
        let mut anchor_master = MaterialBuilder::new(anchor_id)
            .machine(machine_code)
            .weight(10.0)
            .output_age_days(10)
            .due_date(base_date + Duration::days(30))
            .build();
        anchor_master.width_mm = Some(1000.0);
        anchor_master.thickness_mm = Some(10.0);

        let mut candidate_master = MaterialBuilder::new(candidate_id)
            .machine(machine_code)
            .weight(10.0)
            .output_age_days(5)
            .due_date(base_date + Duration::days(1)) // N1 内 => L2
            .build();
        candidate_master.width_mm = Some(1200.0);
        candidate_master.thickness_mm = Some(12.5);

        let anchor_state = MaterialStateBuilder::new(anchor_id).build();
        let candidate_state = MaterialStateBuilder::new(candidate_id).build();

        material_master_repo
            .batch_insert_material_master(vec![anchor_master.clone(), candidate_master.clone()])
            .expect("insert material_master failed");
        material_state_repo
            .batch_insert_material_state(vec![anchor_state, candidate_state])
            .expect("insert material_state failed");

        // 2) 创建版本，并执行第一次重算（期望：candidate 被 PATH_OVERRIDE_REQUIRED 拦截）
        let plan_id = plan_api
            .create_plan("PathRule E2E".to_string(), "tester".to_string())
            .expect("create_plan failed");
        let base_version_id = plan_api
            .create_version(plan_id, 1, None, None, "tester".to_string())
            .expect("create_version failed");

        let res1 = plan_api
            .recalc_full(&base_version_id, base_date, None, "tester")
            .expect("recalc_full (1st) failed");
        let version_1 = res1.version_id.clone();

        let items_v1 = plan_item_repo
            .find_by_version(&version_1)
            .expect("find_by_version v1 failed");
        assert!(
            items_v1.iter().any(|i| i.material_id == anchor_id),
            "首次重算应排入 anchor 材料"
        );
        assert!(
            !items_v1.iter().any(|i| i.material_id == candidate_id),
            "首次重算不应排入待确认材料"
        );

        // 3) 待确认列表应能查到 candidate
        let pending_v1 = path_rule_api
            .list_path_override_pending(&version_1, machine_code, base_date)
            .expect("list_path_override_pending failed");
        assert!(
            pending_v1.iter().any(|p| p.material_id == candidate_id),
            "待确认列表应包含 candidate"
        );

        // 4) 人工确认突破（写 material_state.user_confirmed* + action_log）
        path_rule_api
            .confirm_path_override(&version_1, candidate_id, "tester", "E2E确认")
            .expect("confirm_path_override failed");

        let confirmed_state = material_state_repo
            .find_by_id(candidate_id)
            .expect("find_by_id candidate failed")
            .expect("candidate state missing");
        assert!(confirmed_state.user_confirmed, "candidate 应标记为已确认");

        let logs = action_log_repo
            .find_by_version_id(&version_1)
            .expect("find_by_version_id action_log failed");
        assert!(
            logs.iter().any(|l| l.action_type == "PathOverrideConfirm"),
            "应写入 PathOverrideConfirm 审计"
        );

        // 5) 再次重算：已确认材料应可入池
        let res2 = plan_api
            .recalc_full(&base_version_id, base_date, None, "tester")
            .expect("recalc_full (2nd) failed");
        let version_2 = res2.version_id.clone();

        let items_v2 = plan_item_repo
            .find_by_version(&version_2)
            .expect("find_by_version v2 failed");
        assert!(
            items_v2.iter().any(|i| i.material_id == candidate_id),
            "二次重算后，已确认材料应可入池"
        );

        let pending_v2 = path_rule_api
            .list_path_override_pending(&version_2, machine_code, base_date)
            .expect("list_path_override_pending v2 failed");
        assert!(
            !pending_v2.iter().any(|p| p.material_id == candidate_id),
            "已确认材料不应再次出现在待确认列表"
        );
    }
}

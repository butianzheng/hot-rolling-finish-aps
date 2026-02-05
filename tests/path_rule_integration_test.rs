// ==========================================
// PathRule v0.6 集成测试
// ==========================================
// 目标:
// - 待确认列表可查询（依赖重算落库的 path_override_pending）
// - 人工确认可落库（material_state.user_confirmed*）并写审计（action_log）
// - 确认后同一候选在引擎侧不再被路径门控拦截（user_confirmed -> OK）
// ==========================================

#[path = "test_helpers.rs"]
mod test_helpers;

mod helpers;

#[cfg(test)]
mod path_rule_integration_test {
    use crate::helpers::test_data_builder::{CapacityPoolBuilder, MaterialBuilder, MaterialStateBuilder};
    use crate::test_helpers;
    use crate::test_helpers::create_test_db;
    use chrono::NaiveDate;
    use hot_rolling_aps::api::PathRuleApi;
    use hot_rolling_aps::config::ConfigManager;
    use hot_rolling_aps::domain::roller::RollerCampaign;
    use hot_rolling_aps::domain::types::{AnchorSource, RollStatus, SchedState, UrgentLevel};
    use hot_rolling_aps::engine::{Anchor, CapacityFiller, PathRuleConfig, PathRuleEngine};
    use hot_rolling_aps::repository::action_log_repo::ActionLogRepository;
    use hot_rolling_aps::repository::material_repo::{MaterialMasterRepository, MaterialStateRepository};
    use hot_rolling_aps::repository::path_override_pending_repo::{PathOverridePendingRecord, PathOverridePendingRepository};
    use hot_rolling_aps::repository::plan_repo::PlanItemRepository;
    use hot_rolling_aps::repository::roller_repo::RollerCampaignRepository;
    use rusqlite::{params, Connection};
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_path_override_pending_confirm_and_engine_gate() {
        let (_tmp, db_path) = create_test_db().expect("create_test_db failed");

        let conn = test_helpers::open_test_connection(&db_path).expect("open db failed");
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
        let conn = Arc::new(Mutex::new(conn));

        let plan_id = "P_PATH_RULE";
        let version_id = "V_PATH_RULE";
        let machine_code = "H032";
        let plan_date = NaiveDate::from_ymd_opt(2026, 2, 2).unwrap();

        // 基础计划/版本（用于满足外键：roller_campaign.version_id -> plan_version.version_id）
        {
            let c = conn.lock().unwrap();
            c.execute(
                "INSERT INTO plan (plan_id, plan_name, plan_type, base_plan_id, created_by) VALUES (?1, ?2, ?3, NULL, ?4)",
                params![plan_id, "PathRule 测试方案", "SCENARIO", "tester"],
            )
            .unwrap();
            c.execute(
                "INSERT INTO plan_version (version_id, plan_id, version_no, status, created_by) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![version_id, plan_id, 1, "ACTIVE", "tester"],
            )
            .unwrap();
            c.execute(
                "INSERT OR REPLACE INTO config_kv (scope_id, key, value) VALUES ('global', 'path_override_allowed_urgency_levels', 'L2,L3')",
                [],
            )
            .unwrap();
        }

        let material_master_repo = Arc::new(MaterialMasterRepository::from_connection(conn.clone()));
        let material_state_repo = Arc::new(MaterialStateRepository::from_connection(conn.clone()));
        let roller_campaign_repo = Arc::new(RollerCampaignRepository::from_connection(conn.clone()));
        let action_log_repo = Arc::new(ActionLogRepository::new(conn.clone()));

        // 构造：锚点材料（更窄更薄） + 候选材料（更宽更厚，需人工突破）
        let anchor_material_id = "A001";
        let candidate_material_id = "C001";

        let mut anchor_master = MaterialBuilder::new(anchor_material_id)
            .machine(machine_code)
            .weight(50.0)
            .build();
        anchor_master.width_mm = Some(1000.0);
        anchor_master.thickness_mm = Some(10.0);

        let mut candidate_master = MaterialBuilder::new(candidate_material_id)
            .machine(machine_code)
            .weight(60.0)
            .build();
        candidate_master.width_mm = Some(1300.0);
        candidate_master.thickness_mm = Some(12.0);

        let anchor_state = MaterialStateBuilder::new(anchor_material_id)
            .sched_state(SchedState::Ready)
            .urgent_level(UrgentLevel::L0)
            .build();
        let candidate_state = MaterialStateBuilder::new(candidate_material_id)
            .sched_state(SchedState::Ready)
            .urgent_level(UrgentLevel::L2)
            .build();

        material_master_repo
            .batch_insert_material_master(vec![anchor_master.clone(), candidate_master.clone()])
            .unwrap();
        material_state_repo
            .batch_insert_material_state(vec![anchor_state, candidate_state.clone()])
            .unwrap();

        // 创建活跃换辊周期 + 持久化锚点（锚点 < 候选 -> 需要突破）
        let campaign = RollerCampaign {
            version_id: version_id.to_string(),
            machine_code: machine_code.to_string(),
            campaign_no: 1,
            start_date: plan_date,
            end_date: None,
            cum_weight_t: 0.0,
            suggest_threshold_t: 1500.0,
            hard_limit_t: 2500.0,
            status: RollStatus::Normal,
            path_anchor_material_id: Some(anchor_material_id.to_string()),
            path_anchor_width_mm: Some(1000.0),
            path_anchor_thickness_mm: Some(10.0),
            anchor_source: Some(AnchorSource::None),
        };
        roller_campaign_repo.create(&campaign).unwrap();

        // PathRuleApi：待确认列表应包含候选材料
        let plan_item_repo = Arc::new(PlanItemRepository::new(conn.clone()));
        let config_manager = Arc::new(ConfigManager::new(&db_path).unwrap());
        let path_override_pending_repo = Arc::new(PathOverridePendingRepository::new(conn.clone()));
        let path_rule_api = PathRuleApi::new(
            conn.clone(),
            config_manager,
            plan_item_repo,
            material_master_repo.clone(),
            material_state_repo.clone(),
            roller_campaign_repo.clone(),
            action_log_repo.clone(),
            path_override_pending_repo.clone(),
        );

        // 模拟“最近一次重算”落库的待确认记录（首遇日期=plan_date）
        path_override_pending_repo
            .insert_ignore_many(&[PathOverridePendingRecord {
                version_id: version_id.to_string(),
                machine_code: machine_code.to_string(),
                plan_date,
                material_id: candidate_material_id.to_string(),
                violation_type: "BOTH_EXCEEDED".to_string(),
                urgent_level: "L2".to_string(),
                width_mm: 1300.0,
                thickness_mm: 12.0,
                anchor_width_mm: 1000.0,
                anchor_thickness_mm: 10.0,
                width_delta_mm: 300.0,
                thickness_delta_mm: 2.0,
            }])
            .unwrap();

        let pending = path_rule_api
            .list_path_override_pending(version_id, machine_code, plan_date)
            .expect("list_path_override_pending failed");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].material_id, candidate_material_id);

        // Engine gate：未确认时应被 PATH_OVERRIDE_REQUIRED 拦截
        let engine = PathRuleEngine::new(PathRuleConfig::default());
        let filler = CapacityFiller::new();
        let mut pool = CapacityPoolBuilder::new(machine_code, plan_date)
            .version_id(version_id)
            .target(1000.0)
            .limit(1200.0)
            .build();
        let candidates = vec![(candidate_master.clone(), candidate_state.clone())];
        let res = filler.fill_single_day_with_path_rule(
            &mut pool,
            &candidates,
            vec![],
            version_id,
            Some(&engine),
            Some(Anchor { width_mm: 1000.0, thickness_mm: 10.0 }),
            Some(anchor_material_id.to_string()),
        );
        assert!(res.plan_items.is_empty());
        assert_eq!(res.skipped_materials.len(), 1);
        assert!(res.skipped_materials[0].2.contains("PATH_OVERRIDE_REQUIRED"));

        // API：人工确认突破
        path_rule_api
            .confirm_path_override(version_id, candidate_material_id, "tester", "交付临期")
            .expect("confirm_path_override failed");

        // 确认后：待确认列表应为空（user_confirmed=1 的候选不再提示）
        let pending2 = path_rule_api
            .list_path_override_pending(version_id, machine_code, plan_date)
            .expect("list_path_override_pending (after confirm) failed");
        assert!(pending2.is_empty());

        // 落库校验：material_state.user_confirmed* 已写入
        let confirmed_state = material_state_repo
            .find_by_id(candidate_material_id)
            .unwrap()
            .unwrap();
        assert!(confirmed_state.user_confirmed);
        assert_eq!(
            confirmed_state.user_confirmed_by.as_deref().unwrap_or(""),
            "tester"
        );
        assert_eq!(
            confirmed_state.user_confirmed_reason.as_deref().unwrap_or(""),
            "交付临期"
        );

        // Engine gate：已确认后应放行并可入池
        let mut pool2 = CapacityPoolBuilder::new(machine_code, plan_date)
            .version_id(version_id)
            .target(1000.0)
            .limit(1200.0)
            .build();
        let candidates2 = vec![(candidate_master, confirmed_state)];
        let res2 = filler.fill_single_day_with_path_rule(
            &mut pool2,
            &candidates2,
            vec![],
            version_id,
            Some(&engine),
            Some(Anchor { width_mm: 1000.0, thickness_mm: 10.0 }),
            Some(anchor_material_id.to_string()),
        );
        assert_eq!(res2.plan_items.len(), 1);
        assert!(res2.skipped_materials.is_empty());

        // 审计：应写入 PathOverrideConfirm
        let logs = action_log_repo.find_by_version_id(version_id).unwrap();
        assert!(logs.iter().any(|l| l.action_type == "PathOverrideConfirm"));
    }

    #[test]
    fn test_roll_cycle_reset_creates_new_campaign() {
        let (_tmp, db_path) = create_test_db().expect("create_test_db failed");

        let conn = test_helpers::open_test_connection(&db_path).expect("open db failed");
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
        let conn = Arc::new(Mutex::new(conn));

        let plan_id = "P_ROLL_RESET";
        let version_id = "V_ROLL_RESET";
        let machine_code = "H032";
        let today = NaiveDate::from_ymd_opt(2026, 2, 2).unwrap();

        {
            let c = conn.lock().unwrap();
            c.execute(
                "INSERT INTO plan (plan_id, plan_name, plan_type, base_plan_id, created_by) VALUES (?1, ?2, ?3, NULL, ?4)",
                params![plan_id, "RollReset 测试方案", "SCENARIO", "tester"],
            )
            .unwrap();
            c.execute(
                "INSERT INTO plan_version (version_id, plan_id, version_no, status, created_by) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![version_id, plan_id, 1, "ACTIVE", "tester"],
            )
            .unwrap();
        }

        let roller_campaign_repo = Arc::new(RollerCampaignRepository::from_connection(conn.clone()));
        let campaign = RollerCampaign {
            version_id: version_id.to_string(),
            machine_code: machine_code.to_string(),
            campaign_no: 1,
            start_date: today,
            end_date: None,
            cum_weight_t: 123.0,
            suggest_threshold_t: 1500.0,
            hard_limit_t: 2500.0,
            status: RollStatus::Normal,
            path_anchor_material_id: Some("A001".to_string()),
            path_anchor_width_mm: Some(1000.0),
            path_anchor_thickness_mm: Some(10.0),
            anchor_source: Some(AnchorSource::SeedS2),
        };
        roller_campaign_repo.create(&campaign).unwrap();

        let plan_item_repo = Arc::new(PlanItemRepository::new(conn.clone()));
        let config_manager = Arc::new(ConfigManager::new(&db_path).unwrap());
        let material_master_repo = Arc::new(MaterialMasterRepository::from_connection(conn.clone()));
        let material_state_repo = Arc::new(MaterialStateRepository::from_connection(conn.clone()));
        let action_log_repo = Arc::new(ActionLogRepository::new(conn.clone()));
        let path_override_pending_repo = Arc::new(PathOverridePendingRepository::new(conn.clone()));

        let path_rule_api = PathRuleApi::new(
            conn.clone(),
            config_manager,
            plan_item_repo,
            material_master_repo,
            material_state_repo,
            roller_campaign_repo.clone(),
            action_log_repo,
            path_override_pending_repo,
        );

        path_rule_api
            .reset_roll_cycle(version_id, machine_code, "tester", "现场换辊")
            .expect("reset_roll_cycle failed");

        let anchor = path_rule_api
            .get_roll_cycle_anchor(version_id, machine_code)
            .expect("get_roll_cycle_anchor failed")
            .expect("active campaign missing after reset");

        assert_eq!(anchor.campaign_no, 2);
        assert_eq!(anchor.anchor_source.to_uppercase(), "NONE");
        assert!(anchor.anchor_width_mm.is_none());
        assert!(anchor.anchor_thickness_mm.is_none());
        assert!(anchor.anchor_material_id.is_none());
    }
}

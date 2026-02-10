// ==========================================
// 系统性能测试
// ==========================================
// 职责: 验证系统在大数据量下的性能表现
// ==========================================

#[path = "test_helpers.rs"]
mod test_helpers;

#[cfg(test)]
mod system_performance_test {
    use chrono::{NaiveDate, Utc};
    use hot_rolling_aps::api::PlanApi;
    use hot_rolling_aps::config::config_manager::ConfigManager;
    use hot_rolling_aps::domain::{MaterialMaster, MaterialState};
    use hot_rolling_aps::engine::{
        CapacityFiller, EligibilityEngine, PrioritySorter, RecalcEngine, RiskEngine, UrgencyEngine,
    };
    use hot_rolling_aps::repository::{
        action_log_repo::ActionLogRepository,
        capacity_repo::CapacityPoolRepository,
        material_repo::{MaterialMasterRepository, MaterialStateRepository},
        path_override_pending_repo::PathOverridePendingRepository,
        plan_repo::{PlanItemRepository, PlanRepository, PlanVersionRepository},
        risk_repo::RiskSnapshotRepository,
        roller_repo::RollerCampaignRepository,
        strategy_draft_repo::StrategyDraftRepository,
    };
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use tempfile::NamedTempFile;

    use crate::test_helpers;
    use crate::test_helpers::create_test_db;

    // ==========================================
    // 测试数据生成器
    // ==========================================

    /// 生成大量测试材料数据
    fn generate_test_materials(
        count: usize,
        base_date: NaiveDate,
    ) -> Vec<(MaterialMaster, MaterialState)> {
        let mut materials = Vec::with_capacity(count);

        let steel_grades = vec!["Q235", "Q345", "Q390", "Q420", "Q460"];
        let machines = vec!["M01", "M02", "M03", "M04", "M05"];

        for i in 0..count {
            let material_id = format!("MAT{:06}", i + 1);
            let steel_grade = steel_grades[i % steel_grades.len()].to_string();
            let machine_id = machines[i % machines.len()].to_string();

            // 材料主数据
            let master = MaterialMaster {
                material_id: material_id.clone(),
                manufacturing_order_id: Some(format!("MO{:06}", i)),
                material_status_code_src: Some("READY".to_string()),
                steel_mark: Some(steel_grade.clone()),
                slab_id: Some(format!("SLAB{:06}", i)),
                next_machine_code: Some(machine_id.clone()),
                rework_machine_code: None,
                current_machine_code: Some(machine_id.clone()),
                width_mm: Some(1000.0 + (i % 500) as f64),
                thickness_mm: Some(10.0 + (i % 20) as f64),
                length_m: Some(10.0 + (i % 20) as f64 * 0.1),
                weight_t: Some(5.0 + (i % 10) as f64),
                available_width_mm: Some(1000.0 + (i % 500) as f64),
                due_date: Some(base_date + chrono::Duration::days((i % 60) as i64)),
                stock_age_days: Some((i % 30) as i32),
                output_age_days_raw: Some((i % 30) as i32),
                rolling_output_date: None,
                status_updated_at: Some(Utc::now()),
                contract_no: Some(format!("CONTRACT{:04}", i % 200)),
                contract_nature: Some("NORMAL".to_string()),
                weekly_delivery_flag: Some("0".to_string()),
                export_flag: Some("0".to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            // 材料状态
            use hot_rolling_aps::domain::types::{RushLevel, SchedState, UrgentLevel};
            let state = MaterialState {
                material_id: material_id.clone(),
                sched_state: SchedState::Ready,
                lock_flag: i % 20 == 0, // 5%冻结
                force_release_flag: false,
                urgent_level: if i % 50 == 0 {
                    UrgentLevel::L0
                } else {
                    UrgentLevel::L3
                },
                urgent_reason: None,
                rush_level: if i % 10 == 0 {
                    RushLevel::L1
                } else {
                    RushLevel::L2
                },
                rolling_output_age_days: (i % 30) as i32,
                ready_in_days: 0,
                earliest_sched_date: Some(base_date),
                stock_age_days: (i % 30) as i32,
                scheduled_date: None,
                scheduled_machine_code: None,
                seq_no: None,
                manual_urgent_flag: i % 50 == 0, // 2%紧急
                user_confirmed: false,
                user_confirmed_at: None,
                user_confirmed_by: None,
                user_confirmed_reason: None,
                in_frozen_zone: i % 20 == 0,
                last_calc_version_id: None,
                updated_at: Utc::now(),
                updated_by: Some("test_generator".to_string()),
            };

            materials.push((master, state));
        }

        materials
    }

    /// 批量插入材料数据
    fn bulk_insert_materials(
        material_master_repo: &MaterialMasterRepository,
        material_state_repo: &MaterialStateRepository,
        materials: Vec<(MaterialMaster, MaterialState)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (masters, states): (Vec<_>, Vec<_>) = materials.into_iter().unzip();
        material_master_repo.batch_insert_material_master(masters)?;
        material_state_repo.batch_insert_material_state(states)?;
        Ok(())
    }

    /// 创建性能测试环境
    fn setup_performance_test_env(
        material_count: usize,
    ) -> (
        NamedTempFile,
        String,
        Arc<PlanApi>,
        Arc<MaterialMasterRepository>,
        Arc<MaterialStateRepository>,
    ) {
        let (temp_file, db_path) = create_test_db().unwrap();

        let conn = Arc::new(Mutex::new(
            test_helpers::open_test_connection(&db_path).unwrap(),
        ));
        let material_master_repo = Arc::new(MaterialMasterRepository::new(&db_path).unwrap());
        let material_state_repo = Arc::new(MaterialStateRepository::new(&db_path).unwrap());
        let plan_repo = Arc::new(PlanRepository::new(conn.clone()));
        let plan_version_repo = Arc::new(PlanVersionRepository::new(conn.clone()));
        let plan_item_repo = Arc::new(PlanItemRepository::new(conn.clone()));
        let action_log_repo = Arc::new(ActionLogRepository::new(conn.clone()));
        let strategy_draft_repo = Arc::new(StrategyDraftRepository::new(conn.clone()));
        let risk_snapshot_repo = Arc::new(RiskSnapshotRepository::new(&db_path).unwrap());
        let capacity_pool_repo =
            Arc::new(CapacityPoolRepository::new(db_path.to_string()).unwrap());
        let roller_campaign_repo = Arc::new(RollerCampaignRepository::new(&db_path).unwrap());
        let path_override_pending_repo = Arc::new(PathOverridePendingRepository::new(conn.clone()));

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
            None, // 测试环境不需要事件发布
        ));

        let plan_api = Arc::new(PlanApi::new(
            plan_repo,
            plan_version_repo,
            plan_item_repo,
            material_state_repo.clone(),
            material_master_repo.clone(),
            capacity_pool_repo,
            strategy_draft_repo,
            action_log_repo,
            risk_snapshot_repo,
            config_manager.clone(),
            recalc_engine,
            risk_engine,
            None, // 测试环境不需要事件发布
        ));

        // 生成并插入测试数据
        let base_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let materials = generate_test_materials(material_count, base_date);
        bulk_insert_materials(&material_master_repo, &material_state_repo, materials).unwrap();

        (
            temp_file,
            db_path,
            plan_api,
            material_master_repo,
            material_state_repo,
        )
    }

    // ==========================================
    // 测试1: 大数据量查询性能测试
    // ==========================================

    #[test]
    fn test_query_performance_10k_materials() {
        println!("\n========== 查询性能测试(10000材料) ==========");

        let (_temp_file, _db_path, _plan_api, master_repo, state_repo) =
            setup_performance_test_env(10000);

        // 测试1: 查询所有材料(通过机组查询)
        let start = Instant::now();
        let all_materials = master_repo.find_by_machine("").unwrap();
        let duration = start.elapsed();

        println!(
            "✅ 查询所有材料: {} 条, 耗时: {:?}",
            all_materials.len(),
            duration
        );
        assert!(
            duration.as_secs_f64() < 1.0,
            "查询10000材料应该<1秒,实际: {:?}",
            duration
        );

        // 测试2: 按material_id查询(主键)
        let start = Instant::now();
        let _material = master_repo.find_by_id("MAT005000").unwrap();
        let duration = start.elapsed();

        println!("✅ 按material_id查询(主键): 耗时: {:?}", duration);
        assert!(
            duration.as_micros() < 10000,
            "主键查询应该<10ms,实际: {:?}",
            duration
        );

        // 测试3: 查询适温材料
        let start = Instant::now();
        let ready_materials = state_repo.find_ready_materials(None).unwrap();
        let duration = start.elapsed();

        println!(
            "✅ 查询适温材料: {} 条, 耗时: {:?}",
            ready_materials.len(),
            duration
        );
        assert!(
            duration.as_secs_f64() < 1.0,
            "查询适温材料应该<1秒,实际: {:?}",
            duration
        );

        println!("\n========== 查询性能测试通过 ==========\n");
    }

    // ==========================================
    // 测试2: 排产性能测试
    // ==========================================

    #[test]
    fn test_scheduling_performance() {
        println!("\n========== 排产性能测试(1000材料) ==========");

        let (_temp_file, _db_path, plan_api, _master_repo, _state_repo) =
            setup_performance_test_env(1000);

        // 创建方案
        let plan_id = plan_api
            .create_plan("性能测试方案".to_string(), "test_user".to_string())
            .unwrap();

        // 测试: 创建版本并排产
        let start = Instant::now();
        let version_id = plan_api
            .create_version(
                plan_id.clone(),
                30,
                Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
                Some("性能测试版本".to_string()),
                "test_user".to_string(),
            )
            .unwrap();
        let duration = start.elapsed();

        println!("✅ 创建版本并排产(1000材料): 耗时: {:?}", duration);
        assert!(
            duration.as_secs_f64() < 5.0,
            "排产1000材料应该<5秒,实际: {:?}",
            duration
        );

        // 测试: 重算性能
        let start = Instant::now();
        let base_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        plan_api
            .recalc_full(&version_id, base_date, None, "test_user")
            .unwrap();
        let duration = start.elapsed();

        println!("✅ 重算排产(1000材料): 耗时: {:?}", duration);
        assert!(
            duration.as_secs_f64() < 5.0,
            "重算1000材料应该<5秒,实际: {:?}",
            duration
        );

        println!("\n========== 排产性能测试通过 ==========\n");
    }

    // ==========================================
    // 测试3: 批量操作性能测试
    // ==========================================

    #[test]
    fn test_batch_operations_performance() {
        println!("\n========== 批量操作性能测试 ==========");

        let (_temp_file, _db_path, _plan_api, _master_repo, state_repo) =
            setup_performance_test_env(1000);

        // 测试: 批量查询材料状态
        let material_ids: Vec<String> = (1..=100).map(|i| format!("MAT{:06}", i)).collect();

        let start = Instant::now();
        for material_id in &material_ids {
            let _ = state_repo.find_by_id(material_id).unwrap();
        }
        let duration = start.elapsed();

        println!("✅ 批量查询100个材料状态: 耗时: {:?}", duration);
        assert!(
            duration.as_secs_f64() < 1.0,
            "批量查询100材料应该<1秒,实际: {:?}",
            duration
        );

        println!("\n========== 批量操作性能测试通过 ==========\n");
    }

    // ==========================================
    // 测试4: 数据库索引效果测试
    // ==========================================

    #[test]
    fn test_database_index_effectiveness() {
        println!("\n========== 数据库索引效果测试 ==========");

        let (_temp_file, _db_path, _plan_api, master_repo, _state_repo) =
            setup_performance_test_env(10000);

        // 测试: 按material_id查询(主键索引)
        let start = Instant::now();
        let _material = master_repo.find_by_id("MAT005000").unwrap();
        let duration = start.elapsed();

        println!("✅ 按material_id查询(主键): 耗时: {:?}", duration);
        assert!(
            duration.as_micros() < 10000,
            "主键查询应该<10ms,实际: {:?}",
            duration
        );

        // 测试: 全表扫描性能
        let start = Instant::now();
        let _materials = master_repo.find_by_machine("").unwrap();
        let duration = start.elapsed();

        println!("✅ 全表扫描(10000条): 耗时: {:?}", duration);
        assert!(
            duration.as_secs_f64() < 1.0,
            "全表扫描应该<1秒,实际: {:?}",
            duration
        );

        println!("\n========== 数据库索引效果测试通过 ==========\n");
    }
}

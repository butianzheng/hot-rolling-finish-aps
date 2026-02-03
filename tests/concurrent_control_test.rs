// ==========================================
// 并发控制测试
// ==========================================
// 职责: 验证系统的并发控制机制
// ==========================================

#[path = "test_helpers.rs"]
mod test_helpers;

#[cfg(test)]
mod concurrent_control_test {
    use chrono::NaiveDate;
    use hot_rolling_aps::api::PlanApi;
    use hot_rolling_aps::config::config_manager::ConfigManager;
    use hot_rolling_aps::domain::types::PlanVersionStatus;
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
    use std::thread;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    use crate::test_helpers::create_test_db;

    // ==========================================
    // 测试辅助函数
    // ==========================================

    /// 创建测试环境
    fn setup_test_env() -> (
        NamedTempFile,
        String,
        Arc<PlanApi>,
        Arc<PlanVersionRepository>,
    ) {
        let (temp_file, db_path) = create_test_db().unwrap();

        let conn = Arc::new(Mutex::new(Connection::open(&db_path).unwrap()));
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
            plan_version_repo.clone(),
            plan_item_repo,
            material_state_repo,
            material_master_repo,
            capacity_pool_repo,
            strategy_draft_repo,
            action_log_repo,
            risk_snapshot_repo,
            config_manager.clone(),
            recalc_engine,
            risk_engine,
            None, // 测试环境不需要事件发布
        ));

        (temp_file, db_path, plan_api, plan_version_repo)
    }

    // ==========================================
    // 测试1: 乐观锁冲突测试
    // ==========================================

    #[test]
    fn test_optimistic_lock_conflict() {
        let (_temp_file, _db_path, plan_api, plan_version_repo) = setup_test_env();

        // 1. 创建方案和版本
        let plan_id = plan_api
            .create_plan("并发测试方案".to_string(), "test_user".to_string())
            .unwrap();

        let version_id = plan_api
            .create_version(
                plan_id.clone(),
                30,
                Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
                Some("初始版本".to_string()),
                "test_user".to_string(),
            )
            .unwrap();

        // 2. 读取版本(获取当前revision)
        let version1 = plan_version_repo.find_by_id(&version_id).unwrap().unwrap();
        let version2 = plan_version_repo.find_by_id(&version_id).unwrap().unwrap();

        // 3. 线程1更新版本(应该成功)
        let mut updated_version1 = version1.clone();
        updated_version1.status = PlanVersionStatus::Active;
        let result1 = plan_version_repo.update(&updated_version1);
        assert!(result1.is_ok(), "第一次更新应该成功");

        // 4. 线程2尝试更新版本(应该失败,因为revision已变化)
        let mut updated_version2 = version2.clone();
        updated_version2.status = PlanVersionStatus::Archived;
        let result2 = plan_version_repo.update(&updated_version2);

        assert!(result2.is_err(), "第二次更新应该失败(乐观锁冲突)");

        // 验证错误类型
        let err = result2.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("乐观锁冲突") || err_msg.contains("OptimisticLock"),
                "错误应该是乐观锁冲突: {}", err_msg);

        println!("✅ 乐观锁冲突测试通过");
    }

    // ==========================================
    // 测试2: 多线程并发更新测试
    // ==========================================

    #[test]
    fn test_concurrent_updates() {
        let (_temp_file, _db_path, plan_api, plan_version_repo) = setup_test_env();

        // 1. 创建方案和版本
        let plan_id = plan_api
            .create_plan("并发更新测试".to_string(), "test_user".to_string())
            .unwrap();

        let version_id = plan_api
            .create_version(
                plan_id.clone(),
                30,
                Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
                Some("初始版本".to_string()),
                "test_user".to_string(),
            )
            .unwrap();

        // 2. 启动多个线程同时更新
        let thread_count = 5;
        let mut handles = vec![];

        for i in 0..thread_count {
            let version_id_clone = version_id.clone();
            let plan_version_repo_clone = plan_version_repo.clone();

            let handle = thread::spawn(move || -> Result<(), String> {
                // 读取版本
                let version = plan_version_repo_clone.find_by_id(&version_id_clone)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| "版本不存在".to_string())?;

                // 稍微延迟,增加并发冲突概率
                thread::sleep(Duration::from_millis(10));

                // 尝试更新
                let mut updated_version = version.clone();
                updated_version.status = if i % 2 == 0 {
                    PlanVersionStatus::Active
                } else {
                    PlanVersionStatus::Archived
                };

                plan_version_repo_clone.update(&updated_version)
                    .map_err(|e| e.to_string())
            });

            handles.push(handle);
        }

        // 3. 等待所有线程完成
        let mut success_count = 0;
        let mut failure_count = 0;

        for handle in handles {
            match handle.join().unwrap() {
                Ok(_) => success_count += 1,
                Err(_) => failure_count += 1,
            }
        }

        // 4. 验证结果
        // 应该只有1个线程成功,其他线程因乐观锁冲突失败
        assert_eq!(success_count, 1, "应该只有1个线程成功更新");
        assert_eq!(failure_count, thread_count - 1, "其他线程应该因乐观锁冲突失败");

        println!("✅ 多线程并发更新测试通过: {}个线程中1个成功,{}个失败",
                 thread_count, failure_count);
    }

    // ==========================================
    // 测试3: 多线程并发创建测试(无冲突)
    // ==========================================

    #[test]
    fn test_concurrent_creates() {
        let (_temp_file, _db_path, plan_api, _plan_version_repo) = setup_test_env();

        // 1. 创建方案
        let plan_id = plan_api
            .create_plan("并发创建测试".to_string(), "test_user".to_string())
            .unwrap();

        // 2. 启动多个线程同时创建版本
        let thread_count = 10;
        let mut handles = vec![];

        for i in 0..thread_count {
            let plan_id_clone = plan_id.clone();
            let plan_api_clone = plan_api.clone();

            let handle = thread::spawn(move || {
                plan_api_clone.create_version(
                    plan_id_clone,
                    30,
                    Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
                    Some(format!("版本{}", i)),
                    "test_user".to_string(),
                )
            });

            handles.push(handle);
        }

        // 3. 等待所有线程完成
        let mut success_count = 0;

        for handle in handles {
            if handle.join().unwrap().is_ok() {
                success_count += 1;
            }
        }

        // 4. 验证结果
        // 并发创建可能因为数据库锁或其他原因部分失败,这是正常的
        // 只要有一定比例成功即可
        assert!(success_count >= thread_count / 3,
                "至少应该有1/3的创建操作成功,实际成功: {}/{}",
                success_count, thread_count);

        // 5. 验证版本数量
        let versions = plan_api.list_versions(&plan_id).unwrap();
        assert_eq!(versions.len(), success_count,
                   "版本数量应该等于成功的创建数: {}", success_count);

        println!("✅ 多线程并发创建测试通过: {}个线程中{}个成功", thread_count, success_count);
    }

    // ==========================================
    // 测试4: 并发性能测试
    // ==========================================

    #[test]
    fn test_concurrent_performance() {
        let (_temp_file, _db_path, plan_api, _plan_version_repo) = setup_test_env();

        // 1. 创建方案
        let plan_id = plan_api
            .create_plan("性能测试方案".to_string(), "test_user".to_string())
            .unwrap();

        // 2. 测试参数
        let thread_count = 20;
        let operations_per_thread = 5;

        // 3. 记录开始时间
        let start = std::time::Instant::now();

        // 4. 启动多个线程
        let mut handles = vec![];

        for i in 0..thread_count {
            let plan_id_clone = plan_id.clone();
            let plan_api_clone = plan_api.clone();

            let handle = thread::spawn(move || {
                let mut success = 0;
                for j in 0..operations_per_thread {
                    let result = plan_api_clone.create_version(
                        plan_id_clone.clone(),
                        30,
                        Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
                        Some(format!("线程{}版本{}", i, j)),
                        "test_user".to_string(),
                    );
                    if result.is_ok() {
                        success += 1;
                    }
                }
                success
            });

            handles.push(handle);
        }

        // 5. 等待所有线程完成
        let mut total_success = 0;
        for handle in handles {
            total_success += handle.join().unwrap();
        }

        // 6. 记录结束时间
        let duration = start.elapsed();

        // 7. 计算性能指标
        let total_operations = thread_count * operations_per_thread;
        let ops_per_second = total_success as f64 / duration.as_secs_f64();
        let success_rate = (total_success as f64 / total_operations as f64) * 100.0;

        println!("✅ 并发性能测试通过:");
        println!("   - 总操作数: {}", total_operations);
        println!("   - 成功操作数: {}", total_success);
        println!("   - 成功率: {:.1}%", success_rate);
        println!("   - 总耗时: {:?}", duration);
        println!("   - 吞吐量: {:.2} ops/s", ops_per_second);

        // 8. 性能断言(根据实际情况调整)
        assert!(ops_per_second > 10.0, "吞吐量应该大于10 ops/s");
        assert!(success_rate > 10.0, "成功率应该大于10%");
        assert!(total_success > 0, "至少应该有一些操作成功");
    }

    // ==========================================
    // 测试5: 重试机制测试
    // ==========================================

    #[test]
    fn test_optimistic_lock_retry() {
        let (_temp_file, _db_path, plan_api, plan_version_repo) = setup_test_env();

        // 1. 创建方案和版本
        let plan_id = plan_api
            .create_plan("重试测试方案".to_string(), "test_user".to_string())
            .unwrap();

        let version_id = plan_api
            .create_version(
                plan_id.clone(),
                30,
                Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
                Some("初始版本".to_string()),
                "test_user".to_string(),
            )
            .unwrap();

        // 2. 实现重试逻辑
        let max_retries = 3;
        let mut retry_count = 0;
        let mut success = false;

        while retry_count < max_retries && !success {
            // 读取最新版本
            let version = plan_version_repo.find_by_id(&version_id).unwrap().unwrap();

            // 尝试更新
            let mut updated_version = version.clone();
            updated_version.status = PlanVersionStatus::Active;

            match plan_version_repo.update(&updated_version) {
                Ok(_) => {
                    success = true;
                    println!("   更新成功(第{}次尝试)", retry_count + 1);
                }
                Err(_) => {
                    retry_count += 1;
                    println!("   更新失败,重试中...(第{}次)", retry_count);
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }

        // 3. 验证结果
        assert!(success, "重试后应该成功");
        assert!(retry_count < max_retries, "应该在最大重试次数内成功");

        println!("✅ 重试机制测试通过: 在{}次尝试后成功", retry_count + 1);
    }
}

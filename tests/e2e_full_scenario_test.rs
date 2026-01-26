// ==========================================
// 端到端全场景测试脚本
// ==========================================
// 用途：导入大规模测试数据，创建排产方案，验证所有决策功能
// 运行：cargo test --test e2e_full_scenario_test -- --nocapture
// ==========================================

use hot_rolling_aps::app::AppState;
use hot_rolling_aps::decision::api::DecisionApi;
use hot_rolling_aps::importer::{
    MaterialImporter, MaterialImporterImpl, CsvParser, FieldMapperImpl, DataCleanerImpl,
    DerivationServiceImpl, DqValidatorImpl, ConflictHandlerImpl,
};
use hot_rolling_aps::repository::MaterialImportRepositoryImpl;
use hot_rolling_aps::config::config_manager::ConfigManager;
use hot_rolling_aps::engine::MaterialStateDerivationService;
use std::path::PathBuf;
use std::sync::Arc;
use chrono::Local;

#[tokio::test]
async fn test_full_scenario_workflow() {
    println!("\n");
    println!("==========================================");
    println!("端到端全场景测试开始");
    println!("==========================================");

    // 1. 初始化测试环境
    println!("\n[步骤1] 初始化测试环境...");
    let test_db_path = "/tmp/aps_full_scenario_test.db";

    // 删除旧数据库（如果存在）
    if std::path::Path::new(test_db_path).exists() {
        std::fs::remove_file(test_db_path).unwrap();
        println!("已清理旧数据库");
    }

    // 创建AppState
    let app_state = Arc::new(
        AppState::new(test_db_path.to_string())
            .expect("初始化AppState失败")
    );
    println!("AppState初始化成功");

    // 2. 导入大规模测试数据
    println!("\n[步骤2] 导入大规模测试数据（1000条记录）...");
    let csv_path = PathBuf::from("tests/fixtures/datasets/02_large_dataset.csv");

    // 创建导入器 - 使用新的泛型 API
    let import_config = ConfigManager::new(test_db_path).expect("创建ConfigManager失败");
    let import_repo = MaterialImportRepositoryImpl::new(test_db_path).expect("创建导入仓储失败");
    let file_parser = Box::new(CsvParser);
    let field_mapper = Box::new(FieldMapperImpl);
    let data_cleaner = Box::new(DataCleanerImpl);
    let derivation_service = Box::new(DerivationServiceImpl);
    let dq_validator = Box::new(DqValidatorImpl::new(100.0));
    let conflict_handler = Box::new(ConflictHandlerImpl);
    let state_derivation_service = MaterialStateDerivationService::new();

    let importer = MaterialImporterImpl::new(
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

    let start_time = std::time::Instant::now();
    let import_result = importer
        .import_from_csv(csv_path.to_str().unwrap())
        .await
        .expect("数据导入失败");
    let elapsed = start_time.elapsed();

    println!("导入完成：");
    println!("  - 成功导入: {} 条", import_result.summary.success);
    println!("  - 阻断记录: {} 条", import_result.summary.blocked);
    println!("  - 冲突记录: {} 条", import_result.summary.conflict);
    println!("  - 耗时: {:.2}秒", elapsed.as_secs_f64());
    println!("  - 吞吐量: {:.0} 条/秒", import_result.summary.success as f64 / elapsed.as_secs_f64());

    assert!(import_result.summary.success >= 900, "导入成功率过低");
    assert!(elapsed.as_secs() < 10, "导入性能不达标（应<10秒）");

    // 3. 创建排产方案
    println!("\n[步骤3] 创建排产方案...");
    let plan_name = "全场景测试方案".to_string();
    let created_by = "test_user".to_string();

    let plan_id = app_state
        .plan_api
        .create_plan(plan_name.clone(), created_by.clone())
        .expect("创建排产方案失败");

    println!("排产方案创建成功：{}", plan_id);
    println!("  - 方案名称: {}", plan_name);

    // 4. 创建并激活版本
    println!("\n[步骤4] 创建并激活排产版本...");
    let plan_days = 7; // 定义窗口天数
    let version_id = app_state
        .plan_api
        .create_version(
            plan_id.clone(),
            plan_days,
            None,
            Some("全场景测试版本".to_string()),
            created_by.clone(),
        )
        .expect("创建版本失败");

    println!("版本创建成功：{}", version_id);

    app_state
        .plan_api
        .activate_version(&version_id, &created_by)
        .expect("激活版本失败");

    println!("版本激活成功");

    // 5. 运行排产计算
    println!("\n[步骤5] 执行排产计算...");
    let recalc_start = std::time::Instant::now();
    let today = Local::now().date_naive();

    app_state
        .plan_api
        .recalc_full(&version_id, today, None, &created_by)
        .expect("排产计算失败");

    let recalc_elapsed = recalc_start.elapsed();
    println!("排产计算完成，耗时: {:.2}秒", recalc_elapsed.as_secs_f64());

    // 6. 测试D1：日期风险摘要
    println!("\n[步骤6] 测试D1：日期风险摘要...");
    let date_from = today.format("%Y-%m-%d").to_string();
    let date_to = (today + chrono::Duration::days(30)).format("%Y-%m-%d").to_string();

    let d1_request = hot_rolling_aps::decision::api::GetDecisionDaySummaryRequest {
        version_id: version_id.clone(),
        date_from: date_from.clone(),
        date_to: date_to.clone(),
        risk_level_filter: None,
        limit: Some(30),
        sort_by: None,
    };

    let d1_response = app_state
        .decision_api
        .get_decision_day_summary(d1_request)
        .expect("D1查询失败");

    println!("D1响应成功：");
    println!("  - 版本ID: {}", d1_response.version_id);
    println!("  - 数据时间: {}", d1_response.as_of);
    println!("  - 风险日期数量: {}", d1_response.total_count);

    if let Some(first_item) = d1_response.items.first() {
        println!("  - 第一个风险日期: {}", first_item.plan_date);
        println!("    - 风险评分: {:.1}", first_item.risk_score);
        println!("    - 风险等级: {:?}", first_item.risk_level);
        println!("    - 超载吨位: {:.1}吨", first_item.overload_weight_t);
    }

    // 7. 测试D4：机组堵塞概况
    println!("\n[步骤7] 测试D4：机组堵塞概况...");
    let d4_request = hot_rolling_aps::decision::api::GetMachineBottleneckProfileRequest {
        version_id: version_id.clone(),
        date_from: date_from.clone(),
        date_to: date_to.clone(),
        machine_codes: None,
        bottleneck_level_filter: None,
        bottleneck_type_filter: None,
        limit: Some(50),
    };

    let d4_response = app_state
        .decision_api
        .get_machine_bottleneck_profile(d4_request)
        .expect("D4查询失败");

    println!("D4响应成功：");
    println!("  - 版本ID: {}", d4_response.version_id);
    println!("  - 数据时间: {}", d4_response.as_of);
    println!("  - 堵塞点数量: {}", d4_response.total_count);

    if let Some(first_item) = d4_response.items.first() {
        println!("  - 第一个堵塞点: {} @ {}", first_item.machine_code, first_item.plan_date);
        println!("    - 堵塞评分: {:.1}", first_item.bottleneck_score);
        println!("    - 堵塞等级: {:?}", first_item.bottleneck_level);
        println!("    - 容量利用率: {:.1}%", first_item.capacity_util_pct);
    }

    // 8. 测试D5：换辊警报
    println!("\n[步骤8] 测试D5：换辊警报...");
    let d5_request = hot_rolling_aps::decision::api::ListRollCampaignAlertsRequest {
        version_id: version_id.clone(),
        machine_codes: None,
        alert_level_filter: None,
        alert_type_filter: None,
        date_from: None,
        date_to: None,
        limit: None,
    };

    let d5_response = app_state
        .decision_api
        .list_roll_campaign_alerts(d5_request)
        .expect("D5查询失败");

    println!("D5响应成功：");
    println!("  - 版本ID: {}", d5_response.version_id);
    println!("  - 数据时间: {}", d5_response.as_of);
    println!("  - 警报总数: {}", d5_response.summary.total_alerts);
    println!("  - 近硬停止数: {}", d5_response.summary.near_hard_stop_count);

    if let Some(first_item) = d5_response.items.first() {
        println!("  - 机组: {}", first_item.machine_code);
        println!("    - 警报级别: {}", first_item.alert_level);
        println!("    - 当前吨位: {:.1}吨", first_item.current_tonnage_t);
        println!("    - 软限制: {:.1}吨", first_item.soft_limit_t);
        println!("    - 硬限制: {:.1}吨", first_item.hard_limit_t);
        println!("    - 利用率: {:.1}%",
            (first_item.current_tonnage_t / first_item.hard_limit_t) * 100.0);
    }

    // 9. 测试D6：容量优化机会
    println!("\n[步骤9] 测试D6：容量优化机会...");
    let d6_request = hot_rolling_aps::decision::api::GetCapacityOpportunityRequest {
        version_id: version_id.clone(),
        machine_codes: None,
        date_from: None,
        date_to: None,
        opportunity_type_filter: None,
        min_opportunity_t: Some(10.0),
        limit: Some(50),
    };

    let d6_response = app_state
        .decision_api
        .get_capacity_opportunity(d6_request)
        .expect("D6查询失败");

    println!("D6响应成功：");
    println!("  - 版本ID: {}", d6_response.version_id);
    println!("  - 数据时间: {}", d6_response.as_of);
    println!("  - 机会总数: {}", d6_response.summary.total_opportunities);
    println!("  - 总机会空间: {:.1}吨", d6_response.summary.total_opportunity_space_t);
    println!("  - 平均当前利用率: {:.1}%", d6_response.summary.avg_current_util_pct);
    println!("  - 平均优化利用率: {:.1}%", d6_response.summary.avg_optimized_util_pct);

    if let Some(first_item) = d6_response.items.first() {
        println!("  - 最优机会: {} @ {}", first_item.machine_code, first_item.plan_date);
        println!("    - 机会类型: {}", first_item.opportunity_type);
        println!("    - 当前利用率: {:.1}%", first_item.current_util_pct);
        println!("    - 优化利用率: {:.1}%", first_item.optimized_util_pct);
        println!("    - 机会空间: {:.1}吨", first_item.opportunity_space_t);
    }

    // 10. 验证排产项
    println!("\n[步骤10] 验证排产项...");
    let plan_items = app_state
        .plan_api
        .list_plan_items(&version_id)
        .expect("查询排产项失败");

    println!("排产项查询成功：");
    println!("  - 总排产项数: {}", plan_items.len());

    // 统计各状态的排产项
    let mut scheduled_count = 0;
    let mut pending_count = 0;
    let mut locked_count = 0;

    for item in &plan_items {
        match item.sched_state.as_deref() {
            Some("SCHEDULED") => scheduled_count += 1,
            Some("PENDING_MATURE") => pending_count += 1,
            Some("LOCKED") => locked_count += 1,
            _ => {}
        }
    }

    println!("  - 已排产: {} 项", scheduled_count);
    println!("  - 待适温: {} 项", pending_count);
    println!("  - 已锁定: {} 项", locked_count);

    // 11. 总结
    println!("\n");
    println!("==========================================");
    println!("全场景测试完成！");
    println!("==========================================");
    println!("\n测试摘要：");
    println!("  - 数据导入: {} 条材料成功导入", import_result.summary.success);
    println!("  - 排产方案: {} ({} 天)", plan_id, plan_days);
    println!("  - 排产版本: {} (已激活)", version_id);
    println!("  - 排产计算: 完成 ({:.2}秒)", recalc_elapsed.as_secs_f64());
    println!("  - D1 日期风险: {} 个风险日期", d1_response.total_count);
    println!("  - D4 机组堵塞: {} 个堵塞点", d4_response.total_count);
    println!("  - D5 换辊警报: {} 个警报", d5_response.summary.total_alerts);
    println!("  - D6 容量机会: {} 个机会", d6_response.summary.total_opportunities);
    println!("  - 排产项总数: {}", plan_items.len());
    println!("\n数据库文件: {}", test_db_path);
    println!("可以使用此数据库进行前端UI测试！");
    println!("\n");

    // 保存测试数据库供前端使用
    let production_db = "aps_decision.db";
    println!("提示：可以将测试数据库复制到生产数据库进行前端测试：");
    println!("cp {} {}", test_db_path, production_db);
    println!();
}

/**
 * 产能池日历性能测试
 * 验证365天数据加载性能和批量查询优化
 */
mod test_helpers;

use chrono::NaiveDate;
use hot_rolling_aps::domain::capacity::CapacityPool;
use hot_rolling_aps::repository::{
    CapacityPoolRepository, MachineConfigEntity, MachineConfigRepository,
};
use std::time::Instant;

#[test]
fn test_capacity_calendar_365_days_performance() {
    // ==========================================
    // 测试目标：验证365天数据加载性能 < 2s
    // ==========================================

    // 创建测试数据库
    let (_temp_file, db_path) = test_helpers::create_test_db().expect("Failed to create test db");
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");

    // 初始化仓储
    let capacity_repo = CapacityPoolRepository::new(db_path.clone()).unwrap();
    let machine_config_repo = MachineConfigRepository::new(&db_path).unwrap();

    // 创建测试计划和版本
    conn.execute(
        "INSERT INTO plan (plan_id, plan_name, plan_type, created_by) VALUES (?, ?, ?, ?)",
        rusqlite::params!["plan1", "测试计划", "PRODUCTION", "test_user"],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO plan_version (version_id, plan_id, version_no, status, created_by) VALUES (?, ?, ?, ?, ?)",
        rusqlite::params!["v1", "plan1", 1, "ACTIVE", "test_user"],
    ).unwrap();

    // 准备测试数据：创建365天产能池记录
    let start_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let machine_codes = vec!["H031", "H032", "H033"];

    println!("⏱️  准备测试数据：365天 × 3机组 = 1095条记录");
    let setup_start = Instant::now();

    for machine_code in &machine_codes {
        for i in 0..365 {
            let plan_date = start_date + chrono::Duration::days(i as i64);
            let pool = CapacityPool {
                version_id: "v1".to_string(),
                machine_code: machine_code.to_string(),
                plan_date,
                target_capacity_t: 1200.0,
                used_capacity_t: if i % 3 == 0 { 500.0 } else { 1000.0 },
                limit_capacity_t: 1260.0,
                overflow_t: 0.0,
                frozen_capacity_t: 0.0,
                accumulated_tonnage_t: 0.0,
                roll_campaign_id: None,
            };
            capacity_repo.upsert_single(&pool).unwrap();
        }
    }

    let setup_duration = setup_start.elapsed();
    println!("✅ 数据准备完成，耗时: {:?}", setup_duration);

    // ==========================================
    // 测试1: 单机组365天数据查询
    // ==========================================
    println!("\n📊 测试1: 单机组365天数据查询");
    let query_start = Instant::now();

    let end_date = start_date + chrono::Duration::days(364);

    let result = capacity_repo
        .find_by_date_range("v1", "H031", start_date, end_date)
        .unwrap();

    let query_duration = query_start.elapsed();

    println!("  - 返回记录数: {}", result.len());
    println!("  - 查询耗时: {:?}", query_duration);
    println!(
        "  - 性能指标: {:.2} records/ms",
        result.len() as f64 / query_duration.as_millis() as f64
    );

    assert_eq!(result.len(), 365, "应返回365条记录");
    assert!(
        query_duration.as_secs() < 1,
        "365天单机组查询应在1秒内完成，实际: {:?}",
        query_duration
    );

    // ==========================================
    // 测试2: 分批查询策略（模拟前端分批加载）
    // ==========================================
    println!("\n📊 测试2: 分批查询策略（90天/批，共4批）");
    let batch_start = Instant::now();
    let mut total_records = 0;

    for batch_idx in 0..4 {
        let batch_date_from = start_date + chrono::Duration::days(batch_idx * 90);
        let batch_date_to = start_date + chrono::Duration::days((batch_idx + 1) * 90 - 1);

        let batch_result = capacity_repo
            .find_by_date_range("v1", "H031", batch_date_from, batch_date_to)
            .unwrap();

        total_records += batch_result.len();
        println!("  - 批次{}: {} records", batch_idx + 1, batch_result.len());
    }

    let batch_duration = batch_start.elapsed();
    println!("  - 总记录数: {}", total_records);
    println!("  - 总耗时: {:?}", batch_duration);
    println!(
        "  - 性能指标: {:.2} records/ms",
        total_records as f64 / batch_duration.as_millis() as f64
    );

    assert!(
        batch_duration.as_secs() < 2,
        "分批查询应在2秒内完成，实际: {:?}",
        batch_duration
    );

    // ==========================================
    // 测试3: 机组配置查询性能
    // ==========================================
    println!("\n📊 测试3: 机组配置查询");

    // 创建测试配置
    let config = MachineConfigEntity {
        config_id: "config1".to_string(),
        version_id: "v1".to_string(),
        machine_code: "H031".to_string(),
        default_daily_target_t: 1200.0,
        default_daily_limit_pct: 1.05,
        effective_date: None,
        created_at: chrono::Local::now()
            .naive_local()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        updated_at: chrono::Local::now()
            .naive_local()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        created_by: "test_user".to_string(),
        reason: Some("测试配置".to_string()),
    };

    machine_config_repo.upsert(&config).unwrap();

    let config_query_start = Instant::now();
    let configs = machine_config_repo.list_by_version_id("v1").unwrap();
    let config_query_duration = config_query_start.elapsed();

    println!("  - 返回配置数: {}", configs.len());
    println!("  - 查询耗时: {:?}", config_query_duration);

    assert_eq!(configs.len(), 1);
    assert!(
        config_query_duration.as_millis() < 100,
        "配置查询应在100ms内完成"
    );

    // ==========================================
    // 测试4: 多机组顺序查询
    // ==========================================
    println!("\n📊 测试4: 多机组顺序查询（3机组 × 365天）");
    let multi_query_start = Instant::now();
    let mut total_multi_records = 0;

    for machine_code in &machine_codes {
        let result = capacity_repo
            .find_by_date_range("v1", machine_code, start_date, end_date)
            .unwrap();
        total_multi_records += result.len();
    }

    let multi_query_duration = multi_query_start.elapsed();

    println!("  - 返回记录数: {}", total_multi_records);
    println!("  - 查询耗时: {:?}", multi_query_duration);
    println!(
        "  - 性能指标: {:.2} records/ms",
        total_multi_records as f64 / multi_query_duration.as_millis() as f64
    );

    assert_eq!(
        total_multi_records, 1095,
        "应返回1095条记录 (3机组 × 365天)"
    );
    assert!(
        multi_query_duration.as_secs() < 2,
        "多机组查询应在2秒内完成，实际: {:?}",
        multi_query_duration
    );

    // ==========================================
    // 性能基准报告
    // ==========================================
    println!("\n\n📈 性能基准报告");
    println!("=====================================");
    println!("✅ 单机组365天查询: {:?} (目标: <1s)", query_duration);
    println!("✅ 分批查询(4×90天): {:?} (目标: <2s)", batch_duration);
    println!(
        "✅ 机组配置查询: {:?} (目标: <100ms)",
        config_query_duration
    );
    println!("✅ 多机组顺序查询: {:?} (目标: <2s)", multi_query_duration);
    println!("=====================================");

    // 所有性能指标应满足目标
    assert!(query_duration.as_secs() < 1);
    assert!(batch_duration.as_secs() < 2);
    assert!(config_query_duration.as_millis() < 100);
    assert!(multi_query_duration.as_secs() < 2);
}

#[test]
fn test_batch_update_performance() {
    // ==========================================
    // 测试目标：验证批量更新性能
    // ==========================================

    // 创建测试数据库
    let (_temp_file, db_path) = test_helpers::create_test_db().expect("Failed to create test db");
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");

    let capacity_repo = CapacityPoolRepository::new(db_path.clone()).unwrap();

    // 创建测试计划和版本
    conn.execute(
        "INSERT INTO plan (plan_id, plan_name, plan_type, created_by) VALUES (?, ?, ?, ?)",
        rusqlite::params!["plan1", "测试计划", "PRODUCTION", "test_user"],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO plan_version (version_id, plan_id, version_no, status, created_by) VALUES (?, ?, ?, ?, ?)",
        rusqlite::params!["v1", "plan1", 1, "ACTIVE", "test_user"],
    ).unwrap();

    let start_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

    // 创建100条记录用于批量更新
    let mut pools = Vec::new();
    for i in 0..100 {
        let plan_date = start_date + chrono::Duration::days(i as i64);
        let pool = CapacityPool {
            version_id: "v1".to_string(),
            machine_code: "H031".to_string(),
            plan_date,
            target_capacity_t: 1200.0,
            used_capacity_t: 0.0,
            limit_capacity_t: 1260.0,
            overflow_t: 0.0,
            frozen_capacity_t: 0.0,
            accumulated_tonnage_t: 0.0,
            roll_campaign_id: None,
        };
        pools.push(pool);
    }

    capacity_repo.upsert_batch(pools.clone()).unwrap();

    println!("⏱️  测试批量更新性能（100条记录）");
    let update_start = Instant::now();

    // 批量更新
    let updated_pools: Vec<CapacityPool> = pools
        .into_iter()
        .map(|mut p| {
            p.target_capacity_t = 1300.0;
            p.limit_capacity_t = 1365.0;
            p
        })
        .collect();

    capacity_repo.upsert_batch(updated_pools).unwrap();

    let update_duration = update_start.elapsed();

    println!("  - 更新记录数: 100");
    println!("  - 更新耗时: {:?}", update_duration);
    println!(
        "  - 性能指标: {:.2} updates/ms",
        100.0 / update_duration.as_millis() as f64
    );

    assert!(
        update_duration.as_millis() < 500,
        "批量更新100条应在500ms内完成，实际: {:?}",
        update_duration
    );

    println!("\n✅ 批量更新性能达标");
}

// ==========================================
// 决策视图性能测试
// ==========================================
// 职责: 验证决策层查询和刷新性能
// 性能目标:
//   - 查询性能: < 100ms
//   - 全量刷新: < 5s
//   - 增量刷新: < 1s
// ==========================================

mod test_helpers;

use hot_rolling_aps::decision::repository::{BottleneckRepository, DaySummaryRepository};
use hot_rolling_aps::decision::services::{
    DecisionRefreshService, RefreshQueue, RefreshScope, RefreshTask, RefreshTrigger,
};
use hot_rolling_aps::decision::use_cases::d1_most_risky_day::MostRiskyDayUseCase;
use hot_rolling_aps::decision::use_cases::d4_machine_bottleneck::MachineBottleneckUseCase;
use hot_rolling_aps::decision::use_cases::impls::{
    d1_most_risky_day_impl::MostRiskyDayUseCaseImpl,
    d4_machine_bottleneck_impl::MachineBottleneckUseCaseImpl,
};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tempfile::NamedTempFile;

// ==========================================
// 测试数据生成器
// ==========================================

/// 创建大量测试数据
fn setup_large_dataset(
    conn: &Connection,
    version_id: &str,
    num_dates: usize,
    num_machines: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // 生成日期列表 (从 2026-01-20 开始)
    let start_date = chrono::NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    // 首先插入 plan 和 plan_version 记录 (满足外键约束)
    conn.execute(
        r#"
        INSERT OR IGNORE INTO plan (plan_id, plan_name, plan_type, created_by)
        VALUES ('PLAN001', '性能测试方案', 'NORMAL', 'test_user')
        "#,
        [],
    )?;

    conn.execute(
        r#"
        INSERT OR IGNORE INTO plan_version (version_id, plan_id, version_no, status, created_by)
        VALUES (?1, 'PLAN001', 1, 'ACTIVE', 'test_user')
        "#,
        [version_id],
    )?;

    // 插入 risk_snapshot 数据 (D1 数据源)
    for date_offset in 0..num_dates {
        let plan_date = start_date + chrono::Duration::days(date_offset as i64);
        let date_str = plan_date.format("%Y-%m-%d").to_string();

        for machine_idx in 0..num_machines {
            let machine_code = format!("H{:03}", machine_idx + 32);

            // 模拟不同的风险等级和产能利用率
            let risk_level = match (date_offset + machine_idx) % 4 {
                0 => "CRITICAL",
                1 => "HIGH",
                2 => "MEDIUM",
                _ => "LOW",
            };

            let utilization = 0.5 + (((date_offset + machine_idx) % 10) as f64) * 0.05;
            let target_capacity = 1500.0;
            let used_capacity = target_capacity * utilization;
            let limit_capacity = 2000.0;
            let overflow = if used_capacity > limit_capacity {
                used_capacity - limit_capacity
            } else {
                0.0
            };

            conn.execute(
                r#"
                INSERT OR REPLACE INTO risk_snapshot (
                    version_id, machine_code, snapshot_date, risk_level, risk_reasons,
                    target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t,
                    urgent_total_t, mature_backlog_t, immature_backlog_t,
                    campaign_status, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, datetime('now'))
                "#,
                rusqlite::params![
                    version_id,
                    machine_code,
                    date_str,
                    risk_level,
                    format!("测试原因{}", (date_offset + machine_idx) % 3),
                    target_capacity,
                    used_capacity,
                    limit_capacity,
                    overflow,
                    used_capacity * 0.6, // urgent_total_t
                    used_capacity * 0.3, // mature_backlog_t
                    used_capacity * 0.1, // immature_backlog_t
                    if overflow > 0.0 { "WARNING" } else { "OK" },
                ],
            )?;
        }
    }

    // 插入 machine_master 数据
    for machine_idx in 0..num_machines {
        let machine_code = format!("H{:03}", machine_idx + 32);
        conn.execute(
            "INSERT OR IGNORE INTO machine_master (machine_code) VALUES (?1)",
            [&machine_code],
        )?;
    }

    // 插入 capacity_pool 数据 (D4 数据源)
    for date_offset in 0..num_dates {
        let plan_date = start_date + chrono::Duration::days(date_offset as i64);
        let date_str = plan_date.format("%Y-%m-%d").to_string();

        for machine_idx in 0..num_machines {
            let machine_code = format!("H{:03}", machine_idx + 32);

            let utilization = 0.5 + (((date_offset + machine_idx) % 10) as f64) * 0.05;
            let target_capacity = 1500.0;
            let used_capacity = target_capacity * utilization;
            let limit_capacity = 2000.0;

            conn.execute(
                r#"
                INSERT OR REPLACE INTO capacity_pool (
                    version_id, machine_code, plan_date, target_capacity_t, limit_capacity_t,
                    used_capacity_t, overflow_t, frozen_capacity_t,
                    accumulated_tonnage_t, roll_campaign_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                rusqlite::params![
                    version_id,
                    machine_code,
                    date_str,
                    target_capacity,
                    limit_capacity,
                    used_capacity,
                    if used_capacity > limit_capacity {
                        used_capacity - limit_capacity
                    } else {
                        0.0
                    },
                    used_capacity * 0.1,                          // frozen_capacity_t
                    used_capacity * (date_offset as f64 + 1.0), // accumulated_tonnage_t
                    format!("RC{:03}", machine_idx),
                ],
            )?;
        }
    }

    // 插入 plan_item 数据 (为每个机组-日添加材料)
    let mut material_counter = 1;
    for date_offset in 0..num_dates {
        let plan_date = start_date + chrono::Duration::days(date_offset as i64);
        let date_str = plan_date.format("%Y-%m-%d").to_string();

        for machine_idx in 0..num_machines {
            let machine_code = format!("H{:03}", machine_idx + 32);

            // 每个机组-日添加 5-15 个材料
            let num_materials = 5 + ((date_offset + machine_idx) % 10);

            for seq in 0..num_materials {
                let material_id = format!("MAT{:06}", material_counter);
                material_counter += 1;

                // 先插入 material_master 记录 (满足外键约束)
                conn.execute(
                    r#"
                    INSERT OR IGNORE INTO material_master (
                        material_id, weight_t, created_at, updated_at
                    ) VALUES (?1, ?2, datetime('now'), datetime('now'))
                    "#,
                    rusqlite::params![
                        &material_id,
                        100.0 + ((seq % 10) as f64) * 10.0,
                    ],
                )?;

                // 部分材料有结构违规
                let violation_flags = if seq % 5 == 0 {
                    "STRUCT_CONFLICT"
                } else {
                    ""
                };

                conn.execute(
                    r#"
                    INSERT OR REPLACE INTO plan_item (
                        version_id, material_id, machine_code, plan_date, seq_no,
                        weight_t, source_type, locked_in_plan, force_release_in_plan,
                        violation_flags
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                    "#,
                    rusqlite::params![
                        version_id,
                        material_id,
                        machine_code,
                        date_str,
                        seq + 1,
                        100.0 + ((seq % 10) as f64) * 10.0, // weight_t
                        "AUTO",
                        0, // locked_in_plan
                        0, // force_release_in_plan
                        violation_flags,
                    ],
                )?;
            }
        }
    }

    Ok(())
}

/// 创建性能测试环境
fn setup_performance_test_env(
    num_dates: usize,
    num_machines: usize,
) -> Result<
    (
        NamedTempFile,
        Arc<Mutex<Connection>>,
        Arc<DecisionRefreshService>,
        Arc<MostRiskyDayUseCaseImpl>,
        Arc<MachineBottleneckUseCaseImpl>,
    ),
    Box<dyn std::error::Error>,
> {
    let (temp_file, db_path) = test_helpers::create_test_db()?;
    let conn = Arc::new(Mutex::new(test_helpers::open_test_connection(&db_path)?));

    // 生成测试数据
    {
        let c = conn.lock().unwrap();
        setup_large_dataset(&c, "V001", num_dates, num_machines)?;
    }

    // 创建决策服务和用例
    let refresh_service = Arc::new(DecisionRefreshService::new(conn.clone()));
    let day_summary_repo = Arc::new(DaySummaryRepository::new(conn.clone()));
    let bottleneck_repo = Arc::new(BottleneckRepository::new(conn.clone()));

    let d1_use_case = Arc::new(MostRiskyDayUseCaseImpl::new(day_summary_repo));
    let d4_use_case = Arc::new(MachineBottleneckUseCaseImpl::new(bottleneck_repo));

    Ok((temp_file, conn, refresh_service, d1_use_case, d4_use_case))
}

// ==========================================
// 测试1: D1 查询性能测试 (目标 < 100ms)
// ==========================================

#[test]
fn test_d1_query_performance_small_dataset() {
    println!("\n========== D1 查询性能测试 (10天 x 5机组 = 50条) ==========");

    let (_temp_file, _conn, _refresh_service, d1_use_case, _d4_use_case) =
        setup_performance_test_env(10, 5).unwrap();

    // 查询 10 天的日期摘要
    let start = Instant::now();
    let summaries = d1_use_case
        .get_day_summary("V001", "2026-01-20", "2026-01-29")
        .unwrap();
    let duration = start.elapsed();

    println!("✅ 查询结果: {} 天, 耗时: {:?}", summaries.len(), duration);
    assert!(
        duration.as_millis() < 100,
        "D1 查询应该 < 100ms, 实际: {:?}",
        duration
    );

    println!("========== D1 小数据集查询性能测试通过 ==========\n");
}

#[test]
fn test_d1_query_performance_medium_dataset() {
    println!("\n========== D1 查询性能测试 (30天 x 10机组 = 300条) ==========");

    let (_temp_file, _conn, _refresh_service, d1_use_case, _d4_use_case) =
        setup_performance_test_env(30, 10).unwrap();

    // 查询 30 天的日期摘要
    let start = Instant::now();
    let summaries = d1_use_case
        .get_day_summary("V001", "2026-01-20", "2026-02-18")
        .unwrap();
    let duration = start.elapsed();

    println!("✅ 查询结果: {} 天, 耗时: {:?}", summaries.len(), duration);
    assert!(
        duration.as_millis() < 100,
        "D1 查询应该 < 100ms, 实际: {:?}",
        duration
    );

    println!("========== D1 中数据集查询性能测试通过 ==========\n");
}

#[test]
fn test_d1_query_performance_large_dataset() {
    println!("\n========== D1 查询性能测试 (60天 x 20机组 = 1200条) ==========");

    let (_temp_file, _conn, _refresh_service, d1_use_case, _d4_use_case) =
        setup_performance_test_env(60, 20).unwrap();

    // 查询 60 天的日期摘要
    let start = Instant::now();
    let summaries = d1_use_case
        .get_day_summary("V001", "2026-01-20", "2026-03-20")
        .unwrap();
    let duration = start.elapsed();

    println!("✅ 查询结果: {} 天, 耗时: {:?}", summaries.len(), duration);
    assert!(
        duration.as_millis() < 100,
        "D1 查询应该 < 100ms, 实际: {:?}",
        duration
    );

    println!("========== D1 大数据集查询性能测试通过 ==========\n");
}

#[test]
fn test_d1_top_n_query_performance() {
    println!("\n========== D1 Top-N 查询性能测试 ==========");

    let (_temp_file, _conn, _refresh_service, d1_use_case, _d4_use_case) =
        setup_performance_test_env(60, 20).unwrap();

    // 查询最危险的 10 天
    let start = Instant::now();
    let summaries = d1_use_case
        .get_top_risky_days("V001", "2026-01-20", "2026-03-20", 10)
        .unwrap();
    let duration = start.elapsed();

    println!("✅ Top-10 查询结果: {} 天, 耗时: {:?}", summaries.len(), duration);
    assert!(
        duration.as_millis() < 100,
        "D1 Top-N 查询应该 < 100ms, 实际: {:?}",
        duration
    );

    println!("========== D1 Top-N 查询性能测试通过 ==========\n");
}

// ==========================================
// 测试2: D4 查询性能测试 (目标 < 100ms)
// ==========================================

#[test]
fn test_d4_query_performance_small_dataset() {
    println!("\n========== D4 查询性能测试 (10天 x 5机组 = 50条) ==========");

    let (_temp_file, _conn, _refresh_service, _d1_use_case, d4_use_case) =
        setup_performance_test_env(10, 5).unwrap();

    // 查询所有机组堵塞概况
    let start = Instant::now();
    let profiles = d4_use_case
        .get_machine_bottleneck_profile("V001", None, "2026-01-20", "2026-01-29")
        .unwrap();
    let duration = start.elapsed();

    println!(
        "✅ 查询结果: {} 个机组-日, 耗时: {:?}",
        profiles.len(),
        duration
    );
    assert!(
        duration.as_millis() < 100,
        "D4 查询应该 < 100ms, 实际: {:?}",
        duration
    );

    println!("========== D4 小数据集查询性能测试通过 ==========\n");
}

#[test]
fn test_d4_query_performance_medium_dataset() {
    println!("\n========== D4 查询性能测试 (30天 x 10机组 = 300条) ==========");

    let (_temp_file, _conn, _refresh_service, _d1_use_case, d4_use_case) =
        setup_performance_test_env(30, 10).unwrap();

    // 查询所有机组堵塞概况
    let start = Instant::now();
    let profiles = d4_use_case
        .get_machine_bottleneck_profile("V001", None, "2026-01-20", "2026-02-18")
        .unwrap();
    let duration = start.elapsed();

    println!(
        "✅ 查询结果: {} 个机组-日, 耗时: {:?}",
        profiles.len(),
        duration
    );
    assert!(
        duration.as_millis() < 100,
        "D4 查询应该 < 100ms, 实际: {:?}",
        duration
    );

    println!("========== D4 中数据集查询性能测试通过 ==========\n");
}

#[test]
fn test_d4_query_performance_large_dataset() {
    println!("\n========== D4 查询性能测试 (60天 x 20机组 = 1200条) ==========");

    let (_temp_file, _conn, _refresh_service, _d1_use_case, d4_use_case) =
        setup_performance_test_env(60, 20).unwrap();

    // 查询所有机组堵塞概况
    let start = Instant::now();
    let profiles = d4_use_case
        .get_machine_bottleneck_profile("V001", None, "2026-01-20", "2026-03-20")
        .unwrap();
    let duration = start.elapsed();

    println!(
        "✅ 查询结果: {} 个机组-日, 耗时: {:?}",
        profiles.len(),
        duration
    );
    assert!(
        duration.as_millis() < 200,
        "D4 查询应该 < 200ms, 实际: {:?}",
        duration
    );

    println!("========== D4 大数据集查询性能测试通过 ==========\n");
}

#[test]
fn test_d4_filtered_query_performance() {
    println!("\n========== D4 机组过滤查询性能测试 ==========");

    let (_temp_file, _conn, _refresh_service, _d1_use_case, d4_use_case) =
        setup_performance_test_env(60, 20).unwrap();

    // 查询单个机组的堵塞概况
    let start = Instant::now();
    let profiles = d4_use_case
        .get_machine_bottleneck_profile("V001", Some("H032"), "2026-01-20", "2026-03-20")
        .unwrap();
    let duration = start.elapsed();

    println!(
        "✅ 单机组查询结果: {} 条, 耗时: {:?}",
        profiles.len(),
        duration
    );
    assert!(
        duration.as_millis() < 100,
        "D4 过滤查询应该 < 100ms, 实际: {:?}",
        duration
    );

    println!("========== D4 机组过滤查询性能测试通过 ==========\n");
}

#[test]
fn test_d4_top_n_query_performance() {
    println!("\n========== D4 Top-N 查询性能测试 ==========");

    let (_temp_file, _conn, _refresh_service, _d1_use_case, d4_use_case) =
        setup_performance_test_env(60, 20).unwrap();

    // 查询最堵塞的 10 个机组-日
    let start = Instant::now();
    let profiles = d4_use_case
        .get_top_bottlenecks("V001", "2026-01-20", "2026-03-20", 10)
        .unwrap();
    let duration = start.elapsed();

    println!(
        "✅ Top-10 查询结果: {} 个机组-日, 耗时: {:?}",
        profiles.len(),
        duration
    );
    assert!(
        duration.as_millis() < 150,
        "D4 Top-N 查询应该 < 150ms, 实际: {:?}",
        duration
    );

    println!("========== D4 Top-N 查询性能测试通过 ==========\n");
}

#[test]
fn test_d4_heatmap_query_performance() {
    println!("\n========== D4 热力图查询性能测试 ==========");

    let (_temp_file, _conn, _refresh_service, _d1_use_case, d4_use_case) =
        setup_performance_test_env(30, 10).unwrap();

    // 查询热力图数据
    let start = Instant::now();
    let heatmap = d4_use_case
        .get_bottleneck_heatmap("V001", "2026-01-20", "2026-02-18")
        .unwrap();
    let duration = start.elapsed();

    println!(
        "✅ 热力图查询结果: {} 个机组 x {} 个单元格, 耗时: {:?}",
        heatmap.machines.len(),
        heatmap.data.len(),
        duration
    );
    assert!(
        duration.as_millis() < 100,
        "D4 热力图查询应该 < 100ms, 实际: {:?}",
        duration
    );

    println!("========== D4 热力图查询性能测试通过 ==========\n");
}

// ==========================================
// 测试3: 全量刷新性能测试 (目标 < 5s)
// ==========================================

#[test]
fn test_full_refresh_performance_small_dataset() {
    println!("\n========== 全量刷新性能测试 (10天 x 5机组 = 50条) ==========");

    let (_temp_file, conn, refresh_service, _d1_use_case, _d4_use_case) =
        setup_performance_test_env(10, 5).unwrap();

    // 执行全量刷新
    let start = Instant::now();
    let refresh_id = refresh_service
        .refresh_all(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: true,
                affected_machines: None,
                affected_date_range: None,
            },
            RefreshTrigger::ManualRefresh,
            Some("性能测试".to_string()),
        )
        .unwrap();
    let duration = start.elapsed();

    // 验证刷新成功
    {
        let c = conn.lock().unwrap();
        let d1_count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_day_summary WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let d4_count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_machine_bottleneck WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        println!("✅ 刷新完成: refresh_id={}", refresh_id);
        println!("   - D1 记录数: {}", d1_count);
        println!("   - D4 记录数: {}", d4_count);
        println!("   - 耗时: {:?}", duration);
    }

    assert!(
        duration.as_secs() < 5,
        "全量刷新应该 < 5s, 实际: {:?}",
        duration
    );

    println!("========== 小数据集全量刷新性能测试通过 ==========\n");
}

#[test]
fn test_full_refresh_performance_medium_dataset() {
    println!("\n========== 全量刷新性能测试 (30天 x 10机组 = 300条) ==========");

    let (_temp_file, conn, refresh_service, _d1_use_case, _d4_use_case) =
        setup_performance_test_env(30, 10).unwrap();

    // 执行全量刷新
    let start = Instant::now();
    let refresh_id = refresh_service
        .refresh_all(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: true,
                affected_machines: None,
                affected_date_range: None,
            },
            RefreshTrigger::ManualRefresh,
            Some("性能测试".to_string()),
        )
        .unwrap();
    let duration = start.elapsed();

    // 验证刷新成功
    {
        let c = conn.lock().unwrap();
        let d1_count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_day_summary WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let d4_count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_machine_bottleneck WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        println!("✅ 刷新完成: refresh_id={}", refresh_id);
        println!("   - D1 记录数: {}", d1_count);
        println!("   - D4 记录数: {}", d4_count);
        println!("   - 耗时: {:?}", duration);
    }

    assert!(
        duration.as_secs() < 5,
        "全量刷新应该 < 5s, 实际: {:?}",
        duration
    );

    println!("========== 中数据集全量刷新性能测试通过 ==========\n");
}

#[test]
fn test_full_refresh_performance_large_dataset() {
    println!("\n========== 全量刷新性能测试 (60天 x 20机组 = 1200条) ==========");

    let (_temp_file, conn, refresh_service, _d1_use_case, _d4_use_case) =
        setup_performance_test_env(60, 20).unwrap();

    // 执行全量刷新
    let start = Instant::now();
    let refresh_id = refresh_service
        .refresh_all(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: true,
                affected_machines: None,
                affected_date_range: None,
            },
            RefreshTrigger::ManualRefresh,
            Some("性能测试".to_string()),
        )
        .unwrap();
    let duration = start.elapsed();

    // 验证刷新成功
    {
        let c = conn.lock().unwrap();
        let d1_count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_day_summary WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let d4_count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_machine_bottleneck WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        println!("✅ 刷新完成: refresh_id={}", refresh_id);
        println!("   - D1 记录数: {}", d1_count);
        println!("   - D4 记录数: {}", d4_count);
        println!("   - 耗时: {:?}", duration);
    }

    assert!(
        duration.as_secs() < 5,
        "全量刷新应该 < 5s, 实际: {:?}",
        duration
    );

    println!("========== 大数据集全量刷新性能测试通过 ==========\n");
}

// ==========================================
// 测试4: 增量刷新性能测试 (目标 < 1s)
// ==========================================

#[test]
fn test_incremental_refresh_performance_by_date() {
    println!("\n========== 增量刷新性能测试 (按日期范围) ==========");

    let (_temp_file, conn, refresh_service, _d1_use_case, _d4_use_case) =
        setup_performance_test_env(60, 20).unwrap();

    // 先执行全量刷新
    refresh_service
        .refresh_all(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: true,
                affected_machines: None,
                affected_date_range: None,
            },
            RefreshTrigger::ManualRefresh,
            Some("初始全量刷新".to_string()),
        )
        .unwrap();

    // 增量刷新: 只刷新 7 天
    let start = Instant::now();
    let refresh_id = refresh_service
        .refresh_all(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: false,
                affected_machines: None,
                affected_date_range: Some(("2026-01-25".to_string(), "2026-01-31".to_string())),
            },
            RefreshTrigger::PlanItemChanged,
            Some("增量刷新7天".to_string()),
        )
        .unwrap();
    let duration = start.elapsed();

    // 验证增量刷新成功
    {
        let c = conn.lock().unwrap();
        let d1_count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_day_summary WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        println!("✅ 增量刷新完成: refresh_id={}", refresh_id);
        println!("   - D1 总记录数: {}", d1_count);
        println!("   - 耗时: {:?}", duration);
    }

    assert!(
        duration.as_secs() < 1,
        "增量刷新应该 < 1s, 实际: {:?}",
        duration
    );

    println!("========== 增量刷新 (日期) 性能测试通过 ==========\n");
}

#[test]
fn test_incremental_refresh_performance_by_machine() {
    println!("\n========== 增量刷新性能测试 (按机组) ==========");

    let (_temp_file, conn, refresh_service, _d1_use_case, _d4_use_case) =
        setup_performance_test_env(60, 20).unwrap();

    // 先执行全量刷新
    refresh_service
        .refresh_all(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: true,
                affected_machines: None,
                affected_date_range: None,
            },
            RefreshTrigger::ManualRefresh,
            Some("初始全量刷新".to_string()),
        )
        .unwrap();

    // 增量刷新: 只刷新 3 个机组
    let start = Instant::now();
    let refresh_id = refresh_service
        .refresh_all(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: false,
                affected_machines: Some(vec![
                    "H032".to_string(),
                    "H033".to_string(),
                    "H034".to_string(),
                ]),
                affected_date_range: None,
            },
            RefreshTrigger::CapacityPoolChanged,
            Some("增量刷新3个机组".to_string()),
        )
        .unwrap();
    let duration = start.elapsed();

    // 验证增量刷新成功
    {
        let c = conn.lock().unwrap();
        let d4_count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_machine_bottleneck WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        println!("✅ 增量刷新完成: refresh_id={}", refresh_id);
        println!("   - D4 总记录数: {}", d4_count);
        println!("   - 耗时: {:?}", duration);
    }

    assert!(
        duration.as_secs() < 1,
        "增量刷新应该 < 1s, 实际: {:?}",
        duration
    );

    println!("========== 增量刷新 (机组) 性能测试通过 ==========\n");
}

#[test]
fn test_incremental_refresh_performance_combined() {
    println!("\n========== 增量刷新性能测试 (机组 + 日期) ==========");

    let (_temp_file, conn, refresh_service, _d1_use_case, _d4_use_case) =
        setup_performance_test_env(60, 20).unwrap();

    // 先执行全量刷新
    refresh_service
        .refresh_all(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: true,
                affected_machines: None,
                affected_date_range: None,
            },
            RefreshTrigger::ManualRefresh,
            Some("初始全量刷新".to_string()),
        )
        .unwrap();

    // 增量刷新: 3 个机组 x 7 天
    let start = Instant::now();
    let refresh_id = refresh_service
        .refresh_all(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: false,
                affected_machines: Some(vec![
                    "H032".to_string(),
                    "H033".to_string(),
                    "H034".to_string(),
                ]),
                affected_date_range: Some(("2026-01-25".to_string(), "2026-01-31".to_string())),
            },
            RefreshTrigger::PlanItemChanged,
            Some("增量刷新3机组7天".to_string()),
        )
        .unwrap();
    let duration = start.elapsed();

    // 验证增量刷新成功
    {
        let c = conn.lock().unwrap();
        let d4_count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_machine_bottleneck WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        println!("✅ 增量刷新完成: refresh_id={}", refresh_id);
        println!("   - D4 总记录数: {}", d4_count);
        println!("   - 耗时: {:?}", duration);
    }

    assert!(
        duration.as_millis() < 500,
        "组合增量刷新应该 < 500ms, 实际: {:?}",
        duration
    );

    println!("========== 增量刷新 (组合) 性能测试通过 ==========\n");
}

// ==========================================
// 测试5: 刷新队列性能测试
// ==========================================

#[test]
fn test_refresh_queue_processing_performance() {
    println!("\n========== 刷新队列处理性能测试 ==========");

    let (_temp_file, conn, refresh_service, _d1_use_case, _d4_use_case) =
        setup_performance_test_env(30, 10).unwrap();

    let queue = RefreshQueue::new(conn.clone(), refresh_service).unwrap();

    // 入队 10 个刷新任务
    for i in 0..10 {
        let task = RefreshTask::new(
            RefreshScope {
                version_id: format!("V{:03}", i + 1),
                is_full_refresh: false,
                affected_machines: Some(vec!["H032".to_string()]),
                affected_date_range: Some(("2026-01-25".to_string(), "2026-01-31".to_string())),
            },
            RefreshTrigger::PlanItemChanged,
            Some(format!("测试任务 {}", i + 1)),
            3,
        );

        queue.enqueue(task).unwrap();
    }

    // 处理所有任务
    let start = Instant::now();
    let completed_tasks = queue.process_all().unwrap();
    let duration = start.elapsed();

    println!(
        "✅ 队列处理完成: {} 个任务, 耗时: {:?}",
        completed_tasks.len(),
        duration
    );
    println!(
        "   - 平均每任务: {:?}",
        duration / completed_tasks.len() as u32
    );

    // 验证所有任务都完成
    assert_eq!(completed_tasks.len(), 10);

    let stats = queue.get_queue_stats().unwrap();
    println!("   - 队列统计: {:?}", stats);

    println!("========== 刷新队列处理性能测试通过 ==========\n");
}

// ==========================================
// 测试6: 综合性能对比测试
// ==========================================

#[test]
fn test_performance_comparison_summary() {
    println!("\n========== 综合性能对比汇总 ==========\n");

    let dataset_configs = vec![
        (10, 5, "小数据集 (10天 x 5机组 = 50条)"),
        (30, 10, "中数据集 (30天 x 10机组 = 300条)"),
        (60, 20, "大数据集 (60天 x 20机组 = 1200条)"),
    ];

    println!("| 数据集规模 | D1查询 | D4查询 | 全量刷新 | 增量刷新 |");
    println!("|-----------|--------|--------|----------|----------|");

    for (num_dates, num_machines, label) in dataset_configs {
        let (_temp_file, conn, refresh_service, d1_use_case, d4_use_case) =
            setup_performance_test_env(num_dates, num_machines).unwrap();

        // D1 查询
        let start = Instant::now();
        let _ = d1_use_case
            .get_day_summary(
                "V001",
                "2026-01-20",
                &format!(
                    "{}",
                    (chrono::NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()
                        + chrono::Duration::days(num_dates as i64 - 1))
                    .format("%Y-%m-%d")
                ),
            )
            .unwrap();
        let d1_duration = start.elapsed();

        // D4 查询
        let start = Instant::now();
        let _ = d4_use_case
            .get_machine_bottleneck_profile(
                "V001",
                None,
                "2026-01-20",
                &format!(
                    "{}",
                    (chrono::NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()
                        + chrono::Duration::days(num_dates as i64 - 1))
                    .format("%Y-%m-%d")
                ),
            )
            .unwrap();
        let d4_duration = start.elapsed();

        // 全量刷新
        let start = Instant::now();
        let _ = refresh_service
            .refresh_all(
                RefreshScope {
                    version_id: "V001".to_string(),
                    is_full_refresh: true,
                    affected_machines: None,
                    affected_date_range: None,
                },
                RefreshTrigger::ManualRefresh,
                Some("性能测试".to_string()),
            )
            .unwrap();
        let full_refresh_duration = start.elapsed();

        // 增量刷新 (7天, 3机组)
        let start = Instant::now();
        let _ = refresh_service
            .refresh_all(
                RefreshScope {
                    version_id: "V001".to_string(),
                    is_full_refresh: false,
                    affected_machines: Some(vec![
                        "H032".to_string(),
                        "H033".to_string(),
                        "H034".to_string(),
                    ]),
                    affected_date_range: Some((
                        "2026-01-25".to_string(),
                        "2026-01-31".to_string(),
                    )),
                },
                RefreshTrigger::PlanItemChanged,
                Some("增量刷新".to_string()),
            )
            .unwrap();
        let inc_refresh_duration = start.elapsed();

        // 打印结果
        println!(
            "| {} | {:>6.1}ms | {:>6.1}ms | {:>8.2}s | {:>8.1}ms |",
            label,
            d1_duration.as_secs_f64() * 1000.0,
            d4_duration.as_secs_f64() * 1000.0,
            full_refresh_duration.as_secs_f64(),
            inc_refresh_duration.as_secs_f64() * 1000.0
        );

        // 保留 conn 的作用域,避免过早 drop
        drop(conn);
    }

    println!("\n========== 综合性能对比汇总完成 ==========\n");
}

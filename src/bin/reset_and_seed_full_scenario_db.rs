use chrono::{Duration, Local, Utc};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use hot_rolling_aps::app::get_default_db_path;
use hot_rolling_aps::db::open_sqlite_connection;
use hot_rolling_aps::decision::services::{DecisionRefreshService, RefreshScope, RefreshTrigger};

const ACTIVE_PLAN_ID: &str = "P001";
const ACTIVE_VERSION_ID: &str = "V001";
const DRAFT_VERSION_ID: &str = "V002";
const DEFAULT_MATERIAL_COUNT: i32 = 2000;
const HORIZON_DAYS: i64 = 30;

#[derive(Debug, Clone)]
struct ScheduledItem {
    machine_code: String,
    plan_date: String, // YYYY-MM-DD
    seq_no: i32,
    weight_t: f64,
    source_type: String, // CALC/FROZEN/MANUAL
    locked_in_plan: bool,
    force_release_in_plan: bool,
    violation_flags: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(get_default_db_path);

    let material_count = std::env::args()
        .nth(2)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(DEFAULT_MATERIAL_COUNT)
        .max(1000);

    backup_and_reset_db(&db_path)?;

    let conn = open_sqlite_connection(&db_path)?;

    // Create schema
    let schema_sql = include_str!("../../scripts/dev_db/schema.sql");
    conn.execute_batch(schema_sql)?;

    // Seed data
    seed_full_scenario(&conn, material_count)?;

    // Refresh decision read models (D1-D6) so the UI has deterministic results immediately after seeding.
    let conn_arc = Arc::new(Mutex::new(conn));
    refresh_decision_read_models(conn_arc.clone(), ACTIVE_VERSION_ID)?;
    refresh_decision_read_models(conn_arc.clone(), DRAFT_VERSION_ID)?;

    print_quick_counts(conn_arc)?;

    Ok(())
}

fn backup_and_reset_db(db_path: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(db_path);
    if !path.exists() {
        return Ok(());
    }

    let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_path = format!("{}.bak.{}", db_path, ts);
    fs::copy(path, &backup_path)?;
    fs::remove_file(path)?;

    eprintln!("Backed up {} -> {}", db_path, backup_path);
    Ok(())
}

fn seed_full_scenario(conn: &Connection, material_count: i32) -> Result<(), Box<dyn Error>> {
    let base_date = Local::now().date_naive();

    // Pre-compute scheduling so we can keep plan_item and material_state consistent.
    let schedule_map = build_schedule_map(base_date, material_count);
    let draft_schedule_map = build_draft_schedule_map(&schedule_map, base_date);

    let now_naive = Local::now().naive_local();
    let now_sql_dt = now_naive.format("%Y-%m-%d %H:%M:%S").to_string();
    let now_rfc3339 = Utc::now().to_rfc3339();

    let tx = conn.unchecked_transaction()?;

    // schema_version (dev schema.sql already includes v0.6 features; keep it aligned for startup warnings)
    tx.execute(
        "INSERT INTO schema_version (version, applied_at) VALUES (6, ?1)",
        params![now_sql_dt],
    )?;

    // Global config scope + a minimal, opinionated default set
    tx.execute(
        "INSERT INTO config_scope (scope_id, scope_type, scope_key, created_at) VALUES ('global','GLOBAL','GLOBAL',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','season_mode','MANUAL',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','manual_season','WINTER',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','winter_months','11,12,1,2,3',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','min_temp_days_winter','3',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','min_temp_days_summer','4',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','standard_finishing_machines','H031,H032,H033,H034',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','machine_offset_days','4',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','weight_anomaly_threshold','100.0',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','batch_retention_days','90',?1)",
        params![now_sql_dt],
    )?;

    // 紧急等级阈值配置
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','urgent_n1_days','2',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','urgent_n2_days','7',?1)",
        params![now_sql_dt],
    )?;

    // 换辊配置
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','roll_suggest_threshold_t','1500',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','roll_hard_limit_t','2500',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','roll_change_downtime_minutes','45',?1)",
        params![now_sql_dt],
    )?;

    // 产能配置
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','overflow_pct','0.05',?1)",
        params![now_sql_dt],
    )?;

    // 重算配置
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','recalc_window_days','7',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','cascade_window_days','14',?1)",
        params![now_sql_dt],
    )?;

    // 结构校正配置
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','target_ratio','{}',?1)",
        params![now_sql_dt],
    )?;
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','deviation_threshold','0.1',?1)",
        params![now_sql_dt],
    )?;

    // 每日节奏偏差阈值（与结构偏差阈值解耦）
    tx.execute(
        "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global','rhythm_deviation_threshold','0.1',?1)",
        params![now_sql_dt],
    )?;

    // Machines (frontend defaults include H031-H034)
    let machines = [
        ("H031", "Finishing Line 31", 60.0, 1400.0, 1.15),
        ("H032", "Finishing Line 32", 65.0, 1500.0, 1.15),
        ("H033", "Finishing Line 33", 70.0, 1600.0, 1.15),
        ("H034", "Finishing Line 34", 75.0, 1700.0, 1.15),
    ];
    for (code, name, hourly, daily_target, limit_pct) in machines {
        tx.execute(
            r#"
            INSERT INTO machine_master (
                machine_code, machine_name, hourly_capacity_t,
                default_daily_target_t, default_daily_limit_pct, is_active,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7)
            "#,
            params![code, name, hourly, daily_target, limit_pct, now_sql_dt, now_sql_dt],
        )?;
    }

    // Plan + versions
    tx.execute(
        r#"
        INSERT INTO plan (
            plan_id, plan_name, plan_type, base_plan_id,
            created_by, created_at, updated_at
        ) VALUES (?1, ?2, 'BASELINE', NULL, 'seed', ?3, ?4)
        "#,
        params![ACTIVE_PLAN_ID, "Full Scenario Seed Plan", now_sql_dt, now_sql_dt],
    )?;

    tx.execute(
        r#"
        INSERT INTO plan_version (
            version_id, plan_id, version_no, status,
            frozen_from_date, recalc_window_days, config_snapshot_json,
            created_by, created_at, revision
        ) VALUES (?1, ?2, 1, 'ACTIVE', ?3, 30, '{}', 'seed', ?4, 0)
        "#,
        params![
            ACTIVE_VERSION_ID,
            ACTIVE_PLAN_ID,
            base_date.format("%Y-%m-%d").to_string(),
            now_sql_dt
        ],
    )?;

    tx.execute(
        r#"
        INSERT INTO plan_version (
            version_id, plan_id, version_no, status,
            frozen_from_date, recalc_window_days, config_snapshot_json,
            created_by, created_at, revision
        ) VALUES (?1, ?2, 2, 'DRAFT', ?3, 30, '{}', 'seed', ?4, 0)
        "#,
        params![
            DRAFT_VERSION_ID,
            ACTIVE_PLAN_ID,
            base_date.format("%Y-%m-%d").to_string(),
            now_sql_dt
        ],
    )?;

    // ==========================================
    // 每日生产节奏（品种大类）- 预设 + 目标（按版本×机组×日期）
    // ==========================================

    // Presets (deterministic IDs for easy testing)
    let rhythm_presets: [(&str, &str, &str); 4] = [
        (
            "RP_BALANCED",
            "均衡（普/汽/家/管 各25%）",
            r#"{"普板":0.25,"汽车板":0.25,"家电板":0.25,"管线钢":0.25}"#,
        ),
        (
            "RP_AUTO_HEAVY",
            "汽车优先（40%）",
            r#"{"汽车板":0.40,"家电板":0.25,"管线钢":0.20,"普板":0.15}"#,
        ),
        (
            "RP_APPLIANCE_HEAVY",
            "家电集中（45%）",
            r#"{"家电板":0.45,"汽车板":0.20,"管线钢":0.15,"普板":0.20}"#,
        ),
        (
            "RP_PIPELINE_HEAVY",
            "管线集中（50%）",
            r#"{"管线钢":0.50,"普板":0.25,"汽车板":0.15,"家电板":0.10}"#,
        ),
    ];

    for (preset_id, preset_name, target_json) in rhythm_presets.iter().copied() {
        tx.execute(
            r#"
            INSERT INTO plan_rhythm_preset (
                preset_id, preset_name, dimension, target_json,
                is_active, created_at, updated_at, updated_by
            ) VALUES (?1, ?2, 'PRODUCT_CATEGORY', ?3, 1, ?4, ?5, 'seed')
            "#,
            params![preset_id, preset_name, target_json, now_sql_dt, now_sql_dt],
        )?;
    }

    let rhythm_by_machine: HashMap<&str, &str> = HashMap::from([
        ("H031", "RP_BALANCED"),
        ("H032", "RP_AUTO_HEAVY"),
        ("H033", "RP_PIPELINE_HEAVY"),
        ("H034", "RP_APPLIANCE_HEAVY"),
    ]);

    for version_id in [ACTIVE_VERSION_ID, DRAFT_VERSION_ID] {
        for day in 0..HORIZON_DAYS {
            let plan_date = (base_date + Duration::days(day)).to_string();
            for (machine_code, preset_id) in &rhythm_by_machine {
                let target_json = rhythm_presets
                    .iter()
                    .copied()
                    .find(|(id, _, _)| *id == *preset_id)
                    .map(|(_, _, json)| json)
                    .unwrap_or(r#"{}"#);

                tx.execute(
                    r#"
                    INSERT OR REPLACE INTO plan_rhythm_target (
                        version_id, machine_code, plan_date, dimension,
                        target_json, preset_id, updated_at, updated_by
                    ) VALUES (?1, ?2, ?3, 'PRODUCT_CATEGORY', ?4, ?5, ?6, 'seed')
                    "#,
                    params![
                        version_id,
                        *machine_code,
                        plan_date,
                        target_json,
                        *preset_id,
                        now_sql_dt
                    ],
                )?;
            }
        }
    }

    // Materials (material_master + material_state)
    for i in 1..=material_count {
        let material_id = format!("MAT{:04}", i);
        let machine_code = match i % 4 {
            0 => "H031",
            1 => "H032",
            2 => "H033",
            _ => "H034",
        };

        let (contract_no, due_date) = if (1..=10).contains(&i) {
            ("C_OVERDUE".to_string(), (base_date - Duration::days(2)).to_string())
        } else if (11..=30).contains(&i) {
            ("C_NEAR_DUE".to_string(), (base_date + Duration::days(2)).to_string())
        } else if (31..=60).contains(&i) {
            ("C_CAP_SHORT".to_string(), (base_date + Duration::days(14)).to_string())
        } else {
            (
                format!("C_NORMAL_{:02}", (i % 8) + 1),
                (base_date + Duration::days(30 + (i % 10) as i64)).to_string(),
            )
        };

        let width_mm = 1200.0 + ((i % 10) as f64) * 50.0;
        let thickness_mm = 2.0 + ((i % 5) as f64) * 0.5;
        let length_m = 10.0 + ((i % 3) as f64);
        let weight_t = material_weight_t(i);
        let available_width_mm = width_mm - 20.0;

        let steel_mark: Option<String> = if i % 13 == 0 {
            None
        } else if i % 2 == 0 {
            Some("Q235B".to_string())
        } else {
            Some("Q345B".to_string())
        };

        // 品种大类：用于“每日生产节奏管理”（先做大类，不引入规格系数）
        let product_category: Option<String> = match i % 5 {
            0 => Some("普板".to_string()),
            1 => Some("汽车板".to_string()),
            2 => Some("家电板".to_string()),
            3 => Some("管线钢".to_string()),
            _ => Some("工程机械".to_string()),
        };

        let stock_age_days = if (101..=130).contains(&i) {
            40
        } else {
            (i % 15) as i32
        };

        tx.execute(
            r#"
            INSERT INTO material_master (
                material_id, manufacturing_order_id,
                contract_no, due_date, rush_flag,
                next_machine_code, rework_machine_code, current_machine_code,
                width_mm, thickness_mm, length_m, weight_t, available_width_mm,
                steel_mark, slab_id, material_status_code_src,
                status_updated_at, output_age_days_raw, stock_age_days,
                contract_nature, weekly_delivery_flag, export_flag,
                product_category,
                created_at, updated_at
            ) VALUES (
                ?1, ?2,
                ?3, ?4, ?5,
                ?6, NULL, ?7,
                ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15,
                ?16, ?17, ?18,
                ?19, ?20, ?21,
                ?22,
                ?23, ?24
            )
            "#,
            params![
                material_id,
                format!("MO{:05}", i),
                contract_no,
                due_date,
                if i % 9 == 0 { "Y" } else { "N" },
                machine_code,
                machine_code,
                width_mm,
                thickness_mm,
                length_m,
                weight_t,
                available_width_mm,
                steel_mark,
                format!("SLAB{:05}", i),
                "OK",
                now_rfc3339,
                (i % 20) as i32,
                stock_age_days,
                if i % 9 == 0 { "RUSH" } else { "NORMAL" },
                if i % 7 == 0 { "Y" } else { "N" },
                if i % 11 == 0 { "1" } else { "0" },
                product_category,
                now_rfc3339,
                now_rfc3339,
            ],
        )?;

        // Seed a few immature (not temperature-ready) materials across multiple contracts
        // so D2/D3 have realistic cold-stock signals.
        let is_immature = is_immature_seed(i);
        let ready_in_days = if is_immature { ((i % 5) + 1) as i32 } else { 0 };
        let earliest_sched_date = (base_date + Duration::days(ready_in_days as i64)).to_string();

        let (sched_state, is_mature) = if is_immature {
            ("PENDING_MATURE", 0)
        } else {
            ("READY", 1)
        };

        let urgent_level = if contract_no == "C_OVERDUE" {
            "L3"
        } else if contract_no == "C_NEAR_DUE" {
            "L2"
        } else if contract_no == "C_CAP_SHORT" {
            "L1"
        } else {
            "L0"
        };

        // Manual urgent overrides for a handful of normal contracts
        let manual_urgent_flag = if matches!(i, 77 | 88 | 99) { 1 } else { 0 };
        let urgent_level = if manual_urgent_flag == 1 { "L3" } else { urgent_level };

        let in_frozen_zone = if matches!(i, 5 | 6 | 7 | 8 | 32 | 36) { 1 } else { 0 };
        let lock_flag = if in_frozen_zone == 1 { 1 } else { 0 };
        let force_release_flag = if matches!(i, 15 | 16) { 1 } else { 0 };

        let scheduled = schedule_map.get(&material_id).cloned();
        let (sched_state, scheduled_date, scheduled_machine_code, seq_no) = if let Some(s) = &scheduled {
            (
                "SCHEDULED",
                Some(s.plan_date.clone()),
                Some(s.machine_code.clone()),
                Some(s.seq_no),
            )
        } else {
            (sched_state, None, None, None)
        };

        tx.execute(
            r#"
            INSERT INTO material_state (
                material_id,
                sched_state, lock_flag, force_release_flag,
                urgent_level, urgent_reason, rush_level,
                rolling_output_age_days, ready_in_days, earliest_sched_date,
                last_calc_version_id, updated_at,
                stock_age_days,
                scheduled_date, scheduled_machine_code, seq_no,
                manual_urgent_flag, in_frozen_zone,
                updated_by,
                contract_no, due_date, urgency_level,
                weight_t, is_mature,
                machine_code, spec_width_mm, spec_thick_mm
            ) VALUES (
                ?1,
                ?2, ?3, ?4,
                ?5, ?6, ?7,
                ?8, ?9, ?10,
                ?11, ?12,
                ?13,
                ?14, ?15, ?16,
                ?17, ?18,
                ?19,
                ?20, ?21, ?22,
                ?23, ?24,
                ?25, ?26, ?27
            )
            "#,
            params![
                material_id,
                sched_state,
                lock_flag,
                force_release_flag,
                urgent_level,
                Some(format!("seeded urgent={} frozen={} force_release={}", urgent_level, in_frozen_zone, force_release_flag)),
                if i % 9 == 0 { "L2" } else { "L0" }, // rush_level (simplified)
                (i % 20) as i32,
                ready_in_days,
                earliest_sched_date,
                ACTIVE_VERSION_ID,
                now_rfc3339,
                stock_age_days,
                scheduled_date,
                scheduled_machine_code,
                seq_no,
                manual_urgent_flag,
                in_frozen_zone,
                "seed",
                contract_no,
                due_date,
                urgent_level,
                weight_t,
                is_mature,
                machine_code,
                width_mm,
                thickness_mm,
            ],
        )?;
    }

    // plan_item (active version only)
    for (material_id, s) in &schedule_map {
        tx.execute(
            r#"
            INSERT INTO plan_item (
                version_id, material_id, machine_code, plan_date, seq_no,
                weight_t, source_type, locked_in_plan, force_release_in_plan,
                violation_flags
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                ACTIVE_VERSION_ID,
                material_id,
                s.machine_code,
                s.plan_date,
                s.seq_no,
                s.weight_t,
                s.source_type,
                if s.locked_in_plan { 1 } else { 0 },
                if s.force_release_in_plan { 1 } else { 0 },
                s.violation_flags,
            ],
        )?;
    }

    // plan_item (draft version)
    for (material_id, s) in &draft_schedule_map {
        tx.execute(
            r#"
            INSERT INTO plan_item (
                version_id, material_id, machine_code, plan_date, seq_no,
                weight_t, source_type, locked_in_plan, force_release_in_plan,
                violation_flags
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                DRAFT_VERSION_ID,
                material_id,
                s.machine_code,
                s.plan_date,
                s.seq_no,
                s.weight_t,
                s.source_type,
                if s.locked_in_plan { 1 } else { 0 },
                if s.force_release_in_plan { 1 } else { 0 },
                s.violation_flags,
            ],
        )?;
    }

    // capacity_pool (30 days horizon)
    let machine_targets: HashMap<&str, f64> = HashMap::from([
        ("H031", 1400.0),
        ("H032", 1500.0),
        ("H033", 1600.0),
        ("H034", 1700.0),
    ]);

    // Used capacity per machine/date is derived from plan_item (per version) to keep data consistent.
    for (version_id, schedule) in [
        (ACTIVE_VERSION_ID, &schedule_map),
        (DRAFT_VERSION_ID, &draft_schedule_map),
    ] {
        let mut accumulated: HashMap<&str, f64> = HashMap::new();
        for (machine_code, _) in machine_targets.iter() {
            accumulated.insert(machine_code, 0.0);
        }

        for day in 0..HORIZON_DAYS {
            let plan_date = (base_date + Duration::days(day)).to_string();
            for (machine_code, target_capacity_t) in &machine_targets {
                let limit_capacity_t = target_capacity_t * 1.15;

                let used_capacity_t: f64 = schedule
                    .values()
                    .filter(|s| s.machine_code == *machine_code && s.plan_date == plan_date)
                    .map(|s| s.weight_t)
                    .sum();

                let overflow_t = (used_capacity_t - limit_capacity_t).max(0.0);
                let frozen_capacity_t: f64 = schedule
                    .values()
                    .filter(|s| {
                        s.machine_code == *machine_code
                            && s.plan_date == plan_date
                            && s.locked_in_plan
                    })
                    .map(|s| s.weight_t)
                    .sum();

                let acc = accumulated.get_mut(machine_code).unwrap();
                *acc += used_capacity_t;

                tx.execute(
                    r#"
                    INSERT INTO capacity_pool (
                        version_id,
                        machine_code, plan_date,
                        target_capacity_t, limit_capacity_t,
                        used_capacity_t, overflow_t,
                        frozen_capacity_t, accumulated_tonnage_t,
                        roll_campaign_id
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                    "#,
                    params![
                        version_id,
                        machine_code,
                        plan_date,
                        target_capacity_t,
                        limit_capacity_t,
                        used_capacity_t,
                        overflow_t,
                        frozen_capacity_t,
                        *acc,
                        format!("{}_C1", machine_code),
                    ],
                )?;
            }
        }
    }

    // roller_campaign (1 active campaign per machine, diverse status)
    let roll_thresholds: HashMap<&str, (f64, f64, &str)> = HashMap::from([
        ("H031", (1000.0, 1500.0, "HardStop")),
        ("H032", (1000.0, 1500.0, "Normal")),
        ("H033", (1000.0, 1500.0, "Suggest")),
        ("H034", (1000.0, 1500.0, "Suggest")),
    ]);
    for version_id in [ACTIVE_VERSION_ID, DRAFT_VERSION_ID] {
        for (machine_code, (suggest_t, hard_t, status)) in &roll_thresholds {
            tx.execute(
                r#"
                INSERT INTO roller_campaign (
                    version_id, machine_code, campaign_no,
                    start_date, end_date,
                    cum_weight_t, suggest_threshold_t, hard_limit_t,
                    status
                ) VALUES (?1, ?2, 1, ?3, NULL, 0.0, ?4, ?5, ?6)
                "#,
                params![
                    version_id,
                    machine_code,
                    (base_date - Duration::days(7)).to_string(),
                    suggest_t,
                    hard_t,
                    status,
                ],
            )?;
        }

        // risk_snapshot (per version, 30 days horizon)
        // Note: decision/dashboard expects LOW/MEDIUM/HIGH/CRITICAL (not Green/Yellow/...).
        for day in 0..HORIZON_DAYS {
            let snapshot_date = (base_date + Duration::days(day)).to_string();
            for (machine_code, target_capacity_t) in &machine_targets {
                let limit_capacity_t = target_capacity_t * 1.15;

                let (used_capacity_t, overflow_t): (f64, f64) = tx.query_row(
                    r#"
                    SELECT used_capacity_t, overflow_t
                    FROM capacity_pool
                    WHERE version_id = ?1 AND machine_code = ?2 AND plan_date = ?3
                    "#,
                    params![version_id, machine_code, snapshot_date],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;

                let util = if *target_capacity_t > 0.0 {
                    used_capacity_t / *target_capacity_t
                } else {
                    0.0
                };

                let risk_level = if overflow_t > 0.0 || used_capacity_t > limit_capacity_t {
                    "CRITICAL"
                } else if util >= 0.95 {
                    "HIGH"
                } else if util >= 0.80 {
                    "MEDIUM"
                } else {
                    "LOW"
                };

                let risk_reasons = match risk_level {
                    "CRITICAL" => "Capacity overflow / hard limit exceeded",
                    "HIGH" => "Utilization very high",
                    "MEDIUM" => "Utilization moderate",
                    _ => "Healthy",
                };

                // Backlog metrics are version-aware (based on whether the material is scheduled in this version).
                let (urgent_total_t, mature_backlog_t, immature_backlog_t): (f64, f64, f64) = tx.query_row(
                    r#"
                    SELECT
                        COALESCE(SUM(CASE
                            WHEN ms.urgent_level IN ('L2','L3')
                                 AND NOT EXISTS (
                                     SELECT 1 FROM plan_item pi
                                     WHERE pi.version_id = ?1 AND pi.material_id = ms.material_id
                                 )
                            THEN ms.weight_t ELSE 0 END), 0.0) AS urgent_total_t,
                        COALESCE(SUM(CASE
                            WHEN ms.is_mature = 1
                                 AND NOT EXISTS (
                                     SELECT 1 FROM plan_item pi
                                     WHERE pi.version_id = ?1 AND pi.material_id = ms.material_id
                                 )
                            THEN ms.weight_t ELSE 0 END), 0.0) AS mature_backlog_t,
                        COALESCE(SUM(CASE
                            WHEN ms.is_mature = 0
                                 AND NOT EXISTS (
                                     SELECT 1 FROM plan_item pi
                                     WHERE pi.version_id = ?1 AND pi.material_id = ms.material_id
                                 )
                            THEN ms.weight_t ELSE 0 END), 0.0) AS immature_backlog_t
                    FROM material_state ms
                    WHERE ms.machine_code = ?2
                    "#,
                    params![version_id, machine_code],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )?;

                let campaign_status = if *machine_code == "H034" && day >= 4 && day <= 6 {
                    Some("NEAR_HARD_STOP".to_string())
                } else {
                    None
                };

                tx.execute(
                    r#"
                    INSERT INTO risk_snapshot (
                        version_id, machine_code, snapshot_date,
                        risk_level, risk_reasons,
                        target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t,
                        urgent_total_t, mature_backlog_t, immature_backlog_t,
                        campaign_status, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                    "#,
                    params![
                        version_id,
                        machine_code,
                        snapshot_date,
                        risk_level,
                        risk_reasons,
                        target_capacity_t,
                        used_capacity_t,
                        limit_capacity_t,
                        overflow_t,
                        urgent_total_t,
                        mature_backlog_t,
                        immature_backlog_t,
                        campaign_status,
                        now_sql_dt,
                    ],
                )?;
            }
        }
    }

    // Import batches + conflicts (to unblock import UI paths)
    let batch_id = Uuid::new_v4().to_string();
    let conflict_rows: i32 = 5;
    let warning_rows: i32 = 15;
    let blocked_rows: i32 = 0;
    let success_rows: i32 = (material_count - conflict_rows - warning_rows - blocked_rows).max(0);
    let dq_report_json = format!(
        r#"{{"summary":{{"total":{},"success":{},"warning":{},"conflict":{},"blocked":{}}},"notes":"seeded"}}"#,
        material_count, success_rows, warning_rows, conflict_rows, blocked_rows
    );
    tx.execute(
        r#"
        INSERT INTO import_batch (
            batch_id, file_name, file_path,
            total_rows, success_rows, blocked_rows, warning_rows, conflict_rows,
            imported_at, imported_by, elapsed_ms, dq_report_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        "#,
        params![
            batch_id,
            "seed_full_scenario.csv",
            "tests/fixtures/datasets/01_normal_data.csv",
            material_count,
            success_rows,
            blocked_rows,
            warning_rows,
            conflict_rows,
            now_rfc3339,
            "seed",
            1234i32,
            dq_report_json,
        ],
    )?;

    let conflict_types = [
        "PrimaryKeyDuplicate",
        "DataTypeError",
        "ForeignKeyViolation",
        "PrimaryKeyMissing",
        "DataTypeError",
    ];
    for (idx, ct) in conflict_types.iter().enumerate() {
        let conflict_id = Uuid::new_v4().to_string();
        // Store as JSON string to match serde_json::from_str in repo.
        let ct_json = serde_json::to_string(ct)?;
        tx.execute(
            r#"
            INSERT INTO import_conflict (
                conflict_id, source_batch_id, material_id, row_number,
                conflict_type, source_row_json, existing_row_json,
                resolution_status, resolution_action, resolution_note, resolved_at, detected_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'OPEN', NULL, NULL, NULL, ?8)
            "#,
            params![
                conflict_id,
                batch_id,
                format!("MAT{:04}", idx + 1),
                (idx + 2) as i32,
                ct_json,
                format!(r#"{{"row":{},"material_id":"MAT{:04}","reason":"seed conflict"}}"#, idx + 2, idx + 1),
                Option::<String>::None,
                now_rfc3339,
            ],
        )?;
    }

    // Action logs (enough for "recent actions" widgets)
    for (action_type, detail) in [
        ("SEED_DB", "Reset and seeded full-scenario dev DB"),
        ("CREATE_PLAN", "Created seed plan"),
        ("CREATE_VERSION", "Created seed versions"),
    ] {
        tx.execute(
            r#"
            INSERT INTO action_log (
                action_id, version_id, action_type, action_ts, actor,
                payload_json, impact_summary_json, machine_code,
                date_range_start, date_range_end, detail
            ) VALUES (?1, ?2, ?3, ?4, 'seed', NULL, NULL, NULL, NULL, NULL, ?5)
            "#,
            params![
                Uuid::new_v4().to_string(),
                ACTIVE_VERSION_ID,
                action_type,
                now_sql_dt,
                detail,
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}

fn build_schedule_map(
    base_date: chrono::NaiveDate,
    material_count: i32,
) -> HashMap<String, ScheduledItem> {
    let mut ids_by_machine: HashMap<&str, Vec<String>> = HashMap::from([
        ("H031", Vec::new()),
        ("H032", Vec::new()),
        ("H033", Vec::new()),
        ("H034", Vec::new()),
    ]);

    for i in 1..=material_count {
        let material_id = format!("MAT{:04}", i);
        let machine_code = match i % 4 {
            0 => "H031",
            1 => "H032",
            2 => "H033",
            _ => "H034",
        };
        ids_by_machine.get_mut(machine_code).unwrap().push(material_id);
    }

    let mut seq_per_machine_day: HashMap<(String, String), i32> = HashMap::new();
    let mut out: HashMap<String, ScheduledItem> = HashMap::new();

    for (machine_code, ids) in ids_by_machine {
        let eligible_ids: Vec<String> = ids
            .into_iter()
            .filter(|mid| is_schedule_candidate(mat_index(mid)))
            .collect();

        for (j, material_id) in eligible_ids.into_iter().enumerate() {
            let day_offset = (j as i64 % HORIZON_DAYS) as i64; // spread across horizon days
            let plan_date = (base_date + Duration::days(day_offset)).to_string();

            let key = (machine_code.to_string(), plan_date.clone());
            let seq = seq_per_machine_day.entry(key).or_insert(0);
            *seq += 1;

            let locked_in_plan = matches!(
                material_id.as_str(),
                "MAT0005" | "MAT0006" | "MAT0007" | "MAT0008" | "MAT0032" | "MAT0036"
            );
            let force_release_in_plan = matches!(material_id.as_str(), "MAT0015" | "MAT0016");
            let source_type = if locked_in_plan {
                "FROZEN"
            } else if force_release_in_plan {
                "MANUAL"
            } else {
                "CALC"
            };

            // Inject a couple of STRUCT_CONFLICT samples for D2 blocking-factor demo.
            let violation_flags = if matches!(material_id.as_str(), "MAT0031" | "MAT0032" | "MAT0033") {
                Some(r#"["STRUCT_CONFLICT"]"#.to_string())
            } else {
                None
            };

            out.insert(
                material_id.clone(),
                ScheduledItem {
                    machine_code: machine_code.to_string(),
                    plan_date,
                    seq_no: *seq,
                    weight_t: material_weight_t(mat_index(&material_id)),
                    source_type: source_type.to_string(),
                    locked_in_plan,
                    force_release_in_plan,
                    violation_flags,
                },
            );
        }
    }

    apply_capacity_spikes(base_date, &mut out);
    out
}

fn build_draft_schedule_map(
    base_schedule_map: &HashMap<String, ScheduledItem>,
    base_date: chrono::NaiveDate,
) -> HashMap<String, ScheduledItem> {
    let mut out = base_schedule_map.clone();

    // Deterministic "what-if" variations for version compare / sync testing.
    for (material_id, item) in out.iter_mut() {
        let idx = mat_index(material_id);

        // Shift some items by +1 day.
        if idx % 10 == 0 {
            let original =
                chrono::NaiveDate::parse_from_str(&item.plan_date, "%Y-%m-%d").unwrap_or(base_date);
            let offset_days = original.signed_duration_since(base_date).num_days();
            let next_offset = (offset_days + 1).rem_euclid(HORIZON_DAYS);
            item.plan_date = (base_date + Duration::days(next_offset)).to_string();
        }

        // Move a smaller subset to a different machine.
        if idx % 25 == 0 {
            item.machine_code = match item.machine_code.as_str() {
                "H031" => "H032",
                "H032" => "H033",
                "H033" => "H034",
                _ => "H031",
            }
            .to_string();
        }
    }

    // Recompute seq_no per (machine, date) to keep plan_item ordering sane.
    recompute_seq_no(&mut out);
    out
}

fn apply_capacity_spikes(base_date: chrono::NaiveDate, schedule_map: &mut HashMap<String, ScheduledItem>) {
    // Goal: create a few high-util / overflow days deterministically, while keeping
    // capacity_pool.used_capacity_t == SUM(plan_item.weight_t) (data-consistent).
    //
    // Strategy: move a small batch of scheduled items onto a couple of specific days.

    let spikes = [
        ("H033", 3_i64, 32_usize), // create an overflow-ish day
        ("H034", 5_i64, 28_usize), // create a high utilization day
        ("H032", 2_i64, 24_usize), // create a high utilization day
    ];

    for (machine, day_offset, take_n) in spikes {
        let spike_date = (base_date + Duration::days(day_offset)).to_string();

        let mut ids: Vec<String> = schedule_map
            .iter()
            .filter_map(|(mid, s)| if s.machine_code == machine { Some(mid.clone()) } else { None })
            .collect();
        ids.sort();

        for mid in ids.into_iter().take(take_n) {
            if let Some(s) = schedule_map.get_mut(&mid) {
                s.plan_date = spike_date.clone();
            }
        }
    }

    recompute_seq_no(schedule_map);
}

fn recompute_seq_no(schedule_map: &mut HashMap<String, ScheduledItem>) {
    let mut items: Vec<(String, String, String)> = schedule_map
        .iter()
        .map(|(mid, s)| (s.machine_code.clone(), s.plan_date.clone(), mid.clone()))
        .collect();

    // Stable ordering: (machine_code, plan_date, material_id).
    items.sort();

    let mut seq_per_key: HashMap<(String, String), i32> = HashMap::new();
    for (machine_code, plan_date, material_id) in items {
        let seq = seq_per_key
            .entry((machine_code.clone(), plan_date.clone()))
            .or_insert(0);
        *seq += 1;

        if let Some(s) = schedule_map.get_mut(&material_id) {
            s.seq_no = *seq;
            s.machine_code = machine_code;
            s.plan_date = plan_date;
        }
    }
}

fn mat_index(material_id: &str) -> i32 {
    material_id
        .strip_prefix("MAT")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
}

fn is_schedule_candidate(idx: i32) -> bool {
    // Keep some urgent contracts intentionally under-scheduled so D2 has meaningful failures:
    // - C_OVERDUE (1-10): schedule only 2 / 10
    // - C_NEAR_DUE (11-30): schedule 12 / 20
    // - C_CAP_SHORT (31-60): schedule 10 / 30
    // Ensure a few "frozen/manual" demo items are always scheduled.
    if matches!(idx, 5 | 6 | 7 | 8 | 15 | 16 | 32 | 36) {
        return true;
    }

    if (1..=10).contains(&idx) {
        idx <= 2 || idx == 4
    } else if (11..=30).contains(&idx) {
        idx <= 22
    } else if (31..=60).contains(&idx) {
        idx <= 40
    } else if is_immature_seed(idx) {
        // Keep some immature items unscheduled so cold-stock and due-date signals remain visible.
        idx % 2 == 0
    } else {
        // For normal items, keep a stable unscheduled tail so D2/D3 dashboards don't go empty.
        idx % 5 != 0
    }
}

fn is_immature_seed(idx: i32) -> bool {
    // Hand-picked clusters + a periodic spread for larger data sets.
    (91..=120).contains(&idx) || (7..=10).contains(&idx) || (55..=60).contains(&idx) || (idx > 60 && idx % 17 == 0)
}

fn material_weight_t(idx: i32) -> f64 {
    // Deterministic weight distribution for more realistic KPIs and aggregations.
    // Range: ~35t - 95t, with occasional heavier items up to ~125t.
    let base = 35.0 + ((idx % 11) as f64) * 5.5;
    let bonus = if idx % 29 == 0 { 30.0 } else { 0.0 };
    (base + bonus).min(125.0)
}

fn refresh_decision_read_models(
    conn: Arc<Mutex<Connection>>,
    version_id: &str,
) -> Result<(), Box<dyn Error>> {
    let service = DecisionRefreshService::new(conn.clone());
    let scope = RefreshScope {
        version_id: version_id.to_string(),
        is_full_refresh: true,
        affected_machines: None,
        affected_date_range: None,
    };

    let refresh_id = service.refresh_all(
        scope,
        RefreshTrigger::ManualRefresh,
        Some("reset_and_seed_full_scenario_db bin".to_string()),
    )?;
    eprintln!(
        "Decision read models refreshed: version_id={}, refresh_id={}",
        version_id, refresh_id
    );

    Ok(())
}

fn print_quick_counts(conn: Arc<Mutex<Connection>>) -> Result<(), Box<dyn Error>> {
    let conn = conn.lock().unwrap();
    let tables = [
        "machine_master",
        "material_master",
        "material_state",
        "plan",
        "plan_version",
        "plan_item",
        "capacity_pool",
        "risk_snapshot",
        "roller_campaign",
        "action_log",
        "import_batch",
        "import_conflict",
        "decision_day_summary",
        "decision_machine_bottleneck",
        "decision_order_failure_set",
        "decision_cold_stock_profile",
        "decision_roll_campaign_alert",
        "decision_capacity_opportunity",
    ];

    eprintln!("Row counts:");
    for t in tables {
        let sql = format!("SELECT COUNT(*) FROM {}", t);
        let c: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
        eprintln!("  {:<28} {}", t, c);
    }
    Ok(())
}

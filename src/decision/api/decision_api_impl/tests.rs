use super::DecisionApiImpl;
use crate::decision::api::decision_api::DecisionApi;
use crate::decision::api::dto::*;
use crate::decision::repository::{
    bottleneck_repo::BottleneckRepository, day_summary_repo::DaySummaryRepository,
};
use crate::decision::use_cases::impls::{MachineBottleneckUseCaseImpl, MostRiskyDayUseCaseImpl};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

fn setup_test_api() -> DecisionApiImpl {
    // 创建内存数据库
    let conn = Connection::open_in_memory().unwrap();
    crate::db::configure_sqlite_connection(&conn).unwrap();
    let conn = Arc::new(Mutex::new(conn));

    // 创建 risk_snapshot 表
    {
        let c = conn.lock().unwrap();
        c.execute(
            r#"
            CREATE TABLE IF NOT EXISTS risk_snapshot (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                snapshot_date TEXT NOT NULL,
                risk_level TEXT NOT NULL,
                risk_reasons TEXT,
                target_capacity_t REAL NOT NULL,
                used_capacity_t REAL NOT NULL,
                limit_capacity_t REAL NOT NULL,
                overflow_t REAL NOT NULL,
                urgent_total_t REAL NOT NULL,
                mature_backlog_t REAL,
                immature_backlog_t REAL,
                campaign_status TEXT,
                created_at TEXT NOT NULL,
                PRIMARY KEY (version_id, machine_code, snapshot_date)
            )
            "#,
            [],
        )
        .unwrap();

        // 插入测试数据
        c.execute(
            r#"
            INSERT INTO risk_snapshot VALUES (
                'V001', 'H032', '2026-01-24', 'HIGH', '产能紧张',
                1500.0, 1450.0, 2000.0, 0.0, 800.0, 500.0, 200.0, 'OK',
                datetime('now')
            )
            "#,
            [],
        )
        .unwrap();
    }

    // 创建 D1 仓储和用例
    let d1_repo = Arc::new(DaySummaryRepository::new(conn.clone()));
    let d1_use_case = Arc::new(MostRiskyDayUseCaseImpl::new(d1_repo));

    // 创建 capacity_pool 和 plan_item 表
    {
        let c = conn.lock().unwrap();
        c.execute(
            r#"
            CREATE TABLE IF NOT EXISTS capacity_pool (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                target_capacity_t REAL NOT NULL,
                limit_capacity_t REAL NOT NULL,
                used_capacity_t REAL NOT NULL DEFAULT 0.0,
                overflow_t REAL NOT NULL DEFAULT 0.0,
                frozen_capacity_t REAL NOT NULL DEFAULT 0.0,
                accumulated_tonnage_t REAL NOT NULL DEFAULT 0.0,
                roll_campaign_id TEXT,
                PRIMARY KEY (version_id, machine_code, plan_date)
            )
            "#,
            [],
        )
        .unwrap();

        c.execute(
            r#"
            CREATE TABLE IF NOT EXISTS plan_item (
                version_id TEXT NOT NULL,
                material_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                seq_no INTEGER NOT NULL,
                weight_t REAL NOT NULL,
                source_type TEXT NOT NULL,
                locked_in_plan INTEGER NOT NULL DEFAULT 0,
                force_release_in_plan INTEGER NOT NULL DEFAULT 0,
                violation_flags TEXT,
                PRIMARY KEY (version_id, material_id)
            )
            "#,
            [],
        )
        .unwrap();

        c.execute(
            r#"
            CREATE TABLE IF NOT EXISTS material_master (
                material_id TEXT PRIMARY KEY,
                current_machine_code TEXT,
                next_machine_code TEXT,
                weight_t REAL,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )
            "#,
            [],
        )
        .unwrap();

        c.execute(
            r#"
            CREATE TABLE IF NOT EXISTS material_state (
                material_id TEXT PRIMARY KEY,
                sched_state TEXT NOT NULL DEFAULT 'READY',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )
            "#,
            [],
        )
        .unwrap();

        c.execute(
            r#"
            CREATE TABLE IF NOT EXISTS machine_master (
                machine_code TEXT PRIMARY KEY,
                machine_name TEXT,
                is_active INTEGER NOT NULL DEFAULT 1
            )
            "#,
            [],
        )
        .unwrap();

        // 插入测试数据
        c.execute(
            r#"
            INSERT INTO capacity_pool VALUES (
                'V001', 'H032', '2026-01-24', 1500.0, 2000.0, 1950.0, 0.0, 100.0, 15000.0, 'RC001'
            )
            "#,
            [],
        )
        .unwrap();

        c.execute(
            r#"
            INSERT INTO plan_item VALUES (
                'V001', 'MAT001', 'H032', '2026-01-24', 1, 150.0, 'AUTO', 0, 0, ''
            )
            "#,
            [],
        )
        .unwrap();

        c.execute(
            "INSERT INTO machine_master (machine_code, machine_name, is_active) VALUES ('H031', '机组31', 1)",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO machine_master (machine_code, machine_name, is_active) VALUES ('H032', '机组32', 1)",
            [],
        )
        .unwrap();
    }

    // 创建 D4 仓储和用例
    let d4_repo = Arc::new(BottleneckRepository::new(conn));
    let d4_use_case = Arc::new(MachineBottleneckUseCaseImpl::new(d4_repo));

    DecisionApiImpl::new(d1_use_case, d4_use_case)
}

#[test]
fn test_get_decision_day_summary() {
    let api = setup_test_api();

    let request = GetDecisionDaySummaryRequest {
        version_id: "V001".to_string(),
        date_from: "2026-01-24".to_string(),
        date_to: "2026-01-24".to_string(),
        risk_level_filter: None,
        limit: Some(10),
        sort_by: None,
    };

    let response = api.get_decision_day_summary(request).unwrap();

    assert_eq!(response.version_id, "V001");
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.total_count, 1);

    let day_summary = &response.items[0];
    assert_eq!(day_summary.plan_date, "2026-01-24");
    assert!(day_summary.risk_score > 0.0);
}

#[test]
fn test_get_machine_bottleneck_profile() {
    let api = setup_test_api();

    let request = GetMachineBottleneckProfileRequest {
        version_id: "V001".to_string(),
        date_from: "2026-01-24".to_string(),
        date_to: "2026-01-24".to_string(),
        machine_codes: None,
        bottleneck_level_filter: None,
        bottleneck_type_filter: None,
        limit: Some(50),
    };

    let response = api.get_machine_bottleneck_profile(request).unwrap();

    assert_eq!(response.version_id, "V001");
    assert_eq!(response.items.len(), 2);
    assert_eq!(response.total_count, 2);

    let h032 = response
        .items
        .iter()
        .find(|p| p.machine_code == "H032")
        .expect("H032 should exist");
    assert_eq!(h032.plan_date, "2026-01-24");
    assert!(h032.bottleneck_score >= 0.0);

    let h031 = response
        .items
        .iter()
        .find(|p| p.machine_code == "H031")
        .expect("H031 placeholder should exist");
    assert_eq!(h031.plan_date, "2026-01-24");
    assert_eq!(h031.bottleneck_level, "NONE");
    assert_eq!(h031.bottleneck_score, 0.0);
}

#[test]
fn test_unimplemented_apis() {
    let api = setup_test_api();

    // D2
    let d2_request = ListOrderFailureSetRequest {
        version_id: "V001".to_string(),
        fail_type_filter: None,
        urgency_level_filter: None,
        machine_codes: None,
        due_date_from: None,
        due_date_to: None,
        completion_rate_threshold: None,
        limit: None,
        offset: None,
    };
    assert!(api.list_order_failure_set(d2_request).is_err());

    // D3
    let d3_request = GetColdStockProfileRequest {
        version_id: "V001".to_string(),
        machine_codes: None,
        pressure_level_filter: None,
        age_bin_filter: None,
        limit: None,
    };
    assert!(api.get_cold_stock_profile(d3_request).is_err());

    // D5
    let d5_request = ListRollCampaignAlertsRequest {
        version_id: "V001".to_string(),
        machine_codes: None,
        alert_level_filter: None,
        alert_type_filter: None,
        date_from: None,
        date_to: None,
        limit: None,
    };
    assert!(api.list_roll_campaign_alerts(d5_request).is_err());

    // D6
    let d6_request = GetCapacityOpportunityRequest {
        version_id: "V001".to_string(),
        machine_codes: None,
        date_from: None,
        date_to: None,
        opportunity_type_filter: None,
        min_opportunity_t: None,
        limit: None,
    };
    assert!(api.get_capacity_opportunity(d6_request).is_err());
}

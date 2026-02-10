use super::ActionLogRepository;
use crate::domain::action_log::ActionLog;
use chrono::{NaiveDate, NaiveDateTime, Utc};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

fn setup_test_db() -> Arc<Mutex<Connection>> {
    let conn = Connection::open_in_memory().unwrap();
    crate::db::configure_sqlite_connection(&conn).unwrap();

    conn.execute(
        r#"
        CREATE TABLE action_log (
            action_id TEXT PRIMARY KEY,
            version_id TEXT NOT NULL,
            action_type TEXT NOT NULL,
            action_ts TEXT NOT NULL,
            actor TEXT NOT NULL,
            payload_json TEXT,
            impact_summary_json TEXT,
            machine_code TEXT,
            date_range_start TEXT,
            date_range_end TEXT,
            detail TEXT
        )
        "#,
        [],
    )
    .unwrap();

    Arc::new(Mutex::new(conn))
}

fn make_test_log(action_id: &str, version_id: &str, actor: &str) -> ActionLog {
    ActionLog {
        action_id: action_id.to_string(),
        version_id: Some(version_id.to_string()),
        action_type: "Import".to_string(),
        action_ts: Utc::now().naive_utc(),
        actor: actor.to_string(),
        payload_json: None,
        impact_summary_json: None,
        machine_code: Some("M01".to_string()),
        date_range_start: Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()),
        date_range_end: Some(NaiveDate::from_ymd_opt(2025, 1, 20).unwrap()),
        detail: Some("Test log".to_string()),
    }
}

#[test]
fn test_insert_and_find_by_id() {
    let conn = setup_test_db();
    let repo = ActionLogRepository::new(conn);

    let log = make_test_log("log1", "v1", "user1");
    let result = repo.insert(&log);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "log1");

    let found = repo.find_by_id("log1").unwrap();
    assert!(found.is_some());

    let found_log = found.unwrap();
    assert_eq!(found_log.action_id, "log1");
    assert_eq!(found_log.version_id, Some("v1".to_string()));
    assert_eq!(found_log.actor, "user1");
}

#[test]
fn test_find_by_version_id() {
    let conn = setup_test_db();
    let repo = ActionLogRepository::new(conn);

    let log1 = make_test_log("log1", "v1", "user1");
    let log2 = make_test_log("log2", "v1", "user2");
    let log3 = make_test_log("log3", "v2", "user1");

    repo.insert(&log1).unwrap();
    repo.insert(&log2).unwrap();
    repo.insert(&log3).unwrap();

    let logs = repo.find_by_version_id("v1").unwrap();

    assert_eq!(logs.len(), 2);
    assert!(logs.iter().any(|l| l.action_id == "log1"));
    assert!(logs.iter().any(|l| l.action_id == "log2"));
}

#[test]
fn test_find_by_actor() {
    let conn = setup_test_db();
    let repo = ActionLogRepository::new(conn);

    let log1 = make_test_log("log1", "v1", "user1");
    let log2 = make_test_log("log2", "v1", "user1");
    let log3 = make_test_log("log3", "v1", "user2");

    repo.insert(&log1).unwrap();
    repo.insert(&log2).unwrap();
    repo.insert(&log3).unwrap();

    let logs = repo.find_by_actor("user1", 10).unwrap();

    assert_eq!(logs.len(), 2);
}

#[test]
fn test_find_by_action_type() {
    let conn = setup_test_db();
    let repo = ActionLogRepository::new(conn);

    let mut log1 = make_test_log("log1", "v1", "user1");
    log1.action_type = "Import".to_string();

    let mut log2 = make_test_log("log2", "v1", "user1");
    log2.action_type = "Recalc".to_string();

    let mut log3 = make_test_log("log3", "v1", "user1");
    log3.action_type = "Import".to_string();

    repo.insert(&log1).unwrap();
    repo.insert(&log2).unwrap();
    repo.insert(&log3).unwrap();

    let logs = repo.find_by_action_type("Import", 10).unwrap();

    assert_eq!(logs.len(), 2);
}

#[test]
fn test_find_recent() {
    let conn = setup_test_db();
    let repo = ActionLogRepository::new(conn);

    for i in 1..=5 {
        let log = make_test_log(&format!("log{}", i), "v1", "user1");
        repo.insert(&log).unwrap();
    }

    let logs = repo.find_recent(3).unwrap();

    assert_eq!(logs.len(), 3);
}

#[test]
fn test_batch_insert() {
    let conn = setup_test_db();
    let repo = ActionLogRepository::new(conn);

    let logs = vec![
        make_test_log("log1", "v1", "user1"),
        make_test_log("log2", "v1", "user1"),
        make_test_log("log3", "v1", "user1"),
    ];

    let result = repo.batch_insert(logs);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 3);

    let all_logs = repo.find_by_version_id("v1").unwrap();
    assert_eq!(all_logs.len(), 3);
}

#[test]
fn test_count_by_version() {
    let conn = setup_test_db();
    let repo = ActionLogRepository::new(conn);

    repo.insert(&make_test_log("log1", "v1", "user1")).unwrap();
    repo.insert(&make_test_log("log2", "v1", "user1")).unwrap();
    repo.insert(&make_test_log("log3", "v2", "user1")).unwrap();

    let count = repo.count_by_version("v1").unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_count_by_actor() {
    let conn = setup_test_db();
    let repo = ActionLogRepository::new(conn);

    repo.insert(&make_test_log("log1", "v1", "user1")).unwrap();
    repo.insert(&make_test_log("log2", "v1", "user1")).unwrap();
    repo.insert(&make_test_log("log3", "v1", "user2")).unwrap();

    let count = repo.count_by_actor("user1").unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_find_by_impacted_date_range() {
    let conn = setup_test_db();
    let repo = ActionLogRepository::new(conn);

    let mut log1 = make_test_log("log1", "v1", "user1");
    log1.date_range_start = Some(NaiveDate::from_ymd_opt(2025, 1, 10).unwrap());
    log1.date_range_end = Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());

    let mut log2 = make_test_log("log2", "v1", "user1");
    log2.date_range_start = Some(NaiveDate::from_ymd_opt(2025, 1, 20).unwrap());
    log2.date_range_end = Some(NaiveDate::from_ymd_opt(2025, 1, 25).unwrap());

    repo.insert(&log1).unwrap();
    repo.insert(&log2).unwrap();

    let logs = repo
        .find_by_impacted_date_range(
            NaiveDate::from_ymd_opt(2025, 1, 12).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 18).unwrap(),
        )
        .unwrap();

    // log1 应该被找到 (10-15 overlaps 12-18)
    // log2 不应该被找到 (20-25 不overlaps 12-18)
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].action_id, "log1");
}

#[test]
fn test_find_by_material_id_in_time_range_matches_detail_and_json_token() {
    let conn = setup_test_db();
    let repo = ActionLogRepository::new(conn);

    let t1 = NaiveDateTime::parse_from_str("2026-01-24 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let t2 = NaiveDateTime::parse_from_str("2026-01-24 11:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let t3 = NaiveDateTime::parse_from_str("2026-01-24 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let t4 = NaiveDateTime::parse_from_str("2026-01-24 13:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

    // detail 命中
    let mut log_detail = make_test_log("log_detail", "v1", "user1");
    log_detail.action_ts = t1;
    log_detail.detail = Some("move material MAT001 -> H032/2026-01-24".to_string());
    repo.insert(&log_detail).unwrap();

    // payload_json 命中（\"MAT001\" token）
    let mut log_payload = make_test_log("log_payload", "v1", "user1");
    log_payload.action_ts = t2;
    log_payload.payload_json = Some(serde_json::json!({"material_id":"MAT001","op":"move"}));
    log_payload.detail = Some("payload_hit".to_string());
    repo.insert(&log_payload).unwrap();

    // impact_summary_json 命中（\"MAT001\" token）
    let mut log_impact = make_test_log("log_impact", "v1", "user1");
    log_impact.action_ts = t3;
    log_impact.impact_summary_json = Some(serde_json::json!({"moved":["MAT001"],"added":[]}));
    log_impact.detail = Some("impact_hit".to_string());
    repo.insert(&log_impact).unwrap();

    // 相似 material_id（MAT0010）不应被 \"MAT001\" token 误匹配（此处不写 detail，避免 substring 误判）
    let mut log_similar = make_test_log("log_similar", "v1", "user1");
    log_similar.action_ts = t4;
    log_similar.payload_json = Some(serde_json::json!({"material_id":"MAT0010"}));
    log_similar.detail = None;
    repo.insert(&log_similar).unwrap();

    let start_time =
        NaiveDateTime::parse_from_str("2026-01-24 09:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let end_time =
        NaiveDateTime::parse_from_str("2026-01-24 15:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

    let logs = repo
        .find_by_material_id_in_time_range("MAT001", start_time, end_time, 10)
        .unwrap();

    assert_eq!(logs.len(), 3);
    assert_eq!(logs[0].action_id, "log_impact");
    assert_eq!(logs[1].action_id, "log_payload");
    assert_eq!(logs[2].action_id, "log_detail");

    let logs_limited = repo
        .find_by_material_id_in_time_range("MAT001", start_time, end_time, 2)
        .unwrap();
    assert_eq!(logs_limited.len(), 2);
}

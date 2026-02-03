// Small dev utility: trigger a full decision read-model refresh for the latest ACTIVE version.
//
// Usage:
//   cargo run --bin manual_refresh_decision -- [db_path]
//
// This is intentionally lightweight and does not start the Tauri UI.

use hot_rolling_aps::decision::services::{DecisionRefreshService, RefreshScope, RefreshTrigger};
use hot_rolling_aps::db::open_sqlite_connection;
use rusqlite::{Connection, OptionalExtension};
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "hot_rolling_aps.db".to_string());

    let conn = Arc::new(Mutex::new(open_sqlite_connection(&db_path)?));

    let active_version_id: Option<String> = {
        let c = conn.lock().unwrap();
        c.query_row(
            "SELECT version_id FROM plan_version WHERE status = 'ACTIVE' ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
    };

    let version_id = active_version_id.ok_or("No ACTIVE plan_version found")?;

    let service = DecisionRefreshService::new(conn.clone());
    let scope = RefreshScope {
        version_id: version_id.clone(),
        is_full_refresh: true,
        affected_machines: None,
        affected_date_range: None,
    };

    let refresh_id = service.refresh_all(
        scope,
        RefreshTrigger::ManualRefresh,
        Some("manual_refresh_decision bin".to_string()),
    )?;

    println!("refresh_id={}", refresh_id);
    Ok(())
}

// ==========================================
// 热轧精整排产系统 - 换辊监控计划仓储
// ==========================================
// 职责: 管理 roll_campaign_plan 表 (按版本+机组)
// 说明: 用于“换辊时间监控/微调”，不直接影响排程结果
// ==========================================

use crate::db::open_sqlite_connection;
use crate::repository::error::{RepositoryError, RepositoryResult};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct RollCampaignPlanEntity {
    pub version_id: String,
    pub machine_code: String,
    pub initial_start_at: String, // YYYY-MM-DD HH:MM:SS
    pub next_change_at: Option<String>, // YYYY-MM-DD HH:MM:SS
    pub downtime_minutes: Option<i32>,
    pub updated_at: String,
    pub updated_by: Option<String>,
}

pub struct RollCampaignPlanRepository {
    conn: Arc<Mutex<Connection>>,
}

impl RollCampaignPlanRepository {
    pub fn new(db_path: &str) -> RepositoryResult<Self> {
        let conn = open_sqlite_connection(db_path)?;
        let repo = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        repo.ensure_table()?;
        Ok(repo)
    }

    pub fn from_connection(conn: Arc<Mutex<Connection>>) -> RepositoryResult<Self> {
        let repo = Self { conn };
        repo.ensure_table()?;
        Ok(repo)
    }

    fn get_conn(&self) -> RepositoryResult<std::sync::MutexGuard<Connection>> {
        self.conn
            .lock()
            .map_err(|e| RepositoryError::LockError(e.to_string()))
    }

    fn ensure_table(&self) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS roll_campaign_plan (
              version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
              machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
              initial_start_at TEXT NOT NULL,
              next_change_at TEXT,
              downtime_minutes INTEGER,
              updated_at TEXT NOT NULL DEFAULT (datetime('now')),
              updated_by TEXT,
              PRIMARY KEY (version_id, machine_code)
            );

            CREATE INDEX IF NOT EXISTS idx_roll_campaign_plan_version
              ON roll_campaign_plan(version_id, machine_code);
            "#,
        )?;
        Ok(())
    }

    pub fn upsert(&self, entity: &RollCampaignPlanEntity) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute(
            r#"
            INSERT INTO roll_campaign_plan (
                version_id,
                machine_code,
                initial_start_at,
                next_change_at,
                downtime_minutes,
                updated_at,
                updated_by
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(version_id, machine_code) DO UPDATE SET
                initial_start_at = excluded.initial_start_at,
                next_change_at = excluded.next_change_at,
                downtime_minutes = excluded.downtime_minutes,
                updated_at = excluded.updated_at,
                updated_by = excluded.updated_by
            "#,
            params![
                entity.version_id,
                entity.machine_code,
                entity.initial_start_at,
                entity.next_change_at,
                entity.downtime_minutes,
                entity.updated_at,
                entity.updated_by,
            ],
        )?;
        Ok(())
    }

    pub fn find_by_key(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> RepositoryResult<Option<RollCampaignPlanEntity>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id,
                machine_code,
                initial_start_at,
                next_change_at,
                downtime_minutes,
                updated_at,
                updated_by
            FROM roll_campaign_plan
            WHERE version_id = ?1 AND machine_code = ?2
            "#,
        )?;

        let result = stmt.query_row(params![version_id, machine_code], |row| {
            Ok(RollCampaignPlanEntity {
                version_id: row.get(0)?,
                machine_code: row.get(1)?,
                initial_start_at: row.get(2)?,
                next_change_at: row.get(3)?,
                downtime_minutes: row.get(4)?,
                updated_at: row.get(5)?,
                updated_by: row.get(6)?,
            })
        });

        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_by_version_id(&self, version_id: &str) -> RepositoryResult<Vec<RollCampaignPlanEntity>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id,
                machine_code,
                initial_start_at,
                next_change_at,
                downtime_minutes,
                updated_at,
                updated_by
            FROM roll_campaign_plan
            WHERE version_id = ?1
            ORDER BY machine_code ASC
            "#,
        )?;

        let rows = stmt
            .query_map(params![version_id], |row| {
                Ok(RollCampaignPlanEntity {
                    version_id: row.get(0)?,
                    machine_code: row.get(1)?,
                    initial_start_at: row.get(2)?,
                    next_change_at: row.get(3)?,
                    downtime_minutes: row.get(4)?,
                    updated_at: row.get(5)?,
                    updated_by: row.get(6)?,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(rows)
    }
}

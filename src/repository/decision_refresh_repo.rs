use crate::repository::error::{RepositoryError, RepositoryResult};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct DecisionRefreshQueueCounts {
    pub pending: i64,
    pub running: i64,
    pub failed: i64,
    pub completed: i64,
    pub cancelled: i64,
}

#[derive(Debug, Clone)]
pub struct DecisionRefreshTaskEntity {
    pub task_id: String,
    pub version_id: String,
    pub trigger_type: String,
    pub trigger_source: Option<String>,
    pub is_full_refresh: bool,
    pub affected_machines: Option<String>,
    pub affected_date_range: Option<String>,
    pub status: String,
    pub retry_count: i64,
    pub max_retries: i64,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub refresh_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DecisionRefreshLogEntity {
    pub refresh_id: String,
    pub version_id: String,
    pub trigger_type: String,
    pub trigger_source: Option<String>,
    pub is_full_refresh: bool,
    pub affected_machines: Option<String>,
    pub affected_date_range: Option<String>,
    pub refreshed_tables: String,
    pub rows_affected: i64,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub error_message: Option<String>,
}

pub struct DecisionRefreshRepository {
    conn: Arc<Mutex<Connection>>,
}

impl DecisionRefreshRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        let repo = Self { conn };
        // best-effort: do not block app startup for missing tables.
        if let Err(e) = repo.ensure_tables() {
            tracing::warn!("decision_refresh_* ensure failed: {}", e);
        }
        repo
    }

    fn get_conn(&self) -> RepositoryResult<std::sync::MutexGuard<Connection>> {
        self.conn
            .lock()
            .map_err(|e| RepositoryError::LockError(e.to_string()))
    }

    fn ensure_tables(&self) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS decision_refresh_queue (
              task_id TEXT PRIMARY KEY,
              version_id TEXT NOT NULL,
              trigger_type TEXT NOT NULL,
              trigger_source TEXT,
              is_full_refresh INTEGER NOT NULL DEFAULT 0,
              affected_machines TEXT,
              affected_date_range TEXT,
              status TEXT NOT NULL DEFAULT 'PENDING',
              retry_count INTEGER NOT NULL DEFAULT 0,
              max_retries INTEGER NOT NULL DEFAULT 3,
              created_at TEXT NOT NULL DEFAULT (datetime('now')),
              started_at TEXT,
              completed_at TEXT,
              error_message TEXT,
              refresh_id TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_refresh_queue_status
              ON decision_refresh_queue(status, created_at);
            CREATE INDEX IF NOT EXISTS idx_refresh_queue_version
              ON decision_refresh_queue(version_id, status);

            CREATE TABLE IF NOT EXISTS decision_refresh_log (
              refresh_id TEXT PRIMARY KEY,
              version_id TEXT NOT NULL,
              trigger_type TEXT NOT NULL,
              trigger_source TEXT,
              is_full_refresh INTEGER NOT NULL DEFAULT 0,
              affected_machines TEXT,
              affected_date_range TEXT,
              refreshed_tables TEXT NOT NULL,
              rows_affected INTEGER NOT NULL DEFAULT 0,
              started_at TEXT NOT NULL DEFAULT (datetime('now')),
              completed_at TEXT,
              duration_ms INTEGER,
              status TEXT NOT NULL DEFAULT 'RUNNING',
              error_message TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_refresh_log_version
              ON decision_refresh_log(version_id, started_at DESC);
            CREATE INDEX IF NOT EXISTS idx_refresh_log_trigger
              ON decision_refresh_log(trigger_type, started_at DESC);
            "#,
        )?;
        Ok(())
    }

    pub fn get_queue_counts_by_version(
        &self,
        version_id: &str,
    ) -> RepositoryResult<DecisionRefreshQueueCounts> {
        let conn = self.get_conn()?;

        let (pending, running, failed, completed, cancelled): (i64, i64, i64, i64, i64) = conn
            .query_row(
                r#"
                SELECT
                  COALESCE(SUM(CASE WHEN status = 'PENDING' THEN 1 ELSE 0 END), 0) AS pending_count,
                  COALESCE(SUM(CASE WHEN status = 'RUNNING' THEN 1 ELSE 0 END), 0) AS running_count,
                  COALESCE(SUM(CASE WHEN status = 'FAILED' THEN 1 ELSE 0 END), 0) AS failed_count,
                  COALESCE(SUM(CASE WHEN status = 'COMPLETED' THEN 1 ELSE 0 END), 0) AS completed_count,
                  COALESCE(SUM(CASE WHEN status = 'CANCELLED' THEN 1 ELSE 0 END), 0) AS cancelled_count
                FROM decision_refresh_queue
                WHERE version_id = ?1
                "#,
                params![version_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )?;

        Ok(DecisionRefreshQueueCounts {
            pending,
            running,
            failed,
            completed,
            cancelled,
        })
    }

    pub fn find_latest_task_by_version(
        &self,
        version_id: &str,
    ) -> RepositoryResult<Option<DecisionRefreshTaskEntity>> {
        let conn = self.get_conn()?;
        conn.query_row(
            r#"
            SELECT
              task_id,
              version_id,
              trigger_type,
              trigger_source,
              is_full_refresh,
              affected_machines,
              affected_date_range,
              status,
              retry_count,
              max_retries,
              created_at,
              started_at,
              completed_at,
              error_message,
              refresh_id
            FROM decision_refresh_queue
            WHERE version_id = ?1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            params![version_id],
            |row| map_task_row(row),
        )
        .optional()
        .map_err(|e| e.into())
    }

    pub fn find_latest_log_by_version(
        &self,
        version_id: &str,
    ) -> RepositoryResult<Option<DecisionRefreshLogEntity>> {
        let conn = self.get_conn()?;
        conn.query_row(
            r#"
            SELECT
              refresh_id,
              version_id,
              trigger_type,
              trigger_source,
              is_full_refresh,
              affected_machines,
              affected_date_range,
              refreshed_tables,
              rows_affected,
              started_at,
              completed_at,
              duration_ms,
              status,
              error_message
            FROM decision_refresh_log
            WHERE version_id = ?1
            ORDER BY started_at DESC
            LIMIT 1
            "#,
            params![version_id],
            |row| map_log_row(row),
        )
        .optional()
        .map_err(|e| e.into())
    }

    pub fn find_log_by_id(&self, refresh_id: &str) -> RepositoryResult<Option<DecisionRefreshLogEntity>> {
        let conn = self.get_conn()?;
        conn.query_row(
            r#"
            SELECT
              refresh_id,
              version_id,
              trigger_type,
              trigger_source,
              is_full_refresh,
              affected_machines,
              affected_date_range,
              refreshed_tables,
              rows_affected,
              started_at,
              completed_at,
              duration_ms,
              status,
              error_message
            FROM decision_refresh_log
            WHERE refresh_id = ?1
            "#,
            params![refresh_id],
            |row| map_log_row(row),
        )
        .optional()
        .map_err(|e| e.into())
    }
}

fn map_task_row(row: &Row) -> rusqlite::Result<DecisionRefreshTaskEntity> {
    Ok(DecisionRefreshTaskEntity {
        task_id: row.get(0)?,
        version_id: row.get(1)?,
        trigger_type: row.get(2)?,
        trigger_source: row.get(3)?,
        is_full_refresh: row.get::<_, i64>(4)? != 0,
        affected_machines: row.get(5)?,
        affected_date_range: row.get(6)?,
        status: row.get(7)?,
        retry_count: row.get(8)?,
        max_retries: row.get(9)?,
        created_at: row.get(10)?,
        started_at: row.get(11)?,
        completed_at: row.get(12)?,
        error_message: row.get(13)?,
        refresh_id: row.get(14)?,
    })
}

fn map_log_row(row: &Row) -> rusqlite::Result<DecisionRefreshLogEntity> {
    Ok(DecisionRefreshLogEntity {
        refresh_id: row.get(0)?,
        version_id: row.get(1)?,
        trigger_type: row.get(2)?,
        trigger_source: row.get(3)?,
        is_full_refresh: row.get::<_, i64>(4)? != 0,
        affected_machines: row.get(5)?,
        affected_date_range: row.get(6)?,
        refreshed_tables: row.get(7)?,
        rows_affected: row.get(8)?,
        started_at: row.get(9)?,
        completed_at: row.get(10)?,
        duration_ms: row.get(11)?,
        status: row.get(12)?,
        error_message: row.get(13)?,
    })
}

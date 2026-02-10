use crate::repository::error::{RepositoryError, RepositoryResult};
use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{params, Connection, Result as SqliteResult, Row};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyDraftStatus {
    Draft,
    Published,
    Expired,
}

impl StrategyDraftStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            StrategyDraftStatus::Draft => "DRAFT",
            StrategyDraftStatus::Published => "PUBLISHED",
            StrategyDraftStatus::Expired => "EXPIRED",
        }
    }

    pub fn parse(s: &str) -> StrategyDraftStatus {
        match s.trim().to_uppercase().as_str() {
            "PUBLISHED" => StrategyDraftStatus::Published,
            "EXPIRED" => StrategyDraftStatus::Expired,
            _ => StrategyDraftStatus::Draft,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StrategyDraftEntity {
    pub draft_id: String,
    pub base_version_id: String,
    pub plan_date_from: NaiveDate,
    pub plan_date_to: NaiveDate,

    pub strategy_key: String,
    pub strategy_base: String,
    pub strategy_title_cn: String,
    pub strategy_params_json: Option<String>,

    pub status: StrategyDraftStatus,
    pub created_by: String,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,

    pub published_as_version_id: Option<String>,
    pub published_by: Option<String>,
    pub published_at: Option<NaiveDateTime>,

    pub locked_by: Option<String>,
    pub locked_at: Option<NaiveDateTime>,

    pub summary_json: String,
    pub diff_items_json: String,
    pub diff_items_total: i64,
    pub diff_items_truncated: bool,
}

pub struct StrategyDraftRepository {
    conn: Arc<Mutex<Connection>>,
}

impl StrategyDraftRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        let repo = Self { conn };
        // best-effort: do not fail app startup for a missing table; errors will surface when using it.
        if let Err(e) = repo.ensure_table_and_indexes() {
            tracing::warn!("decision_strategy_draft ensure failed: {}", e);
        }
        repo
    }

    fn get_conn(&self) -> RepositoryResult<std::sync::MutexGuard<Connection>> {
        self.conn
            .lock()
            .map_err(|e| RepositoryError::LockError(e.to_string()))
    }

    fn ensure_table_and_indexes(&self) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS decision_strategy_draft (
              draft_id TEXT PRIMARY KEY,
              base_version_id TEXT NOT NULL REFERENCES plan_version(version_id),
              plan_date_from TEXT NOT NULL,
              plan_date_to TEXT NOT NULL,

              strategy_key TEXT NOT NULL,
              strategy_base TEXT NOT NULL,
              strategy_title_cn TEXT NOT NULL,
              strategy_params_json TEXT,

              status TEXT NOT NULL CHECK(status IN ('DRAFT', 'PUBLISHED', 'EXPIRED')),
              created_by TEXT NOT NULL,
              created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
              expires_at TEXT NOT NULL,
              published_as_version_id TEXT REFERENCES plan_version(version_id),
              published_by TEXT,
              published_at TEXT,

              locked_by TEXT,
              locked_at TEXT,

              summary_json TEXT NOT NULL,
              diff_items_json TEXT NOT NULL,
              diff_items_total INTEGER NOT NULL DEFAULT 0,
              diff_items_truncated INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_strategy_draft_base_version ON decision_strategy_draft(base_version_id);
            CREATE INDEX IF NOT EXISTS idx_strategy_draft_status ON decision_strategy_draft(status);
            CREATE INDEX IF NOT EXISTS idx_strategy_draft_expires_at ON decision_strategy_draft(expires_at);
            CREATE INDEX IF NOT EXISTS idx_strategy_draft_created_at ON decision_strategy_draft(created_at DESC);
            "#,
        )?;
        Ok(())
    }

    pub fn insert(&self, draft: &StrategyDraftEntity) -> RepositoryResult<()> {
        let conn = self.get_conn()?;

        conn.execute(
            r#"
            INSERT INTO decision_strategy_draft (
              draft_id, base_version_id, plan_date_from, plan_date_to,
              strategy_key, strategy_base, strategy_title_cn, strategy_params_json,
              status, created_by, created_at, expires_at,
              published_as_version_id, published_by, published_at,
              locked_by, locked_at,
              summary_json, diff_items_json, diff_items_total, diff_items_truncated
            ) VALUES (
              ?1, ?2, ?3, ?4,
              ?5, ?6, ?7, ?8,
              ?9, ?10, ?11, ?12,
              ?13, ?14, ?15,
              ?16, ?17,
              ?18, ?19, ?20, ?21
            )
            "#,
            params![
                draft.draft_id,
                draft.base_version_id,
                draft.plan_date_from.format("%Y-%m-%d").to_string(),
                draft.plan_date_to.format("%Y-%m-%d").to_string(),
                draft.strategy_key,
                draft.strategy_base,
                draft.strategy_title_cn,
                draft.strategy_params_json,
                draft.status.as_str(),
                draft.created_by,
                draft.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                draft.expires_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                draft.published_as_version_id,
                draft.published_by,
                draft
                    .published_at
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
                draft.locked_by,
                draft
                    .locked_at
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
                draft.summary_json,
                draft.diff_items_json,
                draft.diff_items_total,
                if draft.diff_items_truncated { 1 } else { 0 },
            ],
        )?;

        Ok(())
    }

    pub fn find_by_id(&self, draft_id: &str) -> RepositoryResult<Option<StrategyDraftEntity>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT draft_id, base_version_id, plan_date_from, plan_date_to,
                   strategy_key, strategy_base, strategy_title_cn, strategy_params_json,
                   status, created_by, created_at, expires_at,
                   published_as_version_id, published_by, published_at,
                   locked_by, locked_at,
                   summary_json, diff_items_json, diff_items_total, diff_items_truncated
            FROM decision_strategy_draft
            WHERE draft_id = ?1
            "#,
        )?;

        match stmt.query_row(params![draft_id], |row| map_row(row)) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_by_base_version_and_range(
        &self,
        base_version_id: &str,
        plan_date_from: NaiveDate,
        plan_date_to: NaiveDate,
        status_filter: Option<StrategyDraftStatus>,
        limit: i64,
    ) -> RepositoryResult<Vec<StrategyDraftEntity>> {
        let conn = self.get_conn()?;
        let limit = if limit <= 0 { 200 } else { limit.min(2000) };

        let (from, to) = if plan_date_to < plan_date_from {
            (plan_date_to, plan_date_from)
        } else {
            (plan_date_from, plan_date_to)
        };

        let mut sql = String::from(
            r#"
            SELECT draft_id, base_version_id, plan_date_from, plan_date_to,
                   strategy_key, strategy_base, strategy_title_cn, strategy_params_json,
                   status, created_by, created_at, expires_at,
                   published_as_version_id, published_by, published_at,
                   locked_by, locked_at,
                   summary_json, diff_items_json, diff_items_total, diff_items_truncated
            FROM decision_strategy_draft
            WHERE base_version_id = ?1
              AND plan_date_from = ?2
              AND plan_date_to = ?3
            "#,
        );

        if status_filter.is_some() {
            sql.push_str(" AND status = ?4 ");
        }

        sql.push_str(" ORDER BY created_at DESC LIMIT ?X ");

        // Workaround: rusqlite doesn't support named dynamic placeholder for LIMIT in params! macro.
        // Use literal limit since it is clamped to a safe range.
        sql = sql.replace("?X", &limit.to_string());

        let mut stmt = conn.prepare(&sql)?;
        let rows = match status_filter {
            Some(status) => stmt
                .query_map(
                    params![
                        base_version_id,
                        from.format("%Y-%m-%d").to_string(),
                        to.format("%Y-%m-%d").to_string(),
                        status.as_str(),
                    ],
                    |row| map_row(row),
                )?
                .collect::<SqliteResult<Vec<_>>>()?,
            None => stmt
                .query_map(
                    params![
                        base_version_id,
                        from.format("%Y-%m-%d").to_string(),
                        to.format("%Y-%m-%d").to_string(),
                    ],
                    |row| map_row(row),
                )?
                .collect::<SqliteResult<Vec<_>>>()?,
        };
        Ok(rows)
    }

    pub fn try_lock_for_publish(&self, draft_id: &str, operator: &str) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let actor = operator.trim();
        if actor.is_empty() {
            return Ok(0);
        }

        // Allow taking over a stale lock after 10 minutes.
        let rows = conn.execute(
            r#"
            UPDATE decision_strategy_draft
            SET locked_by = ?1,
                locked_at = datetime('now', 'localtime')
            WHERE draft_id = ?2
              AND status = 'DRAFT'
              AND expires_at > datetime('now', 'localtime')
              AND (
                locked_by IS NULL
                OR locked_by = ?1
                OR (locked_at IS NOT NULL AND locked_at < datetime('now', 'localtime', '-10 minutes'))
              )
            "#,
            params![actor, draft_id],
        )?;
        Ok(rows)
    }

    pub fn unlock(&self, draft_id: &str, operator: &str) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let rows = conn.execute(
            r#"
            UPDATE decision_strategy_draft
            SET locked_by = NULL,
                locked_at = NULL
            WHERE draft_id = ?1
              AND (locked_by IS NULL OR locked_by = ?2)
            "#,
            params![draft_id, operator.trim()],
        )?;
        Ok(rows)
    }

    pub fn mark_published(
        &self,
        draft_id: &str,
        published_as_version_id: &str,
        operator: &str,
        published_at: NaiveDateTime,
    ) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let rows = conn.execute(
            r#"
            UPDATE decision_strategy_draft
            SET status = 'PUBLISHED',
                published_as_version_id = ?1,
                published_by = ?2,
                published_at = ?3,
                locked_by = NULL,
                locked_at = NULL
            WHERE draft_id = ?4
            "#,
            params![
                published_as_version_id,
                operator.trim(),
                published_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                draft_id,
            ],
        )?;
        Ok(rows)
    }

    pub fn expire_if_needed(&self, draft_id: &str) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let rows = conn.execute(
            r#"
            UPDATE decision_strategy_draft
            SET status = 'EXPIRED',
                locked_by = NULL,
                locked_at = NULL
            WHERE draft_id = ?1
              AND status = 'DRAFT'
              AND expires_at <= datetime('now', 'localtime')
            "#,
            params![draft_id],
        )?;
        Ok(rows)
    }

    pub fn cleanup_expired(&self, keep_days: i64) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let days = if keep_days <= 0 { 7 } else { keep_days.min(90) };
        let sql = format!(
            r#"
            DELETE FROM decision_strategy_draft
            WHERE status = 'EXPIRED'
              AND created_at < datetime('now', 'localtime', '-{} days')
            "#,
            days
        );
        let rows = conn.execute(&sql, [])?;
        Ok(rows)
    }

    /// 删除与指定版本关联的所有策略草稿
    ///
    /// # 说明
    /// 删除 base_version_id 或 published_as_version_id 匹配的所有策略草稿
    pub fn delete_by_version(&self, version_id: &str) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let rows = conn.execute(
            r#"
            DELETE FROM decision_strategy_draft
            WHERE base_version_id = ?1 OR published_as_version_id = ?1
            "#,
            params![version_id],
        )?;
        Ok(rows)
    }
}

fn map_row(row: &Row) -> SqliteResult<StrategyDraftEntity> {
    let draft_id: String = row.get(0)?;
    let base_version_id: String = row.get(1)?;
    let plan_date_from_str: String = row.get(2)?;
    let plan_date_to_str: String = row.get(3)?;
    let strategy_key: String = row.get(4)?;
    let strategy_base: String = row.get(5)?;
    let strategy_title_cn: String = row.get(6)?;
    let strategy_params_json: Option<String> = row.get(7)?;
    let status_str: String = row.get(8)?;
    let created_by: String = row.get(9)?;
    let created_at_str: String = row.get(10)?;
    let expires_at_str: String = row.get(11)?;
    let published_as_version_id: Option<String> = row.get(12)?;
    let published_by: Option<String> = row.get(13)?;
    let published_at_str: Option<String> = row.get(14)?;
    let locked_by: Option<String> = row.get(15)?;
    let locked_at_str: Option<String> = row.get(16)?;
    let summary_json: String = row.get(17)?;
    let diff_items_json: String = row.get(18)?;
    let diff_items_total: i64 = row.get(19)?;
    let diff_items_truncated_int: i64 = row.get(20)?;

    let plan_date_from =
        NaiveDate::parse_from_str(&plan_date_from_str, "%Y-%m-%d").map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
        })?;
    let plan_date_to = NaiveDate::parse_from_str(&plan_date_to_str, "%Y-%m-%d").map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let created_at =
        NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S").map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(10, rusqlite::types::Type::Text, Box::new(e))
        })?;
    let expires_at =
        NaiveDateTime::parse_from_str(&expires_at_str, "%Y-%m-%d %H:%M:%S").map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(11, rusqlite::types::Type::Text, Box::new(e))
        })?;

    let published_at =
        published_at_str.and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok());
    let locked_at =
        locked_at_str.and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok());

    Ok(StrategyDraftEntity {
        draft_id,
        base_version_id,
        plan_date_from,
        plan_date_to,
        strategy_key,
        strategy_base,
        strategy_title_cn,
        strategy_params_json,
        status: StrategyDraftStatus::parse(&status_str),
        created_by,
        created_at,
        expires_at,
        published_as_version_id,
        published_by,
        published_at,
        locked_by,
        locked_at,
        summary_json,
        diff_items_json,
        diff_items_total,
        diff_items_truncated: diff_items_truncated_int != 0,
    })
}

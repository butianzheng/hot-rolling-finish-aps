// ==========================================
// 热轧精整排产系统 - 路径规则待确认仓储 (v0.6+)
// ==========================================
// 职责:
// 1) 持久化“路径规则人工确认”待办（由重算生成）
// 2) 提供跨日期/跨机组的待确认汇总查询
// 3) 提供批量确认前的 material_id 拉取
//
// 说明:
// - 表设计为“每个 material 在每个版本+机组仅记录一次”，plan_date 表示首次遇到 OVERRIDE_REQUIRED 的日期
// - 确认状态不在本表存储，查询时通过 material_state.user_confirmed=0 过滤“未确认”
// ==========================================

use std::sync::{Arc, Mutex};

use chrono::NaiveDate;
use rusqlite::{params, params_from_iter, Connection};
use rusqlite::types::Value;

use crate::repository::error::{RepositoryError, RepositoryResult};

#[derive(Debug, Clone)]
pub struct PathOverridePendingRecord {
    pub version_id: String,
    pub machine_code: String,
    pub plan_date: NaiveDate,
    pub material_id: String,
    pub violation_type: String,
    pub urgent_level: String,
    pub width_mm: f64,
    pub thickness_mm: f64,
    pub anchor_width_mm: f64,
    pub anchor_thickness_mm: f64,
    pub width_delta_mm: f64,
    pub thickness_delta_mm: f64,
}

#[derive(Debug, Clone)]
pub struct PathOverridePendingSummaryRow {
    pub machine_code: String,
    pub plan_date: NaiveDate,
    pub pending_count: i32,
}

pub struct PathOverridePendingRepository {
    conn: Arc<Mutex<Connection>>,
}

impl PathOverridePendingRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    fn get_conn(&self) -> RepositoryResult<std::sync::MutexGuard<Connection>> {
        self.conn
            .lock()
            .map_err(|e| RepositoryError::LockError(e.to_string()))
    }

    fn has_material_state_reject_column(conn: &Connection) -> RepositoryResult<bool> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('material_state') WHERE name = 'path_override_rejected'",
            [],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn ensure_schema(&self) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS path_override_pending (
              version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
              machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
              plan_date TEXT NOT NULL,
              material_id TEXT NOT NULL REFERENCES material_master(material_id),
              violation_type TEXT NOT NULL,
              urgent_level TEXT NOT NULL,
              width_mm REAL NOT NULL,
              thickness_mm REAL NOT NULL,
              anchor_width_mm REAL NOT NULL,
              anchor_thickness_mm REAL NOT NULL,
              width_delta_mm REAL NOT NULL,
              thickness_delta_mm REAL NOT NULL,
              created_at TEXT NOT NULL DEFAULT (datetime('now')),
              PRIMARY KEY (version_id, machine_code, material_id)
            );

            CREATE INDEX IF NOT EXISTS idx_path_override_pending_version_date_machine
              ON path_override_pending(version_id, plan_date, machine_code);
            CREATE INDEX IF NOT EXISTS idx_path_override_pending_version_machine_date
              ON path_override_pending(version_id, machine_code, plan_date);
            CREATE INDEX IF NOT EXISTS idx_path_override_pending_version_material
              ON path_override_pending(version_id, material_id);
            "#,
        )?;
        Ok(())
    }

    pub fn delete_by_version(&self, version_id: &str) -> RepositoryResult<usize> {
        self.ensure_schema()?;
        let conn = self.get_conn()?;
        let changed = conn.execute(
            "DELETE FROM path_override_pending WHERE version_id = ?1",
            params![version_id],
        )?;
        Ok(changed)
    }

    pub fn insert_ignore_many(&self, records: &[PathOverridePendingRecord]) -> RepositoryResult<usize> {
        if records.is_empty() {
            return Ok(0);
        }
        self.ensure_schema()?;
        let mut conn = self.get_conn()?;
        let tx = conn.transaction()?;

        let mut inserted = 0usize;
        for r in records {
            let changed = tx.execute(
                r#"
                INSERT OR IGNORE INTO path_override_pending (
                  version_id, machine_code, plan_date, material_id,
                  violation_type, urgent_level,
                  width_mm, thickness_mm,
                  anchor_width_mm, anchor_thickness_mm,
                  width_delta_mm, thickness_delta_mm,
                  created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, datetime('now'))
                "#,
                params![
                    &r.version_id,
                    &r.machine_code,
                    &r.plan_date.format("%Y-%m-%d").to_string(),
                    &r.material_id,
                    &r.violation_type,
                    &r.urgent_level,
                    &r.width_mm,
                    &r.thickness_mm,
                    &r.anchor_width_mm,
                    &r.anchor_thickness_mm,
                    &r.width_delta_mm,
                    &r.thickness_delta_mm,
                ],
            )?;
            inserted += changed;
        }

        tx.commit()?;
        Ok(inserted)
    }

    pub fn list_summary(
        &self,
        version_id: &str,
        date_from: NaiveDate,
        date_to: NaiveDate,
        machine_codes: Option<&[String]>,
    ) -> RepositoryResult<Vec<PathOverridePendingSummaryRow>> {
        self.ensure_schema()?;
        let conn = self.get_conn()?;
        let has_reject_column = Self::has_material_state_reject_column(&conn)?;

        let mut sql = String::from(
            r#"
            SELECT p.machine_code, p.plan_date, COUNT(*) AS pending_count
            FROM path_override_pending p
            JOIN material_state s ON p.material_id = s.material_id
            WHERE p.version_id = ?1
              AND p.plan_date >= ?2
              AND p.plan_date <= ?3
              AND s.user_confirmed = 0
            "#,
        );
        if has_reject_column {
            sql.push_str("\n              AND COALESCE(s.path_override_rejected, 0) = 0\n");
        }

        let mut params: Vec<Value> = vec![
            Value::from(version_id.to_string()),
            Value::from(date_from.format("%Y-%m-%d").to_string()),
            Value::from(date_to.format("%Y-%m-%d").to_string()),
        ];

        if let Some(codes) = machine_codes {
            let codes: Vec<String> = codes
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !codes.is_empty() {
                let placeholders = std::iter::repeat("?")
                    .take(codes.len())
                    .collect::<Vec<_>>()
                    .join(", ");
                sql.push_str(&format!(" AND p.machine_code IN ({})", placeholders));
                for c in codes {
                    params.push(Value::from(c));
                }
            }
        }

        sql.push_str(" GROUP BY p.machine_code, p.plan_date ORDER BY p.plan_date, p.machine_code");

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params_from_iter(params.iter()), |row| {
                let date_str: String = row.get(1)?;
                let plan_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e)))?;
                Ok(PathOverridePendingSummaryRow {
                    machine_code: row.get(0)?,
                    plan_date,
                    pending_count: row.get::<_, i64>(2)? as i32,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn list_pending_material_ids_by_range(
        &self,
        version_id: &str,
        date_from: NaiveDate,
        date_to: NaiveDate,
        machine_codes: Option<&[String]>,
    ) -> RepositoryResult<Vec<String>> {
        self.ensure_schema()?;
        let conn = self.get_conn()?;
        let has_reject_column = Self::has_material_state_reject_column(&conn)?;

        let mut sql = String::from(
            r#"
            SELECT DISTINCT p.material_id
            FROM path_override_pending p
            JOIN material_state s ON p.material_id = s.material_id
            WHERE p.version_id = ?1
              AND p.plan_date >= ?2
              AND p.plan_date <= ?3
              AND s.user_confirmed = 0
            "#,
        );
        if has_reject_column {
            sql.push_str("\n              AND COALESCE(s.path_override_rejected, 0) = 0\n");
        }
        let mut params: Vec<Value> = vec![
            Value::from(version_id.to_string()),
            Value::from(date_from.format("%Y-%m-%d").to_string()),
            Value::from(date_to.format("%Y-%m-%d").to_string()),
        ];

        if let Some(codes) = machine_codes {
            let codes: Vec<String> = codes
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !codes.is_empty() {
                let placeholders = std::iter::repeat("?")
                    .take(codes.len())
                    .collect::<Vec<_>>()
                    .join(", ");
                sql.push_str(&format!(" AND p.machine_code IN ({})", placeholders));
                for c in codes {
                    params.push(Value::from(c));
                }
            }
        }

        sql.push_str(" ORDER BY p.material_id");

        let mut stmt = conn.prepare(&sql)?;
        let ids = stmt
            .query_map(params_from_iter(params.iter()), |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    }

    pub fn list_details(
        &self,
        version_id: &str,
        machine_code: &str,
        plan_date: NaiveDate,
    ) -> RepositoryResult<Vec<PathOverridePendingRecord>> {
        self.ensure_schema()?;
        let conn = self.get_conn()?;
        let has_reject_column = Self::has_material_state_reject_column(&conn)?;

        let date_str = plan_date.format("%Y-%m-%d").to_string();
        let mut sql = String::from(
            r#"
            SELECT
              p.version_id, p.machine_code, p.plan_date, p.material_id,
              p.violation_type, p.urgent_level,
              p.width_mm, p.thickness_mm,
              p.anchor_width_mm, p.anchor_thickness_mm,
              p.width_delta_mm, p.thickness_delta_mm
            FROM path_override_pending p
            JOIN material_state s ON p.material_id = s.material_id
            WHERE p.version_id = ?1
              AND p.machine_code = ?2
              AND p.plan_date = ?3
              AND s.user_confirmed = 0
            "#,
        );
        if has_reject_column {
            sql.push_str("\n              AND COALESCE(s.path_override_rejected, 0) = 0\n");
        }
        sql.push_str(" ORDER BY p.urgent_level DESC, p.material_id ASC");
        let mut stmt = conn.prepare(&sql)?;

        let rows = stmt
            .query_map(params![version_id, machine_code, date_str], |row| {
                let plan_date_str: String = row.get(2)?;
                let plan_date = NaiveDate::parse_from_str(&plan_date_str, "%Y-%m-%d")
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e)))?;
                Ok(PathOverridePendingRecord {
                    version_id: row.get(0)?,
                    machine_code: row.get(1)?,
                    plan_date,
                    material_id: row.get(3)?,
                    violation_type: row.get(4)?,
                    urgent_level: row.get(5)?,
                    width_mm: row.get(6)?,
                    thickness_mm: row.get(7)?,
                    anchor_width_mm: row.get(8)?,
                    anchor_thickness_mm: row.get(9)?,
                    width_delta_mm: row.get(10)?,
                    thickness_delta_mm: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn find_by_key(
        &self,
        version_id: &str,
        machine_code: &str,
        material_id: &str,
    ) -> RepositoryResult<Option<PathOverridePendingRecord>> {
        self.ensure_schema()?;
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT
              p.version_id, p.machine_code, p.plan_date, p.material_id,
              p.violation_type, p.urgent_level,
              p.width_mm, p.thickness_mm,
              p.anchor_width_mm, p.anchor_thickness_mm,
              p.width_delta_mm, p.thickness_delta_mm
            FROM path_override_pending p
            WHERE p.version_id = ?1
              AND p.machine_code = ?2
              AND p.material_id = ?3
            "#,
        )?;

        let mut rows = stmt.query(params![version_id, machine_code, material_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };

        let plan_date_str: String = row.get(2)?;
        let plan_date = NaiveDate::parse_from_str(&plan_date_str, "%Y-%m-%d")
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e)))?;

        Ok(Some(PathOverridePendingRecord {
            version_id: row.get(0)?,
            machine_code: row.get(1)?,
            plan_date,
            material_id: row.get(3)?,
            violation_type: row.get(4)?,
            urgent_level: row.get(5)?,
            width_mm: row.get(6)?,
            thickness_mm: row.get(7)?,
            anchor_width_mm: row.get(8)?,
            anchor_thickness_mm: row.get(9)?,
            width_delta_mm: row.get(10)?,
            thickness_delta_mm: row.get(11)?,
        }))
    }
}

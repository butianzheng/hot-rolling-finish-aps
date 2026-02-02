// ==========================================
// 热轧精整排产系统 - 每日生产节奏仓储
// ==========================================
// 职责:
// - 管理 plan_rhythm_preset / plan_rhythm_target
// - 提供按版本×机组×日期的节奏目标读写
// 说明:
// - 先做“品种大类( PRODUCT_CATEGORY )”维度
// - 目标用于监控/评估，不直接改变排程结果
// ==========================================

use crate::repository::error::{RepositoryError, RepositoryResult};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct PlanRhythmPresetEntity {
    pub preset_id: String,
    pub preset_name: String,
    pub dimension: String,
    pub target_json: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlanRhythmTargetEntity {
    pub version_id: String,
    pub machine_code: String,
    pub plan_date: String, // YYYY-MM-DD
    pub dimension: String,
    pub target_json: String,
    pub preset_id: Option<String>,
    pub updated_at: String,
    pub updated_by: Option<String>,
}

pub struct PlanRhythmRepository {
    conn: Arc<Mutex<Connection>>,
}

impl PlanRhythmRepository {
    pub fn new(db_path: &str) -> RepositoryResult<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let repo = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        repo.ensure_tables()?;
        Ok(repo)
    }

    pub fn from_connection(conn: Arc<Mutex<Connection>>) -> Self {
        let repo = Self { conn };
        let _ = repo.ensure_tables();
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
            CREATE TABLE IF NOT EXISTS plan_rhythm_preset (
              preset_id TEXT PRIMARY KEY,
              preset_name TEXT NOT NULL,
              dimension TEXT NOT NULL,
              target_json TEXT NOT NULL,
              is_active INTEGER NOT NULL DEFAULT 1,
              created_at TEXT NOT NULL DEFAULT (datetime('now')),
              updated_at TEXT NOT NULL DEFAULT (datetime('now')),
              updated_by TEXT
            );

            CREATE TABLE IF NOT EXISTS plan_rhythm_target (
              version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
              machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
              plan_date TEXT NOT NULL,
              dimension TEXT NOT NULL,
              target_json TEXT NOT NULL,
              preset_id TEXT REFERENCES plan_rhythm_preset(preset_id),
              updated_at TEXT NOT NULL DEFAULT (datetime('now')),
              updated_by TEXT,
              PRIMARY KEY (version_id, machine_code, plan_date, dimension)
            );

            CREATE INDEX IF NOT EXISTS idx_plan_rhythm_target_version_machine_date
              ON plan_rhythm_target(version_id, machine_code, plan_date);
            "#,
        )?;

        // Best-effort schema migration for older DBs:
        // - material_master.product_category: used by rhythm monitoring; fallback to steel_mark when absent.
        Self::ensure_material_master_product_category_column(&conn)?;
        Ok(())
    }

    fn ensure_material_master_product_category_column(conn: &Connection) -> RepositoryResult<()> {
        let material_master_exists: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'material_master'",
            [],
            |row| row.get(0),
        )?;
        if material_master_exists <= 0 {
            // Some test/minimal DBs may not contain this table.
            // Rhythm monitoring will fallback to other dimensions when querying.
            return Ok(());
        }

        let has_col: i32 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('material_master') WHERE name = 'product_category'",
            [],
            |row| row.get(0),
        )?;
        if has_col > 0 {
            return Ok(());
        }

        // SQLite supports ADD COLUMN; keep nullable so existing writers remain compatible.
        // Note: if material_master doesn't exist, this will error; that's acceptable and will be surfaced.
        conn.execute_batch("ALTER TABLE material_master ADD COLUMN product_category TEXT;")?;
        Ok(())
    }

    pub fn list_presets(
        &self,
        dimension: Option<&str>,
        active_only: bool,
    ) -> RepositoryResult<Vec<PlanRhythmPresetEntity>> {
        let conn = self.get_conn()?;
        let mut sql = r#"
            SELECT
                preset_id,
                preset_name,
                dimension,
                target_json,
                is_active,
                created_at,
                updated_at,
                updated_by
            FROM plan_rhythm_preset
        "#
        .to_string();

        let mut clauses: Vec<&str> = Vec::new();
        if let Some(_) = dimension {
            clauses.push("dimension = ?1");
        }
        if active_only {
            clauses.push("is_active = 1");
        }
        if !clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&clauses.join(" AND "));
        }
        sql.push_str(" ORDER BY preset_name ASC");

        let mut stmt = conn.prepare(&sql)?;
        let rows = if let Some(dim) = dimension {
            stmt.query_map(params![dim], |row| {
                Ok(PlanRhythmPresetEntity {
                    preset_id: row.get(0)?,
                    preset_name: row.get(1)?,
                    dimension: row.get(2)?,
                    target_json: row.get(3)?,
                    is_active: row.get::<_, i32>(4)? != 0,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    updated_by: row.get(7)?,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?
        } else {
            stmt.query_map([], |row| {
                Ok(PlanRhythmPresetEntity {
                    preset_id: row.get(0)?,
                    preset_name: row.get(1)?,
                    dimension: row.get(2)?,
                    target_json: row.get(3)?,
                    is_active: row.get::<_, i32>(4)? != 0,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    updated_by: row.get(7)?,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?
        };

        Ok(rows)
    }

    pub fn find_preset_by_id(
        &self,
        preset_id: &str,
    ) -> RepositoryResult<Option<PlanRhythmPresetEntity>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                preset_id,
                preset_name,
                dimension,
                target_json,
                is_active,
                created_at,
                updated_at,
                updated_by
            FROM plan_rhythm_preset
            WHERE preset_id = ?1
            "#,
        )?;

        let result = stmt.query_row(params![preset_id], |row| {
            Ok(PlanRhythmPresetEntity {
                preset_id: row.get(0)?,
                preset_name: row.get(1)?,
                dimension: row.get(2)?,
                target_json: row.get(3)?,
                is_active: row.get::<_, i32>(4)? != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                updated_by: row.get(7)?,
            })
        });

        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn upsert_preset(&self, entity: &PlanRhythmPresetEntity) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute(
            r#"
            INSERT INTO plan_rhythm_preset (
                preset_id,
                preset_name,
                dimension,
                target_json,
                is_active,
                created_at,
                updated_at,
                updated_by
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(preset_id) DO UPDATE SET
                preset_name = excluded.preset_name,
                dimension = excluded.dimension,
                target_json = excluded.target_json,
                is_active = excluded.is_active,
                updated_at = excluded.updated_at,
                updated_by = excluded.updated_by
            "#,
            params![
                entity.preset_id,
                entity.preset_name,
                entity.dimension,
                entity.target_json,
                if entity.is_active { 1 } else { 0 },
                entity.created_at,
                entity.updated_at,
                entity.updated_by,
            ],
        )?;
        Ok(())
    }

    pub fn set_preset_active(
        &self,
        preset_id: &str,
        is_active: bool,
        updated_at: &str,
        updated_by: Option<&str>,
    ) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let affected = conn.execute(
            r#"
            UPDATE plan_rhythm_preset
            SET is_active = ?2, updated_at = ?3, updated_by = ?4
            WHERE preset_id = ?1
            "#,
            params![preset_id, if is_active { 1 } else { 0 }, updated_at, updated_by],
        )?;
        Ok(affected)
    }

    pub fn list_targets(
        &self,
        version_id: &str,
        dimension: &str,
        machine_codes: Option<&[String]>,
        date_range: Option<(&str, &str)>,
    ) -> RepositoryResult<Vec<PlanRhythmTargetEntity>> {
        let conn = self.get_conn()?;

        let mut sql = r#"
            SELECT
                version_id,
                machine_code,
                plan_date,
                dimension,
                target_json,
                preset_id,
                updated_at,
                updated_by
            FROM plan_rhythm_target
            WHERE version_id = ?1 AND dimension = ?2
        "#
        .to_string();

        let mut params_vec: Vec<String> = vec![version_id.to_string(), dimension.to_string()];

        if let Some((start, end)) = date_range {
            let start_idx = params_vec.len() + 1;
            let end_idx = params_vec.len() + 2;
            sql.push_str(&format!(" AND plan_date BETWEEN ?{} AND ?{}", start_idx, end_idx));
            params_vec.push(start.to_string());
            params_vec.push(end.to_string());
        }

        if let Some(codes) = machine_codes {
            if !codes.is_empty() {
                let placeholders: Vec<String> = (0..codes.len())
                    .map(|i| format!("?{}", params_vec.len() + i + 1))
                    .collect();
                sql.push_str(&format!(" AND machine_code IN ({})", placeholders.join(", ")));
                params_vec.extend(codes.iter().cloned());
            }
        }

        sql.push_str(" ORDER BY machine_code ASC, plan_date ASC");

        let params_refs: Vec<&str> = params_vec.iter().map(|s| s.as_str()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params_refs.iter()), |row| {
                Ok(PlanRhythmTargetEntity {
                    version_id: row.get(0)?,
                    machine_code: row.get(1)?,
                    plan_date: row.get(2)?,
                    dimension: row.get(3)?,
                    target_json: row.get(4)?,
                    preset_id: row.get(5)?,
                    updated_at: row.get(6)?,
                    updated_by: row.get(7)?,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(rows)
    }

    pub fn find_target(
        &self,
        version_id: &str,
        machine_code: &str,
        plan_date: &str,
        dimension: &str,
    ) -> RepositoryResult<Option<PlanRhythmTargetEntity>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id,
                machine_code,
                plan_date,
                dimension,
                target_json,
                preset_id,
                updated_at,
                updated_by
            FROM plan_rhythm_target
            WHERE version_id = ?1 AND machine_code = ?2 AND plan_date = ?3 AND dimension = ?4
            "#,
        )?;

        let result = stmt.query_row(params![version_id, machine_code, plan_date, dimension], |row| {
            Ok(PlanRhythmTargetEntity {
                version_id: row.get(0)?,
                machine_code: row.get(1)?,
                plan_date: row.get(2)?,
                dimension: row.get(3)?,
                target_json: row.get(4)?,
                preset_id: row.get(5)?,
                updated_at: row.get(6)?,
                updated_by: row.get(7)?,
            })
        });

        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn upsert_target(&self, entity: &PlanRhythmTargetEntity) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute(
            r#"
            INSERT INTO plan_rhythm_target (
                version_id,
                machine_code,
                plan_date,
                dimension,
                target_json,
                preset_id,
                updated_at,
                updated_by
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(version_id, machine_code, plan_date, dimension) DO UPDATE SET
                target_json = excluded.target_json,
                preset_id = excluded.preset_id,
                updated_at = excluded.updated_at,
                updated_by = excluded.updated_by
            "#,
            params![
                entity.version_id,
                entity.machine_code,
                entity.plan_date,
                entity.dimension,
                entity.target_json,
                entity.preset_id,
                entity.updated_at,
                entity.updated_by,
            ],
        )?;
        Ok(())
    }

    pub fn batch_upsert_targets(&self, entities: &[PlanRhythmTargetEntity]) -> RepositoryResult<usize> {
        if entities.is_empty() {
            return Ok(0);
        }
        let conn = self.get_conn()?;
        let tx = conn.unchecked_transaction()?;
        let mut count = 0usize;
        for entity in entities {
            tx.execute(
                r#"
                INSERT INTO plan_rhythm_target (
                    version_id,
                    machine_code,
                    plan_date,
                    dimension,
                    target_json,
                    preset_id,
                    updated_at,
                    updated_by
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(version_id, machine_code, plan_date, dimension) DO UPDATE SET
                    target_json = excluded.target_json,
                    preset_id = excluded.preset_id,
                    updated_at = excluded.updated_at,
                    updated_by = excluded.updated_by
                "#,
                params![
                    entity.version_id,
                    entity.machine_code,
                    entity.plan_date,
                    entity.dimension,
                    entity.target_json,
                    entity.preset_id,
                    entity.updated_at,
                    entity.updated_by,
                ],
            )?;
            count += 1;
        }
        tx.commit()?;
        Ok(count)
    }

    fn has_material_product_category_column(conn: &Connection) -> RepositoryResult<bool> {
        let has_product_category: i32 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('material_master') WHERE name = 'product_category'",
            [],
            |row| row.get(0),
        )?;
        Ok(has_product_category > 0)
    }

    pub fn get_scheduled_weights_by_category(
        &self,
        version_id: &str,
        machine_code: &str,
        plan_date: &str,
    ) -> RepositoryResult<HashMap<String, f64>> {
        let conn = self.get_conn()?;

        let has_product_category = Self::has_material_product_category_column(&conn)?;
        let category_expr = if has_product_category {
            "COALESCE(mm.product_category, '未分类')"
        } else {
            "COALESCE(mm.steel_mark, '未分类')"
        };

        let sql = format!(
            r#"
            SELECT
                {category_expr} AS category,
                COALESCE(SUM(pi.weight_t), 0) AS total_weight_t
            FROM plan_item pi
            JOIN material_master mm ON mm.material_id = pi.material_id
            WHERE pi.version_id = ?1
              AND pi.machine_code = ?2
              AND pi.plan_date = ?3
            GROUP BY category
            "#,
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![version_id, machine_code, plan_date], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        let mut out: HashMap<String, f64> = HashMap::new();
        for (cat, w) in rows {
            out.insert(cat, w);
        }
        Ok(out)
    }
}

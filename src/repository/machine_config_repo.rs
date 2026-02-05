// ==========================================
// 热轧精整排产系统 - 机组产能配置仓储
// ==========================================
// 职责: 管理 machine_capacity_config 表 (按版本+机组)
// 说明: 用于存储版本化的机组级产能配置，支持历史记录追踪
// ==========================================

use crate::db::open_sqlite_connection;
use crate::repository::error::{RepositoryError, RepositoryResult};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// 机组产能配置实体
#[derive(Debug, Clone)]
pub struct MachineConfigEntity {
    pub config_id: String,                // 配置ID (UUID)
    pub version_id: String,               // 版本ID
    pub machine_code: String,             // 机组代码
    pub default_daily_target_t: f64,      // 机组级默认目标产能(吨/天)
    pub default_daily_limit_pct: f64,     // 机组级默认极限产能百分比 (如 1.05 表示 105%)
    pub effective_date: Option<String>,   // 生效日期(可选, ISO DATE格式 YYYY-MM-DD)
    pub created_at: String,               // 创建时间
    pub updated_at: String,               // 更新时间
    pub created_by: String,               // 创建人
    pub reason: Option<String>,           // 配置原因/备注
}

impl MachineConfigEntity {
    /// 创建新的配置实体（自动生成 UUID 和时间戳）
    pub fn new(
        version_id: String,
        machine_code: String,
        default_daily_target_t: f64,
        default_daily_limit_pct: f64,
        created_by: String,
        reason: Option<String>,
        effective_date: Option<String>,
    ) -> Self {
        let now = chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        Self {
            config_id: Uuid::new_v4().to_string(),
            version_id,
            machine_code,
            default_daily_target_t,
            default_daily_limit_pct,
            effective_date,
            created_at: now.clone(),
            updated_at: now,
            created_by,
            reason,
        }
    }
}

pub struct MachineConfigRepository {
    conn: Arc<Mutex<Connection>>,
}

impl MachineConfigRepository {
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

    /// 确保表存在（如果不存在则创建）
    fn ensure_table(&self) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS machine_capacity_config (
              config_id TEXT PRIMARY KEY,
              version_id TEXT NOT NULL,
              machine_code TEXT NOT NULL,
              default_daily_target_t REAL NOT NULL,
              default_daily_limit_pct REAL NOT NULL,
              effective_date TEXT,
              created_at TEXT NOT NULL DEFAULT (datetime('now')),
              updated_at TEXT NOT NULL DEFAULT (datetime('now')),
              created_by TEXT NOT NULL,
              reason TEXT,
              FOREIGN KEY (version_id) REFERENCES plan_version(version_id) ON DELETE CASCADE,
              UNIQUE(version_id, machine_code)
            );

            CREATE INDEX IF NOT EXISTS idx_machine_config_version
              ON machine_capacity_config(version_id);
            CREATE INDEX IF NOT EXISTS idx_machine_config_machine
              ON machine_capacity_config(machine_code);
            CREATE INDEX IF NOT EXISTS idx_machine_config_created_at
              ON machine_capacity_config(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_machine_config_version_machine_date
              ON machine_capacity_config(version_id, machine_code, effective_date);
            "#,
        )?;
        Ok(())
    }

    /// 创建或更新配置（Upsert 操作）
    /// 如果 (version_id, machine_code) 已存在，则更新；否则插入
    pub fn upsert(&self, entity: &MachineConfigEntity) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute(
            r#"
            INSERT INTO machine_capacity_config (
                config_id,
                version_id,
                machine_code,
                default_daily_target_t,
                default_daily_limit_pct,
                effective_date,
                created_at,
                updated_at,
                created_by,
                reason
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(version_id, machine_code) DO UPDATE SET
                default_daily_target_t = excluded.default_daily_target_t,
                default_daily_limit_pct = excluded.default_daily_limit_pct,
                effective_date = excluded.effective_date,
                updated_at = excluded.updated_at,
                created_by = excluded.created_by,
                reason = excluded.reason
            "#,
            params![
                entity.config_id,
                entity.version_id,
                entity.machine_code,
                entity.default_daily_target_t,
                entity.default_daily_limit_pct,
                entity.effective_date,
                entity.created_at,
                entity.updated_at,
                entity.created_by,
                entity.reason,
            ],
        )?;
        Ok(())
    }

    /// 按主键查找配置
    pub fn find_by_key(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> RepositoryResult<Option<MachineConfigEntity>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                config_id,
                version_id,
                machine_code,
                default_daily_target_t,
                default_daily_limit_pct,
                effective_date,
                created_at,
                updated_at,
                created_by,
                reason
            FROM machine_capacity_config
            WHERE version_id = ?1 AND machine_code = ?2
            "#,
        )?;

        let result = stmt.query_row(params![version_id, machine_code], |row| {
            Ok(MachineConfigEntity {
                config_id: row.get(0)?,
                version_id: row.get(1)?,
                machine_code: row.get(2)?,
                default_daily_target_t: row.get(3)?,
                default_daily_limit_pct: row.get(4)?,
                effective_date: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                created_by: row.get(8)?,
                reason: row.get(9)?,
            })
        });

        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 列出某版本下所有配置（按机组代码排序）
    pub fn list_by_version_id(&self, version_id: &str) -> RepositoryResult<Vec<MachineConfigEntity>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                config_id,
                version_id,
                machine_code,
                default_daily_target_t,
                default_daily_limit_pct,
                effective_date,
                created_at,
                updated_at,
                created_by,
                reason
            FROM machine_capacity_config
            WHERE version_id = ?1
            ORDER BY machine_code ASC
            "#,
        )?;

        let rows = stmt
            .query_map(params![version_id], |row| {
                Ok(MachineConfigEntity {
                    config_id: row.get(0)?,
                    version_id: row.get(1)?,
                    machine_code: row.get(2)?,
                    default_daily_target_t: row.get(3)?,
                    default_daily_limit_pct: row.get(4)?,
                    effective_date: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    created_by: row.get(8)?,
                    reason: row.get(9)?,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(rows)
    }

    /// 列出某机组的历史配置（按创建时间倒序，用于查看配置历史）
    /// 此方法跨版本查询，用于审计和历史追踪
    pub fn list_history_by_machine(
        &self,
        machine_code: &str,
        limit: Option<usize>,
    ) -> RepositoryResult<Vec<MachineConfigEntity>> {
        let conn = self.get_conn()?;

        let sql = match limit {
            Some(n) => format!(
                r#"
                SELECT
                    config_id,
                    version_id,
                    machine_code,
                    default_daily_target_t,
                    default_daily_limit_pct,
                    effective_date,
                    created_at,
                    updated_at,
                    created_by,
                    reason
                FROM machine_capacity_config
                WHERE machine_code = ?1
                ORDER BY created_at DESC
                LIMIT {}
                "#,
                n
            ),
            None => r#"
                SELECT
                    config_id,
                    version_id,
                    machine_code,
                    default_daily_target_t,
                    default_daily_limit_pct,
                    effective_date,
                    created_at,
                    updated_at,
                    created_by,
                    reason
                FROM machine_capacity_config
                WHERE machine_code = ?1
                ORDER BY created_at DESC
                "#
            .to_string(),
        };

        let mut stmt = conn.prepare(&sql)?;

        let rows = stmt
            .query_map(params![machine_code], |row| {
                Ok(MachineConfigEntity {
                    config_id: row.get(0)?,
                    version_id: row.get(1)?,
                    machine_code: row.get(2)?,
                    default_daily_target_t: row.get(3)?,
                    default_daily_limit_pct: row.get(4)?,
                    effective_date: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    created_by: row.get(8)?,
                    reason: row.get(9)?,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(rows)
    }

    /// 按配置ID删除（谨慎使用，一般不需要删除）
    pub fn delete_by_config_id(&self, config_id: &str) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let affected = conn.execute(
            "DELETE FROM machine_capacity_config WHERE config_id = ?1",
            params![config_id],
        )?;
        Ok(affected)
    }

    /// 删除某版本下的所有配置（一般在删除版本时级联删除）
    pub fn delete_by_version_id(&self, version_id: &str) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let affected = conn.execute(
            "DELETE FROM machine_capacity_config WHERE version_id = ?1",
            params![version_id],
        )?;
        Ok(affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_repo() -> MachineConfigRepository {
        let repo = MachineConfigRepository::new(":memory:").expect("Failed to create test repository");

        // 创建必需的 plan_version 表（外键依赖）
        let conn = repo.conn.lock().expect("Failed to lock connection");
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS plan_version (
              version_id TEXT PRIMARY KEY,
              plan_id TEXT NOT NULL,
              version_number INTEGER NOT NULL,
              status TEXT NOT NULL,
              created_by TEXT NOT NULL,
              created_at TEXT NOT NULL DEFAULT (datetime('now')),
              updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- 插入测试版本
            INSERT INTO plan_version (version_id, plan_id, version_number, status, created_by)
            VALUES ('v1', 'plan1', 1, 'ACTIVE', 'test_user');
            INSERT INTO plan_version (version_id, plan_id, version_number, status, created_by)
            VALUES ('v2', 'plan1', 2, 'ACTIVE', 'test_user');
            INSERT INTO plan_version (version_id, plan_id, version_number, status, created_by)
            VALUES ('v3', 'plan1', 3, 'ACTIVE', 'test_user');
            "#,
        ).expect("Failed to create plan_version table");
        drop(conn);

        repo
    }

    #[test]
    fn test_upsert_and_find() {
        let repo = setup_test_repo();

        let entity = MachineConfigEntity::new(
            "v1".to_string(),
            "H031".to_string(),
            1200.0,
            1.05,
            "test_user".to_string(),
            Some("初始配置".to_string()),
            Some("2026-02-01".to_string()),
        );

        // 插入
        repo.upsert(&entity).expect("Failed to upsert");

        // 查询
        let found = repo
            .find_by_key("v1", "H031")
            .expect("Failed to find")
            .expect("Entity not found");

        assert_eq!(found.version_id, "v1");
        assert_eq!(found.machine_code, "H031");
        assert_eq!(found.default_daily_target_t, 1200.0);
        assert_eq!(found.default_daily_limit_pct, 1.05);
    }

    #[test]
    fn test_upsert_conflict_update() {
        let repo = setup_test_repo();

        // 第一次插入
        let entity1 = MachineConfigEntity::new(
            "v1".to_string(),
            "H031".to_string(),
            1200.0,
            1.05,
            "user1".to_string(),
            Some("初始配置".to_string()),
            None,
        );
        repo.upsert(&entity1).expect("Failed to upsert 1");

        // 第二次插入（冲突，应该更新）
        let mut entity2 = entity1.clone();
        entity2.default_daily_target_t = 1300.0;
        entity2.default_daily_limit_pct = 1.08;
        entity2.created_by = "user2".to_string();
        entity2.reason = Some("调整产能".to_string());

        repo.upsert(&entity2).expect("Failed to upsert 2");

        // 查询验证
        let found = repo
            .find_by_key("v1", "H031")
            .expect("Failed to find")
            .expect("Entity not found");

        assert_eq!(found.default_daily_target_t, 1300.0);
        assert_eq!(found.default_daily_limit_pct, 1.08);
        assert_eq!(found.created_by, "user2");
    }

    #[test]
    fn test_list_by_version_id() {
        let repo = setup_test_repo();

        // 插入3条不同机组的配置
        for (idx, machine) in ["H031", "H032", "H033"].iter().enumerate() {
            let entity = MachineConfigEntity::new(
                "v1".to_string(),
                machine.to_string(),
                1200.0 + (idx as f64 * 100.0),
                1.05,
                "test_user".to_string(),
                None,
                None,
            );
            repo.upsert(&entity).expect("Failed to upsert");
        }

        // 查询
        let configs = repo.list_by_version_id("v1").expect("Failed to list");

        assert_eq!(configs.len(), 3);
        assert_eq!(configs[0].machine_code, "H031");
        assert_eq!(configs[1].machine_code, "H032");
        assert_eq!(configs[2].machine_code, "H033");
    }

    #[test]
    fn test_list_history_by_machine() {
        let repo = setup_test_repo();

        // 插入同一机组的多个版本配置
        for (idx, version) in ["v1", "v2", "v3"].iter().enumerate() {
            let entity = MachineConfigEntity::new(
                version.to_string(),
                "H031".to_string(),
                1200.0 + (idx as f64 * 100.0),
                1.05 + (idx as f64 * 0.01),
                "test_user".to_string(),
                Some(format!("版本{}", idx + 1)),
                None,
            );
            repo.upsert(&entity).expect("Failed to upsert");
        }

        // 查询历史（不限制条数）
        let history = repo
            .list_history_by_machine("H031", None)
            .expect("Failed to list history");

        assert_eq!(history.len(), 3);
        // 应该按时间倒序，但由于测试中创建时间可能相同，只检查数量

        // 查询历史（限制2条）
        let history_limited = repo
            .list_history_by_machine("H031", Some(2))
            .expect("Failed to list history");

        assert_eq!(history_limited.len(), 2);
    }

    #[test]
    fn test_delete_by_version_id() {
        let repo = setup_test_repo();

        // 插入配置
        let entity = MachineConfigEntity::new(
            "v1".to_string(),
            "H031".to_string(),
            1200.0,
            1.05,
            "test_user".to_string(),
            None,
            None,
        );
        repo.upsert(&entity).expect("Failed to upsert");

        // 删除
        let affected = repo.delete_by_version_id("v1").expect("Failed to delete");
        assert_eq!(affected, 1);

        // 验证已删除
        let found = repo.find_by_key("v1", "H031").expect("Failed to find");
        assert!(found.is_none());
    }
}

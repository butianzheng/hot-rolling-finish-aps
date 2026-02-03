use crate::domain::plan::PlanVersion;
use crate::domain::types::PlanVersionStatus;
use crate::repository::error::{RepositoryError, RepositoryResult};
use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

// ==========================================
// PlanVersionRepository - 方案版本仓储
// ==========================================
pub struct PlanVersionRepository {
    conn: Arc<Mutex<Connection>>,
}

impl PlanVersionRepository {
    /// 创建新的PlanVersionRepository实例
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 获取数据库连接
    fn get_conn(&self) -> RepositoryResult<std::sync::MutexGuard<Connection>> {
        self.conn
            .lock()
            .map_err(|e| RepositoryError::LockError(e.to_string()))
    }

    /// 创建版本
    pub fn create(&self, version: &PlanVersion) -> RepositoryResult<String> {
        let conn = self.get_conn()?;

        conn.execute(
            r#"INSERT INTO plan_version (
                version_id, plan_id, version_no, status,
                frozen_from_date, recalc_window_days, config_snapshot_json,
                created_by, created_at, revision
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            params![
                &version.version_id,
                &version.plan_id,
                &version.version_no,
                version.status.to_db_str(),
                &version.frozen_from_date.map(|d| d.format("%Y-%m-%d").to_string()),
                &version.recalc_window_days,
                &version.config_snapshot_json,
                &version.created_by,
                &version.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                &version.revision,
            ],
        )?;

        Ok(version.version_id.clone())
    }

    /// 创建版本（自动分配 version_no，避免并发下 version_no 冲突）
    ///
    /// 说明：
    /// - 在同一事务内查询 MAX(version_no) 并写入，保证对同一 plan_id 的 version_no 分配原子性。
    /// - 该方法会覆盖传入的 `version.version_no`。
    pub fn create_with_next_version_no(&self, version: &mut PlanVersion) -> RepositoryResult<String> {
        let mut conn = self.get_conn()?;
        let tx = conn.transaction()?;

        let max_version_no: Option<i32> = tx.query_row(
            "SELECT MAX(version_no) FROM plan_version WHERE plan_id = ?",
            params![&version.plan_id],
            |row| row.get(0),
        )?;

        version.version_no = max_version_no.unwrap_or(0) + 1;

        tx.execute(
            r#"INSERT INTO plan_version (
                version_id, plan_id, version_no, status,
                frozen_from_date, recalc_window_days, config_snapshot_json,
                created_by, created_at, revision
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            params![
                &version.version_id,
                &version.plan_id,
                &version.version_no,
                version.status.to_db_str(),
                &version.frozen_from_date.map(|d| d.format("%Y-%m-%d").to_string()),
                &version.recalc_window_days,
                &version.config_snapshot_json,
                &version.created_by,
                &version.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                &version.revision,
            ],
        )?;

        tx.commit()?;
        Ok(version.version_id.clone())
    }

    /// 按version_id查询版本
    pub fn find_by_id(&self, version_id: &str) -> RepositoryResult<Option<PlanVersion>> {
        let conn = self.get_conn()?;

        match conn.query_row(
            r#"SELECT version_id, plan_id, version_no, status,
                      frozen_from_date, recalc_window_days, config_snapshot_json,
                      created_by, created_at, revision
               FROM plan_version
               WHERE version_id = ?"#,
            params![version_id],
            |row| self.map_row(row),
        ) {
            Ok(version) => Ok(Some(version)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 查询方案的所有版本
    pub fn find_by_plan_id(&self, plan_id: &str) -> RepositoryResult<Vec<PlanVersion>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"SELECT version_id, plan_id, version_no, status,
                      frozen_from_date, recalc_window_days, config_snapshot_json,
                      created_by, created_at, revision
               FROM plan_version
               WHERE plan_id = ?
               ORDER BY version_no DESC"#,
        )?;

        let versions = stmt
            .query_map(params![plan_id], |row| self.map_row(row))?
            .collect::<Result<Vec<PlanVersion>, _>>()?;

        Ok(versions)
    }

    /// 更新版本
    /// 更新版本 (带乐观锁检查)
    ///
    /// # 并发控制
    /// 使用乐观锁 (revision字段) 防止并发更新冲突
    ///
    /// # 错误
    /// - `RepositoryError::OptimisticLockFailure`: revision不匹配 (其他用户已更新)
    /// - `RepositoryError::NotFound`: version_id不存在
    pub fn update(&self, version: &PlanVersion) -> RepositoryResult<()> {
        let conn = self.get_conn()?;

        // 执行更新，带revision检查
        let rows_affected = conn.execute(
            r#"UPDATE plan_version
               SET status = ?, frozen_from_date = ?, recalc_window_days = ?,
                   config_snapshot_json = ?, revision = revision + 1
               WHERE version_id = ? AND revision = ?"#,
            params![
                version.status.to_db_str(),
                &version.frozen_from_date.map(|d| d.format("%Y-%m-%d").to_string()),
                &version.recalc_window_days,
                &version.config_snapshot_json,
                &version.version_id,
                &version.revision,
            ],
        )?;

        // 检查是否更新成功
        if rows_affected == 0 {
            // 判断是记录不存在还是revision冲突
            let exists: Result<i32, _> = conn.query_row(
                "SELECT revision FROM plan_version WHERE version_id = ?",
                params![&version.version_id],
                |row| row.get(0),
            );

            match exists {
                Ok(actual_revision) => {
                    // 记录存在，但revision不匹配 -> 乐观锁冲突
                    return Err(RepositoryError::OptimisticLockFailure {
                        version_id: version.version_id.clone(),
                        expected: version.revision,
                        actual: actual_revision,
                    });
                }
                Err(_) => {
                    // 记录不存在
                    return Err(RepositoryError::NotFound {
                        entity: "PlanVersion".to_string(),
                        id: version.version_id.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// 删除版本
    pub fn delete(&self, version_id: &str) -> RepositoryResult<()> {
        let conn = self.get_conn()?;

        conn.execute(
            "DELETE FROM plan_version WHERE version_id = ?",
            params![version_id],
        )?;

        Ok(())
    }

    /// 查询激活版本
    pub fn find_active_version(&self, plan_id: &str) -> RepositoryResult<Option<PlanVersion>> {
        let conn = self.get_conn()?;

        match conn.query_row(
            r#"SELECT version_id, plan_id, version_no, status,
                      frozen_from_date, recalc_window_days, config_snapshot_json,
                      created_by, created_at, revision
               FROM plan_version
               WHERE plan_id = ? AND status = 'ACTIVE'"#,
            params![plan_id],
            |row| self.map_row(row),
        ) {
            Ok(version) => Ok(Some(version)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 查询最近创建的激活版本ID（跨方案）
    ///
    /// 用途：前端启动时自动回填当前工作版本，避免“已有激活版本但界面提示未选择”。
    pub fn find_latest_active_version_id(&self) -> RepositoryResult<Option<String>> {
        let conn = self.get_conn()?;

        match conn.query_row(
            r#"SELECT version_id
               FROM plan_version
               WHERE status = 'ACTIVE'
               ORDER BY created_at DESC
               LIMIT 1"#,
            [],
            |row| row.get(0),
        ) {
            Ok(version_id) => Ok(Some(version_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 激活版本 (同时归档其他激活版本)
    ///
    /// # 红线
    /// - 必须在事务中完成，确保原子性
    /// - 同一方案只能有一个激活版本
    pub fn activate_version(&self, version_id: &str) -> RepositoryResult<()> {
        let mut conn = self.get_conn()?;
        let tx = conn.transaction()?;

        // 1. 获取plan_id
        let plan_id: String = tx.query_row(
            "SELECT plan_id FROM plan_version WHERE version_id = ?",
            params![version_id],
            |row| row.get(0),
        )?;

        // 2. 将其他激活版本归档
        tx.execute(
            "UPDATE plan_version SET status = 'ARCHIVED' WHERE plan_id = ? AND status = 'ACTIVE'",
            params![&plan_id],
        )?;

        // 3. 激活指定版本
        tx.execute(
            "UPDATE plan_version SET status = 'ACTIVE' WHERE version_id = ?",
            params![version_id],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// 获取下一个版本号
    pub fn get_next_version_no(&self, plan_id: &str) -> RepositoryResult<i32> {
        let conn = self.get_conn()?;

        let max_version_no: Option<i32> = conn.query_row(
            "SELECT MAX(version_no) FROM plan_version WHERE plan_id = ?",
            params![plan_id],
            |row| row.get(0),
        )?;

        Ok(max_version_no.unwrap_or(0) + 1)
    }

    /// 映射数据库行到PlanVersion对象
    fn map_row(&self, row: &rusqlite::Row) -> rusqlite::Result<PlanVersion> {
        let status_str: String = row.get(3)?;
        Ok(PlanVersion {
            version_id: row.get(0)?,
            plan_id: row.get(1)?,
            version_no: row.get(2)?,
            status: PlanVersionStatus::from_str(&status_str),
            frozen_from_date: row
                .get::<_, Option<String>>(4)?
                .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            recalc_window_days: row.get(5)?,
            config_snapshot_json: row.get(6)?,
            created_by: row.get(7)?,
            created_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(8)?, "%Y-%m-%d %H:%M:%S")
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(e)))?,
            revision: row.get(9)?,
        })
    }
}

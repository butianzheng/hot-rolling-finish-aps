use crate::domain::plan::Plan;
use crate::repository::error::{RepositoryError, RepositoryResult};
use chrono::NaiveDateTime;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

// ==========================================
// PlanRepository - 排产方案仓储
// ==========================================
pub struct PlanRepository {
    conn: Arc<Mutex<Connection>>,
}

impl PlanRepository {
    /// 创建新的PlanRepository实例
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 获取数据库连接
    fn get_conn(&self) -> RepositoryResult<std::sync::MutexGuard<Connection>> {
        self.conn
            .lock()
            .map_err(|e| RepositoryError::LockError(e.to_string()))
    }

    /// 创建方案
    ///
    /// # 参数
    /// - `plan`: 方案对象
    ///
    /// # 返回
    /// - `Ok(plan_id)`: 成功，返回plan_id
    /// - `Err`: 失败，返回错误信息
    pub fn create(&self, plan: &Plan) -> RepositoryResult<String> {
        let conn = self.get_conn()?;

        conn.execute(
            r#"INSERT INTO plan (
                plan_id, plan_name, plan_type, base_plan_id,
                created_by, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)"#,
            params![
                &plan.plan_id,
                &plan.plan_name,
                &plan.plan_type,
                &plan.base_plan_id,
                &plan.created_by,
                &plan.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                &plan.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            ],
        )?;

        Ok(plan.plan_id.clone())
    }

    /// 按plan_id查询方案
    ///
    /// # 参数
    /// - `plan_id`: 方案ID
    ///
    /// # 返回
    /// - `Ok(Some(Plan))`: 找到方案
    /// - `Ok(None)`: 未找到方案
    /// - `Err`: 数据库错误
    pub fn find_by_id(&self, plan_id: &str) -> RepositoryResult<Option<Plan>> {
        let conn = self.get_conn()?;

        match conn.query_row(
            r#"SELECT plan_id, plan_name, plan_type, base_plan_id,
                      created_by, created_at, updated_at
               FROM plan
               WHERE plan_id = ?"#,
            params![plan_id],
            |row| self.map_row(row),
        ) {
            Ok(plan) => Ok(Some(plan)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 查询所有方案
    ///
    /// # 返回
    /// - `Ok(Vec<Plan>)`: 方案列表，按created_at降序
    /// - `Err`: 数据库错误
    pub fn list_all(&self) -> RepositoryResult<Vec<Plan>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"SELECT plan_id, plan_name, plan_type, base_plan_id,
                      created_by, created_at, updated_at
               FROM plan
               ORDER BY created_at DESC"#,
        )?;

        let plans = stmt
            .query_map([], |row| self.map_row(row))?
            .collect::<Result<Vec<Plan>, _>>()?;

        Ok(plans)
    }

    /// 更新方案
    ///
    /// # 参数
    /// - `plan`: 方案对象
    ///
    /// # 返回
    /// - `Ok(())`: 更新成功
    /// - `Err`: 数据库错误
    pub fn update(&self, plan: &Plan) -> RepositoryResult<()> {
        let conn = self.get_conn()?;

        conn.execute(
            r#"UPDATE plan
               SET plan_name = ?, plan_type = ?, base_plan_id = ?,
                   updated_at = ?
               WHERE plan_id = ?"#,
            params![
                &plan.plan_name,
                &plan.plan_type,
                &plan.base_plan_id,
                &plan.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                &plan.plan_id,
            ],
        )?;

        Ok(())
    }

    /// 删除方案 (级联删除版本和明细)
    ///
    /// # 参数
    /// - `plan_id`: 方案ID
    ///
    /// # 返回
    /// - `Ok(())`: 删除成功
    /// - `Err`: 数据库错误
    pub fn delete(&self, plan_id: &str) -> RepositoryResult<()> {
        let conn = self.get_conn()?;

        conn.execute("DELETE FROM plan WHERE plan_id = ?", params![plan_id])?;

        Ok(())
    }

    /// 映射数据库行到Plan对象
    fn map_row(&self, row: &rusqlite::Row) -> rusqlite::Result<Plan> {
        Ok(Plan {
            plan_id: row.get(0)?,
            plan_name: row.get(1)?,
            plan_type: row.get(2)?,
            base_plan_id: row.get(3)?,
            created_by: row.get(4)?,
            created_at: NaiveDateTime::parse_from_str(
                &row.get::<_, String>(5)?,
                "%Y-%m-%d %H:%M:%S",
            )
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
            updated_at: NaiveDateTime::parse_from_str(
                &row.get::<_, String>(6)?,
                "%Y-%m-%d %H:%M:%S",
            )
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
        })
    }
}

// ==========================================
// 热轧精整排产系统 - 产能池数据仓储
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART D 引擎铁律
// 红线: Repository 不含业务逻辑
// ==========================================

use crate::domain::capacity::CapacityPool;
use crate::repository::error::{RepositoryError, RepositoryResult};
use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use std::sync::{Arc, Mutex};

// ==========================================
// CapacityPoolRepository - 产能池仓储
// ==========================================

/// 产能池仓储
/// 职责: 管理capacity_pool表的CRUD操作
pub struct CapacityPoolRepository {
    conn: Arc<Mutex<Connection>>,
}

impl CapacityPoolRepository {
    /// 创建新的产能池仓储实例
    ///
    /// # 参数
    /// - db_path: 数据库文件路径
    ///
    /// # 返回
    /// - Ok(CapacityPoolRepository): 仓储实例
    /// - Err: 数据库连接错误
    pub fn new(db_path: String) -> RepositoryResult<Self> {
        let conn = Connection::open(&db_path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 从已有连接创建仓储实例
    pub fn from_connection(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 获取数据库连接
    fn get_conn(&self) -> RepositoryResult<std::sync::MutexGuard<Connection>> {
        self.conn
            .lock()
            .map_err(|e| RepositoryError::LockError(e.to_string()))
    }

    /// 按机组代码和计划日期查询单个产能池
    ///
    /// # 参数
    /// - machine_code: 机组代码
    /// - plan_date: 计划日期
    ///
    /// # 返回
    /// - Ok(Some(CapacityPool)): 找到产能池
    /// - Ok(None): 未找到
    /// - Err: 数据库错误
    pub fn find_by_machine_and_date(
        &self,
        machine_code: &str,
        plan_date: NaiveDate,
    ) -> RepositoryResult<Option<CapacityPool>> {
        let conn = self.get_conn()?;
        let plan_date_str = plan_date.format("%Y-%m-%d").to_string();

        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t,
                accumulated_tonnage_t, roll_campaign_id
            FROM capacity_pool
            WHERE machine_code = ?1 AND plan_date = ?2
            "#,
        )?;

        let pool = stmt
            .query_row(params![machine_code, plan_date_str], |row| {
                Ok(CapacityPool {
                    machine_code: row.get(0)?,
                    plan_date: NaiveDate::parse_from_str(&row.get::<_, String>(1)?, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                    target_capacity_t: row.get(2)?,
                    limit_capacity_t: row.get(3)?,
                    used_capacity_t: row.get(4)?,
                    overflow_t: row.get(5)?,
                    frozen_capacity_t: row.get(6)?,
                    accumulated_tonnage_t: row.get(7)?,
                    roll_campaign_id: row.get(8)?,
                })
            })
            .optional()?;

        Ok(pool)
    }

    /// 按机组代码和日期范围查询产能池列表
    ///
    /// # 参数
    /// - machine_code: 机组代码
    /// - start_date: 起始日期
    /// - end_date: 结束日期
    ///
    /// # 返回
    /// - Ok(Vec<CapacityPool>): 产能池列表
    /// - Err: 数据库错误
    pub fn find_by_date_range(
        &self,
        machine_code: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> RepositoryResult<Vec<CapacityPool>> {
        let conn = self.get_conn()?;
        let start_date_str = start_date.format("%Y-%m-%d").to_string();
        let end_date_str = end_date.format("%Y-%m-%d").to_string();

        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t,
                accumulated_tonnage_t, roll_campaign_id
            FROM capacity_pool
            WHERE machine_code = ?1
              AND plan_date BETWEEN ?2 AND ?3
            ORDER BY plan_date
            "#,
        )?;

        let pools = stmt
            .query_map(
                params![machine_code, start_date_str, end_date_str],
                |row| {
                    Ok(CapacityPool {
                        machine_code: row.get(0)?,
                        plan_date: NaiveDate::parse_from_str(
                            &row.get::<_, String>(1)?,
                            "%Y-%m-%d",
                        )
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                        target_capacity_t: row.get(2)?,
                        limit_capacity_t: row.get(3)?,
                        used_capacity_t: row.get(4)?,
                        overflow_t: row.get(5)?,
                        frozen_capacity_t: row.get(6)?,
                        accumulated_tonnage_t: row.get(7)?,
                        roll_campaign_id: row.get(8)?,
                    })
                },
            )?
            .collect::<SqliteResult<Vec<CapacityPool>>>()?;

        Ok(pools)
    }

    /// 插入或更新单个产能池
    ///
    /// # 参数
    /// - pool: 产能池数据
    ///
    /// # 返回
    /// - Ok(()): 操作成功
    /// - Err: 数据库错误
    pub fn upsert_single(&self, pool: &CapacityPool) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        let plan_date_str = pool.plan_date.format("%Y-%m-%d").to_string();

        conn.execute(
            r#"
            INSERT OR REPLACE INTO capacity_pool (
                machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t,
                accumulated_tonnage_t, roll_campaign_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                pool.machine_code,
                plan_date_str,
                pool.target_capacity_t,
                pool.limit_capacity_t,
                pool.used_capacity_t,
                pool.overflow_t,
                pool.frozen_capacity_t,
                pool.accumulated_tonnage_t,
                pool.roll_campaign_id,
            ],
        )?;

        Ok(())
    }

    /// 批量插入或更新产能池
    ///
    /// # 参数
    /// - pools: 产能池列表
    ///
    /// # 返回
    /// - Ok(usize): 成功更新的记录数
    /// - Err: 数据库错误
    pub fn upsert_batch(&self, pools: Vec<CapacityPool>) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;

        // 开启事务
        conn.execute("BEGIN TRANSACTION", [])?;

        let mut updated_count = 0;

        for pool in &pools {
            let plan_date_str = pool.plan_date.format("%Y-%m-%d").to_string();

            let affected = conn.execute(
                r#"
                INSERT OR REPLACE INTO capacity_pool (
                    machine_code, plan_date, target_capacity_t, limit_capacity_t,
                    used_capacity_t, overflow_t, frozen_capacity_t,
                    accumulated_tonnage_t, roll_campaign_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
                params![
                    pool.machine_code,
                    plan_date_str,
                    pool.target_capacity_t,
                    pool.limit_capacity_t,
                    pool.used_capacity_t,
                    pool.overflow_t,
                    pool.frozen_capacity_t,
                    pool.accumulated_tonnage_t,
                    pool.roll_campaign_id,
                ],
            )?;

            updated_count += affected;
        }

        // 提交事务
        conn.execute("COMMIT", [])?;

        Ok(updated_count)
    }

    /// 查询超限产能池列表（overflow_t > 0）
    ///
    /// # 参数
    /// - date_range: 日期范围 (start_date, end_date)
    ///
    /// # 返回
    /// - Ok(Vec<CapacityPool>): 超限产能池列表
    /// - Err: 数据库错误
    pub fn find_overflow_pools(
        &self,
        date_range: (NaiveDate, NaiveDate),
    ) -> RepositoryResult<Vec<CapacityPool>> {
        let conn = self.get_conn()?;
        let (start_date, end_date) = date_range;
        let start_date_str = start_date.format("%Y-%m-%d").to_string();
        let end_date_str = end_date.format("%Y-%m-%d").to_string();

        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t,
                accumulated_tonnage_t, roll_campaign_id
            FROM capacity_pool
            WHERE plan_date BETWEEN ?1 AND ?2
              AND overflow_t > 0
            ORDER BY overflow_t DESC, plan_date
            "#,
        )?;

        let pools = stmt
            .query_map(params![start_date_str, end_date_str], |row| {
                Ok(CapacityPool {
                    machine_code: row.get(0)?,
                    plan_date: NaiveDate::parse_from_str(&row.get::<_, String>(1)?, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                    target_capacity_t: row.get(2)?,
                    limit_capacity_t: row.get(3)?,
                    used_capacity_t: row.get(4)?,
                    overflow_t: row.get(5)?,
                    frozen_capacity_t: row.get(6)?,
                    accumulated_tonnage_t: row.get(7)?,
                    roll_campaign_id: row.get(8)?,
                })
            })?
            .collect::<SqliteResult<Vec<CapacityPool>>>()?;

        Ok(pools)
    }

    /// 查询所有机组的指定日期产能池
    ///
    /// # 参数
    /// - plan_date: 计划日期
    ///
    /// # 返回
    /// - Ok(Vec<CapacityPool>): 产能池列表
    /// - Err: 数据库错误
    pub fn find_all_machines_by_date(
        &self,
        plan_date: NaiveDate,
    ) -> RepositoryResult<Vec<CapacityPool>> {
        let conn = self.get_conn()?;
        let plan_date_str = plan_date.format("%Y-%m-%d").to_string();

        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t,
                accumulated_tonnage_t, roll_campaign_id
            FROM capacity_pool
            WHERE plan_date = ?1
            ORDER BY machine_code
            "#,
        )?;

        let pools = stmt
            .query_map(params![plan_date_str], |row| {
                Ok(CapacityPool {
                    machine_code: row.get(0)?,
                    plan_date: NaiveDate::parse_from_str(&row.get::<_, String>(1)?, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                    target_capacity_t: row.get(2)?,
                    limit_capacity_t: row.get(3)?,
                    used_capacity_t: row.get(4)?,
                    overflow_t: row.get(5)?,
                    frozen_capacity_t: row.get(6)?,
                    accumulated_tonnage_t: row.get(7)?,
                    roll_campaign_id: row.get(8)?,
                })
            })?
            .collect::<SqliteResult<Vec<CapacityPool>>>()?;

        Ok(pools)
    }
}

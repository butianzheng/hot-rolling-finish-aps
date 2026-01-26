// ==========================================
// 热轧精整排产系统 - 排产方案数据仓储
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART D 引擎铁律
// 依据: docs/plan_repository_design.md - 设计规格
// 红线: Repository 不含业务逻辑
// ==========================================

use crate::domain::plan::{Plan, PlanItem, PlanVersion};
use crate::repository::error::{RepositoryError, RepositoryResult};
use chrono::{NaiveDate, NaiveDateTime};
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
            created_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(5)?, "%Y-%m-%d %H:%M:%S")
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e)))?,
            updated_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(6)?, "%Y-%m-%d %H:%M:%S")
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e)))?,
        })
    }
}

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
                &version.status,
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
                &version.status,
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
        Ok(PlanVersion {
            version_id: row.get(0)?,
            plan_id: row.get(1)?,
            version_no: row.get(2)?,
            status: row.get(3)?,
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

// ==========================================
// PlanItemRepository - 排产明细仓储
// ==========================================
// 红线: 只是方案快照,不可反向污染 material_state
pub struct PlanItemRepository {
    conn: Arc<Mutex<Connection>>,
}

impl PlanItemRepository {
    /// 创建新的PlanItemRepository实例
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 获取数据库连接
    fn get_conn(&self) -> RepositoryResult<std::sync::MutexGuard<Connection>> {
        self.conn
            .lock()
            .map_err(|e| RepositoryError::LockError(e.to_string()))
    }

    /// 批量插入明细
    ///
    /// # 参数
    /// - `items`: PlanItem列表
    ///
    /// # 返回
    /// - `Ok(count)`: 插入成功的记录数
    /// - `Err`: 数据库错误
    ///
    /// # 红线
    /// - 必须在事务中完成
    /// - plan_item只是快照，不可反向污染material_state
    pub fn batch_insert(&self, items: &[PlanItem]) -> RepositoryResult<usize> {
        if items.is_empty() {
            return Ok(0);
        }

        let mut conn = self.get_conn()?;
        let tx = conn.transaction()?;

        for item in items {
            tx.execute(
                r#"INSERT INTO plan_item (
                    version_id, material_id, machine_code, plan_date, seq_no,
                    weight_t, source_type, locked_in_plan, force_release_in_plan,
                    violation_flags
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                params![
                    &item.version_id,
                    &item.material_id,
                    &item.machine_code,
                    &item.plan_date.format("%Y-%m-%d").to_string(),
                    &item.seq_no,
                    &item.weight_t,
                    &item.source_type,
                    if item.locked_in_plan { 1 } else { 0 },
                    if item.force_release_in_plan { 1 } else { 0 },
                    &item.violation_flags,
                ],
            )?;
        }

        tx.commit()?;
        Ok(items.len())
    }

    /// 批量插入或更新明细 (UPSERT)
    pub fn batch_upsert(&self, items: &[PlanItem]) -> RepositoryResult<usize> {
        if items.is_empty() {
            return Ok(0);
        }

        let mut conn = self.get_conn()?;
        let tx = conn.transaction()?;

        for item in items {
            tx.execute(
                r#"INSERT OR REPLACE INTO plan_item (
                    version_id, material_id, machine_code, plan_date, seq_no,
                    weight_t, source_type, locked_in_plan, force_release_in_plan,
                    violation_flags
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                params![
                    &item.version_id,
                    &item.material_id,
                    &item.machine_code,
                    &item.plan_date.format("%Y-%m-%d").to_string(),
                    &item.seq_no,
                    &item.weight_t,
                    &item.source_type,
                    if item.locked_in_plan { 1 } else { 0 },
                    if item.force_release_in_plan { 1 } else { 0 },
                    &item.violation_flags,
                ],
            )?;
        }

        tx.commit()?;
        Ok(items.len())
    }

    /// 查询版本的所有明细
    pub fn find_by_version(&self, version_id: &str) -> RepositoryResult<Vec<PlanItem>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"SELECT version_id, material_id, machine_code, plan_date, seq_no,
                      weight_t, source_type, locked_in_plan, force_release_in_plan,
                      violation_flags
               FROM plan_item
               WHERE version_id = ?
               ORDER BY machine_code, plan_date, seq_no"#,
        )?;

        let items = stmt
            .query_map(params![version_id], |row| self.map_row(row))?
            .collect::<Result<Vec<PlanItem>, _>>()?;

        Ok(items)
    }

    /// 查询指定日期的明细
    pub fn find_by_date(
        &self,
        version_id: &str,
        plan_date: NaiveDate,
    ) -> RepositoryResult<Vec<PlanItem>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"SELECT version_id, material_id, machine_code, plan_date, seq_no,
                      weight_t, source_type, locked_in_plan, force_release_in_plan,
                      violation_flags
               FROM plan_item
               WHERE version_id = ? AND plan_date = ?
               ORDER BY machine_code, seq_no"#,
        )?;

        let items = stmt
            .query_map(
                params![version_id, plan_date.format("%Y-%m-%d").to_string()],
                |row| self.map_row(row),
            )?
            .collect::<Result<Vec<PlanItem>, _>>()?;

        Ok(items)
    }

    /// 查询日期范围的明细
    pub fn find_by_date_range(
        &self,
        version_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> RepositoryResult<Vec<PlanItem>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"SELECT version_id, material_id, machine_code, plan_date, seq_no,
                      weight_t, source_type, locked_in_plan, force_release_in_plan,
                      violation_flags
               FROM plan_item
               WHERE version_id = ? AND plan_date BETWEEN ? AND ?
               ORDER BY plan_date, machine_code, seq_no"#,
        )?;

        let items = stmt
            .query_map(
                params![
                    version_id,
                    start_date.format("%Y-%m-%d").to_string(),
                    end_date.format("%Y-%m-%d").to_string(),
                ],
                |row| self.map_row(row),
            )?
            .collect::<Result<Vec<PlanItem>, _>>()?;

        Ok(items)
    }

    /// 查询指定机组的明细
    pub fn find_by_machine(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> RepositoryResult<Vec<PlanItem>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"SELECT version_id, material_id, machine_code, plan_date, seq_no,
                      weight_t, source_type, locked_in_plan, force_release_in_plan,
                      violation_flags
               FROM plan_item
               WHERE version_id = ? AND machine_code = ?
               ORDER BY plan_date, seq_no"#,
        )?;

        let items = stmt
            .query_map(params![version_id, machine_code], |row| self.map_row(row))?
            .collect::<Result<Vec<PlanItem>, _>>()?;

        Ok(items)
    }

    /// 查询冻结区明细
    pub fn find_frozen_items(&self, version_id: &str) -> RepositoryResult<Vec<PlanItem>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"SELECT version_id, material_id, machine_code, plan_date, seq_no,
                      weight_t, source_type, locked_in_plan, force_release_in_plan,
                      violation_flags
               FROM plan_item
               WHERE version_id = ? AND locked_in_plan = 1
               ORDER BY plan_date, machine_code, seq_no"#,
        )?;

        let items = stmt
            .query_map(params![version_id], |row| self.map_row(row))?
            .collect::<Result<Vec<PlanItem>, _>>()?;

        Ok(items)
    }

    /// 查询材料的排产历史
    pub fn find_material_history(
        &self,
        material_id: &str,
    ) -> RepositoryResult<Vec<PlanItem>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"SELECT pi.version_id, pi.material_id, pi.machine_code, pi.plan_date, pi.seq_no,
                      pi.weight_t, pi.source_type, pi.locked_in_plan, pi.force_release_in_plan,
                      pi.violation_flags
               FROM plan_item pi
               INNER JOIN plan_version pv ON pi.version_id = pv.version_id
               WHERE pi.material_id = ?
               ORDER BY pv.created_at DESC"#,
        )?;

        let items = stmt
            .query_map(params![material_id], |row| self.map_row(row))?
            .collect::<Result<Vec<PlanItem>, _>>()?;

        Ok(items)
    }

    /// 删除版本的所有明细
    pub fn delete_by_version(&self, version_id: &str) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;

        let count = conn.execute(
            "DELETE FROM plan_item WHERE version_id = ?",
            params![version_id],
        )?;

        Ok(count)
    }

    /// 删除指定日期范围的明细 (用于重算)
    ///
    /// # 红线
    /// - 业务层必须确保不删除冻结区明细
    pub fn delete_by_date_range(
        &self,
        version_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;

        let count = conn.execute(
            r#"DELETE FROM plan_item
               WHERE version_id = ? AND plan_date BETWEEN ? AND ?"#,
            params![
                version_id,
                start_date.format("%Y-%m-%d").to_string(),
                end_date.format("%Y-%m-%d").to_string(),
            ],
        )?;

        Ok(count)
    }

    /// 映射数据库行到PlanItem对象
    fn map_row(&self, row: &rusqlite::Row) -> rusqlite::Result<PlanItem> {
        Ok(PlanItem {
            version_id: row.get(0)?,
            material_id: row.get(1)?,
            machine_code: row.get(2)?,
            plan_date: NaiveDate::parse_from_str(&row.get::<_, String>(3)?, "%Y-%m-%d")
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
            seq_no: row.get(4)?,
            weight_t: row.get(5)?,
            source_type: row.get(6)?,
            locked_in_plan: row.get::<_, i32>(7)? == 1,
            force_release_in_plan: row.get::<_, i32>(8)? == 1,
            violation_flags: row.get(9)?,
            // 快照字段 (不存储在schema中，从material_state动态获取)
            urgent_level: None,
            sched_state: None,
            assign_reason: None,
        })
    }
}

// TODO: 实现错误处理
// TODO: 实现事务支持

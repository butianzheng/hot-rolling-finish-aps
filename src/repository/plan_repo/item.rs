use crate::domain::plan::PlanItem;
use crate::repository::error::{RepositoryError, RepositoryResult};
use chrono::NaiveDate;
use rusqlite::{params, params_from_iter, Connection};
use rusqlite::types::Value;
use std::sync::{Arc, Mutex};

// ==========================================
// PlanItemRepository - 排产明细仓储
// ==========================================
// 红线: 只是方案快照,不可反向污染 material_state
pub struct PlanItemRepository {
    conn: Arc<Mutex<Connection>>,
}

/// PlanItem 聚合统计（用于版本对比 KPI，避免拉取全量明细）
#[derive(Debug, Clone)]
pub struct PlanItemVersionAgg {
    pub plan_items_count: usize,
    pub total_weight_t: f64,
    pub locked_in_plan_count: usize,
    pub force_release_in_plan_count: usize,
    pub plan_date_from: Option<NaiveDate>,
    pub plan_date_to: Option<NaiveDate>,
}

/// 两版本间的 diff 计数（口径与 `compare_versions` 保持一致：仅比较机组/日期）
#[derive(Debug, Clone)]
pub struct PlanItemDiffCounts {
    pub moved_count: usize,
    pub added_count: usize,
    pub removed_count: usize,
    pub squeezed_out_count: usize,
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

        {
            let mut stmt = tx.prepare(
                r#"INSERT INTO plan_item (
                        version_id, material_id, machine_code, plan_date, seq_no,
                        weight_t, source_type, locked_in_plan, force_release_in_plan,
                        violation_flags
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )?;

            for item in items {
                stmt.execute(
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

        {
            let mut stmt = tx.prepare(
                r#"INSERT OR REPLACE INTO plan_item (
                        version_id, material_id, machine_code, plan_date, seq_no,
                        weight_t, source_type, locked_in_plan, force_release_in_plan,
                        violation_flags
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )?;

            for item in items {
                stmt.execute(
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

    /// 查询版本明细（可选过滤 + 分页）
    ///
    /// 说明：
    /// - plan_date 字段在库中为 YYYY-MM-DD 文本，ISO 格式支持字符串比较（>= / <= / BETWEEN）；
    /// - limit/offset 用于“增量加载”，避免一次性拉取全量明细。
    pub fn find_by_filters_paged(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> RepositoryResult<Vec<PlanItem>> {
        let conn = self.get_conn()?;

        let mut sql = String::from(
            r#"SELECT version_id, material_id, machine_code, plan_date, seq_no,
                      weight_t, source_type, locked_in_plan, force_release_in_plan,
                      violation_flags
               FROM plan_item
               WHERE version_id = ?1"#,
        );

        let mut values: Vec<Value> = vec![Value::from(version_id.to_string())];
        let mut idx: i32 = 2;

        if let Some(code) = machine_code.map(str::trim).filter(|s| !s.is_empty()) {
            sql.push_str(&format!(" AND machine_code = ?{}", idx));
            values.push(Value::from(code.to_string()));
            idx += 1;
        }

        match (start_date, end_date) {
            (Some(from), Some(to)) => {
                sql.push_str(&format!(" AND plan_date BETWEEN ?{} AND ?{}", idx, idx + 1));
                values.push(Value::from(from.format("%Y-%m-%d").to_string()));
                values.push(Value::from(to.format("%Y-%m-%d").to_string()));
                idx += 2;
            }
            (Some(from), None) => {
                sql.push_str(&format!(" AND plan_date >= ?{}", idx));
                values.push(Value::from(from.format("%Y-%m-%d").to_string()));
                idx += 1;
            }
            (None, Some(to)) => {
                sql.push_str(&format!(" AND plan_date <= ?{}", idx));
                values.push(Value::from(to.format("%Y-%m-%d").to_string()));
                idx += 1;
            }
            (None, None) => {}
        }

        sql.push_str(" ORDER BY plan_date, machine_code, seq_no");

        if let Some(limit) = limit {
            sql.push_str(&format!(" LIMIT ?{}", idx));
            values.push(Value::from(limit));
            idx += 1;
        }
        if let Some(offset) = offset {
            sql.push_str(&format!(" OFFSET ?{}", idx));
            values.push(Value::from(offset));
            idx += 1;
        }

        let mut stmt = conn.prepare(&sql)?;
        let items = stmt
            .query_map(params_from_iter(values), |row| self.map_row(row))?
            .collect::<Result<Vec<PlanItem>, _>>()?;

        Ok(items)
    }

    /// 查询版本内排产日期边界（min/max）及数量
    ///
    /// 用途：
    /// - Workbench AUTO 日期范围计算（避免拉取全量 plan_item）
    /// - 大数据量下“先取边界/总数，再按范围分页拉取”
    pub fn get_plan_date_bounds(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
    ) -> RepositoryResult<(Option<NaiveDate>, Option<NaiveDate>, i64)> {
        let conn = self.get_conn()?;

        let (min_str, max_str, count): (Option<String>, Option<String>, i64) = if let Some(code) =
            machine_code.map(str::trim).filter(|s| !s.is_empty())
        {
            conn.query_row(
                r#"
                SELECT MIN(plan_date), MAX(plan_date), COUNT(*)
                FROM plan_item
                WHERE version_id = ?1 AND machine_code = ?2
                "#,
                params![version_id, code],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?
        } else {
            conn.query_row(
                r#"
                SELECT MIN(plan_date), MAX(plan_date), COUNT(*)
                FROM plan_item
                WHERE version_id = ?1
                "#,
                params![version_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?
        };

        let parse_date = |s: Option<String>| -> Option<NaiveDate> {
            s.and_then(|v| NaiveDate::parse_from_str(v.trim(), "%Y-%m-%d").ok())
        };

        Ok((parse_date(min_str), parse_date(max_str), count))
    }

    /// 统计版本明细数量（可选过滤）
    pub fn count_by_filters(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> RepositoryResult<i64> {
        let conn = self.get_conn()?;

        let mut sql = String::from(
            r#"SELECT COUNT(*)
               FROM plan_item
               WHERE version_id = ?1"#,
        );

        let mut values: Vec<Value> = vec![Value::from(version_id.to_string())];
        let mut idx: i32 = 2;

        if let Some(code) = machine_code.map(str::trim).filter(|s| !s.is_empty()) {
            sql.push_str(&format!(" AND machine_code = ?{}", idx));
            values.push(Value::from(code.to_string()));
            idx += 1;
        }

        match (start_date, end_date) {
            (Some(from), Some(to)) => {
                sql.push_str(&format!(" AND plan_date BETWEEN ?{} AND ?{}", idx, idx + 1));
                values.push(Value::from(from.format("%Y-%m-%d").to_string()));
                values.push(Value::from(to.format("%Y-%m-%d").to_string()));
            }
            (Some(from), None) => {
                sql.push_str(&format!(" AND plan_date >= ?{}", idx));
                values.push(Value::from(from.format("%Y-%m-%d").to_string()));
            }
            (None, Some(to)) => {
                sql.push_str(&format!(" AND plan_date <= ?{}", idx));
                values.push(Value::from(to.format("%Y-%m-%d").to_string()));
            }
            (None, None) => {}
        }

        let mut stmt = conn.prepare(&sql)?;
        let count: i64 = stmt.query_row(params_from_iter(values), |row| row.get(0))?;
        Ok(count)
    }

    /// 查询版本中所有 (machine_code, plan_date) 组合（去重）
    ///
    /// 用途：
    /// - 删除版本/删除明细前获取受影响的产能池 keys
    /// - 避免拉取全量 plan_item（50k+ 时会显著拖慢）
    pub fn list_machine_date_keys(
        &self,
        version_id: &str,
    ) -> RepositoryResult<Vec<(String, NaiveDate)>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT DISTINCT machine_code, plan_date
            FROM plan_item
            WHERE version_id = ?1
            ORDER BY machine_code, plan_date
            "#,
        )?;

        let keys = stmt
            .query_map(params![version_id], |row| {
                let machine_code: String = row.get(0)?;
                let plan_date_str: String = row.get(1)?;
                let plan_date = NaiveDate::parse_from_str(&plan_date_str, "%Y-%m-%d").map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok((machine_code, plan_date))
            })?
            .collect::<Result<Vec<(String, NaiveDate)>, _>>()?;

        Ok(keys)
    }

    /// 按 (machine_code, plan_date) 聚合版本内排产吨位
    ///
    /// 说明：
    /// - 用于“版本激活后同步刷新产能池”场景；
    /// - 通过 SQL 聚合避免拉取全量 plan_item 明细（50k+ 时会显著拖慢且占用内存）。
    pub fn sum_weight_by_machine_and_date_range(
        &self,
        version_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> RepositoryResult<Vec<(String, NaiveDate, f64)>> {
        let conn = self.get_conn()?;
        let start_date_str = start_date.format("%Y-%m-%d").to_string();
        let end_date_str = end_date.format("%Y-%m-%d").to_string();

        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code,
                plan_date,
                COALESCE(SUM(weight_t), 0.0) AS used_capacity_t
            FROM plan_item
            WHERE version_id = ?1
              AND plan_date BETWEEN ?2 AND ?3
            GROUP BY machine_code, plan_date
            ORDER BY machine_code, plan_date
            "#,
        )?;

        let rows = stmt
            .query_map(params![version_id, start_date_str, end_date_str], |row| {
                let machine_code: String = row.get(0)?;
                let plan_date_str: String = row.get(1)?;
                let used_capacity_t: f64 = row.get(2)?;

                let plan_date = NaiveDate::parse_from_str(&plan_date_str, "%Y-%m-%d").map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                Ok((machine_code, plan_date, used_capacity_t))
            })?
            .collect::<Result<Vec<(String, NaiveDate, f64)>, _>>()?;

        Ok(rows)
    }

    /// 获取版本的排产明细聚合统计（count/sum/min/max）
    pub fn get_version_agg(&self, version_id: &str) -> RepositoryResult<PlanItemVersionAgg> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT
                COUNT(*) AS plan_items_count,
                COALESCE(SUM(weight_t), 0.0) AS total_weight_t,
                COALESCE(SUM(locked_in_plan), 0) AS locked_in_plan_count,
                COALESCE(SUM(force_release_in_plan), 0) AS force_release_in_plan_count,
                MIN(plan_date) AS plan_date_from,
                MAX(plan_date) AS plan_date_to
            FROM plan_item
            WHERE version_id = ?1
            "#,
        )?;

        let agg = stmt.query_row(params![version_id], |row| {
            let plan_date_from: Option<String> = row.get(4)?;
            let plan_date_to: Option<String> = row.get(5)?;

            Ok(PlanItemVersionAgg {
                plan_items_count: row.get::<_, i64>(0)? as usize,
                total_weight_t: row.get::<_, f64>(1)?,
                locked_in_plan_count: row.get::<_, i64>(2)? as usize,
                force_release_in_plan_count: row.get::<_, i64>(3)? as usize,
                plan_date_from: plan_date_from
                    .as_deref()
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
                plan_date_to: plan_date_to
                    .as_deref()
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
            })
        })?;

        Ok(agg)
    }

    /// 获取两版本间的 diff 计数（SQL 聚合，避免全量拉取 plan_item）
    pub fn get_versions_diff_counts(
        &self,
        version_id_a: &str,
        version_id_b: &str,
    ) -> RepositoryResult<PlanItemDiffCounts> {
        let conn = self.get_conn()?;

        let moved_count: i64 = conn.query_row(
            r#"
            SELECT COUNT(*)
            FROM plan_item a
            INNER JOIN plan_item b ON a.material_id = b.material_id
            WHERE a.version_id = ?1
              AND b.version_id = ?2
              AND (a.plan_date <> b.plan_date OR a.machine_code <> b.machine_code)
            "#,
            params![version_id_a, version_id_b],
            |row| row.get(0),
        )?;

        let added_count: i64 = conn.query_row(
            r#"
            SELECT COUNT(*)
            FROM plan_item b
            WHERE b.version_id = ?2
              AND NOT EXISTS (
                SELECT 1
                FROM plan_item a
                WHERE a.version_id = ?1
                  AND a.material_id = b.material_id
              )
            "#,
            params![version_id_a, version_id_b],
            |row| row.get(0),
        )?;

        let removed_count: i64 = conn.query_row(
            r#"
            SELECT COUNT(*)
            FROM plan_item a
            WHERE a.version_id = ?1
              AND NOT EXISTS (
                SELECT 1
                FROM plan_item b
                WHERE b.version_id = ?2
                  AND b.material_id = a.material_id
              )
            "#,
            params![version_id_a, version_id_b],
            |row| row.get(0),
        )?;

        Ok(PlanItemDiffCounts {
            moved_count: moved_count as usize,
            added_count: added_count as usize,
            removed_count: removed_count as usize,
            squeezed_out_count: removed_count as usize,
        })
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
            // 快照字段 (不存储在schema中，由API层从material_state/material_master动态补充)
            urgent_level: None,
            sched_state: None,
            assign_reason: None,
            steel_grade: None,
            width_mm: None,
            thickness_mm: None,
        })
    }
}

// TODO: 实现错误处理
// TODO: 实现事务支持

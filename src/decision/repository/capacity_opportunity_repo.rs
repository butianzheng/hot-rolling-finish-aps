// ==========================================
// 热轧精整排产系统 - D6 仓储实现
// ==========================================
// 依据: DECISION_READ_MODELS.md - D6 表设计
// 职责: "是否存在产能优化空间" 数据访问层
// ==========================================

use crate::decision::common::{
    build_in_clause, build_optional_filter_sql, deserialize_json_array_optional,
    deserialize_json_optional, serialize_json_optional, serialize_json_vec,
};
use crate::decision::use_cases::d6_capacity_opportunity::{
    BindingConstraint, CapacityOpportunity, MachineOpportunityStat, OptimizationSummary,
    SensitivityAnalysis,
};
use rusqlite::{params, Connection, Result as SqlResult};
use std::sync::{Arc, Mutex};

/// D6 仓储：产能优化机会
pub struct CapacityOpportunityRepository {
    /// 数据库连接
    conn: Arc<Mutex<Connection>>,
}

impl CapacityOpportunityRepository {
    /// 创建新的产能优化机会仓储
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 查询产能优化机会
    pub fn get_capacity_opportunity(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: &str,
        end_date: &str,
    ) -> SqlResult<Vec<CapacityOpportunity>> {
        let conn = self.conn.lock().map_err(|e| rusqlite::Error::InvalidParameterName(format!("锁获取失败: {}", e)))?;

        let base_sql = r#"
            SELECT
                version_id,
                machine_code,
                plan_date,
                slack_t,
                soft_adjust_space_t,
                utilization_rate,
                binding_constraints,
                opportunity_level,
                sensitivity,
                suggested_optimizations
            FROM decision_capacity_opportunity
            WHERE version_id = ? AND plan_date BETWEEN ? AND ?
        "#;

        let additional_filter = machine_code.map(|_| "machine_code = ?");
        let sql = build_optional_filter_sql(
            base_sql,
            additional_filter,
            "slack_t DESC, plan_date ASC",
        );

        let map_row = |row: &rusqlite::Row| -> SqlResult<CapacityOpportunity> {
            let binding_constraints_json: Option<String> = row.get(6)?;
            let binding_constraints: Vec<BindingConstraint> =
                deserialize_json_array_optional(binding_constraints_json.as_deref());

            let sensitivity_json: Option<String> = row.get(8)?;
            let sensitivity: Option<SensitivityAnalysis> =
                deserialize_json_optional(sensitivity_json.as_deref());

            let suggested_optimizations_json: Option<String> = row.get(9)?;
            let suggested_optimizations: Vec<String> =
                deserialize_json_array_optional(suggested_optimizations_json.as_deref());

            Ok(CapacityOpportunity {
                version_id: row.get(0)?,
                machine_code: row.get(1)?,
                plan_date: row.get(2)?,
                slack_t: row.get(3)?,
                soft_adjust_space_t: row.get(4)?,
                utilization_rate: row.get(5)?,
                binding_constraints,
                opportunity_level: row.get(7)?,
                sensitivity,
                suggested_optimizations,
            })
        };

        let mut stmt = conn.prepare(&sql)?;
        let rows = if let Some(mc) = machine_code {
            stmt.query_map(params![version_id, start_date, end_date, mc], map_row)?
        } else {
            stmt.query_map(params![version_id, start_date, end_date], map_row)?
        };

        rows.collect()
    }

    /// 查询最大优化空间
    pub fn get_top_opportunities(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        top_n: usize,
    ) -> SqlResult<Vec<CapacityOpportunity>> {
        let conn = self.conn.lock().map_err(|e| rusqlite::Error::InvalidParameterName(format!("锁获取失败: {}", e)))?;

        let sql = format!(
            r#"
            SELECT
                version_id,
                machine_code,
                plan_date,
                slack_t,
                soft_adjust_space_t,
                utilization_rate,
                binding_constraints,
                opportunity_level,
                sensitivity,
                suggested_optimizations
            FROM decision_capacity_opportunity
            WHERE version_id = ? AND plan_date BETWEEN ? AND ?
                AND opportunity_level IN ('HIGH', 'MEDIUM')
            ORDER BY slack_t DESC
            LIMIT {}
        "#,
            top_n
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![version_id, start_date, end_date], |row| {
            let binding_constraints_json: Option<String> = row.get(6)?;
            let binding_constraints: Vec<BindingConstraint> =
                deserialize_json_array_optional(binding_constraints_json.as_deref());

            let sensitivity_json: Option<String> = row.get(8)?;
            let sensitivity: Option<SensitivityAnalysis> =
                deserialize_json_optional(sensitivity_json.as_deref());

            let suggested_optimizations_json: Option<String> = row.get(9)?;
            let suggested_optimizations: Vec<String> =
                deserialize_json_array_optional(suggested_optimizations_json.as_deref());

            Ok(CapacityOpportunity {
                version_id: row.get(0)?,
                machine_code: row.get(1)?,
                plan_date: row.get(2)?,
                slack_t: row.get(3)?,
                soft_adjust_space_t: row.get(4)?,
                utilization_rate: row.get(5)?,
                binding_constraints,
                opportunity_level: row.get(7)?,
                sensitivity,
                suggested_optimizations,
            })
        })?;

        rows.collect()
    }

    /// 获取优化总结
    pub fn get_optimization_summary(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> SqlResult<OptimizationSummary> {
        let conn = self.conn.lock().map_err(|e| rusqlite::Error::InvalidParameterName(format!("锁获取失败: {}", e)))?;

        // 查询总体统计
        let mut stmt = conn.prepare(
            r#"
            SELECT
                SUM(slack_t) AS total_slack_t,
                SUM(COALESCE(soft_adjust_space_t, 0.0)) AS total_soft_adjust_space_t,
                AVG(utilization_rate) AS avg_utilization_rate,
                SUM(CASE WHEN opportunity_level IN ('HIGH', 'MEDIUM') THEN 1 ELSE 0 END) AS high_opportunity_count
            FROM decision_capacity_opportunity
            WHERE version_id = ? AND plan_date BETWEEN ? AND ?
        "#,
        )?;

        let (total_slack_t, total_soft_adjust_space_t, avg_utilization_rate, high_opportunity_count) =
            stmt.query_row(params![version_id, start_date, end_date], |row| {
                Ok((
                    row.get::<_, Option<f64>>(0)?.unwrap_or(0.0),
                    row.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                    row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                    row.get::<_, Option<i32>>(3)?.unwrap_or(0),
                ))
            })?;

        // 按机组分组统计
        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code,
                SUM(slack_t) AS slack_t,
                AVG(utilization_rate) AS avg_utilization,
                COUNT(*) AS opportunity_count
            FROM decision_capacity_opportunity
            WHERE version_id = ? AND plan_date BETWEEN ? AND ?
            GROUP BY machine_code
            ORDER BY slack_t DESC
        "#,
        )?;

        let by_machine: Vec<MachineOpportunityStat> = stmt
            .query_map(params![version_id, start_date, end_date], |row| {
                Ok(MachineOpportunityStat {
                    machine_code: row.get(0)?,
                    slack_t: row.get(1)?,
                    avg_utilization: row.get(2)?,
                    opportunity_count: row.get(3)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        // 计算总潜在增益
        let total_potential_gain_t = self.calculate_total_potential_gain(&conn, version_id, start_date, end_date)?;

        Ok(OptimizationSummary {
            version_id: version_id.to_string(),
            date_range: (start_date.to_string(), end_date.to_string()),
            total_slack_t,
            total_soft_adjust_space_t,
            avg_utilization_rate,
            high_opportunity_count,
            by_machine,
            total_potential_gain_t,
        })
    }

    /// 全量刷新 D6 读模型
    pub fn refresh_full(&self, version_id: &str) -> SqlResult<usize> {
        let conn = self.conn.lock().map_err(|e| rusqlite::Error::InvalidParameterName(format!("锁获取失败: {}", e)))?;

        // 1. 删除旧数据
        conn.execute(
            "DELETE FROM decision_capacity_opportunity WHERE version_id = ?",
            params![version_id],
        )?;

        // 2. 查询产能池信息和排产数据
        let opportunities = self.calculate_capacity_opportunities(&conn, version_id)?;

        // 3. 插入到数据库
        for opp in &opportunities {
            self.insert_opportunity(&conn, opp)?;
        }

        Ok(opportunities.len())
    }

    /// 增量刷新 D6 读模型
    pub fn refresh_incremental(
        &self,
        version_id: &str,
        machine_codes: &[String],
        affected_dates: &[(String, String)],
    ) -> SqlResult<usize> {
        if machine_codes.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.lock().map_err(|e| rusqlite::Error::InvalidParameterName(format!("锁获取失败: {}", e)))?;

        // 1. 删除受影响的记录
        let in_clause = build_in_clause("machine_code", machine_codes);
        let mut delete_sql = format!(
            "DELETE FROM decision_capacity_opportunity WHERE version_id = ? AND {}",
            in_clause
        );

        // 添加日期范围条件
        if !affected_dates.is_empty() {
            delete_sql.push_str(" AND (");
            for (i, _) in affected_dates.iter().enumerate() {
                if i > 0 {
                    delete_sql.push_str(" OR ");
                }
                delete_sql.push_str("(plan_date BETWEEN ? AND ?)");
            }
            delete_sql.push(')');
        }

        let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&version_id];
        for mc in machine_codes {
            params_vec.push(mc);
        }
        for (start, end) in affected_dates {
            params_vec.push(start);
            params_vec.push(end);
        }

        conn.execute(&delete_sql, params_vec.as_slice())?;

        // 2. 重新计算受影响的记录
        let opportunities = self.calculate_capacity_opportunities(&conn, version_id)?;
        let filtered_opportunities: Vec<_> = opportunities
            .iter()
            .filter(|opp| {
                machine_codes.contains(&opp.machine_code)
                    && affected_dates.iter().any(|(start, end)| {
                        opp.plan_date.as_str() >= start.as_str() && opp.plan_date.as_str() <= end.as_str()
                    })
            })
            .collect();

        // 3. 插入到数据库
        let mut inserted = 0;
        for opp in filtered_opportunities {
            self.insert_opportunity(&conn, opp)?;
            inserted += 1;
        }

        Ok(inserted)
    }

    /// 计算产能优化机会
    fn calculate_capacity_opportunities(
        &self,
        conn: &Connection,
        version_id: &str,
    ) -> SqlResult<Vec<CapacityOpportunity>> {
        // 查询容量池和排产数据
        let mut stmt = conn.prepare(
            r#"
            SELECT
                cp.machine_code,
                cp.plan_date,
                cp.limit_capacity_t,
                COALESCE(SUM(ms.weight_t), 0.0) AS used_capacity_t
            FROM capacity_pool cp
            LEFT JOIN plan_item pi ON pi.version_id = ? AND pi.machine_code = cp.machine_code
                AND pi.plan_date = cp.plan_date
            LEFT JOIN material_state ms ON ms.material_id = pi.material_id
            GROUP BY cp.machine_code, cp.plan_date
            ORDER BY cp.machine_code, cp.plan_date
        "#,
        )?;

        let mut opportunities = Vec::new();
        let mut rows = stmt.query(params![version_id])?;

        while let Some(row) = rows.next()? {
            let machine_code: String = row.get(0)?;
            let plan_date: String = row.get(1)?;
            let limit_capacity_t: f64 = row.get(2)?;
            let used_capacity_t: f64 = row.get(3)?;

            let slack_t = (limit_capacity_t - used_capacity_t).max(0.0);
            let utilization_rate = if limit_capacity_t > 0.0 {
                used_capacity_t / limit_capacity_t
            } else {
                0.0
            };

            let mut opp = CapacityOpportunity::new(
                version_id.to_string(),
                machine_code.clone(),
                plan_date.clone(),
                slack_t,
                utilization_rate,
            );

            // 生成优化建议
            generate_optimizations(&mut opp);

            opportunities.push(opp);
        }

        Ok(opportunities)
    }

    /// 插入产能优化机会记录
    fn insert_opportunity(&self, conn: &Connection, opp: &CapacityOpportunity) -> SqlResult<()> {
        let binding_constraints_json = serialize_json_vec(&opp.binding_constraints);
        let sensitivity_json = serialize_json_optional(opp.sensitivity.as_ref());
        let suggested_optimizations_json = serialize_json_vec(&opp.suggested_optimizations);

        conn.execute(
            r#"
            INSERT INTO decision_capacity_opportunity (
                version_id, machine_code, plan_date,
                slack_t, soft_adjust_space_t,
                utilization_rate,
                binding_constraints,
                opportunity_level,
                sensitivity,
                suggested_optimizations
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
            params![
                opp.version_id,
                opp.machine_code,
                opp.plan_date,
                opp.slack_t,
                opp.soft_adjust_space_t,
                opp.utilization_rate,
                binding_constraints_json,
                opp.opportunity_level,
                sensitivity_json,
                suggested_optimizations_json,
            ],
        )?;

        Ok(())
    }

    /// 计算总潜在增益
    fn calculate_total_potential_gain(
        &self,
        conn: &Connection,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> SqlResult<f64> {
        let mut stmt = conn.prepare(
            r#"
            SELECT COALESCE(SUM(
                CASE
                    WHEN opportunity_level = 'HIGH' THEN slack_t * 0.8
                    WHEN opportunity_level = 'MEDIUM' THEN slack_t * 0.5
                    WHEN opportunity_level = 'LOW' THEN slack_t * 0.2
                    ELSE 0.0
                END
            ), 0.0) AS total_gain
            FROM decision_capacity_opportunity
            WHERE version_id = ? AND plan_date BETWEEN ? AND ?
        "#,
        )?;

        let total_gain: f64 = stmt.query_row(params![version_id, start_date, end_date], |row| {
            row.get(0)
        })?;

        Ok(total_gain)
    }
}

/// 生成优化建议
fn generate_optimizations(opp: &mut CapacityOpportunity) {
    match opp.opportunity_level.as_str() {
        "HIGH" => {
            opp.suggested_optimizations.push(format!(
                "高优化潜力: 松弛产能 {:.1}t,建议优先优化",
                opp.slack_t
            ));
            opp.suggested_optimizations.push("评估是否可调整目标产能或放松约束".to_string());
            opp.suggested_optimizations.push("考虑从其他机组转移负荷".to_string());
        }
        "MEDIUM" => {
            opp.suggested_optimizations.push(format!(
                "中等优化潜力: 松弛产能 {:.1}t",
                opp.slack_t
            ));
            opp.suggested_optimizations.push("评估优化价值,确定是否调整".to_string());
        }
        "LOW" => {
            opp.suggested_optimizations.push(format!(
                "低优化潜力: 松弛产能 {:.1}t",
                opp.slack_t
            ));
        }
        _ => {
            opp.suggested_optimizations
                .push("产能已充分利用,无明显优化空间".to_string());
        }
    }
}

// ==========================================
// 单元测试
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::configure_sqlite_connection(&conn).unwrap();

        // 创建必要的表
        conn.execute(
            r#"
            CREATE TABLE decision_capacity_opportunity (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                slack_t REAL NOT NULL,
                soft_adjust_space_t REAL,
                utilization_rate REAL NOT NULL,
                binding_constraints TEXT,
                opportunity_level TEXT NOT NULL,
                sensitivity TEXT,
                suggested_optimizations TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, machine_code, plan_date)
            )
        "#,
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            CREATE TABLE capacity_pool (
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                limit_capacity_t REAL NOT NULL,
                PRIMARY KEY (machine_code, plan_date)
            )
        "#,
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            CREATE TABLE plan_item (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                material_id TEXT NOT NULL
            )
        "#,
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            CREATE TABLE material_state (
                material_id TEXT PRIMARY KEY,
                weight_t REAL NOT NULL
            )
        "#,
            [],
        )
        .unwrap();

        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_refresh_full() {
        let conn = setup_test_db();
        {
            let c = conn.lock().expect("锁获取失败");

            // 插入产能池数据
            c.execute(
                "INSERT INTO capacity_pool VALUES ('H032', '2026-01-24', 2000.0)",
                [],
            )
            .unwrap();

            c.execute(
                "INSERT INTO capacity_pool VALUES ('H032', '2026-01-25', 2000.0)",
                [],
            )
            .unwrap();

            c.execute(
                "INSERT INTO capacity_pool VALUES ('H033', '2026-01-24', 1500.0)",
                [],
            )
            .unwrap();

            // 插入排产数据
            c.execute(
                "INSERT INTO material_state VALUES ('M001', 500.0)",
                [],
            )
            .unwrap();

            c.execute(
                "INSERT INTO plan_item VALUES ('V001', 'H032', '2026-01-24', 'M001')",
                [],
            )
            .unwrap();
        }

        let repo = CapacityOpportunityRepository::new(conn.clone());
        let rows = repo.refresh_full("V001").unwrap();

        assert_eq!(rows, 3); // 应该有 3 条记录

        // 验证数据正确性
        let c = conn.lock().expect("锁获取失败");
        let count: i32 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_capacity_opportunity WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_get_capacity_opportunity() {
        let conn = setup_test_db();
        {
            let c = conn.lock().expect("锁获取失败");

            // 插入测试数据
            c.execute(
                r#"
                INSERT INTO decision_capacity_opportunity (
                    version_id, machine_code, plan_date,
                    slack_t, soft_adjust_space_t, utilization_rate,
                    binding_constraints, opportunity_level,
                    sensitivity, suggested_optimizations
                ) VALUES ('V001', 'H032', '2026-01-24', 800.0, 200.0, 0.6,
                    '[]', 'HIGH', NULL, '["优化建议1"]')
            "#,
                [],
            )
            .unwrap();

            c.execute(
                r#"
                INSERT INTO decision_capacity_opportunity (
                    version_id, machine_code, plan_date,
                    slack_t, soft_adjust_space_t, utilization_rate,
                    binding_constraints, opportunity_level,
                    sensitivity, suggested_optimizations
                ) VALUES ('V001', 'H033', '2026-01-24', 300.0, 100.0, 0.8,
                    '[]', 'MEDIUM', NULL, '["优化建议2"]')
            "#,
                [],
            )
            .unwrap();
        }

        let repo = CapacityOpportunityRepository::new(conn);

        // 测试：查询所有机组
        let result = repo
            .get_capacity_opportunity("V001", None, "2026-01-24", "2026-01-25")
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].slack_t, 800.0); // 按 slack_t 降序排列

        // 测试：查询特定机组
        let result = repo
            .get_capacity_opportunity("V001", Some("H032"), "2026-01-24", "2026-01-25")
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].machine_code, "H032");
    }

    #[test]
    fn test_get_top_opportunities() {
        let conn = setup_test_db();
        {
            let c = conn.lock().expect("锁获取失败");

            // 插入 5 条测试数据
            for i in 1..=5 {
                let slack_t = (1000 - i * 100) as f64;
                let level = if i <= 2 { "HIGH" } else if i <= 4 { "MEDIUM" } else { "LOW" };
                c.execute(
                    &format!(
                        r#"
                        INSERT INTO decision_capacity_opportunity (
                            version_id, machine_code, plan_date,
                            slack_t, utilization_rate,
                            opportunity_level, suggested_optimizations
                        ) VALUES ('V001', 'H03{}', '2026-01-24', {}, 0.{},
                            '{}', '[]')
                    "#,
                        i, slack_t, i * 15, level
                    ),
                    [],
                )
                .unwrap();
            }
        }

        let repo = CapacityOpportunityRepository::new(conn);

        // 测试：查询 Top 3（只包含 HIGH/MEDIUM）
        let result = repo
            .get_top_opportunities("V001", "2026-01-24", "2026-01-25", 3)
            .unwrap();
        assert_eq!(result.len(), 3);
        assert!(result[0].slack_t > result[1].slack_t); // 降序排列

        // 验证只包含 HIGH/MEDIUM
        for opp in result {
            assert!(opp.opportunity_level == "HIGH" || opp.opportunity_level == "MEDIUM");
        }
    }

    #[test]
    fn test_get_optimization_summary() {
        let conn = setup_test_db();
        {
            let c = conn.lock().expect("锁获取失败");

            // 插入多条测试数据
            c.execute(
                r#"
                INSERT INTO decision_capacity_opportunity (
                    version_id, machine_code, plan_date,
                    slack_t, soft_adjust_space_t, utilization_rate,
                    opportunity_level, suggested_optimizations
                ) VALUES ('V001', 'H032', '2026-01-24', 800.0, 200.0, 0.6,
                    'HIGH', '[]')
            "#,
                [],
            )
            .unwrap();

            c.execute(
                r#"
                INSERT INTO decision_capacity_opportunity (
                    version_id, machine_code, plan_date,
                    slack_t, soft_adjust_space_t, utilization_rate,
                    opportunity_level, suggested_optimizations
                ) VALUES ('V001', 'H032', '2026-01-25', 500.0, 100.0, 0.7,
                    'MEDIUM', '[]')
            "#,
                [],
            )
            .unwrap();

            c.execute(
                r#"
                INSERT INTO decision_capacity_opportunity (
                    version_id, machine_code, plan_date,
                    slack_t, utilization_rate,
                    opportunity_level, suggested_optimizations
                ) VALUES ('V001', 'H033', '2026-01-24', 300.0, 0.8,
                    'LOW', '[]')
            "#,
                [],
            )
            .unwrap();
        }

        let repo = CapacityOpportunityRepository::new(conn);

        // 测试：获取优化总结
        let summary = repo
            .get_optimization_summary("V001", "2026-01-24", "2026-01-25")
            .unwrap();

        assert_eq!(summary.version_id, "V001");
        assert_eq!(summary.date_range, ("2026-01-24".to_string(), "2026-01-25".to_string()));
        assert_eq!(summary.total_slack_t, 1600.0); // 800 + 500 + 300
        assert_eq!(summary.high_opportunity_count, 2); // HIGH + MEDIUM
        assert_eq!(summary.by_machine.len(), 2); // H032 和 H033

        // 验证按机组统计
        let h032_stat = summary.by_machine.iter().find(|s| s.machine_code == "H032").unwrap();
        assert_eq!(h032_stat.slack_t, 1300.0); // 800 + 500
        assert_eq!(h032_stat.opportunity_count, 2);
    }

    #[test]
    fn test_refresh_incremental() {
        let conn = setup_test_db();
        {
            let c = conn.lock().expect("锁获取失败");

            // 插入产能池数据
            c.execute(
                "INSERT INTO capacity_pool VALUES ('H032', '2026-01-24', 2000.0)",
                [],
            )
            .unwrap();
            c.execute(
                "INSERT INTO capacity_pool VALUES ('H033', '2026-01-24', 1500.0)",
                [],
            )
            .unwrap();

            // 先做全量刷新
            c.execute(
                "INSERT INTO material_state VALUES ('M001', 500.0)",
                [],
            )
            .unwrap();
            c.execute(
                "INSERT INTO plan_item VALUES ('V001', 'H032', '2026-01-24', 'M001')",
                [],
            )
            .unwrap();
        }

        let repo = CapacityOpportunityRepository::new(conn.clone());
        repo.refresh_full("V001").unwrap();

        // 测试：增量刷新特定机组
        let affected_machines = vec!["H032".to_string()];
        let affected_dates = vec![("2026-01-24".to_string(), "2026-01-24".to_string())];
        let rows = repo
            .refresh_incremental("V001", &affected_machines, &affected_dates)
            .unwrap();

        assert_eq!(rows, 1); // 应该只刷新 1 条记录

        // 验证 H033 的数据仍然存在
        let c = conn.lock().expect("锁获取失败");
        let count: i32 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_capacity_opportunity WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2); // H032 和 H033
    }
}

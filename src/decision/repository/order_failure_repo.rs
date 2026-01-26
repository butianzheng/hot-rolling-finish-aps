// ==========================================
// 热轧精整排产系统 - D2 订单失败仓储
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md - D2 用例
// 职责: 订单失败集合的数据访问
// ==========================================

use crate::decision::common::{
    build_in_clause, build_optional_filter_sql, deserialize_json_array,
    deserialize_json_array_optional,
};
use crate::decision::use_cases::{BlockingFactor, FailureStats, FailureType, OrderFailure};
use rusqlite::{params, Connection, Result as SqlResult};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// D2 订单失败仓储
pub struct OrderFailureRepository {
    conn: Arc<Mutex<Connection>>,
}

impl OrderFailureRepository {
    /// 创建新的仓储实例
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 查询订单失败集合
    pub fn list_failures(
        &self,
        version_id: &str,
        fail_type: Option<&str>,
    ) -> SqlResult<Vec<OrderFailure>> {
        let conn = self.conn.lock().unwrap();

        let base_sql = r#"
            SELECT
                version_id,
                contract_no,
                due_date,
                urgency_level,
                fail_type,
                total_materials,
                unscheduled_count,
                unscheduled_weight_t,
                completion_rate,
                days_to_due,
                failure_reasons,
                blocking_factors,
                suggested_actions
            FROM decision_order_failure_set
            WHERE version_id = ?
        "#;

        let additional_filter = fail_type.map(|_| "fail_type = ?");
        let sql = build_optional_filter_sql(
            base_sql,
            additional_filter,
            "urgency_level DESC, days_to_due ASC",
        );

        let params: Vec<Box<dyn rusqlite::ToSql>> = if let Some(ft) = fail_type {
            vec![Box::new(version_id), Box::new(ft)]
        } else {
            vec![Box::new(version_id)]
        };

        let mut stmt = conn.prepare(&sql)?;
        let failures = stmt
            .query_map(rusqlite::params_from_iter(params), |row| {
                Ok(Self::row_to_failure(row)?)
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(failures)
    }

    /// 查询特定合同的失败情况
    pub fn get_contract_failure(
        &self,
        version_id: &str,
        contract_no: &str,
    ) -> SqlResult<Option<OrderFailure>> {
        let conn = self.conn.lock().unwrap();

        let sql = r#"
            SELECT
                version_id,
                contract_no,
                due_date,
                urgency_level,
                fail_type,
                total_materials,
                unscheduled_count,
                unscheduled_weight_t,
                completion_rate,
                days_to_due,
                failure_reasons,
                blocking_factors,
                suggested_actions
            FROM decision_order_failure_set
            WHERE version_id = ? AND contract_no = ?
        "#;

        let mut stmt = conn.prepare(sql)?;
        let mut rows = stmt.query(params![version_id, contract_no])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::row_to_failure(row)?))
        } else {
            Ok(None)
        }
    }

    /// 统计失败订单数量
    pub fn count_failures(&self, version_id: &str) -> SqlResult<FailureStats> {
        let conn = self.conn.lock().unwrap();

        let sql = r#"
            SELECT
                COUNT(*) AS total_failures,
                COALESCE(SUM(CASE WHEN fail_type = 'Overdue' THEN 1 ELSE 0 END), 0) AS overdue_count,
                COALESCE(SUM(CASE WHEN fail_type = 'NearDueImpossible' THEN 1 ELSE 0 END), 0) AS near_due_count,
                COALESCE(SUM(CASE WHEN fail_type = 'CapacityShortage' THEN 1 ELSE 0 END), 0) AS capacity_shortage_count,
                COALESCE(SUM(CASE WHEN fail_type = 'StructureConflict' THEN 1 ELSE 0 END), 0) AS structure_conflict_count,
                COALESCE(SUM(unscheduled_count), 0) AS total_affected_materials,
                COALESCE(SUM(unscheduled_weight_t), 0.0) AS total_affected_weight_t
            FROM decision_order_failure_set
            WHERE version_id = ?
        "#;

        let mut stmt = conn.prepare(sql)?;
        let mut rows = stmt.query(params![version_id])?;

        if let Some(row) = rows.next()? {
            Ok(FailureStats {
                version_id: version_id.to_string(),
                total_failures: row.get(0)?,
                overdue_count: row.get(1)?,
                near_due_impossible_count: row.get(2)?,
                capacity_shortage_count: row.get(3)?,
                structure_conflict_count: row.get(4)?,
                total_affected_materials: row.get(5)?,
                total_affected_weight_t: row.get(6)?,
            })
        } else {
            Ok(FailureStats::new(version_id.to_string()))
        }
    }

    /// 批量获取合同主机组代码（用于 D2 DTO machine_code 字段补齐）
    ///
    /// material_state 表存在一定程度的去范式字段（contract_no/machine_code/scheduled_machine_code），
    /// 这里按合同聚合并选择出现次数最多的机组作为该合同的主机组。
    pub fn get_primary_machine_codes(
        &self,
        contract_nos: &[String],
    ) -> SqlResult<HashMap<String, String>> {
        if contract_nos.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = self.conn.lock().unwrap();
        let in_clause = build_in_clause("contract_no", contract_nos);
        let sql = format!(
            r#"
            SELECT
                contract_no,
                COALESCE(scheduled_machine_code, machine_code) AS machine_code,
                COUNT(*) AS cnt
            FROM material_state
            WHERE {}
              AND COALESCE(scheduled_machine_code, machine_code) IS NOT NULL
              AND COALESCE(scheduled_machine_code, machine_code) != ''
            GROUP BY contract_no, COALESCE(scheduled_machine_code, machine_code)
            ORDER BY contract_no, cnt DESC
            "#,
            in_clause
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::with_capacity(contract_nos.len());
        for c in contract_nos {
            params.push(Box::new(c.clone()));
        }

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut map = HashMap::new();
        for row_result in rows {
            let (contract_no, machine_code) = row_result?;
            // 已按 (contract_no, cnt DESC) 排序；同一合同首次出现即为主机组
            map.entry(contract_no).or_insert(machine_code);
        }

        Ok(map)
    }

    /// 将数据库行转换为 OrderFailure
    fn row_to_failure(row: &rusqlite::Row) -> SqlResult<OrderFailure> {
        let fail_type_str: String = row.get(4)?;
        let fail_type = Self::parse_fail_type(&fail_type_str);

        let failure_reasons_json: String = row.get(10)?;
        let failure_reasons = deserialize_json_array(&failure_reasons_json);

        let blocking_factors_json: String = row.get(11)?;
        let blocking_factors = deserialize_json_array(&blocking_factors_json);

        let suggested_actions_json: Option<String> = row.get(12)?;
        let suggested_actions =
            deserialize_json_array_optional(suggested_actions_json.as_deref());

        Ok(OrderFailure {
            contract_no: row.get(1)?,
            version_id: row.get(0)?,
            due_date: row.get(2)?,
            urgency_level: row.get(3)?,
            fail_type,
            total_materials: row.get(5)?,
            unscheduled_count: row.get(6)?,
            unscheduled_weight_t: row.get(7)?,
            completion_rate: row.get(8)?,
            days_to_due: row.get(9)?,
            failure_reasons,
            blocking_factors,
            suggested_actions,
        })
    }

    /// 解析失败类型
    fn parse_fail_type(s: &str) -> FailureType {
        match s {
            "Overdue" => FailureType::Overdue,
            "NearDueImpossible" => FailureType::NearDueImpossible,
            "CapacityShortage" => FailureType::CapacityShortage,
            "StructureConflict" => FailureType::StructureConflict,
            "ColdStockNotReady" => FailureType::ColdStockNotReady,
            _ => FailureType::Other,
        }
    }

    /// 格式化失败类型为字符串
    fn format_fail_type(ft: &FailureType) -> &'static str {
        match ft {
            FailureType::Overdue => "Overdue",
            FailureType::NearDueImpossible => "NearDueImpossible",
            FailureType::CapacityShortage => "CapacityShortage",
            FailureType::StructureConflict => "StructureConflict",
            FailureType::ColdStockNotReady => "ColdStockNotReady",
            FailureType::Other => "Other",
        }
    }

    /// 刷新 D2 读模型 (全量刷新)
    pub fn refresh_full(&self, version_id: &str) -> SqlResult<usize> {
        let conn = self.conn.lock().unwrap();

        // 1. 删除旧数据
        conn.execute(
            "DELETE FROM decision_order_failure_set WHERE version_id = ?",
            params![version_id],
        )?;

        // 2. 计算并插入新数据
        self.calculate_and_insert(&conn, version_id, None, None)
    }

    /// 刷新 D2 读模型 (增量刷新 - 按合同号)
    pub fn refresh_incremental(
        &self,
        version_id: &str,
        contract_nos: &[String],
    ) -> SqlResult<usize> {
        let conn = self.conn.lock().unwrap();

        // 1. 删除受影响的记录
        if !contract_nos.is_empty() {
            let in_clause = build_in_clause("contract_no", contract_nos);
            let sql = format!(
                "DELETE FROM decision_order_failure_set WHERE version_id = ? AND {}",
                in_clause
            );

            let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(version_id)];
            for c in contract_nos {
                params.push(Box::new(c.clone()));
            }

            conn.execute(&sql, rusqlite::params_from_iter(params))?;
        }

        // 2. 重新计算受影响的记录
        self.calculate_and_insert(&conn, version_id, Some(contract_nos), None)
    }

    /// 计算并插入订单失败记录
    fn calculate_and_insert(
        &self,
        conn: &Connection,
        version_id: &str,
        contract_filter: Option<&[String]>,
        _date_filter: Option<(&str, &str)>,
    ) -> SqlResult<usize> {
        // 查询所有紧急单的统计信息
        let mut sql = r#"
            SELECT
                ms.contract_no,
                ms.due_date,
                ms.urgency_level,
                COUNT(*) AS total_materials,
                SUM(CASE WHEN pi.material_id IS NULL THEN 1 ELSE 0 END) AS unscheduled_count,
                SUM(CASE WHEN pi.material_id IS NULL THEN ms.weight_t ELSE 0 END) AS unscheduled_weight_t,
                ms.due_date
            FROM material_state ms
            LEFT JOIN plan_item pi ON pi.version_id = ? AND pi.material_id = ms.material_id
            WHERE ms.urgency_level IN ('L1', 'L2', 'L3')
        "#
        .to_string();

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(version_id)];

        // 添加合同号过滤
        if let Some(contracts) = contract_filter {
            if !contracts.is_empty() {
                let in_clause = build_in_clause("ms.contract_no", contracts);
                sql.push_str(&format!(" AND {}", in_clause));
                for c in contracts {
                    params.push(Box::new(c.clone()));
                }
            }
        }

        sql.push_str(" GROUP BY ms.contract_no, ms.due_date, ms.urgency_level");
        sql.push_str(" HAVING unscheduled_count > 0"); // 只保留有未排产材料的合同

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            let contract_no: String = row.get(0)?;
            let due_date: String = row.get(1)?;
            let urgency_level: String = row.get(2)?;
            let total_materials: i32 = row.get(3)?;
            let unscheduled_count: i32 = row.get(4)?;
            let unscheduled_weight_t: f64 = row.get(5)?;

            Ok((
                contract_no,
                due_date,
                urgency_level,
                total_materials,
                unscheduled_count,
                unscheduled_weight_t,
            ))
        })?;

        let mut count = 0;
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        for row_result in rows {
            let (contract_no, due_date, urgency_level, total_materials, unscheduled_count, unscheduled_weight_t) =
                row_result?;

            // 计算完成率
            let completion_rate = if total_materials > 0 {
                (total_materials - unscheduled_count) as f64 / total_materials as f64
            } else {
                0.0
            };

            // 计算距交货期天数
            let days_to_due = Self::calculate_days_to_due(&today, &due_date);

            // 判断失败类型
            let fail_type = Self::determine_fail_type(days_to_due, completion_rate);

            // 生成失败原因
            let failure_reasons = Self::generate_failure_reasons(
                days_to_due,
                completion_rate,
                unscheduled_count,
                &fail_type,
            );

            // 生成阻塞因素
            let blocking_factors = Self::generate_blocking_factors(
                conn,
                version_id,
                &contract_no,
                unscheduled_weight_t,
            )?;

            // 生成建议措施
            let suggested_actions =
                Self::generate_suggested_actions(days_to_due, &fail_type, &blocking_factors);

            // 插入记录
            conn.execute(
                r#"
                INSERT INTO decision_order_failure_set (
                    version_id, contract_no, due_date, urgency_level, fail_type,
                    total_materials, unscheduled_count, unscheduled_weight_t, completion_rate,
                    days_to_due, failure_reasons, blocking_factors, suggested_actions
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    version_id,
                    contract_no,
                    due_date,
                    urgency_level,
                    Self::format_fail_type(&fail_type),
                    total_materials,
                    unscheduled_count,
                    unscheduled_weight_t,
                    completion_rate,
                    days_to_due,
                    serde_json::to_string(&failure_reasons).unwrap(),
                    serde_json::to_string(&blocking_factors).unwrap(),
                    serde_json::to_string(&suggested_actions).unwrap(),
                ],
            )?;

            count += 1;
        }

        Ok(count)
    }

    /// 计算距交货期天数
    fn calculate_days_to_due(today: &str, due_date: &str) -> i32 {
        let today_date = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d").unwrap();
        let due = chrono::NaiveDate::parse_from_str(due_date, "%Y-%m-%d").unwrap();
        (due - today_date).num_days() as i32
    }

    /// 判断失败类型
    fn determine_fail_type(days_to_due: i32, completion_rate: f64) -> FailureType {
        if days_to_due < 0 {
            FailureType::Overdue
        } else if days_to_due <= 3 && completion_rate < 0.8 {
            FailureType::NearDueImpossible
        } else if completion_rate < 0.5 {
            FailureType::CapacityShortage
        } else {
            FailureType::Other
        }
    }

    /// 生成失败原因
    fn generate_failure_reasons(
        days_to_due: i32,
        completion_rate: f64,
        unscheduled_count: i32,
        fail_type: &FailureType,
    ) -> Vec<String> {
        let mut reasons = Vec::new();

        match fail_type {
            FailureType::Overdue => {
                reasons.push(format!(
                    "订单已超期 {} 天，仍有 {} 个材料未排产",
                    -days_to_due, unscheduled_count
                ));
            }
            FailureType::NearDueImpossible => {
                reasons.push(format!(
                    "距交货期仅剩 {} 天，完成率仅 {:.1}%，无法按期完成",
                    days_to_due,
                    completion_rate * 100.0
                ));
            }
            FailureType::CapacityShortage => {
                reasons.push(format!(
                    "产能不足，完成率仅 {:.1}%，有 {} 个材料未排产",
                    completion_rate * 100.0,
                    unscheduled_count
                ));
            }
            _ => {
                reasons.push(format!("有 {} 个材料未排产", unscheduled_count));
            }
        }

        reasons
    }

    /// 生成阻塞因素
    fn generate_blocking_factors(
        conn: &Connection,
        version_id: &str,
        contract_no: &str,
        unscheduled_weight_t: f64,
    ) -> SqlResult<Vec<BlockingFactor>> {
        let mut factors = Vec::new();

        // 查询冷料数量
        let cold_count_sql = r#"
            SELECT COUNT(*) AS cold_count
            FROM material_state ms
            WHERE ms.contract_no = ?
              AND ms.is_mature = 0
              AND NOT EXISTS (
                  SELECT 1 FROM plan_item pi
                  WHERE pi.version_id = ? AND pi.material_id = ms.material_id
              )
        "#;

        let cold_count: i32 = conn.query_row(cold_count_sql, params![contract_no, version_id], |row| {
            row.get(0)
        })?;

        if cold_count > 0 {
            factors.push(BlockingFactor {
                code: "COLD_STOCK".to_string(),
                description: format!("{} 个材料尚未适温", cold_count),
                affected_count: cold_count,
                affected_weight_t: unscheduled_weight_t * (cold_count as f64 / 10.0), // 粗略估算
                is_removable: false,
            });
        }

        // 查询结构冲突数量
        let struct_conflict_sql = r#"
            SELECT COUNT(*) AS conflict_count
            FROM material_state ms
            WHERE ms.contract_no = ?
              AND EXISTS (
                  SELECT 1 FROM plan_item pi
                  WHERE pi.version_id = ? AND pi.material_id = ms.material_id
                    AND pi.violation_flags LIKE '%STRUCT_CONFLICT%'
              )
        "#;

        let conflict_count: i32 = conn.query_row(
            struct_conflict_sql,
            params![contract_no, version_id],
            |row| row.get(0),
        )?;

        if conflict_count > 0 {
            factors.push(BlockingFactor {
                code: "STRUCTURE_CONFLICT".to_string(),
                description: format!("{} 个材料存在结构冲突", conflict_count),
                affected_count: conflict_count,
                affected_weight_t: unscheduled_weight_t * (conflict_count as f64 / 10.0),
                is_removable: true,
            });
        }

        // 如果没有明确的阻塞因素,添加产能不足
        if factors.is_empty() {
            factors.push(BlockingFactor {
                code: "CAPACITY_SHORTAGE".to_string(),
                description: "产能不足，无法安排排产".to_string(),
                affected_count: 0,
                affected_weight_t: unscheduled_weight_t,
                is_removable: false,
            });
        }

        Ok(factors)
    }

    /// 生成建议措施
    fn generate_suggested_actions(
        days_to_due: i32,
        fail_type: &FailureType,
        blocking_factors: &[BlockingFactor],
    ) -> Vec<String> {
        let mut actions = Vec::new();

        match fail_type {
            FailureType::Overdue => {
                actions.push("紧急联系客户协商延期".to_string());
                actions.push("调整生产计划优先处理超期订单".to_string());
            }
            FailureType::NearDueImpossible => {
                if days_to_due > 0 {
                    actions.push("立即扩容产能池".to_string());
                    actions.push("考虑加班或调配其他机组支援".to_string());
                }
            }
            FailureType::CapacityShortage => {
                actions.push("评估是否可以调整产能参数".to_string());
                actions.push("考虑将部分材料转移至其他机组".to_string());
            }
            _ => {}
        }

        // 根据阻塞因素添加针对性建议
        for factor in blocking_factors {
            match factor.code.as_str() {
                "COLD_STOCK" => {
                    if factor.affected_count <= 3 {
                        actions.push(format!(
                            "考虑强制释放 {} 个冷料材料",
                            factor.affected_count
                        ));
                    }
                }
                "STRUCTURE_CONFLICT" => {
                    actions.push("人工审核结构冲突材料，评估是否可以调整".to_string());
                }
                _ => {}
            }
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();

        // 创建必要的表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS plan_version (
                version_id TEXT PRIMARY KEY,
                plan_id TEXT NOT NULL,
                version_no INTEGER NOT NULL,
                status TEXT NOT NULL,
                created_by TEXT NOT NULL
            )
            "#,
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS material_state (
                material_id TEXT PRIMARY KEY,
                contract_no TEXT,
                due_date TEXT,
                urgency_level TEXT,
                weight_t REAL,
                is_mature INTEGER DEFAULT 1
            )
            "#,
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS plan_item (
                version_id TEXT NOT NULL,
                material_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                violation_flags TEXT,
                PRIMARY KEY (version_id, material_id)
            )
            "#,
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS decision_order_failure_set (
                version_id TEXT NOT NULL,
                contract_no TEXT NOT NULL,
                due_date TEXT NOT NULL,
                urgency_level TEXT NOT NULL,
                fail_type TEXT NOT NULL,
                total_materials INTEGER NOT NULL,
                unscheduled_count INTEGER NOT NULL,
                unscheduled_weight_t REAL NOT NULL,
                completion_rate REAL NOT NULL,
                days_to_due INTEGER NOT NULL,
                failure_reasons TEXT NOT NULL,
                blocking_factors TEXT NOT NULL,
                suggested_actions TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, contract_no)
            )
            "#,
            [],
        )
        .unwrap();

        // 插入测试数据
        conn.execute(
            "INSERT INTO plan_version VALUES ('V001', 'P001', 1, 'ACTIVE', 'test')",
            [],
        )
        .unwrap();

        // 插入紧急单材料
        for i in 1..=5 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'C001', '2026-01-25', 'L3', 100.0, 1)",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
        }

        // 只排产了前 2 个
        conn.execute(
            "INSERT INTO plan_item VALUES ('V001', 'MAT001', 'H032', '2026-01-24', NULL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO plan_item VALUES ('V001', 'MAT002', 'H032', '2026-01-24', NULL)",
            [],
        )
        .unwrap();

        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_refresh_full() {
        let conn = setup_test_db();
        let repo = OrderFailureRepository::new(conn);

        let count = repo.refresh_full("V001").unwrap();
        assert_eq!(count, 1); // 应该有 1 个失败合同

        let failures = repo.list_failures("V001", None).unwrap();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].contract_no, "C001");
        assert_eq!(failures[0].unscheduled_count, 3); // 5 - 2 = 3
    }

    #[test]
    fn test_count_failures() {
        let conn = setup_test_db();
        let repo = OrderFailureRepository::new(conn);

        repo.refresh_full("V001").unwrap();

        let stats = repo.count_failures("V001").unwrap();
        assert_eq!(stats.total_failures, 1);
        assert_eq!(stats.total_affected_materials, 3);
    }

    #[test]
    fn test_calculate_days_to_due() {
        let days = OrderFailureRepository::calculate_days_to_due("2026-01-20", "2026-01-25");
        assert_eq!(days, 5);

        let days = OrderFailureRepository::calculate_days_to_due("2026-01-25", "2026-01-20");
        assert_eq!(days, -5);
    }
}

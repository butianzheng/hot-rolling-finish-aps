// ==========================================
// 热轧精整排产系统 - D1 日期摘要仓储
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md - D1 用例
// 职责: 查询日期风险摘要数据
// ==========================================
// P2 阶段: 优先从 decision_day_summary 读模型表读取
//         如果读模型表为空，回退到 risk_snapshot 实时计算
// ==========================================

use crate::decision::common::sql_builder::SqlQueryBuilder;
use crate::decision::use_cases::d1_most_risky_day::DaySummary;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

/// D1 日期摘要仓储
///
/// 职责: 查询日期风险摘要数据
/// 策略: 优先从 decision_day_summary 读模型表读取，回退到 risk_snapshot 实时计算
pub struct DaySummaryRepository {
    conn: Arc<Mutex<Connection>>,
}

impl DaySummaryRepository {
    /// 创建新的 DaySummaryRepository 实例
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 查询日期风险摘要
    ///
    /// 策略: 优先从 decision_day_summary 读模型表读取，如果为空则回退到 risk_snapshot 实时计算
    ///
    /// # 参数
    /// - version_id: 方案版本 ID
    /// - start_date: 开始日期
    /// - end_date: 结束日期
    ///
    /// # 返回
    /// - Ok(Vec<DaySummary>): 日期摘要列表，按风险分数降序排列
    /// - Err: 数据库错误
    pub fn get_day_summary(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<DaySummary>, Box<dyn Error>> {
        // 优先尝试从读模型表读取
        if let Ok(summaries) = self.get_day_summary_from_read_model(version_id, start_date, end_date) {
            if !summaries.is_empty() {
                tracing::debug!(
                    version_id = version_id,
                    count = summaries.len(),
                    "D1: 从 decision_day_summary 读模型表读取"
                );
                return Ok(summaries);
            }
        }

        // 回退到 risk_snapshot 实时计算
        tracing::debug!(
            version_id = version_id,
            "D1: 回退到 risk_snapshot 实时计算"
        );
        self.get_day_summary_realtime(version_id, start_date, end_date)
    }

    /// 从 decision_day_summary 读模型表读取（P2 优先路径）
    fn get_day_summary_from_read_model(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<DaySummary>, Box<dyn Error>> {
        let conn = self.conn.lock().unwrap();

        let sql = SqlQueryBuilder::new(
            r#"SELECT
                plan_date,
                risk_score,
                risk_level,
                capacity_util_pct,
                top_reasons,
                affected_machines,
                bottleneck_machines,
                has_roll_risk,
                suggested_actions
            FROM decision_day_summary"#,
        )
        .where_clause("version_id = ?")
        .where_clause("plan_date >= ?")
        .where_clause("plan_date <= ?")
        .order_by("risk_score DESC")
        .build();

        let mut stmt = conn.prepare(&sql)?;

        let summaries = stmt
            .query_map(params![version_id, start_date, end_date], |row| {
                let plan_date: String = row.get(0)?;
                let risk_score: f64 = row.get(1)?;
                let risk_level: String = row.get(2)?;
                let capacity_util_pct: f64 = row.get(3)?;
                let top_reasons: String = row.get(4)?;
                let affected_machines: i32 = row.get(5)?;
                let bottleneck_machines: i32 = row.get(6)?;
                let has_roll_risk: i32 = row.get(7)?;
                let suggested_actions: Option<String> = row.get(8)?;

                let mut summary = DaySummary::new(plan_date);
                summary.risk_score = risk_score;
                summary.risk_level = risk_level;
                summary.capacity_util_pct = capacity_util_pct;
                summary.affected_machines = affected_machines;
                summary.bottleneck_machines = bottleneck_machines;
                summary.has_roll_risk = has_roll_risk != 0;

                // 解析 JSON 原因
                if let Ok(reasons) = serde_json::from_str::<Vec<serde_json::Value>>(&top_reasons) {
                    for reason in reasons {
                        if let (Some(code), Some(msg), Some(weight), Some(severity)) = (
                            reason.get("code").and_then(|v| v.as_str()),
                            reason.get("msg").and_then(|v| v.as_str()),
                            reason.get("weight").and_then(|v| v.as_f64()),
                            reason.get("severity").and_then(|v| v.as_f64()),
                        ) {
                            summary.add_reason(code.to_string(), msg.to_string(), weight, severity);
                        }
                    }
                }

                // 解析建议措施
                if let Some(actions) = suggested_actions {
                    if let Ok(action_list) = serde_json::from_str::<Vec<String>>(&actions) {
                        for action in action_list {
                            summary.add_suggested_action(action);
                        }
                    }
                }

                Ok(summary)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(summaries)
    }

    /// 从 risk_snapshot 表实时计算（P1 回退路径）
    fn get_day_summary_realtime(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<DaySummary>, Box<dyn Error>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"
            SELECT
                snapshot_date,
                machine_code,
                risk_level,
                risk_reasons,
                target_capacity_t,
                used_capacity_t,
                limit_capacity_t,
                overflow_t,
                urgent_total_t
            FROM risk_snapshot
            WHERE version_id = ?1
              AND snapshot_date BETWEEN ?2 AND ?3
            ORDER BY snapshot_date ASC
            "#,
        )?;

        let rows = stmt.query_map(params![version_id, start_date, end_date], |row| {
            Ok((
                row.get::<_, String>(0)?, // snapshot_date
                row.get::<_, String>(1)?, // machine_code
                row.get::<_, String>(2)?, // risk_level
                row.get::<_, String>(3)?, // risk_reasons
                row.get::<_, f64>(4)?,    // target_capacity_t
                row.get::<_, f64>(5)?,    // used_capacity_t
                row.get::<_, f64>(6)?,    // limit_capacity_t
                row.get::<_, f64>(7)?,    // overflow_t
                row.get::<_, f64>(8)?,    // urgent_total_t
            ))
        })?;

        // 按日期聚合数据
        let mut day_map: HashMap<String, DayAggregateData> = HashMap::new();

        for row_result in rows {
            let (snapshot_date, machine_code, risk_level, risk_reasons,
                 target_capacity_t, used_capacity_t, limit_capacity_t,
                 overflow_t, urgent_total_t) = row_result?;

            let entry = day_map.entry(snapshot_date.clone()).or_insert_with(|| {
                DayAggregateData::new(snapshot_date.clone())
            });

            entry.add_machine_data(
                machine_code,
                risk_level,
                risk_reasons,
                target_capacity_t,
                used_capacity_t,
                limit_capacity_t,
                overflow_t,
                urgent_total_t,
            );
        }

        // 转换为 DaySummary 并排序
        let mut summaries: Vec<DaySummary> = day_map
            .into_values()
            .map(|data| data.into_day_summary())
            .collect();

        // 按风险分数降序排序
        summaries.sort_by(|a, b| {
            b.risk_score
                .partial_cmp(&a.risk_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(summaries)
    }

    /// 查询最危险的 N 天
    pub fn get_top_risky_days(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        top_n: usize,
    ) -> Result<Vec<DaySummary>, Box<dyn Error>> {
        let mut summaries = self.get_day_summary(version_id, start_date, end_date)?;
        summaries.truncate(top_n);
        Ok(summaries)
    }
}

/// 日期聚合数据（中间结构）
struct DayAggregateData {
    plan_date: String,
    machine_risk_levels: Vec<String>,
    machine_codes: Vec<String>,
    total_target_capacity_t: f64,
    total_used_capacity_t: f64,
    total_overflow_t: f64,
    total_urgent_t: f64,
    risk_reasons: Vec<String>,
}

impl DayAggregateData {
    fn new(plan_date: String) -> Self {
        Self {
            plan_date,
            machine_risk_levels: Vec::new(),
            machine_codes: Vec::new(),
            total_target_capacity_t: 0.0,
            total_used_capacity_t: 0.0,
            total_overflow_t: 0.0,
            total_urgent_t: 0.0,
            risk_reasons: Vec::new(),
        }
    }

    fn add_machine_data(
        &mut self,
        machine_code: String,
        risk_level: String,
        risk_reasons: String,
        target_capacity_t: f64,
        used_capacity_t: f64,
        _limit_capacity_t: f64,
        overflow_t: f64,
        urgent_total_t: f64,
    ) {
        self.machine_codes.push(machine_code);
        self.machine_risk_levels.push(risk_level);
        self.total_target_capacity_t += target_capacity_t;
        self.total_used_capacity_t += used_capacity_t;
        self.total_overflow_t += overflow_t;
        self.total_urgent_t += urgent_total_t;

        if !risk_reasons.is_empty() {
            self.risk_reasons.push(risk_reasons);
        }
    }

    fn into_day_summary(self) -> DaySummary {
        let mut summary = DaySummary::new(self.plan_date);

        // 计算产能利用率
        let capacity_util_pct = if self.total_target_capacity_t > 0.0 {
            self.total_used_capacity_t / self.total_target_capacity_t
        } else {
            0.0
        };

        // 统计堵塞机组数量（风险等级为 HIGH 或 CRITICAL）
        let bottleneck_count = self
            .machine_risk_levels
            .iter()
            .filter(|level| matches!(level.as_str(), "HIGH" | "CRITICAL"))
            .count() as i32;

        // 设置产能信息
        summary.set_capacity_info(
            capacity_util_pct,
            self.machine_codes.len() as i32,
            bottleneck_count,
        );

        // 添加风险原因
        if self.total_overflow_t > 0.0 {
            summary.add_reason(
                "CAPACITY_OVERFLOW".to_string(),
                format!("产能超载 {:.1}t", self.total_overflow_t),
                0.5,
                0.9,
            );
        }

        if capacity_util_pct > 0.95 {
            summary.add_reason(
                "HIGH_UTILIZATION".to_string(),
                format!("产能利用率高 {:.1}%", capacity_util_pct * 100.0),
                0.3,
                0.8,
            );
        }

        if bottleneck_count > 0 {
            summary.add_reason(
                "BOTTLENECK".to_string(),
                format!("{} 个机组存在堵塞", bottleneck_count),
                0.4,
                0.85,
            );
        }

        // 解析并添加来自 risk_snapshot 的原因
        for reason in self.risk_reasons {
            summary.add_reason(
                "RISK_FACTOR".to_string(),
                reason,
                0.2,
                0.7,
            );
        }

        // 添加建议措施
        if summary.is_high_risk() {
            if self.total_overflow_t > 0.0 {
                summary.add_suggested_action("调整产能池上限".to_string());
            }
            if bottleneck_count > 0 {
                summary.add_suggested_action("优化机组负载分配".to_string());
            }
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();

        // 创建 risk_snapshot 表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS risk_snapshot (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                snapshot_date TEXT NOT NULL,
                risk_level TEXT NOT NULL,
                risk_reasons TEXT,
                target_capacity_t REAL NOT NULL,
                used_capacity_t REAL NOT NULL,
                limit_capacity_t REAL NOT NULL,
                overflow_t REAL NOT NULL,
                urgent_total_t REAL NOT NULL,
                mature_backlog_t REAL,
                immature_backlog_t REAL,
                campaign_status TEXT,
                created_at TEXT NOT NULL,
                PRIMARY KEY (version_id, machine_code, snapshot_date)
            )
            "#,
            [],
        )
        .unwrap();

        Arc::new(Mutex::new(conn))
    }

    fn insert_test_data(conn: &Connection) {
        conn.execute(
            r#"
            INSERT INTO risk_snapshot (
                version_id, machine_code, snapshot_date, risk_level, risk_reasons,
                target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t, urgent_total_t,
                mature_backlog_t, immature_backlog_t, campaign_status, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
            "#,
            params![
                "V001", "H032", "2026-01-24", "HIGH", "产能紧张",
                1500.0, 1450.0, 2000.0, 0.0, 800.0,
                500.0, 200.0, "OK",
            ],
        )
        .unwrap();

        conn.execute(
            r#"
            INSERT INTO risk_snapshot (
                version_id, machine_code, snapshot_date, risk_level, risk_reasons,
                target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t, urgent_total_t,
                mature_backlog_t, immature_backlog_t, campaign_status, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
            "#,
            params![
                "V001", "H033", "2026-01-24", "CRITICAL", "严重超载",
                1500.0, 1800.0, 2000.0, 300.0, 1000.0,
                600.0, 300.0, "WARNING",
            ],
        )
        .unwrap();
    }

    #[test]
    fn test_get_day_summary() {
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().unwrap();
            insert_test_data(&conn);
        }

        let repo = DaySummaryRepository::new(conn_arc);
        let summaries = repo
            .get_day_summary("V001", "2026-01-24", "2026-01-24")
            .unwrap();

        assert_eq!(summaries.len(), 1);
        let summary = &summaries[0];
        assert_eq!(summary.plan_date, "2026-01-24");
        assert!(summary.risk_score > 0.0);
        assert_eq!(summary.affected_machines, 2);
        assert_eq!(summary.bottleneck_machines, 2);
    }

    #[test]
    fn test_get_top_risky_days() {
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().unwrap();
            insert_test_data(&conn);
        }

        let repo = DaySummaryRepository::new(conn_arc);
        let summaries = repo
            .get_top_risky_days("V001", "2026-01-24", "2026-01-24", 1)
            .unwrap();

        assert_eq!(summaries.len(), 1);
    }
}

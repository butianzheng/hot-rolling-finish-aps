// ==========================================
// 热轧精整排产系统 - D5 仓储实现
// ==========================================
// 依据: DECISION_READ_MODELS.md - D5 表设计
// 职责: "换辊是否异常" 数据访问层
// ==========================================

use crate::decision::common::{
    build_in_clause, build_optional_filter_sql, deserialize_json_array_optional, serialize_json_vec,
};
use crate::decision::use_cases::d5_roll_campaign_alert::{
    MachineRollStat, RollAlert, RollAlertSummary,
};
use rusqlite::{params, Connection, Result as SqlResult};
use std::sync::{Arc, Mutex};

/// D5 仓储：换辊预警
pub struct RollAlertRepository {
    /// 数据库连接
    conn: Arc<Mutex<Connection>>,
}

impl RollAlertRepository {
    /// 创建新的换辊预警仓储
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        let repo = Self { conn };
        let _ = repo.ensure_schema();
        repo
    }

    fn ensure_schema(&self) -> SqlResult<()> {
        let conn = self.conn.lock().expect("锁获取失败");

        // Ensure extension columns exist so old DBs remain queryable.
        conn.execute_batch(
            r#"
            ALTER TABLE decision_roll_campaign_alert ADD COLUMN campaign_start_at TEXT;
            "#,
        )
        .ok();
        conn.execute_batch(
            r#"
            ALTER TABLE decision_roll_campaign_alert ADD COLUMN planned_change_at TEXT;
            "#,
        )
        .ok();
        conn.execute_batch(
            r#"
            ALTER TABLE decision_roll_campaign_alert ADD COLUMN planned_downtime_minutes INTEGER;
            "#,
        )
        .ok();
        conn.execute_batch(
            r#"
            ALTER TABLE decision_roll_campaign_alert ADD COLUMN estimated_soft_reach_at TEXT;
            "#,
        )
        .ok();
        conn.execute_batch(
            r#"
            ALTER TABLE decision_roll_campaign_alert ADD COLUMN estimated_hard_reach_at TEXT;
            "#,
        )
        .ok();

        Ok(())
    }

    /// 查询换辊预警列表
    ///
    /// # 参数
    /// - `version_id`: 方案版本 ID
    /// - `alert_level`: 可选预警等级 (None 表示所有等级)
    ///
    /// # 返回
    /// 按 alert_level 降序的换辊预警列表
    pub fn list_roll_campaign_alerts(
        &self,
        version_id: &str,
        alert_level: Option<&str>,
    ) -> SqlResult<Vec<RollAlert>> {
        let conn = self.conn.lock().expect("锁获取失败");

        let base_sql = r#"
            SELECT
                version_id,
                machine_code,
                campaign_no,
                cum_weight_t,
                suggest_threshold_t,
                hard_limit_t,
                alert_level,
                reason,
                distance_to_suggest,
                distance_to_hard,
                utilization_rate,
                estimated_change_date,
                needs_immediate_change,
                suggested_actions,
                campaign_start_at,
                planned_change_at,
                planned_downtime_minutes,
                estimated_soft_reach_at,
                estimated_hard_reach_at
            FROM decision_roll_campaign_alert
            WHERE version_id = ?
        "#;

        let additional_filter = alert_level.map(|_| "alert_level = ?");
        let sql = build_optional_filter_sql(
            base_sql,
            additional_filter,
            "CASE alert_level WHEN 'EMERGENCY' THEN 4 WHEN 'CRITICAL' THEN 3 WHEN 'WARNING' THEN 2 ELSE 1 END DESC, utilization_rate DESC",
        );

        let map_row = |row: &rusqlite::Row| -> SqlResult<RollAlert> {
            let reason: Option<String> = row.get(7)?;
            let estimated_change_date: Option<String> = row.get(11)?;
            let needs_immediate_change: i32 = row.get(12)?;

            let suggested_actions_json: Option<String> = row.get(13)?;
            let suggested_actions: Vec<String> =
                deserialize_json_array_optional(suggested_actions_json.as_deref());

            Ok(RollAlert {
                version_id: row.get(0)?,
                machine_code: row.get(1)?,
                campaign_no: row.get(2)?,
                cum_weight_t: row.get(3)?,
                suggest_threshold_t: row.get(4)?,
                hard_limit_t: row.get(5)?,
                alert_level: row.get(6)?,
                reason,
                distance_to_suggest: row.get(8)?,
                distance_to_hard: row.get(9)?,
                utilization_rate: row.get(10)?,
                estimated_change_date,
                campaign_start_at: row.get(14)?,
                planned_change_at: row.get(15)?,
                planned_downtime_minutes: row.get(16)?,
                estimated_soft_reach_at: row.get(17)?,
                estimated_hard_reach_at: row.get(18)?,
                needs_immediate_change: needs_immediate_change != 0,
                suggested_actions,
            })
        };

        let mut stmt = conn.prepare(&sql)?;
        let rows = if let Some(level) = alert_level {
            stmt.query_map(params![version_id, level], map_row)?
        } else {
            stmt.query_map(params![version_id], map_row)?
        };

        rows.collect()
    }

    /// 查询特定机组的换辊预警
    pub fn get_machine_roll_alerts(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> SqlResult<Vec<RollAlert>> {
        let conn = self.conn.lock().expect("锁获取失败");

        let sql = r#"
            SELECT
                version_id,
                machine_code,
                campaign_no,
                cum_weight_t,
                suggest_threshold_t,
                hard_limit_t,
                alert_level,
                reason,
                distance_to_suggest,
                distance_to_hard,
                utilization_rate,
                estimated_change_date,
                needs_immediate_change,
                suggested_actions,
                campaign_start_at,
                planned_change_at,
                planned_downtime_minutes,
                estimated_soft_reach_at,
                estimated_hard_reach_at
            FROM decision_roll_campaign_alert
            WHERE version_id = ? AND machine_code = ?
            ORDER BY campaign_no DESC
        "#;

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params![version_id, machine_code], |row| {
            let reason: Option<String> = row.get(7)?;
            let estimated_change_date: Option<String> = row.get(11)?;
            let needs_immediate_change: i32 = row.get(12)?;

            let suggested_actions_json: Option<String> = row.get(13)?;
            let suggested_actions: Vec<String> =
                deserialize_json_array_optional(suggested_actions_json.as_deref());

            Ok(RollAlert {
                version_id: row.get(0)?,
                machine_code: row.get(1)?,
                campaign_no: row.get(2)?,
                cum_weight_t: row.get(3)?,
                suggest_threshold_t: row.get(4)?,
                hard_limit_t: row.get(5)?,
                alert_level: row.get(6)?,
                reason,
                distance_to_suggest: row.get(8)?,
                distance_to_hard: row.get(9)?,
                utilization_rate: row.get(10)?,
                estimated_change_date,
                campaign_start_at: row.get(14)?,
                planned_change_at: row.get(15)?,
                planned_downtime_minutes: row.get(16)?,
                estimated_soft_reach_at: row.get(17)?,
                estimated_hard_reach_at: row.get(18)?,
                needs_immediate_change: needs_immediate_change != 0,
                suggested_actions,
            })
        })?;

        rows.collect()
    }

    /// 统计换辊预警
    pub fn get_roll_alert_summary(&self, version_id: &str) -> SqlResult<RollAlertSummary> {
        let conn = self.conn.lock().expect("锁获取失败");

        // 查询总体统计
        let mut stmt = conn.prepare(
            r#"
            SELECT
                COUNT(*) AS total_alerts,
                SUM(CASE WHEN alert_level = 'EMERGENCY' THEN 1 ELSE 0 END) AS emergency_count,
                SUM(CASE WHEN alert_level = 'CRITICAL' THEN 1 ELSE 0 END) AS critical_count,
                SUM(CASE WHEN alert_level = 'WARNING' THEN 1 ELSE 0 END) AS warning_count,
                SUM(CASE WHEN needs_immediate_change = 1 THEN 1 ELSE 0 END) AS immediate_change_needed
            FROM decision_roll_campaign_alert
            WHERE version_id = ? AND alert_level != 'NONE'
        "#,
        )?;

        let (total_alerts, emergency_count, critical_count, warning_count, immediate_change_needed) =
            stmt.query_row(params![version_id], |row| {
                Ok((
                    row.get::<_, Option<i32>>(0)?.unwrap_or(0),
                    row.get::<_, Option<i32>>(1)?.unwrap_or(0),
                    row.get::<_, Option<i32>>(2)?.unwrap_or(0),
                    row.get::<_, Option<i32>>(3)?.unwrap_or(0),
                    row.get::<_, Option<i32>>(4)?.unwrap_or(0),
                ))
            })?;

        // 按机组分组统计
        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code,
                campaign_no,
                cum_weight_t,
                utilization_rate,
                alert_level
            FROM decision_roll_campaign_alert
            WHERE version_id = ?
            ORDER BY alert_level DESC, utilization_rate DESC
        "#,
        )?;

        let by_machine: Vec<MachineRollStat> = stmt
            .query_map(params![version_id], |row| {
                Ok(MachineRollStat {
                    machine_code: row.get(0)?,
                    campaign_no: row.get(1)?,
                    cum_weight_t: row.get(2)?,
                    utilization_rate: row.get(3)?,
                    alert_level: row.get(4)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(RollAlertSummary {
            version_id: version_id.to_string(),
            total_alerts,
            emergency_count,
            critical_count,
            warning_count,
            immediate_change_needed,
            by_machine,
        })
    }

    /// 全量刷新 D5 读模型
    ///
    /// # 逻辑
    /// 1. 查询所有活跃的换辊批次 (从 roller_campaign 表)
    /// 2. 计算每个批次的累计重量 (从 plan_item 表聚合)
    /// 3. 计算预警等级和指标
    /// 4. 生成建议措施
    pub fn refresh_full(&self, version_id: &str) -> SqlResult<usize> {
        let conn = self.conn.lock().expect("锁获取失败");

        // 1. 删除旧数据
        conn.execute(
            "DELETE FROM decision_roll_campaign_alert WHERE version_id = ?",
            params![version_id],
        )?;

        // 2. 查询活跃的换辊批次（以 end_date IS NULL 作为“进行中”判定）
        let campaigns = self.query_active_campaigns(&conn, version_id)?;

        // 3. 对每个批次计算累计重量和预警
        let mut alerts = Vec::new();
        for campaign in campaigns {
            let cum_weight =
                self.calculate_cum_weight(&conn, version_id, &campaign.machine_code)?;

            let mut alert = RollAlert::new(
                version_id.to_string(),
                campaign.machine_code.clone(),
                campaign.campaign_no,
                cum_weight,
                campaign.suggest_threshold_t,
                campaign.hard_limit_t,
            );

            // 生成建议措施
            generate_suggestions(&mut alert);

            alerts.push(alert);
        }

        // 4. 插入到数据库
        for alert in &alerts {
            self.insert_alert(&conn, alert)?;
        }

        Ok(alerts.len())
    }

    /// 增量刷新 D5 读模型 (按机组)
    ///
    /// # 参数
    /// - `version_id`: 方案版本 ID
    /// - `machine_codes`: 受影响的机组列表
    pub fn refresh_incremental(
        &self,
        version_id: &str,
        machine_codes: &[String],
    ) -> SqlResult<usize> {
        if machine_codes.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.lock().expect("锁获取失败");

        // 1. 删除受影响机组的记录
        let in_clause = build_in_clause("machine_code", machine_codes);
        let delete_sql = format!(
            "DELETE FROM decision_roll_campaign_alert WHERE version_id = ? AND {}",
            in_clause
        );

        let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&version_id];
        for mc in machine_codes {
            params_vec.push(mc);
        }

        conn.execute(&delete_sql, params_vec.as_slice())?;

        // 2. 查询受影响机组的活跃批次（以 end_date IS NULL 作为“进行中”判定）
        let campaigns = self.query_campaigns_by_machines(&conn, version_id, machine_codes)?;

        // 3. 计算预警
        let mut alerts = Vec::new();
        for campaign in campaigns {
            let cum_weight =
                self.calculate_cum_weight(&conn, version_id, &campaign.machine_code)?;

            let mut alert = RollAlert::new(
                version_id.to_string(),
                campaign.machine_code.clone(),
                campaign.campaign_no,
                cum_weight,
                campaign.suggest_threshold_t,
                campaign.hard_limit_t,
            );

            generate_suggestions(&mut alert);
            alerts.push(alert);
        }

        // 4. 插入到数据库
        for alert in &alerts {
            self.insert_alert(&conn, alert)?;
        }

        Ok(alerts.len())
    }

    /// 插入换辊预警记录
    fn insert_alert(&self, conn: &Connection, alert: &RollAlert) -> SqlResult<()> {
        let suggested_actions_json = serialize_json_vec(&alert.suggested_actions);

        conn.execute(
            r#"
            INSERT INTO decision_roll_campaign_alert (
                version_id, machine_code, campaign_no,
                cum_weight_t, suggest_threshold_t, hard_limit_t,
                alert_level, reason,
                distance_to_suggest, distance_to_hard,
                utilization_rate,
                estimated_change_date,
                needs_immediate_change,
                suggested_actions
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
            params![
                alert.version_id,
                alert.machine_code,
                alert.campaign_no,
                alert.cum_weight_t,
                alert.suggest_threshold_t,
                alert.hard_limit_t,
                alert.alert_level,
                alert.reason,
                alert.distance_to_suggest,
                alert.distance_to_hard,
                alert.utilization_rate,
                alert.estimated_change_date,
                if alert.needs_immediate_change { 1 } else { 0 },
                suggested_actions_json,
            ],
        )?;

        Ok(())
    }

    /// 查询所有活跃的换辊批次
    fn query_active_campaigns(
        &self,
        conn: &Connection,
        version_id: &str,
    ) -> SqlResult<Vec<RollCampaign>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code,
                campaign_no,
                suggest_threshold_t,
                hard_limit_t
            FROM roller_campaign
            WHERE version_id = ?1
              AND end_date IS NULL
            ORDER BY machine_code, campaign_no DESC
        "#,
        )?;

        let campaigns = stmt
            .query_map(params![version_id], |row| {
                Ok(RollCampaign {
                    machine_code: row.get(0)?,
                    campaign_no: row.get(1)?,
                    suggest_threshold_t: row.get(2)?,
                    hard_limit_t: row.get(3)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(campaigns)
    }

    /// 查询特定机组的活跃批次
    fn query_campaigns_by_machines(
        &self,
        conn: &Connection,
        version_id: &str,
        machine_codes: &[String],
    ) -> SqlResult<Vec<RollCampaign>> {
        let in_clause = build_in_clause("machine_code", machine_codes);
        let sql = format!(
            r#"
            SELECT
                machine_code,
                campaign_no,
                suggest_threshold_t,
                hard_limit_t
            FROM roller_campaign
            WHERE version_id = ? AND end_date IS NULL AND {}
            ORDER BY machine_code, campaign_no DESC
        "#,
            in_clause
        );

        let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&version_id];
        for mc in machine_codes {
            params_vec.push(mc);
        }

        let mut stmt = conn.prepare(&sql)?;
        let campaigns = stmt
            .query_map(params_vec.as_slice(), |row| {
                Ok(RollCampaign {
                    machine_code: row.get(0)?,
                    campaign_no: row.get(1)?,
                    suggest_threshold_t: row.get(2)?,
                    hard_limit_t: row.get(3)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(campaigns)
    }

    /// 计算累计重量
    fn calculate_cum_weight(
        &self,
        conn: &Connection,
        version_id: &str,
        machine_code: &str,
    ) -> SqlResult<f64> {
        let mut stmt = conn.prepare(
            r#"
            SELECT COALESCE(SUM(ms.weight_t), 0.0) AS cum_weight_t
            FROM plan_item pi
            JOIN material_state ms ON ms.material_id = pi.material_id
            WHERE pi.version_id = ? AND pi.machine_code = ?
        "#,
        )?;

        let cum_weight: f64 =
            stmt.query_row(params![version_id, machine_code], |row| row.get(0))?;

        Ok(cum_weight)
    }
}

/// 换辊批次信息
#[derive(Debug)]
struct RollCampaign {
    machine_code: String,
    campaign_no: i32,
    suggest_threshold_t: f64,
    hard_limit_t: f64,
}

/// 生成建议措施
fn generate_suggestions(alert: &mut RollAlert) {
    match alert.alert_level.as_str() {
        "EMERGENCY" => {
            alert
                .suggested_actions
                .push("紧急: 立即安排换辊,暂停排产".to_string());
            alert
                .suggested_actions
                .push("通知维护团队准备换辊作业".to_string());
            alert
                .suggested_actions
                .push("检查换辊物料和工具是否就位".to_string());
        }
        "CRITICAL" => {
            if alert.needs_immediate_change {
                alert
                    .suggested_actions
                    .push("尽快安排换辊,避免超过硬限制".to_string());
                alert
                    .suggested_actions
                    .push("评估换辊窗口期,制定换辊计划".to_string());
                // 如果同时超过建议阈值,也添加相关建议
                if alert.cum_weight_t >= alert.suggest_threshold_t {
                    alert.suggested_actions.push(format!(
                        "已超过建议阈值 {:.1} 吨",
                        alert.suggest_threshold_t
                    ));
                }
            } else {
                alert.suggested_actions.push(format!(
                    "已超过建议阈值 {:.1} 吨,建议尽快换辊",
                    alert.suggest_threshold_t
                ));
                alert
                    .suggested_actions
                    .push("评估辊套磨损情况,确定换辊时间".to_string());
            }
        }
        "WARNING" => {
            alert.suggested_actions.push(format!(
                "接近建议阈值,剩余容量 {:.1} 吨",
                alert.remaining_capacity()
            ));
            alert
                .suggested_actions
                .push("关注累计吨数,提前准备换辊物料".to_string());
        }
        "NONE" => {
            alert.suggested_actions.push(format!(
                "换辊状态正常,剩余容量 {:.1} 吨",
                alert.remaining_capacity()
            ));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_refresh_full() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::configure_sqlite_connection(&conn).unwrap();

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
                machine_code TEXT NOT NULL,
                weight_t REAL NOT NULL
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
                PRIMARY KEY (version_id, material_id)
            )
            "#,
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS roller_campaign (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                campaign_no INTEGER NOT NULL,
                start_date TEXT NOT NULL,
                end_date TEXT,
                cum_weight_t REAL NOT NULL DEFAULT 0,
                status TEXT NOT NULL,
                suggest_threshold_t REAL NOT NULL,
                hard_limit_t REAL NOT NULL,
                PRIMARY KEY (version_id, machine_code, campaign_no)
            )
            "#,
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS decision_roll_campaign_alert (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                campaign_no INTEGER NOT NULL,
                cum_weight_t REAL NOT NULL,
                suggest_threshold_t REAL NOT NULL,
                hard_limit_t REAL NOT NULL,
                alert_level TEXT NOT NULL,
                reason TEXT,
                distance_to_suggest REAL NOT NULL,
                distance_to_hard REAL NOT NULL,
                utilization_rate REAL NOT NULL,
                estimated_change_date TEXT,
                needs_immediate_change INTEGER NOT NULL DEFAULT 0,
                suggested_actions TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, machine_code, campaign_no)
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

        // H032: 正常 (5000t / 10000t = 50%)
        conn.execute(
            "INSERT INTO roller_campaign (version_id, machine_code, campaign_no, start_date, end_date, cum_weight_t, status, suggest_threshold_t, hard_limit_t) VALUES ('V001', 'H032', 1, '2026-01-01', NULL, 0.0, 'ACTIVE', 10000.0, 12000.0)",
            [],
        )
        .unwrap();

        for i in 1..=10 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'H032', 500.0)",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO plan_item VALUES ('V001', ?, 'H032', '2026-01-24')",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
        }

        // H033: 警告 (9000t / 10000t = 90%)
        conn.execute(
            "INSERT INTO roller_campaign (version_id, machine_code, campaign_no, start_date, end_date, cum_weight_t, status, suggest_threshold_t, hard_limit_t) VALUES ('V001', 'H033', 1, '2026-01-01', NULL, 0.0, 'ACTIVE', 10000.0, 12000.0)",
            [],
        )
        .unwrap();

        for i in 11..=28 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'H033', 500.0)",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO plan_item VALUES ('V001', ?, 'H033', '2026-01-24')",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
        }

        // H034: 严重 (10500t / 10000t = 105%)
        conn.execute(
            "INSERT INTO roller_campaign (version_id, machine_code, campaign_no, start_date, end_date, cum_weight_t, status, suggest_threshold_t, hard_limit_t) VALUES ('V001', 'H034', 1, '2026-01-01', NULL, 0.0, 'ACTIVE', 10000.0, 12000.0)",
            [],
        )
        .unwrap();

        for i in 29..=49 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'H034', 500.0)",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO plan_item VALUES ('V001', ?, 'H034', '2026-01-24')",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
        }

        let conn_arc = Arc::new(Mutex::new(conn));
        let repo = RollAlertRepository::new(conn_arc);

        // 执行全量刷新
        let result = repo.refresh_full("V001").unwrap();

        // 验证结果
        assert_eq!(result, 3);

        // 查询预警列表
        let alerts = repo.list_roll_campaign_alerts("V001", None).unwrap();
        assert_eq!(alerts.len(), 3);

        // 验证预警等级
        let h032_alert = alerts.iter().find(|a| a.machine_code == "H032").unwrap();
        assert_eq!(h032_alert.alert_level, "NONE");

        let h033_alert = alerts.iter().find(|a| a.machine_code == "H033").unwrap();
        assert_eq!(h033_alert.alert_level, "WARNING");

        let h034_alert = alerts.iter().find(|a| a.machine_code == "H034").unwrap();
        assert_eq!(h034_alert.alert_level, "CRITICAL");
    }

    #[test]
    fn test_generate_suggestions() {
        let mut alert = RollAlert::new(
            "V001".to_string(),
            "H032".to_string(),
            1,
            10500.0,
            10000.0,
            12000.0,
        );

        generate_suggestions(&mut alert);

        assert!(!alert.suggested_actions.is_empty());
        assert!(alert
            .suggested_actions
            .iter()
            .any(|s| s.contains("超过建议阈值")));
    }
}

// ==========================================
// 热轧精整排产系统 - D5 用例实现
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md - D5 用例
// 职责: "换辊是否异常" 用例的具体实现
// ==========================================

use crate::decision::repository::roll_alert_repo::RollAlertRepository;
use crate::decision::use_cases::d5_roll_campaign_alert::{
    RollAlert, RollAlertSummary, RollCampaignAlertUseCase,
};
use std::sync::Arc;

/// D5 用例实现：换辊是否异常
pub struct RollCampaignAlertUseCaseImpl {
    /// 换辊预警仓储
    repo: Arc<RollAlertRepository>,
}

impl RollCampaignAlertUseCaseImpl {
    /// 创建新的 D5 用例实例
    pub fn new(repo: Arc<RollAlertRepository>) -> Self {
        Self { repo }
    }
}

impl RollCampaignAlertUseCase for RollCampaignAlertUseCaseImpl {
    /// 查询换辊预警列表
    fn list_roll_campaign_alerts(
        &self,
        version_id: &str,
        alert_level: Option<&str>,
    ) -> Result<Vec<RollAlert>, String> {
        self.repo
            .list_roll_campaign_alerts(version_id, alert_level)
            .map_err(|e| format!("查询换辊预警列表失败: {}", e))
    }

    /// 查询特定机组的换辊预警
    fn get_machine_roll_alerts(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> Result<Vec<RollAlert>, String> {
        self.repo
            .get_machine_roll_alerts(version_id, machine_code)
            .map_err(|e| format!("查询机组换辊预警失败: {}", e))
    }

    /// 统计换辊预警
    fn get_roll_alert_summary(&self, version_id: &str) -> Result<RollAlertSummary, String> {
        self.repo
            .get_roll_alert_summary(version_id)
            .map_err(|e| format!("统计换辊预警失败: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};
    use std::sync::{Arc, Mutex};

    fn setup_test_use_case() -> RollCampaignAlertUseCaseImpl {
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

        // H035: 紧急 (12000t / 10000t = 120%)
        conn.execute(
            "INSERT INTO roller_campaign (version_id, machine_code, campaign_no, start_date, end_date, cum_weight_t, status, suggest_threshold_t, hard_limit_t) VALUES ('V001', 'H035', 1, '2026-01-01', NULL, 0.0, 'ACTIVE', 10000.0, 12000.0)",
            [],
        )
        .unwrap();

        for i in 50..=73 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'H035', 500.0)",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO plan_item VALUES ('V001', ?, 'H035', '2026-01-24')",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
        }

        let conn_arc = Arc::new(Mutex::new(conn));
        let repo = Arc::new(RollAlertRepository::new(conn_arc));

        // 刷新读模型
        repo.refresh_full("V001").unwrap();

        RollCampaignAlertUseCaseImpl::new(repo)
    }

    #[test]
    fn test_list_roll_campaign_alerts() {
        let use_case = setup_test_use_case();
        let alerts = use_case.list_roll_campaign_alerts("V001", None).unwrap();

        // 应该有 4 个预警 (包括 NONE)
        assert_eq!(alerts.len(), 4);

        // 验证按预警等级排序 (EMERGENCY > CRITICAL > WARNING > NONE)
        assert_eq!(alerts[0].alert_level, "EMERGENCY");
        assert_eq!(alerts[1].alert_level, "CRITICAL");
        assert_eq!(alerts[2].alert_level, "WARNING");
        assert_eq!(alerts[3].alert_level, "NONE");
    }

    #[test]
    fn test_list_roll_campaign_alerts_with_filter() {
        let use_case = setup_test_use_case();
        let alerts = use_case
            .list_roll_campaign_alerts("V001", Some("CRITICAL"))
            .unwrap();

        // 只应该有 CRITICAL 的记录
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_level, "CRITICAL");
        assert_eq!(alerts[0].machine_code, "H034");
    }

    #[test]
    fn test_get_machine_roll_alerts() {
        let use_case = setup_test_use_case();
        let alerts = use_case.get_machine_roll_alerts("V001", "H033").unwrap();

        // 只应该有 H033 的记录
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].machine_code, "H033");
        assert_eq!(alerts[0].alert_level, "WARNING");
    }

    #[test]
    fn test_get_roll_alert_summary() {
        let use_case = setup_test_use_case();
        let summary = use_case.get_roll_alert_summary("V001").unwrap();

        // 总共 3 个预警 (排除 NONE)
        assert_eq!(summary.total_alerts, 3);

        // 统计各等级数量
        assert_eq!(summary.emergency_count, 1);
        assert_eq!(summary.critical_count, 1);
        assert_eq!(summary.warning_count, 1);

        // 需要立即换辊的数量
        assert!(summary.immediate_change_needed > 0);

        // 按机组统计
        assert_eq!(summary.by_machine.len(), 4);

        // 存在紧急预警
        assert!(summary.has_emergency());
        assert!(summary.has_critical());
    }

    #[test]
    fn test_high_priority_alerts() {
        let use_case = setup_test_use_case();
        let alerts = use_case.list_roll_campaign_alerts("V001", None).unwrap();

        // 验证高优先级预警
        let high_priority_alerts: Vec<_> = alerts.iter().filter(|a| a.is_high_priority()).collect();
        assert_eq!(high_priority_alerts.len(), 2); // EMERGENCY + CRITICAL
    }

    #[test]
    fn test_immediate_change_needed() {
        let use_case = setup_test_use_case();
        let alerts = use_case.list_roll_campaign_alerts("V001", None).unwrap();

        // 验证需要立即换辊的预警
        let immediate_alerts: Vec<_> = alerts
            .iter()
            .filter(|a| a.needs_immediate_change)
            .collect();
        assert!(immediate_alerts.len() > 0);
    }

    #[test]
    fn test_suggested_actions() {
        let use_case = setup_test_use_case();
        let alerts = use_case.list_roll_campaign_alerts("V001", None).unwrap();

        // 所有预警都应该有建议措施
        for alert in &alerts {
            assert!(!alert.suggested_actions.is_empty());
        }
    }

    #[test]
    fn test_error_handling() {
        let use_case = setup_test_use_case();

        // 查询不存在的版本
        let alerts = use_case.list_roll_campaign_alerts("V999", None).unwrap();
        assert_eq!(alerts.len(), 0);

        // 查询不存在的机组
        let alerts = use_case.get_machine_roll_alerts("V001", "H999").unwrap();
        assert_eq!(alerts.len(), 0);
    }
}

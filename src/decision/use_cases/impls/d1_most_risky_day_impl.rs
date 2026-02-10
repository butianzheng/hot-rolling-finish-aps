// ==========================================
// 热轧精整排产系统 - D1 用例实现
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md - D1 用例
// 职责: "哪天最危险" 用例的具体实现
// ==========================================

use crate::decision::repository::day_summary_repo::DaySummaryRepository;
use crate::decision::use_cases::d1_most_risky_day::{DaySummary, MostRiskyDayUseCase};
use std::sync::Arc;

/// D1 用例实现：哪天最危险
pub struct MostRiskyDayUseCaseImpl {
    /// 日期摘要仓储
    repo: Arc<DaySummaryRepository>,
}

impl MostRiskyDayUseCaseImpl {
    /// 创建新的 D1 用例实例
    pub fn new(repo: Arc<DaySummaryRepository>) -> Self {
        Self { repo }
    }
}

impl MostRiskyDayUseCase for MostRiskyDayUseCaseImpl {
    /// 查询日期风险摘要
    fn get_day_summary(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<DaySummary>, String> {
        self.repo
            .get_day_summary(version_id, start_date, end_date)
            .map_err(|e| format!("查询日期风险摘要失败: {}", e))
    }

    /// 查询最危险的 N 天
    fn get_top_risky_days(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        top_n: usize,
    ) -> Result<Vec<DaySummary>, String> {
        self.repo
            .get_top_risky_days(version_id, start_date, end_date, top_n)
            .map_err(|e| format!("查询最危险的 {} 天失败: {}", top_n, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn setup_test_use_case() -> MostRiskyDayUseCaseImpl {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::configure_sqlite_connection(&conn).unwrap();

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

        // 插入测试数据 - 三天不同的风险等级
        // 2026-01-24: HIGH 风险
        conn.execute(
            r#"
            INSERT INTO risk_snapshot (
                version_id, machine_code, snapshot_date, risk_level, risk_reasons,
                target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t, urgent_total_t,
                mature_backlog_t, immature_backlog_t, campaign_status, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
            "#,
            [
                "V001",
                "H032",
                "2026-01-24",
                "HIGH",
                "产能紧张",
                "1500.0",
                "1450.0",
                "2000.0",
                "0.0",
                "800.0",
                "500.0",
                "200.0",
                "OK",
            ],
        )
        .unwrap();

        // 2026-01-25: CRITICAL 风险
        conn.execute(
            r#"
            INSERT INTO risk_snapshot (
                version_id, machine_code, snapshot_date, risk_level, risk_reasons,
                target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t, urgent_total_t,
                mature_backlog_t, immature_backlog_t, campaign_status, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
            "#,
            [
                "V001",
                "H032",
                "2026-01-25",
                "CRITICAL",
                "严重超载",
                "1500.0",
                "1800.0",
                "2000.0",
                "300.0",
                "1000.0",
                "600.0",
                "300.0",
                "WARNING",
            ],
        )
        .unwrap();

        // 2026-01-26: MEDIUM 风险
        conn.execute(
            r#"
            INSERT INTO risk_snapshot (
                version_id, machine_code, snapshot_date, risk_level, risk_reasons,
                target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t, urgent_total_t,
                mature_backlog_t, immature_backlog_t, campaign_status, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
            "#,
            [
                "V001",
                "H033",
                "2026-01-26",
                "MEDIUM",
                "正常运行",
                "1500.0",
                "1200.0",
                "2000.0",
                "0.0",
                "500.0",
                "300.0",
                "100.0",
                "OK",
            ],
        )
        .unwrap();

        let conn_arc = Arc::new(Mutex::new(conn));
        let repo = Arc::new(DaySummaryRepository::new(conn_arc));

        MostRiskyDayUseCaseImpl::new(repo)
    }

    #[test]
    fn test_get_day_summary() {
        let use_case = setup_test_use_case();
        let summaries = use_case
            .get_day_summary("V001", "2026-01-24", "2026-01-26")
            .unwrap();

        assert_eq!(summaries.len(), 3);

        // 验证按风险分数降序排列
        assert!(summaries[0].risk_score >= summaries[1].risk_score);
        assert!(summaries[1].risk_score >= summaries[2].risk_score);
    }

    #[test]
    fn test_get_top_risky_days() {
        let use_case = setup_test_use_case();
        let summaries = use_case
            .get_top_risky_days("V001", "2026-01-24", "2026-01-26", 2)
            .unwrap();

        assert_eq!(summaries.len(), 2);

        // 最危险的应该是有产能超载的那天（2026-01-25）
        assert!(summaries[0]
            .top_reasons
            .iter()
            .any(|r| r.code == "CAPACITY_OVERFLOW"));
    }

    #[test]
    fn test_error_handling() {
        let use_case = setup_test_use_case();

        // 查询不存在的版本
        let result = use_case.get_day_summary("V999", "2026-01-24", "2026-01-26");

        // 应该成功但返回空列表
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}

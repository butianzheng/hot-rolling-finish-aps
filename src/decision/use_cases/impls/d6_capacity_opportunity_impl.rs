// ==========================================
// 热轧精整排产系统 - D6 用例实现
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md - D6 用例
// 职责: "是否存在产能优化空间" 用例的具体实现
// ==========================================

use crate::decision::repository::capacity_opportunity_repo::CapacityOpportunityRepository;
use crate::decision::use_cases::d6_capacity_opportunity::{
    CapacityOpportunity, CapacityOpportunityUseCase, OptimizationSummary,
};
use std::sync::Arc;

/// D6 用例实现：是否存在产能优化空间
pub struct CapacityOpportunityUseCaseImpl {
    /// 产能优化机会仓储
    repo: Arc<CapacityOpportunityRepository>,
}

impl CapacityOpportunityUseCaseImpl {
    /// 创建新的 D6 用例实例
    pub fn new(repo: Arc<CapacityOpportunityRepository>) -> Self {
        Self { repo }
    }
}

impl CapacityOpportunityUseCase for CapacityOpportunityUseCaseImpl {
    /// 查询产能优化机会
    fn get_capacity_opportunity(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<CapacityOpportunity>, String> {
        self.repo
            .get_capacity_opportunity(version_id, machine_code, start_date, end_date)
            .map_err(|e| format!("查询产能优化机会失败: {}", e))
    }

    /// 查询最大优化空间
    fn get_top_opportunities(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        top_n: usize,
    ) -> Result<Vec<CapacityOpportunity>, String> {
        self.repo
            .get_top_opportunities(version_id, start_date, end_date, top_n)
            .map_err(|e| format!("查询最大优化空间失败: {}", e))
    }

    /// 获取产能优化总结
    fn get_optimization_summary(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<OptimizationSummary, String> {
        self.repo
            .get_optimization_summary(version_id, start_date, end_date)
            .map_err(|e| format!("获取优化总结失败: {}", e))
    }
}

// ==========================================
// 单元测试
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::Mutex;

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

        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_get_capacity_opportunity_all_machines() {
        let conn = setup_test_db();
        {
            let c = conn.lock().unwrap();

            // 插入测试数据
            c.execute(
                r#"
                INSERT INTO decision_capacity_opportunity (
                    version_id, machine_code, plan_date,
                    slack_t, utilization_rate,
                    opportunity_level, suggested_optimizations
                ) VALUES ('V001', 'H032', '2026-01-24', 800.0, 0.6,
                    'HIGH', '[]')
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
                    'MEDIUM', '[]')
            "#,
                [],
            )
            .unwrap();
        }

        let repo = Arc::new(CapacityOpportunityRepository::new(conn));
        let use_case = CapacityOpportunityUseCaseImpl::new(repo);

        // 测试：查询所有机组
        let result = use_case
            .get_capacity_opportunity("V001", None, "2026-01-24", "2026-01-25")
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].slack_t, 800.0); // 按 slack_t 降序
        assert_eq!(result[0].machine_code, "H032");
        assert_eq!(result[1].slack_t, 300.0);
        assert_eq!(result[1].machine_code, "H033");
    }

    #[test]
    fn test_get_capacity_opportunity_specific_machine() {
        let conn = setup_test_db();
        {
            let c = conn.lock().unwrap();

            c.execute(
                r#"
                INSERT INTO decision_capacity_opportunity (
                    version_id, machine_code, plan_date,
                    slack_t, utilization_rate,
                    opportunity_level, suggested_optimizations
                ) VALUES ('V001', 'H032', '2026-01-24', 800.0, 0.6,
                    'HIGH', '[]')
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
                    'MEDIUM', '[]')
            "#,
                [],
            )
            .unwrap();
        }

        let repo = Arc::new(CapacityOpportunityRepository::new(conn));
        let use_case = CapacityOpportunityUseCaseImpl::new(repo);

        // 测试：查询特定机组
        let result = use_case
            .get_capacity_opportunity("V001", Some("H032"), "2026-01-24", "2026-01-25")
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].machine_code, "H032");
        assert_eq!(result[0].slack_t, 800.0);
        assert_eq!(result[0].opportunity_level, "HIGH");
    }

    #[test]
    fn test_get_capacity_opportunity_empty_result() {
        let conn = setup_test_db();
        let repo = Arc::new(CapacityOpportunityRepository::new(conn));
        let use_case = CapacityOpportunityUseCaseImpl::new(repo);

        // 测试：空结果
        let result = use_case
            .get_capacity_opportunity("V001", None, "2026-01-24", "2026-01-25")
            .unwrap();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_get_top_opportunities() {
        let conn = setup_test_db();
        {
            let c = conn.lock().unwrap();

            // 插入 5 条测试数据
            for i in 1..=5 {
                let slack_t = (1000 - i * 100) as f64;
                let level = if i <= 2 {
                    "HIGH"
                } else if i <= 4 {
                    "MEDIUM"
                } else {
                    "LOW"
                };
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
                        i,
                        slack_t,
                        i * 15,
                        level
                    ),
                    [],
                )
                .unwrap();
            }
        }

        let repo = Arc::new(CapacityOpportunityRepository::new(conn));
        let use_case = CapacityOpportunityUseCaseImpl::new(repo);

        // 测试：查询 Top 3
        let result = use_case
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
    fn test_get_top_opportunities_limit() {
        let conn = setup_test_db();
        {
            let c = conn.lock().unwrap();

            // 插入 2 条 HIGH/MEDIUM 数据
            c.execute(
                r#"
                INSERT INTO decision_capacity_opportunity (
                    version_id, machine_code, plan_date,
                    slack_t, utilization_rate,
                    opportunity_level, suggested_optimizations
                ) VALUES ('V001', 'H032', '2026-01-24', 800.0, 0.6,
                    'HIGH', '[]')
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
                ) VALUES ('V001', 'H033', '2026-01-24', 400.0, 0.7,
                    'MEDIUM', '[]')
            "#,
                [],
            )
            .unwrap();
        }

        let repo = Arc::new(CapacityOpportunityRepository::new(conn));
        let use_case = CapacityOpportunityUseCaseImpl::new(repo);

        // 测试：查询 Top 5，但只有 2 条符合条件
        let result = use_case
            .get_top_opportunities("V001", "2026-01-24", "2026-01-25", 5)
            .unwrap();

        assert_eq!(result.len(), 2); // 只返回实际存在的 2 条
    }

    #[test]
    fn test_get_optimization_summary() {
        let conn = setup_test_db();
        {
            let c = conn.lock().unwrap();

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

        let repo = Arc::new(CapacityOpportunityRepository::new(conn));
        let use_case = CapacityOpportunityUseCaseImpl::new(repo);

        // 测试：获取优化总结
        let summary = use_case
            .get_optimization_summary("V001", "2026-01-24", "2026-01-25")
            .unwrap();

        assert_eq!(summary.version_id, "V001");
        assert_eq!(
            summary.date_range,
            ("2026-01-24".to_string(), "2026-01-25".to_string())
        );
        assert_eq!(summary.total_slack_t, 1600.0); // 800 + 500 + 300
        assert_eq!(summary.high_opportunity_count, 2); // HIGH + MEDIUM
        assert_eq!(summary.by_machine.len(), 2); // H032 和 H033

        // 验证按机组统计
        let h032_stat = summary
            .by_machine
            .iter()
            .find(|s| s.machine_code == "H032")
            .unwrap();
        assert_eq!(h032_stat.slack_t, 1300.0); // 800 + 500
        assert_eq!(h032_stat.opportunity_count, 2);

        let h033_stat = summary
            .by_machine
            .iter()
            .find(|s| s.machine_code == "H033")
            .unwrap();
        assert_eq!(h033_stat.slack_t, 300.0);
        assert_eq!(h033_stat.opportunity_count, 1);
    }

    #[test]
    fn test_get_optimization_summary_empty() {
        let conn = setup_test_db();
        let repo = Arc::new(CapacityOpportunityRepository::new(conn));
        let use_case = CapacityOpportunityUseCaseImpl::new(repo);

        // 测试：空数据的优化总结
        let summary = use_case
            .get_optimization_summary("V001", "2026-01-24", "2026-01-25")
            .unwrap();

        assert_eq!(summary.version_id, "V001");
        assert_eq!(summary.total_slack_t, 0.0);
        assert_eq!(summary.high_opportunity_count, 0);
        assert_eq!(summary.by_machine.len(), 0);
        assert_eq!(summary.total_potential_gain_t, 0.0);
    }

    #[test]
    fn test_error_handling() {
        // 创建一个无效的数据库连接（已关闭）
        let conn = Connection::open_in_memory().unwrap();
        crate::db::configure_sqlite_connection(&conn).unwrap();
        drop(conn); // 关闭连接

        let invalid_conn = Connection::open_in_memory().unwrap();
        crate::db::configure_sqlite_connection(&invalid_conn).unwrap();
        let invalid_conn = Arc::new(Mutex::new(invalid_conn));
        let repo = Arc::new(CapacityOpportunityRepository::new(invalid_conn));
        let use_case = CapacityOpportunityUseCaseImpl::new(repo);

        // 测试：错误处理（表不存在）
        let result = use_case.get_capacity_opportunity("V001", None, "2026-01-24", "2026-01-25");

        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("查询产能优化机会失败"));
    }
}

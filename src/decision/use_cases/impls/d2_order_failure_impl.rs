// ==========================================
// 热轧精整排产系统 - D2 用例实现
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md - D2 用例
// 职责: \"哪些紧急单无法完成\" 用例的具体实现
// ==========================================

use crate::decision::repository::order_failure_repo::OrderFailureRepository;
use crate::decision::use_cases::d2_order_failure::{FailureStats, OrderFailure, OrderFailureUseCase};
use std::collections::HashMap;
use std::sync::Arc;

/// D2 用例实现：哪些紧急单无法完成
pub struct OrderFailureUseCaseImpl {
    /// 订单失败仓储
    repo: Arc<OrderFailureRepository>,
}

impl OrderFailureUseCaseImpl {
    /// 创建新的 D2 用例实例
    pub fn new(repo: Arc<OrderFailureRepository>) -> Self {
        Self { repo }
    }

    /// 批量获取合同主机组代码（用于 API 层补齐 DTO machine_code）
    pub fn get_primary_machine_codes(
        &self,
        contract_nos: &[String],
    ) -> Result<HashMap<String, String>, String> {
        self.repo
            .get_primary_machine_codes(contract_nos)
            .map_err(|e| format!("查询合同主机组失败: {}", e))
    }
}

impl OrderFailureUseCase for OrderFailureUseCaseImpl {
    /// 查询订单失败集合
    fn list_order_failures(
        &self,
        version_id: &str,
        fail_type: Option<&str>,
    ) -> Result<Vec<OrderFailure>, String> {
        self.repo
            .list_failures(version_id, fail_type)
            .map_err(|e| format!("查询订单失败集合失败: {}", e))
    }

    /// 查询特定合同的失败情况
    fn get_contract_failure(
        &self,
        version_id: &str,
        contract_no: &str,
    ) -> Result<Option<OrderFailure>, String> {
        self.repo
            .get_contract_failure(version_id, contract_no)
            .map_err(|e| format!("查询合同失败情况失败: {}", e))
    }

    /// 统计失败订单数量
    fn count_failures(&self, version_id: &str) -> Result<FailureStats, String> {
        self.repo
            .count_failures(version_id)
            .map_err(|e| format!("统计失败订单数量失败: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};
    use std::sync::{Arc, Mutex};

    fn setup_test_use_case() -> OrderFailureUseCaseImpl {
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

        // 注意：D2 读模型使用 UTC “今天”计算 days_to_due；测试数据需避免固定日期导致随时间漂移。
        let today = chrono::Utc::now().date_naive();
        let overdue_due_date = (today - chrono::Duration::days(4)).format("%Y-%m-%d").to_string();
        let near_due_date = (today + chrono::Duration::days(2)).format("%Y-%m-%d").to_string();
        let future_due_date = (today + chrono::Duration::days(10)).format("%Y-%m-%d").to_string();

        // 插入 3 个合同的紧急单材料
        // C001: L3, 5个材料, 2个已排产 (超期)
        for i in 1..=5 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'C001', ?, 'L3', 100.0, 1)",
                params![format!("MAT{:03}", i), &overdue_due_date],
            )
            .unwrap();
        }
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

        // C002: L2, 10个材料, 6个已排产 (临期)
        for i in 6..=15 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'C002', ?, 'L2', 150.0, 1)",
                params![format!("MAT{:03}", i), &near_due_date],
            )
            .unwrap();
        }
        for i in 6..=11 {
            conn.execute(
                &format!(
                    "INSERT INTO plan_item VALUES ('V001', 'MAT{:03}', 'H033', '2026-01-25', NULL)",
                    i
                ),
                [],
            )
            .unwrap();
        }

        // C003: L1, 8个材料, 全部未排产
        for i in 16..=23 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'C003', ?, 'L1', 120.0, 1)",
                params![format!("MAT{:03}", i), &future_due_date],
            )
            .unwrap();
        }

        let conn_arc = Arc::new(Mutex::new(conn));
        let repo = Arc::new(OrderFailureRepository::new(conn_arc));

        // 刷新读模型
        repo.refresh_full("V001").unwrap();

        OrderFailureUseCaseImpl::new(repo)
    }

    #[test]
    fn test_list_order_failures() {
        let use_case = setup_test_use_case();
        let failures = use_case.list_order_failures("V001", None).unwrap();

        assert_eq!(failures.len(), 3);

        // 验证按紧急等级排序 (L3 > L2 > L1)
        assert_eq!(failures[0].urgency_level, "L3");
        assert_eq!(failures[1].urgency_level, "L2");
        assert_eq!(failures[2].urgency_level, "L1");
    }

    #[test]
    fn test_list_order_failures_with_filter() {
        let use_case = setup_test_use_case();
        let failures = use_case
            .list_order_failures("V001", Some("Overdue"))
            .unwrap();

        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].contract_no, "C001");
    }

    #[test]
    fn test_get_contract_failure() {
        let use_case = setup_test_use_case();
        let failure = use_case
            .get_contract_failure("V001", "C001")
            .unwrap()
            .unwrap();

        assert_eq!(failure.contract_no, "C001");
        assert_eq!(failure.urgency_level, "L3");
        assert_eq!(failure.total_materials, 5);
        assert_eq!(failure.unscheduled_count, 3);
        assert!(failure.is_high_urgency());
        assert!(failure.is_overdue());
    }

    #[test]
    fn test_count_failures() {
        let use_case = setup_test_use_case();
        let stats = use_case.count_failures("V001").unwrap();

        assert_eq!(stats.total_failures, 3);
        assert_eq!(stats.overdue_count, 1);
        assert!(stats.total_affected_materials > 0);
    }

    #[test]
    fn test_error_handling() {
        let use_case = setup_test_use_case();

        // 查询不存在的版本
        let failures = use_case.list_order_failures("V999", None).unwrap();
        assert_eq!(failures.len(), 0);

        // 查询不存在的合同
        let failure = use_case.get_contract_failure("V001", "C999").unwrap();
        assert!(failure.is_none());
    }
}

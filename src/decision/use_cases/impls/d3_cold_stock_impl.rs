// ==========================================
// 热轧精整排产系统 - D3 用例实现
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md - D3 用例
// 职责: "哪些冷料压库" 用例的具体实现
// ==========================================

use crate::decision::repository::cold_stock_repo::ColdStockRepository;
use crate::decision::use_cases::d3_cold_stock::{
    ColdStockProfile, ColdStockSummary, ColdStockUseCase,
};
use std::sync::Arc;

/// D3 用例实现：哪些冷料压库
pub struct ColdStockUseCaseImpl {
    /// 冷料压库仓储
    repo: Arc<ColdStockRepository>,
}

impl ColdStockUseCaseImpl {
    /// 创建新的 D3 用例实例
    pub fn new(repo: Arc<ColdStockRepository>) -> Self {
        Self { repo }
    }
}

impl ColdStockUseCase for ColdStockUseCaseImpl {
    /// 查询冷料压库概况
    fn get_cold_stock_profile(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
    ) -> Result<Vec<ColdStockProfile>, String> {
        self.repo
            .get_cold_stock_profile(version_id, machine_code)
            .map_err(|e| format!("查询冷料压库概况失败: {}", e))
    }

    /// 查询特定机组的冷料分桶
    fn get_machine_cold_stock(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> Result<Vec<ColdStockProfile>, String> {
        self.repo
            .get_machine_cold_stock(version_id, machine_code)
            .map_err(|e| format!("查询机组冷料分桶失败: {}", e))
    }

    /// 统计冷料总量
    fn get_cold_stock_summary(&self, version_id: &str) -> Result<ColdStockSummary, String> {
        self.repo
            .get_cold_stock_summary(version_id)
            .map_err(|e| format!("统计冷料总量失败: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};
    use std::sync::{Arc, Mutex};

    fn setup_test_use_case() -> ColdStockUseCaseImpl {
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
                weight_t REAL NOT NULL,
                stock_age_days INTEGER NOT NULL DEFAULT 0,
                is_mature INTEGER NOT NULL DEFAULT 1,
                spec_width_mm REAL,
                spec_thick_mm REAL
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
            CREATE TABLE IF NOT EXISTS decision_cold_stock_profile (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                age_bin TEXT NOT NULL,
                age_min_days INTEGER NOT NULL,
                age_max_days INTEGER,
                count INTEGER NOT NULL,
                weight_t REAL NOT NULL,
                pressure_score REAL NOT NULL,
                pressure_level TEXT NOT NULL,
                reasons TEXT NOT NULL,
                structure_gap TEXT,
                estimated_ready_date TEXT,
                can_force_release INTEGER NOT NULL DEFAULT 0,
                suggested_actions TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, machine_code, age_bin)
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

        // H032: 5 块冷料 (0-7天), 8 块冷料 (15-30天), 3 块冷料 (30+天)
        for i in 1..=5 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'H032', 100.0, 5, 0, 1250.0, 3.5)",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
        }

        for i in 6..=13 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'H032', 150.0, 20, 0, 1250.0, 3.5)",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
        }

        for i in 14..=16 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'H032', 120.0, 35, 0, 1500.0, 4.0)",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
        }

        // H033: 10 块冷料 (8-14天)
        for i in 17..=26 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'H033', 80.0, 10, 0, 1000.0, 2.5)",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
        }

        let conn_arc = Arc::new(Mutex::new(conn));
        let repo = Arc::new(ColdStockRepository::new(conn_arc));

        // 刷新读模型
        repo.refresh_full("V001").unwrap();

        ColdStockUseCaseImpl::new(repo)
    }

    #[test]
    fn test_get_cold_stock_profile() {
        let use_case = setup_test_use_case();
        let profiles = use_case.get_cold_stock_profile("V001", None).unwrap();

        // 应该有 4 个分桶 (H032: 0-7, 15-30, 30+; H033: 8-14)
        assert!(profiles.len() >= 4);

        // 验证按压库分数降序排列
        for i in 1..profiles.len() {
            assert!(profiles[i - 1].pressure_score >= profiles[i].pressure_score);
        }
    }

    #[test]
    fn test_get_cold_stock_profile_with_filter() {
        let use_case = setup_test_use_case();
        let profiles = use_case
            .get_cold_stock_profile("V001", Some("H032"))
            .unwrap();

        // 只应该有 H032 的记录
        for profile in &profiles {
            assert_eq!(profile.machine_code, "H032");
        }

        assert!(profiles.len() >= 3);
    }

    #[test]
    fn test_get_machine_cold_stock() {
        let use_case = setup_test_use_case();
        let profiles = use_case.get_machine_cold_stock("V001", "H033").unwrap();

        // 只应该有 H033 的记录
        assert!(profiles.len() >= 1);
        for profile in &profiles {
            assert_eq!(profile.machine_code, "H033");
        }
    }

    #[test]
    fn test_get_cold_stock_summary() {
        let use_case = setup_test_use_case();
        let summary = use_case.get_cold_stock_summary("V001").unwrap();

        // 总共 26 块冷料
        assert_eq!(summary.total_count, 26);

        // 总重量应该大于 0
        assert!(summary.total_weight_t > 0.0);

        // 按机组统计
        assert_eq!(summary.by_machine.len(), 2);

        // 按年龄统计
        assert!(summary.by_age.len() >= 3);

        // 平均年龄应该大于 0
        assert!(summary.avg_age_days > 0.0);
    }

    #[test]
    fn test_pressure_levels() {
        let use_case = setup_test_use_case();
        let profiles = use_case.get_cold_stock_profile("V001", None).unwrap();

        // 应该有不同的压库等级
        let levels: Vec<_> = profiles.iter().map(|p| p.pressure_level.as_str()).collect();
        assert!(levels.contains(&"HIGH") || levels.contains(&"MEDIUM") || levels.contains(&"LOW"));
    }

    #[test]
    fn test_suggested_actions() {
        let use_case = setup_test_use_case();
        let profiles = use_case.get_cold_stock_profile("V001", None).unwrap();

        // 高压库应该有建议措施
        for profile in &profiles {
            if profile.is_high_pressure() {
                assert!(!profile.suggested_actions.is_empty());
            }
        }
    }

    #[test]
    fn test_error_handling() {
        let use_case = setup_test_use_case();

        // 查询不存在的版本
        let profiles = use_case.get_cold_stock_profile("V999", None).unwrap();
        assert_eq!(profiles.len(), 0);

        // 查询不存在的机组
        let profiles = use_case
            .get_machine_cold_stock("V001", "H999")
            .unwrap();
        assert_eq!(profiles.len(), 0);
    }
}

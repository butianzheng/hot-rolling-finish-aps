// ==========================================
// 热轧精整排产系统 - D4 用例实现
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md - D4 用例
// 职责: "哪个机组最堵" 用例的具体实现
// ==========================================

use crate::decision::repository::bottleneck_repo::BottleneckRepository;
use crate::decision::use_cases::d4_machine_bottleneck::{
    BottleneckHeatmap, MachineBottleneckProfile, MachineBottleneckUseCase,
};
use std::sync::Arc;

/// D4 用例实现：哪个机组最堵
pub struct MachineBottleneckUseCaseImpl {
    /// 堵塞仓储
    repo: Arc<BottleneckRepository>,
}

impl MachineBottleneckUseCaseImpl {
    /// 创建新的 D4 用例实例
    pub fn new(repo: Arc<BottleneckRepository>) -> Self {
        Self { repo }
    }
}

impl MachineBottleneckUseCase for MachineBottleneckUseCaseImpl {
    /// 查询机组堵塞概况
    fn get_machine_bottleneck_profile(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<MachineBottleneckProfile>, String> {
        self.repo
            .get_bottleneck_profile(version_id, machine_code, start_date, end_date)
            .map_err(|e| format!("查询机组堵塞概况失败: {}", e))
    }

    /// 查询最堵塞的机组-日组合
    fn get_top_bottlenecks(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        top_n: usize,
    ) -> Result<Vec<MachineBottleneckProfile>, String> {
        self.repo
            .get_top_bottlenecks(version_id, start_date, end_date, top_n)
            .map_err(|e| format!("查询最堵塞的 {} 个机组-日失败: {}", top_n, e))
    }

    /// 获取机组堵塞热力图数据
    fn get_bottleneck_heatmap(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<BottleneckHeatmap, String> {
        self.repo
            .get_bottleneck_heatmap(version_id, start_date, end_date)
            .map_err(|e| format!("获取堵塞热力图失败: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn setup_test_use_case() -> MachineBottleneckUseCaseImpl {
        let conn = Connection::open_in_memory().unwrap();

        // 创建 capacity_pool 表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS capacity_pool (
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                target_capacity_t REAL NOT NULL,
                limit_capacity_t REAL NOT NULL,
                used_capacity_t REAL NOT NULL DEFAULT 0.0,
                overflow_t REAL NOT NULL DEFAULT 0.0,
                frozen_capacity_t REAL NOT NULL DEFAULT 0.0,
                accumulated_tonnage_t REAL NOT NULL DEFAULT 0.0,
                roll_campaign_id TEXT,
                PRIMARY KEY (machine_code, plan_date)
            )
            "#,
            [],
        )
        .unwrap();

        // 创建 plan_item 表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS plan_item (
                version_id TEXT NOT NULL,
                material_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                seq_no INTEGER NOT NULL,
                weight_t REAL NOT NULL,
                source_type TEXT NOT NULL,
                locked_in_plan INTEGER NOT NULL DEFAULT 0,
                force_release_in_plan INTEGER NOT NULL DEFAULT 0,
                violation_flags TEXT,
                PRIMARY KEY (version_id, material_id)
            )
            "#,
            [],
        )
        .unwrap();

        // 插入产能数据 - H032: 高利用率，H033: 产能超载，H034: 正常
        conn.execute(
            r#"
            INSERT INTO capacity_pool (
                machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            [
                "H032",
                "2026-01-24",
                "1500.0",
                "2000.0",
                "1950.0",
                "0.0",
                "100.0",
                "15000.0",
                "RC001",
            ],
        )
        .unwrap();

        conn.execute(
            r#"
            INSERT INTO capacity_pool (
                machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            [
                "H033",
                "2026-01-24",
                "1500.0",
                "2000.0",
                "2300.0",
                "300.0",
                "150.0",
                "18000.0",
                "RC002",
            ],
        )
        .unwrap();

        conn.execute(
            r#"
            INSERT INTO capacity_pool (
                machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            [
                "H034",
                "2026-01-24",
                "1500.0",
                "2000.0",
                "1200.0",
                "0.0",
                "80.0",
                "12000.0",
                "RC003",
            ],
        )
        .unwrap();

        // 插入计划项数据
        // H032: 10 个材料，2 个结构违规
        for i in 1..=10 {
            let violation_flags = if i <= 2 { "STRUCT_CONFLICT" } else { "" };
            conn.execute(
                r#"
                INSERT INTO plan_item (
                    version_id, material_id, machine_code, plan_date, seq_no, weight_t,
                    source_type, locked_in_plan, force_release_in_plan, violation_flags
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                [
                    "V001",
                    &format!("MAT{:03}", i),
                    "H032",
                    "2026-01-24",
                    &i.to_string(),
                    "150.0",
                    "AUTO",
                    "0",
                    "0",
                    violation_flags,
                ],
            )
            .unwrap();
        }

        // H033: 25 个材料，5 个结构违规
        for i in 11..=35 {
            let violation_flags = if i <= 15 { "STRUCT_CONFLICT" } else { "" };
            conn.execute(
                r#"
                INSERT INTO plan_item (
                    version_id, material_id, machine_code, plan_date, seq_no, weight_t,
                    source_type, locked_in_plan, force_release_in_plan, violation_flags
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                [
                    "V001",
                    &format!("MAT{:03}", i),
                    "H033",
                    "2026-01-24",
                    &(i - 10).to_string(),
                    "100.0",
                    "AUTO",
                    "0",
                    "0",
                    violation_flags,
                ],
            )
            .unwrap();
        }

        // H034: 5 个材料，无违规
        for i in 36..=40 {
            conn.execute(
                r#"
                INSERT INTO plan_item (
                    version_id, material_id, machine_code, plan_date, seq_no, weight_t,
                    source_type, locked_in_plan, force_release_in_plan, violation_flags
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                [
                    "V001",
                    &format!("MAT{:03}", i),
                    "H034",
                    "2026-01-24",
                    &(i - 35).to_string(),
                    "120.0",
                    "AUTO",
                    "0",
                    "0",
                    "",
                ],
            )
            .unwrap();
        }

        let conn_arc = Arc::new(Mutex::new(conn));
        let repo = Arc::new(BottleneckRepository::new(conn_arc));

        MachineBottleneckUseCaseImpl::new(repo)
    }

    #[test]
    fn test_get_machine_bottleneck_profile() {
        let use_case = setup_test_use_case();
        let profiles = use_case
            .get_machine_bottleneck_profile("V001", None, "2026-01-24", "2026-01-24")
            .unwrap();

        assert_eq!(profiles.len(), 3);

        // 验证按堵塞分数降序排列
        assert!(profiles[0].bottleneck_score >= profiles[1].bottleneck_score);
        assert!(profiles[1].bottleneck_score >= profiles[2].bottleneck_score);

        // H033 应该是最堵的（产能超载）
        assert_eq!(profiles[0].machine_code, "H033");
        assert!(profiles[0].is_severe());
        assert_eq!(profiles[0].pending_materials, 25);
    }

    #[test]
    fn test_get_machine_bottleneck_profile_filtered() {
        let use_case = setup_test_use_case();
        let profiles = use_case
            .get_machine_bottleneck_profile("V001", Some("H032"), "2026-01-24", "2026-01-24")
            .unwrap();

        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].machine_code, "H032");
        assert_eq!(profiles[0].pending_materials, 10);
    }

    #[test]
    fn test_get_top_bottlenecks() {
        let use_case = setup_test_use_case();
        let profiles = use_case
            .get_top_bottlenecks("V001", "2026-01-24", "2026-01-24", 2)
            .unwrap();

        assert_eq!(profiles.len(), 2);

        // 第一个应该是 H033（产能超载）
        assert_eq!(profiles[0].machine_code, "H033");
        assert!(profiles[0].reasons.iter().any(|r| r.code == "CAPACITY_OVERFLOW"));

        // 第二个应该是 H032（高利用率）
        assert_eq!(profiles[1].machine_code, "H032");
    }

    #[test]
    fn test_get_bottleneck_heatmap() {
        let use_case = setup_test_use_case();
        let heatmap = use_case
            .get_bottleneck_heatmap("V001", "2026-01-24", "2026-01-24")
            .unwrap();

        assert_eq!(heatmap.version_id, "V001");
        assert_eq!(heatmap.machines.len(), 3);
        assert_eq!(heatmap.data.len(), 3);
        assert!(heatmap.max_score > 0.0);
        assert!(heatmap.avg_score > 0.0);

        // 验证可以获取特定机组-日的分数
        let h033_score = heatmap.get_score("H033", "2026-01-24");
        assert!(h033_score.is_some());
        assert!(h033_score.unwrap() > 0.0);
    }

    #[test]
    fn test_bottleneck_reasons() {
        let use_case = setup_test_use_case();
        let profiles = use_case
            .get_machine_bottleneck_profile("V001", Some("H033"), "2026-01-24", "2026-01-24")
            .unwrap();

        assert_eq!(profiles.len(), 1);
        let h033 = &profiles[0];

        // H033 应该有多个堵塞原因
        assert!(!h033.reasons.is_empty());

        // 应该包含产能超载原因
        assert!(h033.reasons.iter().any(|r| r.code == "CAPACITY_OVERFLOW"));

        // 应该包含结构冲突原因（5 个违规）
        assert!(h033.reasons.iter().any(|r| r.code == "STRUCTURE_CONFLICT"));

        // 应该包含待排材料过多原因（25 个材料）
        assert!(h033.reasons.iter().any(|r| r.code == "HIGH_PENDING_COUNT"));
    }

    #[test]
    fn test_suggested_actions() {
        let use_case = setup_test_use_case();
        let profiles = use_case
            .get_machine_bottleneck_profile("V001", Some("H033"), "2026-01-24", "2026-01-24")
            .unwrap();

        assert_eq!(profiles.len(), 1);
        let h033 = &profiles[0];

        // H033 堵塞严重，应该有建议措施
        assert!(!h033.suggested_actions.is_empty());
    }

    #[test]
    fn test_error_handling() {
        let use_case = setup_test_use_case();

        // 查询不存在的版本
        let result = use_case.get_machine_bottleneck_profile(
            "V999",
            None,
            "2026-01-24",
            "2026-01-24",
        );

        // 应该成功（capacity_pool 不按 version_id 过滤），但没有 plan_item 数据
        assert!(result.is_ok());
        let profiles = result.unwrap();
        assert_eq!(profiles.len(), 3);

        // 所有 profile 的 pending_materials 应该为 0（因为没有 plan_item）
        for profile in &profiles {
            assert_eq!(profile.pending_materials, 0);
            assert_eq!(profile.structure_violations, 0);
        }
    }
}

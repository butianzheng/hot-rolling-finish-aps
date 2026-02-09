// ==========================================
// 热轧精整排产系统 - 决策刷新服务
// ==========================================
// 依据: REFACTOR_PLAN_v1.0.md - P1 阶段
// 职责: 刷新决策读模型表（decision_* 表）
// ==========================================

use chrono::{Local, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction};
use std::error::Error;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// P2 阶段: refresh_d2~d6 方法已重构,不再需要 Repository import

/// 刷新触发类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshTrigger {
    /// 计划项变更
    PlanItemChanged,
    /// 风险快照更新
    RiskSnapshotUpdated,
    /// 物料状态变更
    MaterialStateChanged,
    /// 产能池变更
    CapacityPoolChanged,
    /// 换辊批次变更
    RollCampaignChanged,
    /// 每日生产节奏目标变更
    RhythmTargetChanged,
    /// 版本创建
    VersionCreated,
    /// 手动刷新
    ManualRefresh,
}

impl RefreshTrigger {
    pub fn as_str(&self) -> &str {
        match self {
            RefreshTrigger::PlanItemChanged => "PlanItemChanged",
            RefreshTrigger::RiskSnapshotUpdated => "RiskSnapshotUpdated",
            RefreshTrigger::MaterialStateChanged => "MaterialStateChanged",
            RefreshTrigger::CapacityPoolChanged => "CapacityPoolChanged",
            RefreshTrigger::RollCampaignChanged => "RollCampaignChanged",
            RefreshTrigger::RhythmTargetChanged => "RhythmTargetChanged",
            RefreshTrigger::VersionCreated => "VersionCreated",
            RefreshTrigger::ManualRefresh => "ManualRefresh",
        }
    }
}

/// 刷新范围
#[derive(Debug, Clone)]
pub struct RefreshScope {
    /// 版本 ID
    pub version_id: String,
    /// 是否全量刷新
    pub is_full_refresh: bool,
    /// 受影响的机组（可选）
    pub affected_machines: Option<Vec<String>>,
    /// 受影响的日期范围（可选）
    pub affected_date_range: Option<(String, String)>,
}

/// 决策刷新服务
pub struct DecisionRefreshService {
    conn: Arc<Mutex<Connection>>,
}

impl DecisionRefreshService {
    /// 创建新的刷新服务实例
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }


    // 其余刷新逻辑已拆分到子模块（按域/职责），入口仅保留构造方法。
}

mod core;
mod d1;
mod d2;
mod d3;
mod d4;
mod d5;
mod d6;
mod logging;

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::configure_sqlite_connection(&conn).unwrap();

        // 创建必要的表
        conn.execute_batch(
            r#"
            CREATE TABLE config_kv (
                scope_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (scope_id, key)
            );

            CREATE TABLE plan_version (
                version_id TEXT PRIMARY KEY
            );

            CREATE TABLE machine_master (
                machine_code TEXT PRIMARY KEY
            );

            CREATE TABLE risk_snapshot (
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
            );

            CREATE TABLE capacity_pool (
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                target_capacity_t REAL NOT NULL,
                limit_capacity_t REAL NOT NULL,
                used_capacity_t REAL NOT NULL DEFAULT 0.0,
                overflow_t REAL NOT NULL DEFAULT 0.0,
                PRIMARY KEY (machine_code, plan_date)
            );

            CREATE TABLE plan_item (
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
            );

            CREATE TABLE decision_day_summary (
                version_id TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                risk_score REAL NOT NULL,
                risk_level TEXT NOT NULL,
                capacity_util_pct REAL NOT NULL,
                top_reasons TEXT NOT NULL,
                affected_machines INTEGER NOT NULL,
                bottleneck_machines INTEGER NOT NULL,
                has_roll_risk INTEGER NOT NULL DEFAULT 0,
                suggested_actions TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, plan_date)
            );

            CREATE TABLE decision_machine_bottleneck (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                bottleneck_score REAL NOT NULL,
                bottleneck_level TEXT NOT NULL,
                bottleneck_types TEXT NOT NULL,
                reasons TEXT NOT NULL,
                remaining_capacity_t REAL NOT NULL,
                capacity_utilization REAL NOT NULL,
                needs_roll_change INTEGER NOT NULL DEFAULT 0,
                structure_violations INTEGER NOT NULL DEFAULT 0,
                pending_materials INTEGER NOT NULL DEFAULT 0,
                suggested_actions TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, machine_code, plan_date)
            );

            CREATE TABLE decision_refresh_log (
                refresh_id TEXT PRIMARY KEY,
                version_id TEXT NOT NULL,
                trigger_type TEXT NOT NULL,
                trigger_source TEXT,
                is_full_refresh INTEGER NOT NULL DEFAULT 0,
                affected_machines TEXT,
                affected_date_range TEXT,
                refreshed_tables TEXT NOT NULL,
                rows_affected INTEGER NOT NULL DEFAULT 0,
                started_at TEXT NOT NULL DEFAULT (datetime('now')),
                completed_at TEXT,
                duration_ms INTEGER,
                status TEXT NOT NULL DEFAULT 'RUNNING',
                error_message TEXT
            );

            -- D2: 订单失败表
            CREATE TABLE decision_order_failure_set (
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
            );

            -- D3: 冷料压库表
            CREATE TABLE decision_cold_stock_profile (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                age_bin TEXT NOT NULL,
                age_min_days INTEGER NOT NULL,
                age_max_days INTEGER NOT NULL,
                count INTEGER NOT NULL,
                weight_t REAL NOT NULL,
                avg_age_days REAL NOT NULL,
                pressure_score REAL NOT NULL,
                pressure_level TEXT NOT NULL,
                reasons TEXT NOT NULL,
                structure_gap TEXT,
                estimated_ready_date TEXT,
                can_force_release INTEGER NOT NULL DEFAULT 0,
                suggested_actions TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, machine_code, age_bin)
            );

            -- D5: 换辊时间监控表（与真实 schema 对齐）
            CREATE TABLE decision_roll_campaign_alert (
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
                campaign_start_at TEXT,
                planned_change_at TEXT,
                planned_downtime_minutes INTEGER,
                estimated_soft_reach_at TEXT,
                estimated_hard_reach_at TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, machine_code, campaign_no)
            );

            -- D6: 产能优化机会表
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
            );

            -- 创建必要的源表
            CREATE TABLE material_state (
                material_id TEXT PRIMARY KEY,
                age_days INTEGER NOT NULL,
                weight_t REAL NOT NULL
            );

            CREATE TABLE roll_campaign (
                campaign_id TEXT PRIMARY KEY,
                machine_code TEXT NOT NULL,
                cum_weight_t REAL NOT NULL
            );

            INSERT INTO plan_version VALUES ('V001');
            INSERT INTO machine_master VALUES ('H032');

            INSERT INTO risk_snapshot VALUES (
                'V001', 'H032', '2026-01-24', 'HIGH', '产能紧张',
                1500.0, 1450.0, 2000.0, 0.0, 800.0, 500.0, 200.0, 'OK',
                datetime('now')
            );

            INSERT INTO capacity_pool VALUES (
                'H032', '2026-01-24', 1500.0, 2000.0, 1450.0, 0.0
            );

            INSERT INTO plan_item VALUES (
                'V001', 'MAT001', 'H032', '2026-01-24', 1, 100.0, 'AUTO', 0, 0, ''
            );
            "#,
        )
        .unwrap();

        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_refresh_all_with_d1_and_d4() {
        let conn = setup_test_db();
        let service = DecisionRefreshService::new(conn.clone());

        let scope = RefreshScope {
            version_id: "V001".to_string(),
            is_full_refresh: true,
            affected_machines: None,
            affected_date_range: None,
        };

        let refresh_id = service
            .refresh_all(scope, RefreshTrigger::ManualRefresh, Some("test".to_string()))
            .unwrap();

        assert!(!refresh_id.is_empty());

        // 验证 D1 数据
        let c = conn.lock().unwrap();
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_day_summary WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // 验证 D4 数据
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_machine_bottleneck WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // 验证刷新日志
        let status: String = c
            .query_row(
                "SELECT status FROM decision_refresh_log WHERE refresh_id = ?1",
                [&refresh_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "SUCCESS");
    }

    #[test]
    fn test_should_refresh_d2() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::configure_sqlite_connection(&conn).unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let service = DecisionRefreshService::new(conn);

        assert!(service.should_refresh_d2(&RefreshTrigger::PlanItemChanged));
        assert!(service.should_refresh_d2(&RefreshTrigger::MaterialStateChanged));
        assert!(service.should_refresh_d2(&RefreshTrigger::RiskSnapshotUpdated));
        assert!(service.should_refresh_d2(&RefreshTrigger::VersionCreated));
        assert!(service.should_refresh_d2(&RefreshTrigger::ManualRefresh));
        assert!(!service.should_refresh_d2(&RefreshTrigger::CapacityPoolChanged));
    }

    #[test]
    fn test_should_refresh_d3() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::configure_sqlite_connection(&conn).unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let service = DecisionRefreshService::new(conn);

        assert!(service.should_refresh_d3(&RefreshTrigger::MaterialStateChanged));
        assert!(service.should_refresh_d3(&RefreshTrigger::PlanItemChanged));
        assert!(service.should_refresh_d3(&RefreshTrigger::VersionCreated));
        assert!(service.should_refresh_d3(&RefreshTrigger::ManualRefresh));
        assert!(!service.should_refresh_d3(&RefreshTrigger::RollCampaignChanged));
    }

    #[test]
    fn test_should_refresh_d5() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::configure_sqlite_connection(&conn).unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let service = DecisionRefreshService::new(conn);

        assert!(service.should_refresh_d5(&RefreshTrigger::RollCampaignChanged));
        assert!(service.should_refresh_d5(&RefreshTrigger::MaterialStateChanged));
        assert!(service.should_refresh_d5(&RefreshTrigger::PlanItemChanged));
        assert!(service.should_refresh_d5(&RefreshTrigger::VersionCreated));
        assert!(service.should_refresh_d5(&RefreshTrigger::ManualRefresh));
        assert!(!service.should_refresh_d5(&RefreshTrigger::CapacityPoolChanged));
    }

    #[test]
    fn test_should_refresh_d6() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::configure_sqlite_connection(&conn).unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let service = DecisionRefreshService::new(conn);

        assert!(service.should_refresh_d6(&RefreshTrigger::CapacityPoolChanged));
        assert!(service.should_refresh_d6(&RefreshTrigger::PlanItemChanged));
        assert!(service.should_refresh_d6(&RefreshTrigger::MaterialStateChanged));
        assert!(service.should_refresh_d6(&RefreshTrigger::VersionCreated));
        assert!(service.should_refresh_d6(&RefreshTrigger::ManualRefresh));
        assert!(!service.should_refresh_d6(&RefreshTrigger::RollCampaignChanged));
    }

    #[test]
    fn test_incremental_refresh_d1_by_date_range() {
        let conn = setup_test_db();
        let c = conn.lock().unwrap();

        // 添加多个日期的数据
        c.execute(
            r#"
            INSERT INTO risk_snapshot VALUES (
                'V001', 'H032', '2026-01-25', 'MEDIUM', '产能正常',
                1500.0, 1200.0, 2000.0, 0.0, 600.0, 400.0, 150.0, 'OK',
                datetime('now')
            )
            "#,
            [],
        )
        .unwrap();
        drop(c);

        let service = DecisionRefreshService::new(conn.clone());

        // 全量刷新
        let scope = RefreshScope {
            version_id: "V001".to_string(),
            is_full_refresh: true,
            affected_machines: None,
            affected_date_range: None,
        };
        service
            .refresh_all(scope, RefreshTrigger::ManualRefresh, Some("test".to_string()))
            .unwrap();

        // 验证有 2 个日期的数据
        let c = conn.lock().unwrap();
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_day_summary WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
        drop(c);

        // 增量刷新：只刷新 2026-01-25
        let scope = RefreshScope {
            version_id: "V001".to_string(),
            is_full_refresh: false,
            affected_machines: None,
            affected_date_range: Some(("2026-01-25".to_string(), "2026-01-25".to_string())),
        };
        service
            .refresh_all(scope, RefreshTrigger::RiskSnapshotUpdated, Some("test".to_string()))
            .unwrap();

        // 验证仍然有 2 个日期的数据
        let c = conn.lock().unwrap();
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_day_summary WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);

        // 验证 2026-01-25 的数据已更新（refreshed_at 应该更新）
        let risk_score: f64 = c
            .query_row(
                "SELECT risk_score FROM decision_day_summary WHERE version_id = 'V001' AND plan_date = '2026-01-25'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(risk_score > 0.0);
    }

    #[test]
    fn test_incremental_refresh_d4_by_machine_and_date() {
        let conn = setup_test_db();
        let c = conn.lock().unwrap();

        // 添加另一个机组和日期的数据
        c.execute_batch(
            r#"
            INSERT INTO machine_master VALUES ('H033');
            INSERT INTO capacity_pool VALUES (
                'H033', '2026-01-24', 1600.0, 2100.0, 1550.0, 0.0
            );
            INSERT INTO capacity_pool VALUES (
                'H032', '2026-01-25', 1500.0, 2000.0, 1960.0, 0.0
            );
            "#,
        )
        .unwrap();
        drop(c);

        let service = DecisionRefreshService::new(conn.clone());

        // 全量刷新
        let scope = RefreshScope {
            version_id: "V001".to_string(),
            is_full_refresh: true,
            affected_machines: None,
            affected_date_range: None,
        };
        service
            .refresh_all(scope, RefreshTrigger::ManualRefresh, Some("test".to_string()))
            .unwrap();

        // 验证有 3 条记录（H032-01-24, H033-01-24, H032-01-25）
        let c = conn.lock().unwrap();
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_machine_bottleneck WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);
        drop(c);

        // 增量刷新：只刷新 H032 机组的 2026-01-25
        let scope = RefreshScope {
            version_id: "V001".to_string(),
            is_full_refresh: false,
            affected_machines: Some(vec!["H032".to_string()]),
            affected_date_range: Some(("2026-01-25".to_string(), "2026-01-25".to_string())),
        };
        service
            .refresh_all(scope, RefreshTrigger::CapacityPoolChanged, Some("test".to_string()))
            .unwrap();

        // 验证仍然有 3 条记录
        let c = conn.lock().unwrap();
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_machine_bottleneck WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);

        // 验证 H032-2026-01-25 的数据存在
        let bottleneck_score: f64 = c
            .query_row(
                "SELECT bottleneck_score FROM decision_machine_bottleneck WHERE version_id = 'V001' AND machine_code = 'H032' AND plan_date = '2026-01-25'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(bottleneck_score > 0.0);
    }
}

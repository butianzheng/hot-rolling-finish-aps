// ==========================================
// 测试辅助函数
// ==========================================
// 职责: 提供测试所需的数据库初始化、测试数据生成等功能
// ==========================================

use rusqlite::Connection;
use std::error::Error;
use tempfile::NamedTempFile;

/// 创建临时测试数据库并初始化 schema
///
/// # 返回
/// - NamedTempFile: 临时数据库文件（需要保持存活）
/// - String: 数据库文件路径
pub fn create_test_db() -> Result<(NamedTempFile, String), Box<dyn Error>> {
    let temp_file = NamedTempFile::new()?;
    let db_path = temp_file.path().to_str().unwrap().to_string();

    let conn = Connection::open(&db_path)?;

    // 初始化 schema
    init_schema(&conn)?;

    Ok((temp_file, db_path))
}

/// 初始化数据库 schema
fn init_schema(conn: &Connection) -> Result<(), Box<dyn Error>> {
    // 创建 schema_version 表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
        [],
    )?;

    // 创建 config_scope 表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS config_scope (
            scope_id TEXT PRIMARY KEY,
            scope_type TEXT NOT NULL,
            scope_key TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(scope_type, scope_key)
        )
        "#,
        [],
    )?;

    // 插入 global scope
    conn.execute(
        r#"
        INSERT OR IGNORE INTO config_scope (scope_id, scope_type, scope_key)
        VALUES ('global', 'GLOBAL', 'global')
        "#,
        [],
    )?;

    // 创建 config_kv 表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS config_kv (
            scope_id TEXT NOT NULL REFERENCES config_scope(scope_id) ON DELETE CASCADE,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (scope_id, key)
        )
        "#,
        [],
    )?;

    // 创建 material_master 表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS material_master (
            material_id TEXT PRIMARY KEY,
            manufacturing_order_id TEXT,
            material_status_code_src TEXT,
            steel_mark TEXT,
            slab_id TEXT,
            next_machine_code TEXT,
            rework_machine_code TEXT,
            current_machine_code TEXT,
            width_mm REAL,
            thickness_mm REAL,
            length_m REAL,
            weight_t REAL,
            available_width_mm REAL,
            due_date TEXT,
            stock_age_days INTEGER,
            output_age_days_raw INTEGER,
            status_updated_at TEXT,
            contract_no TEXT,
            contract_nature TEXT,
            weekly_delivery_flag TEXT,
            export_flag TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
        [],
    )?;

    // 创建 material_state 表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS material_state (
            material_id TEXT PRIMARY KEY,
            sched_state TEXT NOT NULL,
            lock_flag INTEGER NOT NULL DEFAULT 0,
            force_release_flag INTEGER NOT NULL DEFAULT 0,
            urgent_level TEXT NOT NULL,
            urgent_reason TEXT,
            rush_level TEXT NOT NULL,
            rolling_output_age_days INTEGER,
            ready_in_days INTEGER,
            earliest_sched_date TEXT,
            stock_age_days INTEGER,
            scheduled_date TEXT,
            scheduled_machine_code TEXT,
            -- material 当前所属机组(真实库字段,用于 D3 冷料压库口径)
            machine_code TEXT,
            seq_no INTEGER,
            -- 是否已适温(真实库字段,用于 D3 冷料压库口径)
            is_mature INTEGER NOT NULL DEFAULT 0,
            manual_urgent_flag INTEGER NOT NULL DEFAULT 0,
            user_confirmed INTEGER NOT NULL DEFAULT 0,
            user_confirmed_at TEXT,
            user_confirmed_by TEXT,
            user_confirmed_reason TEXT,
            in_frozen_zone INTEGER NOT NULL DEFAULT 0,
            last_calc_version_id TEXT,
            updated_at TEXT NOT NULL,
            updated_by TEXT,
            -- P2 阶段新增字段（用于 D2-D6 决策）
            contract_no TEXT,
            urgency_level TEXT,
            age_days INTEGER,
            weight_t REAL,
            due_date TEXT,
            eligible_machine_code TEXT
        )
        "#,
        [],
    )?;

    // 创建 import_batch 表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS import_batch (
            batch_id TEXT PRIMARY KEY,
            file_name TEXT,
            file_path TEXT,
            total_rows INTEGER NOT NULL,
            success_rows INTEGER NOT NULL,
            blocked_rows INTEGER NOT NULL,
            warning_rows INTEGER NOT NULL,
            conflict_rows INTEGER NOT NULL,
            imported_at TEXT,
            imported_by TEXT,
            elapsed_ms INTEGER,
            dq_report_json TEXT
        )
        "#,
        [],
    )?;

    // 创建 import_conflict 表（与 spec/schema_v0.1.sql 保持一致）
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS import_conflict (
            conflict_id TEXT PRIMARY KEY,
            source_batch_id TEXT NOT NULL,
            material_id TEXT NOT NULL,
            detected_at TEXT NOT NULL DEFAULT (datetime('now')),
            conflict_type TEXT NOT NULL,
            source_row_json TEXT NOT NULL,
            existing_row_json TEXT,
            resolution_status TEXT NOT NULL DEFAULT 'OPEN',
            resolution_note TEXT,
            row_number INTEGER NOT NULL DEFAULT 0
        )
        "#,
        [],
    )?;

    // 创建 plan 表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS plan (
            plan_id TEXT PRIMARY KEY,
            plan_name TEXT NOT NULL,
            plan_type TEXT NOT NULL,
            base_plan_id TEXT,
            created_by TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
        [],
    )?;

    // 创建 plan_version 表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS plan_version (
            version_id TEXT PRIMARY KEY,
            plan_id TEXT NOT NULL REFERENCES plan(plan_id) ON DELETE CASCADE,
            version_no INTEGER NOT NULL,
            status TEXT NOT NULL,
            frozen_from_date TEXT,
            recalc_window_days INTEGER,
            config_snapshot_json TEXT,
            created_by TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            revision INTEGER NOT NULL DEFAULT 0,
            UNIQUE(plan_id, version_no)
        )
        "#,
        [],
    )?;

    // 创建 plan_item 表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS plan_item (
            version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
            material_id TEXT NOT NULL REFERENCES material_master(material_id),
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
    )?;

    // 创建 machine_master 表 (用于外键约束)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS machine_master (
            machine_code TEXT PRIMARY KEY,
            machine_name TEXT,
            machine_type TEXT,
            is_active INTEGER NOT NULL DEFAULT 1
        )
        "#,
        [],
    )?;

    // 插入测试机组
    conn.execute(
        r#"
        INSERT OR IGNORE INTO machine_master (machine_code, machine_name, machine_type) VALUES
        ('H032', '精整线32', 'FINISHING'),
        ('H033', '精整线33', 'FINISHING'),
        ('H034', '精整线34', 'FINISHING')
        "#,
        [],
    )?;

    // 创建 action_log 表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS action_log (
            action_id TEXT PRIMARY KEY,
            -- version_id 可空：部分操作（如创建方案/导入等）可能不绑定具体版本
            version_id TEXT,
            action_type TEXT NOT NULL,
            action_ts TEXT NOT NULL,
            actor TEXT NOT NULL,
            payload_json TEXT,
            impact_summary_json TEXT,
            machine_code TEXT,
            date_range_start TEXT,
            date_range_end TEXT,
            detail TEXT
        )
        "#,
        [],
    )?;

    // 创建 decision_strategy_draft 表（策略草案持久化）
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS decision_strategy_draft (
            draft_id TEXT PRIMARY KEY,
            base_version_id TEXT NOT NULL REFERENCES plan_version(version_id),
            plan_date_from TEXT NOT NULL,
            plan_date_to TEXT NOT NULL,

            strategy_key TEXT NOT NULL,
            strategy_base TEXT NOT NULL,
            strategy_title_cn TEXT NOT NULL,
            strategy_params_json TEXT,

            status TEXT NOT NULL,
            created_by TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            expires_at TEXT NOT NULL,
            published_as_version_id TEXT,
            published_by TEXT,
            published_at TEXT,

            locked_by TEXT,
            locked_at TEXT,

            summary_json TEXT NOT NULL,
            diff_items_json TEXT NOT NULL,
            diff_items_total INTEGER NOT NULL DEFAULT 0,
            diff_items_truncated INTEGER NOT NULL DEFAULT 0
        )
        "#,
        [],
    )?;

    // 创建 risk_snapshot 表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS risk_snapshot (
            version_id TEXT NOT NULL,
            snapshot_date TEXT NOT NULL,
            machine_code TEXT NOT NULL,
            risk_level TEXT NOT NULL DEFAULT 'Green',
            risk_reasons TEXT,
            target_capacity_t REAL NOT NULL DEFAULT 0.0,
            used_capacity_t REAL NOT NULL DEFAULT 0.0,
            limit_capacity_t REAL NOT NULL DEFAULT 0.0,
            overflow_t REAL NOT NULL DEFAULT 0.0,
            urgent_total_t REAL NOT NULL DEFAULT 0.0,
            mature_backlog_t REAL NOT NULL DEFAULT 0.0,
            immature_backlog_t REAL NOT NULL DEFAULT 0.0,
            campaign_status TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            l3_count INTEGER NOT NULL DEFAULT 0,
            l2_count INTEGER NOT NULL DEFAULT 0,
            l1_count INTEGER NOT NULL DEFAULT 0,
            l0_count INTEGER NOT NULL DEFAULT 0,
            frozen_count INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (version_id, snapshot_date, machine_code)
        )
        "#,
        [],
    )?;

    // 创建 capacity_pool 表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS capacity_pool (
            version_id TEXT NOT NULL,
            machine_code TEXT NOT NULL,
            plan_date TEXT NOT NULL,
            target_capacity_t REAL NOT NULL,
            limit_capacity_t REAL NOT NULL,
            used_capacity_t REAL NOT NULL DEFAULT 0.0,
            overflow_t REAL NOT NULL DEFAULT 0.0,
            frozen_capacity_t REAL NOT NULL DEFAULT 0.0,
            accumulated_tonnage_t REAL NOT NULL DEFAULT 0.0,
            roll_campaign_id TEXT,
            PRIMARY KEY (version_id, machine_code, plan_date)
        )
        "#,
        [],
    )?;

    // 创建 roll_campaign 表 (换辊活动表，用于 D5 决策)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS roll_campaign (
            campaign_id TEXT PRIMARY KEY,
            machine_code TEXT NOT NULL,
            status TEXT NOT NULL,
            cum_weight_t REAL NOT NULL DEFAULT 0.0,
            suggest_threshold_t REAL NOT NULL,
            hard_limit_t REAL NOT NULL,
            start_date TEXT NOT NULL,
            end_date TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
        [],
    )?;

    // 创建 roller_campaign 表 (真实库表,用于 D5 决策)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS roller_campaign (
            version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
            machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
            campaign_no INTEGER NOT NULL,
            start_date TEXT NOT NULL,
            end_date TEXT,
            cum_weight_t REAL NOT NULL DEFAULT 0,
            suggest_threshold_t REAL NOT NULL,
            hard_limit_t REAL NOT NULL,
            status TEXT NOT NULL,
            path_anchor_material_id TEXT,
            path_anchor_width_mm REAL,
            path_anchor_thickness_mm REAL,
            anchor_source TEXT,
            PRIMARY KEY (version_id, machine_code, campaign_no)
        )
        "#,
        [],
    )?;

    // 创建决策层表 - decision_refresh_log (刷新日志表)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS decision_refresh_log (
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
        )
        "#,
        [],
    )?;

    // 创建决策层表 - decision_refresh_queue (刷新队列表)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS decision_refresh_queue (
            task_id TEXT PRIMARY KEY,
            version_id TEXT NOT NULL,
            trigger_type TEXT NOT NULL,
            trigger_source TEXT,
            is_full_refresh INTEGER NOT NULL DEFAULT 0,
            affected_machines TEXT,
            affected_date_range TEXT,
            status TEXT NOT NULL DEFAULT 'PENDING',
            retry_count INTEGER NOT NULL DEFAULT 0,
            max_retries INTEGER NOT NULL DEFAULT 3,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            started_at TEXT,
            completed_at TEXT,
            error_message TEXT,
            refresh_id TEXT
        )
        "#,
        [],
    )?;

    // 创建决策层表 - decision_day_summary (D1: 日期风险摘要表)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS decision_day_summary (
            version_id TEXT NOT NULL,
            plan_date TEXT NOT NULL,
            risk_score REAL NOT NULL DEFAULT 0.0,
            risk_level TEXT NOT NULL DEFAULT 'LOW',
            capacity_util_pct REAL NOT NULL DEFAULT 0.0,
            top_reasons TEXT NOT NULL DEFAULT '[]',
            affected_machines INTEGER NOT NULL DEFAULT 0,
            bottleneck_machines INTEGER NOT NULL DEFAULT 0,
            has_roll_risk INTEGER NOT NULL DEFAULT 0,
            suggested_actions TEXT NOT NULL DEFAULT '[]',
            refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (version_id, plan_date)
        )
        "#,
        [],
    )?;

    // 创建决策层表 - decision_machine_bottleneck (D4: 机组堵塞概况表)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS decision_machine_bottleneck (
            version_id TEXT NOT NULL,
            machine_code TEXT NOT NULL,
            plan_date TEXT NOT NULL,
            bottleneck_score REAL NOT NULL DEFAULT 0.0,
            bottleneck_level TEXT NOT NULL DEFAULT 'NONE',
            bottleneck_types TEXT NOT NULL DEFAULT '[]',
            reasons TEXT NOT NULL DEFAULT '[]',
            remaining_capacity_t REAL NOT NULL DEFAULT 0.0,
            capacity_utilization REAL NOT NULL DEFAULT 0.0,
            needs_roll_change INTEGER NOT NULL DEFAULT 0,
            structure_violations INTEGER NOT NULL DEFAULT 0,
            pending_materials INTEGER NOT NULL DEFAULT 0,
            suggested_actions TEXT NOT NULL DEFAULT '[]',
            refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (version_id, machine_code, plan_date)
        )
        "#,
        [],
    )?;

    // 创建决策层表 - decision_order_failure (D2: 订单失败表)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS decision_order_failure_set (
            version_id TEXT NOT NULL,
            contract_no TEXT NOT NULL,
            due_date TEXT NOT NULL,
            urgency_level TEXT NOT NULL,
            fail_type TEXT NOT NULL,
            total_materials INTEGER NOT NULL DEFAULT 0,
            unscheduled_count INTEGER NOT NULL DEFAULT 0,
            unscheduled_weight_t REAL NOT NULL DEFAULT 0.0,
            completion_rate REAL NOT NULL,
            days_to_due INTEGER NOT NULL,
            failure_reasons TEXT,
            blocking_factors TEXT NOT NULL,
            suggested_actions TEXT,
            refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (version_id, contract_no)
        )
        "#,
        [],
    )?;

    // 创建决策层表 - decision_cold_stock_profile (D3: 冷料压库表)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS decision_cold_stock_profile (
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
        )
        "#,
        [],
    )?;

    // 创建决策层表 - decision_roll_campaign_alert (D5: 换辊预警表)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS decision_roll_campaign_alert (
            version_id TEXT NOT NULL,
            machine_code TEXT NOT NULL,
            cum_weight_t REAL NOT NULL,
            suggest_threshold_t REAL NOT NULL,
            hard_limit_t REAL NOT NULL,
            campaign_no INTEGER NOT NULL,
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
    )?;

    // 创建决策层表 - decision_capacity_opportunity (D6: 产能优化机会表)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS decision_capacity_opportunity (
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
    )?;

    Ok(())
}

/// 插入测试配置数据
pub fn insert_test_config(conn: &Connection) -> Result<(), Box<dyn Error>> {
    // 季节与适温配置
    conn.execute(
        r#"
        INSERT OR REPLACE INTO config_kv (scope_id, key, value, updated_at) VALUES
        ('global', 'season_mode', 'MANUAL', datetime('now')),
        ('global', 'manual_season', 'WINTER', datetime('now')),
        ('global', 'winter_months', '11,12,1,2,3', datetime('now')),
        ('global', 'min_temp_days_winter', '3', datetime('now')),
        ('global', 'min_temp_days_summer', '4', datetime('now'))
        "#,
        [],
    )?;

    // 机组代码配置
    conn.execute(
        r#"
        INSERT OR REPLACE INTO config_kv (scope_id, key, value, updated_at) VALUES
        ('global', 'standard_finishing_machines', 'H032,H033,H034', datetime('now')),
        ('global', 'machine_offset_days', '4', datetime('now'))
        "#,
        [],
    )?;

    // 数据质量配置
    conn.execute(
        r#"
        INSERT OR REPLACE INTO config_kv (scope_id, key, value, updated_at) VALUES
        ('global', 'weight_anomaly_threshold', '100.0', datetime('now')),
        ('global', 'batch_retention_days', '90', datetime('now'))
        "#,
        [],
    )?;

    // 紧急等级阈值配置
    conn.execute(
        r#"
        INSERT OR REPLACE INTO config_kv (scope_id, key, value, updated_at) VALUES
        ('global', 'urgent_n1_days', '2', datetime('now')),
        ('global', 'urgent_n2_days', '7', datetime('now'))
        "#,
        [],
    )?;

    // 换辊配置
    conn.execute(
        r#"
        INSERT OR REPLACE INTO config_kv (scope_id, key, value, updated_at) VALUES
        ('global', 'roll_suggest_threshold_t', '1500', datetime('now')),
        ('global', 'roll_hard_limit_t', '2500', datetime('now'))
        "#,
        [],
    )?;

    // 产能配置
    conn.execute(
        r#"
        INSERT OR REPLACE INTO config_kv (scope_id, key, value, updated_at) VALUES
        ('global', 'overflow_pct', '0.05', datetime('now'))
        "#,
        [],
    )?;

    // 重算配置
    conn.execute(
        r#"
        INSERT OR REPLACE INTO config_kv (scope_id, key, value, updated_at) VALUES
        ('global', 'recalc_window_days', '7', datetime('now')),
        ('global', 'cascade_window_days', '14', datetime('now'))
        "#,
        [],
    )?;

    // 结构校正配置
    conn.execute(
        r#"
        INSERT OR REPLACE INTO config_kv (scope_id, key, value, updated_at) VALUES
        ('global', 'target_ratio', '{}', datetime('now')),
        ('global', 'deviation_threshold', '0.1', datetime('now'))
        "#,
        [],
    )?;

    Ok(())
}

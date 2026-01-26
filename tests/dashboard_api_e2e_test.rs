// ==========================================
// DashboardAPI 端到端测试
// ==========================================
// 职责: 验证 DashboardAPI 封装 DecisionApi 的完整数据流
// 测试范围: D1-D4 决策查询 + 操作日志查询
// ==========================================

#[path = "test_helpers.rs"]
mod test_helpers;

#[cfg(test)]
mod dashboard_api_e2e_test {
    use std::sync::{Arc, Mutex};
    use rusqlite::Connection;
    use chrono::{NaiveDate, NaiveDateTime};

    // 导入 DashboardAPI
    use hot_rolling_aps::api::dashboard_api::DashboardApi;
    use hot_rolling_aps::repository::action_log_repo::ActionLogRepository;

    // 导入 DecisionAPI 相关（使用重导出路径）
    use hot_rolling_aps::decision::api::{DecisionApi, DecisionApiImpl};
    use hot_rolling_aps::decision::repository::*;
    use hot_rolling_aps::decision::services::{DecisionRefreshService, RefreshScope, RefreshTrigger};
    use hot_rolling_aps::decision::use_cases::impls::*;

    use crate::test_helpers::create_test_db;
    use tempfile::NamedTempFile;

    // ==========================================
    // 测试辅助函数
    // ==========================================

    /// 创建 DashboardAPI 测试环境
    fn setup_dashboard_test_env() -> (
        NamedTempFile,
        String,
        Arc<DashboardApi>,
        Arc<DecisionRefreshService>,
    ) {
        let (temp_file, db_path) = create_test_db().unwrap();
        let conn = Arc::new(Mutex::new(Connection::open(&db_path).unwrap()));

        // 决策层 Repositories
        let day_summary_repo = Arc::new(DaySummaryRepository::new(conn.clone()));
        let bottleneck_repo = Arc::new(BottleneckRepository::new(conn.clone()));

        // 决策层 Use Cases
        let d1_use_case = Arc::new(MostRiskyDayUseCaseImpl::new(day_summary_repo));
        let d4_use_case = Arc::new(MachineBottleneckUseCaseImpl::new(bottleneck_repo));

        // Decision API（只支持 D1 和 D4）
        let decision_api: Arc<dyn DecisionApi> = Arc::new(DecisionApiImpl::new(
            d1_use_case,
            d4_use_case,
        ));

        // ActionLog Repository
        let action_log_repo = Arc::new(ActionLogRepository::new(conn.clone()));

        // DashboardAPI（封装 DecisionApi）
        let dashboard_api = Arc::new(DashboardApi::new(
            decision_api,
            action_log_repo,
        ));

        // 刷新服务
        let refresh_service = Arc::new(DecisionRefreshService::new(conn.clone()));

        (temp_file, db_path, dashboard_api, refresh_service)
    }

    // ==========================================
    // 测试1: DashboardAPI 基本可访问性
    // ==========================================

    #[test]
    fn test_dashboard_api_basic_access() {
        let (_temp, _db_path, dashboard_api, _refresh_service) = setup_dashboard_test_env();

        // 验证 DashboardAPI 可以正常创建
        assert!(Arc::strong_count(&dashboard_api) > 0, "DashboardAPI 应该被正确创建");
    }

    // ==========================================
    // 测试2: D1 决策查询 - 简化版本（向后兼容 Tauri 命令）
    // ==========================================

    #[test]
    fn test_d1_get_most_risky_date_simple() {
        let (_temp, db_path, dashboard_api, refresh_service) = setup_dashboard_test_env();

        let version_id = "dashboard_d1_simple_v001";
        let conn = Connection::open(&db_path).unwrap();

        // 1. 创建版本
        conn.execute(
            "INSERT INTO plan (plan_id, plan_name, plan_type, created_by) VALUES ('TEST_PLAN', 'Test Plan', 'manual', 'test_user')",
            rusqlite::params![],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO plan_version (version_id, plan_id, version_no, status)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![version_id, "TEST_PLAN", 1, "active"],
        )
        .unwrap();

        // 2. 创建测试数据
        let date1 = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2026, 1, 26).unwrap();

        conn.execute(
            "INSERT INTO risk_snapshot (version_id, snapshot_date, machine_code, risk_level, risk_reasons, target_capacity_t, used_capacity_t, limit_capacity_t)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![version_id, date1.to_string(), "M01", "HIGH", "[]", 1000.0, 850.0, 1000.0],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO risk_snapshot (version_id, snapshot_date, machine_code, risk_level, risk_reasons, target_capacity_t, used_capacity_t, limit_capacity_t)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![version_id, date2.to_string(), "M02", "MEDIUM", "[]", 1000.0, 750.0, 1000.0],
        )
        .unwrap();

        // 3. 刷新 D1 决策视图
        let scope = RefreshScope {
            version_id: version_id.to_string(),
            is_full_refresh: true,
            affected_machines: None,
            affected_date_range: None,
        };
        refresh_service
            .refresh_all(scope, RefreshTrigger::ManualRefresh, Some("e2e_test".to_string()))
            .unwrap();

        // 4. 调用 DashboardAPI 简化版本（向后兼容 Tauri 命令）
        let response = dashboard_api.get_most_risky_date(version_id).unwrap();

        // 5. 验证结果
        assert!(!response.items.is_empty(), "应该返回风险日期摘要");
        assert!(response.items.len() <= 10, "简化版本默认限制 10 条记录");

        // 验证第一条记录是最高风险
        let first_item = &response.items[0];
        assert_eq!(first_item.plan_date, date1.to_string(), "第一条应该是风险最高的日期");
        assert!((first_item.risk_score - 85.0).abs() < 6.0, "风险分数应该接近 85.0");
    }

    // ==========================================
    // 测试3: D1 决策查询 - 完整参数版本
    // ==========================================

    #[test]
    fn test_d1_get_most_risky_date_full() {
        let (_temp, db_path, dashboard_api, refresh_service) = setup_dashboard_test_env();

        let version_id = "dashboard_d1_full_v001";
        let conn = Connection::open(&db_path).unwrap();

        // 1. 创建版本
        conn.execute(
            "INSERT INTO plan (plan_id, plan_name, plan_type, created_by) VALUES ('TEST_PLAN', 'Test Plan', 'manual', 'test_user')",
            rusqlite::params![],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO plan_version (version_id, plan_id, version_no, status)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![version_id, "TEST_PLAN", 1, "active"],
        )
        .unwrap();

        // 2. 创建测试数据
        let date1 = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2026, 1, 26).unwrap();

        conn.execute(
            "INSERT INTO risk_snapshot (version_id, snapshot_date, machine_code, risk_level, risk_reasons, target_capacity_t, used_capacity_t, limit_capacity_t)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![version_id, date1.to_string(), "M01", "CRITICAL", "[]", 1000.0, 950.0, 1000.0],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO risk_snapshot (version_id, snapshot_date, machine_code, risk_level, risk_reasons, target_capacity_t, used_capacity_t, limit_capacity_t)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![version_id, date2.to_string(), "M02", "MEDIUM", "[]", 1000.0, 650.0, 1000.0],
        )
        .unwrap();

        // 3. 刷新 D1 决策视图
        let scope = RefreshScope {
            version_id: version_id.to_string(),
            is_full_refresh: true,
            affected_machines: None,
            affected_date_range: None,
        };
        refresh_service
            .refresh_all(scope, RefreshTrigger::ManualRefresh, Some("e2e_test".to_string()))
            .unwrap();

        // 4. 调用 DashboardAPI 完整参数版本（指定日期范围和风险等级过滤）
        let response = dashboard_api
            .get_most_risky_date_full(
                version_id,
                Some(&date1.to_string()),
                Some(&date2.to_string()),
                Some(vec!["CRITICAL".to_string(), "HIGH".to_string()]),
                Some(5),
            )
            .unwrap();

        // 5. 验证结果
        assert!(!response.items.is_empty(), "应该返回过滤后的风险日期摘要");
        assert!(response.items.len() <= 5, "应该限制 5 条记录");

        // 验证只返回 CRITICAL 或 HIGH 级别
        for item in &response.items {
            assert!(
                item.risk_level == "CRITICAL" || item.risk_level == "HIGH",
                "应该只返回 CRITICAL 或 HIGH 级别，实际: {}",
                item.risk_level
            );
        }
    }

    // ==========================================
    // 测试4: 向后兼容方法 - list_risk_snapshots
    // ==========================================

    #[test]
    fn test_backward_compat_list_risk_snapshots() {
        let (_temp, db_path, dashboard_api, refresh_service) = setup_dashboard_test_env();

        let version_id = "dashboard_backward_v001";
        let conn = Connection::open(&db_path).unwrap();

        // 1. 创建版本
        conn.execute(
            "INSERT INTO plan (plan_id, plan_name, plan_type, created_by) VALUES ('TEST_PLAN', 'Test Plan', 'manual', 'test_user')",
            rusqlite::params![],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO plan_version (version_id, plan_id, version_no, status)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![version_id, "TEST_PLAN", 1, "active"],
        )
        .unwrap();

        // 2. 创建测试数据
        let today = chrono::Local::now().date_naive();
        let date_in_range = today + chrono::Duration::days(15);

        conn.execute(
            "INSERT INTO risk_snapshot (version_id, snapshot_date, machine_code, risk_level, risk_reasons, target_capacity_t, used_capacity_t, limit_capacity_t)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![version_id, date_in_range.to_string(), "M01", "HIGH", "[]", 1000.0, 800.0, 1000.0],
        )
        .unwrap();

        // 3. 刷新 D1 决策视图
        let scope = RefreshScope {
            version_id: version_id.to_string(),
            is_full_refresh: true,
            affected_machines: None,
            affected_date_range: None,
        };
        refresh_service
            .refresh_all(scope, RefreshTrigger::ManualRefresh, Some("e2e_test".to_string()))
            .unwrap();

        // 4. 调用向后兼容方法（Tauri 命令使用的接口）
        let response = dashboard_api.list_risk_snapshots(version_id).unwrap();

        // 5. 验证结果
        assert!(!response.items.is_empty(), "应该返回未来 30 天的风险快照");
    }

    // ==========================================
    // 测试5: 操作日志查询 - 按时间范围
    // ==========================================

    #[test]
    fn test_action_log_by_time_range() {
        let (_temp, db_path, dashboard_api, _refresh_service) = setup_dashboard_test_env();

        let conn = Connection::open(&db_path).unwrap();

        // 1. 创建测试日志
        let log_time1 = NaiveDateTime::parse_from_str("2026-01-24 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let log_time2 = NaiveDateTime::parse_from_str("2026-01-24 11:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        conn.execute(
            "INSERT INTO action_log (action_id, version_id, action_type, detail, actor, action_ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                "LOG_001",
                "V001",
                "MANUAL_SCHEDULE",
                "手动排产测试",
                "test_user",
                log_time1.to_string()
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO action_log (action_id, version_id, action_type, detail, actor, action_ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                "LOG_002",
                "V001",
                "MANUAL_ADJUSTMENT",
                "手动调整测试",
                "test_user",
                log_time2.to_string()
            ],
        )
        .unwrap();

        // 2. 查询时间范围内的日志
        let start_time = NaiveDateTime::parse_from_str("2026-01-24 09:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end_time = NaiveDateTime::parse_from_str("2026-01-24 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        let logs = dashboard_api.list_action_logs(start_time, end_time).unwrap();

        // 3. 验证结果
        assert_eq!(logs.len(), 2, "应该返回 2 条日志");
        assert_eq!(logs[0].action_id, "LOG_002", "第一条日志应该是 LOG_002 (降序)");
        assert_eq!(logs[1].action_id, "LOG_001", "第二条日志应该是 LOG_001 (降序)");
    }

    // ==========================================
    // 测试6: 操作日志查询 - 按版本ID
    // ==========================================

    #[test]
    fn test_action_log_by_version() {
        let (_temp, db_path, dashboard_api, _refresh_service) = setup_dashboard_test_env();

        let conn = Connection::open(&db_path).unwrap();

        // 1. 创建测试日志（不同版本）
        let log_time = NaiveDateTime::parse_from_str("2026-01-24 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        conn.execute(
            "INSERT INTO action_log (action_id, version_id, action_type, detail, actor, action_ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                "LOG_V1",
                "VERSION_001",
                "CREATE_VERSION",
                "创建版本 001",
                "test_user",
                log_time.to_string()
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO action_log (action_id, version_id, action_type, detail, actor, action_ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                "LOG_V2",
                "VERSION_002",
                "CREATE_VERSION",
                "创建版本 002",
                "test_user",
                log_time.to_string()
            ],
        )
        .unwrap();

        // 2. 查询指定版本的日志
        let logs = dashboard_api.list_action_logs_by_version("VERSION_001").unwrap();

        // 3. 验证结果
        assert_eq!(logs.len(), 1, "应该只返回 VERSION_001 的日志");
        assert_eq!(logs[0].action_id, "LOG_V1", "日志应该是 LOG_V1");
        assert_eq!(logs[0].version_id, "VERSION_001", "版本ID 应该是 VERSION_001");
    }

    // ==========================================
    // 测试7: 操作日志查询 - 最近操作
    // ==========================================

    #[test]
    fn test_recent_actions() {
        let (_temp, db_path, dashboard_api, _refresh_service) = setup_dashboard_test_env();

        let conn = Connection::open(&db_path).unwrap();

        // 1. 创建多条测试日志
        for i in 1..=5 {
            let log_time = NaiveDateTime::parse_from_str(
                &format!("2026-01-24 10:{:02}:00", i),
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap();

            conn.execute(
                "INSERT INTO action_log (action_id, version_id, action_type, detail, actor, action_ts)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    format!("LOG_{:03}", i),
                    "V001",
                    "TEST_ACTION",
                    format!("测试操作 {}", i),
                    "test_user",
                    log_time.to_string()
                ],
            )
            .unwrap();
        }

        // 2. 查询最近 3 条操作
        let logs = dashboard_api.get_recent_actions(3).unwrap();

        // 3. 验证结果
        assert_eq!(logs.len(), 3, "应该返回最近 3 条日志");

        // 验证按时间降序排列
        for i in 0..logs.len() - 1 {
            assert!(
                logs[i].action_ts >= logs[i + 1].action_ts,
                "日志应该按时间降序排列"
            );
        }
    }

    // ==========================================
    // 测试8: 参数验证 - 空版本ID
    // ==========================================

    #[test]
    fn test_validation_empty_version_id() {
        let (_temp, _db_path, dashboard_api, _refresh_service) = setup_dashboard_test_env();

        // 1. 调用 DashboardAPI 方法，传入空版本ID
        let result = dashboard_api.get_most_risky_date("");

        // 2. 验证返回错误
        assert!(result.is_err(), "空版本ID 应该返回错误");

        if let Err(e) = result {
            assert!(
                e.to_string().contains("版本ID不能为空"),
                "错误信息应该提示版本ID为空"
            );
        }
    }

    // ==========================================
    // 测试9: 参数验证 - 无效时间范围
    // ==========================================

    #[test]
    fn test_validation_invalid_time_range() {
        let (_temp, _db_path, dashboard_api, _refresh_service) = setup_dashboard_test_env();

        // 1. 创建无效的时间范围（开始时间晚于结束时间）
        let start_time = NaiveDateTime::parse_from_str("2026-01-24 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end_time = NaiveDateTime::parse_from_str("2026-01-24 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        // 2. 调用查询方法
        let result = dashboard_api.list_action_logs(start_time, end_time);

        // 3. 验证返回错误
        assert!(result.is_err(), "无效时间范围应该返回错误");

        if let Err(e) = result {
            assert!(
                e.to_string().contains("开始时间不能晚于结束时间"),
                "错误信息应该提示时间范围无效"
            );
        }
    }
}

// ==========================================
// P0-2: 决策刷新状态 API 测试
// ==========================================

#[path = "test_helpers.rs"]
mod test_helpers;

#[cfg(test)]
mod decision_refresh_status_test {
    use hot_rolling_aps::api::dashboard_api::DashboardApi;
    use hot_rolling_aps::decision::api::dto::{
        CapacityOpportunityResponse, ColdStockProfileResponse, DecisionDaySummaryResponse,
        GetCapacityOpportunityRequest, GetColdStockProfileRequest, GetDecisionDaySummaryRequest,
        GetMachineBottleneckProfileRequest, ListOrderFailureSetRequest, ListRollCampaignAlertsRequest,
        MachineBottleneckProfileResponse, OrderFailureSetResponse, RollCampaignAlertsResponse,
    };
    use hot_rolling_aps::decision::api::decision_api::DecisionApi;
    use hot_rolling_aps::repository::action_log_repo::ActionLogRepository;
    use hot_rolling_aps::repository::decision_refresh_repo::DecisionRefreshRepository;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    use crate::test_helpers::create_test_db;

    struct StubDecisionApi;

    impl DecisionApi for StubDecisionApi {
        fn get_decision_day_summary(
            &self,
            _request: GetDecisionDaySummaryRequest,
        ) -> Result<DecisionDaySummaryResponse, String> {
            Err("NOT_IMPLEMENTED".to_string())
        }

        fn get_machine_bottleneck_profile(
            &self,
            _request: GetMachineBottleneckProfileRequest,
        ) -> Result<MachineBottleneckProfileResponse, String> {
            Err("NOT_IMPLEMENTED".to_string())
        }

        fn list_order_failure_set(
            &self,
            _request: ListOrderFailureSetRequest,
        ) -> Result<OrderFailureSetResponse, String> {
            Err("NOT_IMPLEMENTED".to_string())
        }

        fn get_cold_stock_profile(
            &self,
            _request: GetColdStockProfileRequest,
        ) -> Result<ColdStockProfileResponse, String> {
            Err("NOT_IMPLEMENTED".to_string())
        }

        fn list_roll_campaign_alerts(
            &self,
            _request: ListRollCampaignAlertsRequest,
        ) -> Result<RollCampaignAlertsResponse, String> {
            Err("NOT_IMPLEMENTED".to_string())
        }

        fn get_capacity_opportunity(
            &self,
            _request: GetCapacityOpportunityRequest,
        ) -> Result<CapacityOpportunityResponse, String> {
            Err("NOT_IMPLEMENTED".to_string())
        }
    }

    #[test]
    fn test_get_refresh_status_inflight_failed_and_completed() {
        let (_temp_file, db_path) = create_test_db().unwrap();
        let conn = Arc::new(Mutex::new(Connection::open(&db_path).unwrap()));

        let action_log_repo = Arc::new(ActionLogRepository::new(conn.clone()));
        let decision_refresh_repo = Arc::new(DecisionRefreshRepository::new(conn.clone()));
        let decision_api: Arc<dyn DecisionApi> = Arc::new(StubDecisionApi);

        let dashboard_api = DashboardApi::new(decision_api, action_log_repo, decision_refresh_repo);

        let version_id_idle = "V_REFRESH_STATUS_IDLE";
        let version_id_pending = "V_REFRESH_STATUS_PENDING";
        let version_id_failed = "V_REFRESH_STATUS_FAILED";
        let version_id_completed = "V_REFRESH_STATUS_COMPLETED";

        // 1) 初始：无任务
        let status = dashboard_api.get_refresh_status(version_id_idle).unwrap();
        assert!(!status.is_refreshing);
        assert_eq!(status.status, "IDLE");
        assert_eq!(status.queue_counts.pending, 0);
        assert_eq!(status.queue_counts.running, 0);

        // 2) 插入一个 PENDING 任务 → is_refreshing=true
        {
            let c = conn.lock().unwrap();
            c.execute(
                r#"
                INSERT INTO decision_refresh_queue (
                    task_id, version_id, trigger_type, trigger_source, is_full_refresh,
                    status, retry_count, max_retries
                ) VALUES (?1, ?2, ?3, ?4, ?5, 'PENDING', 0, 3)
                "#,
                rusqlite::params![
                    Uuid::new_v4().to_string(),
                    version_id_pending,
                    "ManualRefresh",
                    "test",
                    1
                ],
            )
            .unwrap();
        }

        let status = dashboard_api.get_refresh_status(version_id_pending).unwrap();
        assert!(status.is_refreshing);
        assert_eq!(status.status, "REFRESHING");
        assert!(status.queue_counts.pending >= 1);

        // 3) 插入一个 FAILED 任务，并确保它是最新的 → status=FAILED
        let failed_task_id = Uuid::new_v4().to_string();
        {
            let c = conn.lock().unwrap();
            c.execute(
                r#"
                INSERT INTO decision_refresh_queue (
                    task_id, version_id, trigger_type, trigger_source, is_full_refresh,
                    status, retry_count, max_retries, error_message, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, 'FAILED', 1, 3, ?6, datetime('now', '+1 second'))
                "#,
                rusqlite::params![
                    failed_task_id,
                    version_id_failed,
                    "PlanItemChanged",
                    "test",
                    1,
                    "boom"
                ],
            )
            .unwrap();
        }

        let status = dashboard_api.get_refresh_status(version_id_failed).unwrap();
        assert!(!status.is_refreshing);
        assert_eq!(status.status, "FAILED");
        assert_eq!(status.latest_task.as_ref().unwrap().task_id, failed_task_id);
        assert_eq!(status.last_error.as_deref(), Some("boom"));

        // 4) 插入一个 COMPLETED 任务 + 对应 log，并确保它是最新的 → latest_log 取到
        let refresh_id = "R_TEST_001";
        let completed_task_id = Uuid::new_v4().to_string();
        {
            let c = conn.lock().unwrap();
            c.execute(
                r#"
                INSERT INTO decision_refresh_queue (
                    task_id, version_id, trigger_type, trigger_source, is_full_refresh,
                    status, retry_count, max_retries, refresh_id, completed_at, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, 'COMPLETED', 0, 3, ?6, datetime('now'), datetime('now', '+2 second'))
                "#,
                rusqlite::params![
                    completed_task_id,
                    version_id_completed,
                    "ManualRefresh",
                    "test",
                    1,
                    refresh_id
                ],
            )
            .unwrap();

            c.execute(
                r#"
                INSERT INTO decision_refresh_log (
                    refresh_id, version_id, trigger_type, trigger_source, is_full_refresh,
                    refreshed_tables, rows_affected, started_at, completed_at, duration_ms, status
                ) VALUES (?1, ?2, ?3, ?4, ?5, '[]', 0, datetime('now'), datetime('now'), 10, 'SUCCESS')
                "#,
                rusqlite::params![refresh_id, version_id_completed, "ManualRefresh", "test", 1],
            )
            .unwrap();
        }

        let status = dashboard_api.get_refresh_status(version_id_completed).unwrap();
        assert!(!status.is_refreshing);
        assert_eq!(status.status, "IDLE");
        assert_eq!(status.latest_task.as_ref().unwrap().task_id, completed_task_id);
        assert_eq!(status.latest_log.as_ref().unwrap().refresh_id, refresh_id);
    }
}

// ==========================================
// 策略草案持久化测试（P0-1）
// ==========================================
// 目标:
// - 草案生成后可从数据库恢复（模拟“重启后仍可查询/发布”）
// - 发布后草案状态变更为 PUBLISHED，且不再出现在 DRAFT 列表
// ==========================================

#[path = "test_helpers.rs"]
mod test_helpers;

#[cfg(test)]
mod strategy_draft_persistence_test {
    use chrono::{Duration, NaiveDateTime};
    use chrono::NaiveDate;
    use hot_rolling_aps::api::PlanApi;
    use hot_rolling_aps::api::plan_api::StrategyDraftSummary;
    use hot_rolling_aps::config::config_manager::ConfigManager;
    use hot_rolling_aps::engine::{
        capacity_filler::CapacityFiller,
        eligibility::EligibilityEngine,
        priority::PrioritySorter,
        recalc::RecalcEngine,
        risk::RiskEngine,
        urgency::UrgencyEngine,
    };
    use hot_rolling_aps::repository::{
        action_log_repo::ActionLogRepository,
        capacity_repo::CapacityPoolRepository,
        material_repo::{MaterialMasterRepository, MaterialStateRepository},
        plan_repo::{PlanItemRepository, PlanRepository, PlanVersionRepository},
        risk_repo::RiskSnapshotRepository,
        strategy_draft_repo::{StrategyDraftEntity, StrategyDraftRepository, StrategyDraftStatus},
    };
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    use crate::test_helpers::create_test_db;

    fn build_plan_api(db_path: &str) -> Arc<PlanApi> {
        let conn = Arc::new(Mutex::new(Connection::open(db_path).unwrap()));

        let material_master_repo = Arc::new(MaterialMasterRepository::new(db_path).unwrap());
        let material_state_repo = Arc::new(MaterialStateRepository::new(db_path).unwrap());
        let plan_repo = Arc::new(PlanRepository::new(conn.clone()));
        let plan_version_repo = Arc::new(PlanVersionRepository::new(conn.clone()));
        let plan_item_repo = Arc::new(PlanItemRepository::new(conn.clone()));
        let action_log_repo = Arc::new(ActionLogRepository::new(conn.clone()));
        let strategy_draft_repo = Arc::new(StrategyDraftRepository::new(conn.clone()));
        let risk_snapshot_repo = Arc::new(RiskSnapshotRepository::new(db_path).unwrap());
        let capacity_pool_repo = Arc::new(CapacityPoolRepository::new(db_path.to_string()).unwrap());

        let config_manager = Arc::new(ConfigManager::new(db_path).unwrap());
        let eligibility_engine = Arc::new(EligibilityEngine::new(config_manager.clone()));
        let urgency_engine = Arc::new(UrgencyEngine::new());
        let priority_sorter = Arc::new(PrioritySorter::new());
        let capacity_filler = Arc::new(CapacityFiller::new());
        let risk_engine = Arc::new(RiskEngine::new());

        let recalc_engine = Arc::new(RecalcEngine::with_default_config(
            plan_version_repo.clone(),
            plan_item_repo.clone(),
            material_state_repo.clone(),
            material_master_repo.clone(),
            capacity_pool_repo,
            action_log_repo.clone(),
            eligibility_engine,
            urgency_engine,
            priority_sorter,
            capacity_filler,
            config_manager.clone(),
            None, // 测试环境不需要事件发布
        ));

        Arc::new(PlanApi::new(
            plan_repo,
            plan_version_repo,
            plan_item_repo,
            material_state_repo,
            material_master_repo,
            strategy_draft_repo,
            action_log_repo,
            risk_snapshot_repo,
            config_manager,
            recalc_engine,
            risk_engine,
            None, // 测试环境不需要事件发布
        ))
    }

    #[test]
    fn test_strategy_draft_persisted_can_be_listed_after_restart_and_published() {
        let (_temp_file, db_path) = create_test_db().unwrap();

        let operator = "test_user";

        // ===== 1) 创建基准版本并激活 =====
        let plan_api = build_plan_api(&db_path);

        let plan_id = plan_api
            .create_plan("策略草案持久化测试".to_string(), operator.to_string())
            .unwrap();

        let base_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let base_version_id = plan_api
            .create_version(
                plan_id,
                7,
                Some(base_date),
                Some("基准版本".to_string()),
                operator.to_string(),
            )
            .unwrap();

        plan_api.activate_version(&base_version_id, operator).unwrap();

        // ===== 2) 生成草案（落库） =====
        let from = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 1, 26).unwrap();

        let gen = plan_api
            .generate_strategy_drafts(
                &base_version_id,
                from,
                to,
                vec!["balanced".to_string()],
                operator,
            )
            .unwrap();

        assert_eq!(gen.drafts.len(), 1);
        let draft_id = gen.drafts[0].draft_id.clone();

        // ===== 3) 模拟“重启后恢复”：重建 PlanApi，再次查询 =====
        let restarted_api = build_plan_api(&db_path);

        let list = restarted_api
            .list_strategy_drafts(
                &base_version_id,
                from,
                to,
                Some("DRAFT".to_string()),
                Some(200),
            )
            .unwrap();

        assert!(
            list.drafts.iter().any(|d| d.draft_id == draft_id),
            "应能在 list_strategy_drafts 中找到生成的草案"
        );

        let detail = restarted_api.get_strategy_draft_detail(&draft_id).unwrap();
        assert_eq!(detail.draft_id, draft_id);

        // ===== 4) 发布草案：生成正式版本 + 草案标记为 PUBLISHED =====
        let apply = restarted_api.apply_strategy_draft(&draft_id, operator).unwrap();
        assert!(apply.success);

        let list_draft_after_apply = restarted_api
            .list_strategy_drafts(
                &base_version_id,
                from,
                to,
                Some("DRAFT".to_string()),
                Some(200),
            )
            .unwrap();
        assert!(
            list_draft_after_apply.drafts.is_empty(),
            "草案发布后，不应再出现在 DRAFT 列表中"
        );

        let list_published = restarted_api
            .list_strategy_drafts(
                &base_version_id,
                from,
                to,
                Some("PUBLISHED".to_string()),
                Some(200),
            )
            .unwrap();
        assert!(
            list_published.drafts.iter().any(|d| d.draft_id == draft_id),
            "草案发布后，应可在 PUBLISHED 列表中找到该草案"
        );
    }

    #[test]
    fn test_expired_draft_cannot_be_published_and_can_be_cleaned() {
        let (_temp_file, db_path) = create_test_db().unwrap();

        let operator = "test_user";

        // ===== 1) 创建基准版本并激活 =====
        let plan_api = build_plan_api(&db_path);

        let plan_id = plan_api
            .create_plan("过期草案测试".to_string(), operator.to_string())
            .unwrap();

        let base_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let base_version_id = plan_api
            .create_version(
                plan_id,
                7,
                Some(base_date),
                Some("基准版本".to_string()),
                operator.to_string(),
            )
            .unwrap();

        plan_api.activate_version(&base_version_id, operator).unwrap();

        // ===== 2) 直接插入一个“已过期但仍是 DRAFT 状态”的草案 =====
        let from = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 1, 26).unwrap();

        let now = chrono::Local::now().naive_local();
        let created_at: NaiveDateTime = now - Duration::days(10);
        let expires_at: NaiveDateTime = now - Duration::hours(1);

        let draft_id = Uuid::new_v4().to_string();
        let summary = StrategyDraftSummary {
            draft_id: draft_id.clone(),
            base_version_id: base_version_id.clone(),
            strategy: "balanced".to_string(),
            plan_items_count: 0,
            frozen_items_count: 0,
            calc_items_count: 0,
            mature_count: 0,
            immature_count: 0,
            total_capacity_used_t: 0.0,
            overflow_days: 0,
            moved_count: 0,
            added_count: 0,
            removed_count: 0,
            squeezed_out_count: 0,
            message: "expired draft".to_string(),
        };

        let draft = StrategyDraftEntity {
            draft_id: draft_id.clone(),
            base_version_id: base_version_id.clone(),
            plan_date_from: from,
            plan_date_to: to,
            strategy_key: "balanced".to_string(),
            strategy_base: "balanced".to_string(),
            strategy_title_cn: "均衡方案".to_string(),
            strategy_params_json: None,
            status: StrategyDraftStatus::Draft,
            created_by: operator.to_string(),
            created_at,
            expires_at,
            published_as_version_id: None,
            published_by: None,
            published_at: None,
            locked_by: None,
            locked_at: None,
            summary_json: serde_json::to_string(&summary).unwrap(),
            diff_items_json: "[]".to_string(),
            diff_items_total: 0,
            diff_items_truncated: false,
        };

        let conn = Arc::new(Mutex::new(Connection::open(&db_path).unwrap()));
        let draft_repo = StrategyDraftRepository::new(conn);
        draft_repo.insert(&draft).unwrap();

        // list_strategy_drafts 不应返回该草案（会 best-effort 标记为 EXPIRED）
        let list = plan_api
            .list_strategy_drafts(
                &base_version_id,
                from,
                to,
                Some("DRAFT".to_string()),
                Some(200),
            )
            .unwrap();
        assert!(
            list.drafts.iter().all(|d| d.draft_id != draft_id),
            "已过期草案不应出现在 DRAFT 列表中"
        );

        // apply_strategy_draft 不应允许发布过期草案
        let err = plan_api.apply_strategy_draft(&draft_id, operator).unwrap_err();
        match err {
            hot_rolling_aps::api::ApiError::InvalidInput(_) => {}
            other => panic!("过期草案发布应返回 InvalidInput，实际: {:?}", other),
        }

        // cleanup_expired_strategy_drafts 可以清理 created_at 很久之前的过期草案
        let cleanup = plan_api.cleanup_expired_strategy_drafts(7).unwrap();
        assert!(
            cleanup.deleted_count >= 1,
            "应至少清理 1 条过期草案，实际: {}",
            cleanup.deleted_count
        );
    }
}

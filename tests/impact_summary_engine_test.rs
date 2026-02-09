// ==========================================
// ImpactSummaryEngine 集成测试
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 9. Impact Summary Engine
// 职责: 验证影响摘要引擎的6大分析维度
// ==========================================

use chrono::NaiveDate;
use hot_rolling_aps::domain::capacity::CapacityPool;
use hot_rolling_aps::domain::material::MaterialState;
use hot_rolling_aps::domain::plan::PlanItem;
use hot_rolling_aps::domain::risk::RiskSnapshot;
use hot_rolling_aps::domain::types::{RiskLevel, SchedState, UrgentLevel, RushLevel};
use hot_rolling_aps::engine::ImpactSummaryEngine;
use std::collections::HashMap;

// ==========================================
// 测试辅助函数
// ==========================================

/// 创建测试用PlanItem
fn create_test_plan_item(
    material_id: &str,
    plan_date: NaiveDate,
    machine_code: &str,
    sequence_no: i32,
    weight_t: f64,
) -> PlanItem {
    PlanItem {
        version_id: "test_v1".to_string(),
        material_id: material_id.to_string(),
        machine_code: machine_code.to_string(),
        plan_date,
        seq_no: sequence_no,
        weight_t,
        source_type: "CALC".to_string(),
        locked_in_plan: false,
        force_release_in_plan: false,
        violation_flags: None,
        urgent_level: Some("L1".to_string()),
        sched_state: Some("READY".to_string()),
        assign_reason: Some("test".to_string()),
        steel_grade: None,
        width_mm: None,
        thickness_mm: None,
        contract_no: None,
        due_date: None,
        scheduled_date: None,
        scheduled_machine_code: None,
    }
}

/// 创建测试用MaterialState
fn create_test_material_state(
    material_id: &str,
    urgent_level: UrgentLevel,
    lock_flag: bool,
    sched_state: SchedState,
) -> MaterialState {
    MaterialState {
        material_id: material_id.to_string(),
        sched_state,
        lock_flag,
        force_release_flag: false,
        urgent_level,
        urgent_reason: None,
        rush_level: RushLevel::L0,
        rolling_output_age_days: 7,
        ready_in_days: 0,
        earliest_sched_date: Some(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()),
        stock_age_days: 5,
        scheduled_date: Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
        scheduled_machine_code: Some("H032".to_string()),
        seq_no: Some(1),
        manual_urgent_flag: false,
        user_confirmed: false,
        user_confirmed_at: None,
        user_confirmed_by: None,
        user_confirmed_reason: None,
        in_frozen_zone: false,
        last_calc_version_id: Some("test_v1".to_string()),
        updated_at: chrono::Utc::now(),
        updated_by: Some("test_user".to_string()),
    }
}

/// 创建测试用CapacityPool
fn create_test_capacity_pool(
    machine_code: &str,
    plan_date: NaiveDate,
    target_capacity_t: f64,
    limit_capacity_t: f64,
    used_capacity_t: f64,
) -> CapacityPool {
    CapacityPool {
        version_id: "test_v1".to_string(),
        machine_code: machine_code.to_string(),
        plan_date,
        target_capacity_t,
        limit_capacity_t,
        used_capacity_t,
        overflow_t: (used_capacity_t - limit_capacity_t).max(0.0),
        frozen_capacity_t: 0.0,
        accumulated_tonnage_t: 0.0,
        roll_campaign_id: None,
    }
}

/// 创建测试用RiskSnapshot
fn create_test_risk_snapshot(
    machine_code: &str,
    snapshot_date: NaiveDate,
    risk_level: RiskLevel,
) -> RiskSnapshot {
    RiskSnapshot {
        snapshot_id: format!("risk_{}_{}", machine_code, snapshot_date),
        version_id: "test_v1".to_string(),
        machine_code: machine_code.to_string(),
        snapshot_date,
        used_capacity_t: 80.0,
        target_capacity_t: 100.0,
        limit_capacity_t: 120.0,
        overflow_t: 0.0,
        urgent_total_t: 0.0,
        l3_count: 0,
        l2_count: 0,
        mature_backlog_t: 0.0,
        immature_backlog_t: 0.0,
        risk_level,
        risk_reason: format!("风险等级: {:?}", risk_level),
        roll_status: Some("NORMAL".to_string()),
        roll_risk: None,
        created_at: chrono::Utc::now().naive_utc(),
    }
}

// ==========================================
// 测试1: 材料移动分析
// ==========================================
#[test]
fn test_impact_summary_material_moved() {
    let engine = ImpactSummaryEngine::new();

    let date1 = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2026, 1, 16).unwrap();

    // M001从1月15日移动到1月16日
    let before_items = vec![
        create_test_plan_item("M001", date1, "H032", 1, 50.0),
        create_test_plan_item("M002", date1, "H032", 2, 50.0),
    ];

    let after_items = vec![
        create_test_plan_item("M001", date2, "H032", 1, 50.0), // 移动到16日
        create_test_plan_item("M002", date1, "H032", 2, 50.0), // 不变
    ];

    let before_pools = vec![];
    let after_pools = vec![];
    let before_risks = vec![];
    let after_risks = vec![];
    let all_materials = vec![
        create_test_material_state("M001", UrgentLevel::L1, false, SchedState::Ready),
        create_test_material_state("M002", UrgentLevel::L1, false, SchedState::Ready),
    ];
    let material_weights = HashMap::from([
        ("M001".to_string(), 50.0),
        ("M002".to_string(), 50.0),
    ]);

    let impact = engine.generate_impact(
        &before_items,
        &after_items,
        &before_pools,
        &after_pools,
        &before_risks,
        &after_risks,
        &all_materials,
        &material_weights,
    );

    // 验证移动计数
    assert_eq!(impact.moved_count, 1, "应该有1个材料被移动");
    assert_eq!(impact.squeezed_out_count, 0);
    assert_eq!(impact.added_count, 0);

    // 验证材料变更详情
    assert_eq!(impact.material_changes.len(), 1);
    let change = &impact.material_changes[0];
    assert_eq!(change.material_no, "M001");
    assert_eq!(change.change_type, "moved");
    assert_eq!(change.from_date, Some(date1));
    assert_eq!(change.to_date, Some(date2));
}

// ==========================================
// 测试2: 材料挤出分析
// ==========================================
#[test]
fn test_impact_summary_material_squeezed_out() {
    let engine = ImpactSummaryEngine::new();

    let date1 = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();

    // M002在before中存在,在after中被挤出
    let before_items = vec![
        create_test_plan_item("M001", date1, "H032", 1, 50.0),
        create_test_plan_item("M002", date1, "H032", 2, 50.0),
        create_test_plan_item("M003", date1, "H032", 3, 30.0),
    ];

    let after_items = vec![
        create_test_plan_item("M001", date1, "H032", 1, 50.0),
        create_test_plan_item("M003", date1, "H032", 2, 30.0),
        // M002被挤出
    ];

    let before_pools = vec![];
    let after_pools = vec![];
    let before_risks = vec![];
    let after_risks = vec![];
    let all_materials = vec![
        create_test_material_state("M001", UrgentLevel::L1, false, SchedState::Ready),
        create_test_material_state("M002", UrgentLevel::L1, false, SchedState::Ready),
        create_test_material_state("M003", UrgentLevel::L1, false, SchedState::Ready),
    ];
    let material_weights = HashMap::from([
        ("M001".to_string(), 50.0),
        ("M002".to_string(), 50.0),
        ("M003".to_string(), 30.0),
    ]);

    let impact = engine.generate_impact(
        &before_items,
        &after_items,
        &before_pools,
        &after_pools,
        &before_risks,
        &after_risks,
        &all_materials,
        &material_weights,
    );

    // 验证挤出计数
    assert_eq!(impact.moved_count, 0);
    assert_eq!(impact.squeezed_out_count, 1, "应该有1个材料被挤出");
    assert_eq!(impact.added_count, 0);

    // 验证材料变更详情
    let squeezed_change = impact
        .material_changes
        .iter()
        .find(|c| c.change_type == "squeezed_out")
        .expect("应该找到squeezed_out类型的变更");
    assert_eq!(squeezed_change.material_no, "M002");
    assert_eq!(squeezed_change.from_date, Some(date1));
    assert_eq!(squeezed_change.to_date, None);
}

// ==========================================
// 测试3: 材料新增分析
// ==========================================
#[test]
fn test_impact_summary_material_added() {
    let engine = ImpactSummaryEngine::new();

    let date1 = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();

    // M003在before中不存在,在after中新增
    let before_items = vec![
        create_test_plan_item("M001", date1, "H032", 1, 50.0),
    ];

    let after_items = vec![
        create_test_plan_item("M001", date1, "H032", 1, 50.0),
        create_test_plan_item("M003", date1, "H032", 2, 30.0), // 新增
    ];

    let before_pools = vec![];
    let after_pools = vec![];
    let before_risks = vec![];
    let after_risks = vec![];
    let all_materials = vec![
        create_test_material_state("M001", UrgentLevel::L1, false, SchedState::Ready),
        create_test_material_state("M003", UrgentLevel::L1, false, SchedState::Ready),
    ];
    let material_weights = HashMap::from([
        ("M001".to_string(), 50.0),
        ("M003".to_string(), 30.0),
    ]);

    let impact = engine.generate_impact(
        &before_items,
        &after_items,
        &before_pools,
        &after_pools,
        &before_risks,
        &after_risks,
        &all_materials,
        &material_weights,
    );

    // 验证新增计数
    assert_eq!(impact.moved_count, 0);
    assert_eq!(impact.squeezed_out_count, 0);
    assert_eq!(impact.added_count, 1, "应该有1个材料被新增");

    // 验证材料变更详情
    let added_change = impact
        .material_changes
        .iter()
        .find(|c| c.change_type == "added")
        .expect("应该找到added类型的变更");
    assert_eq!(added_change.material_no, "M003");
    assert_eq!(added_change.from_date, None);
    assert_eq!(added_change.to_date, Some(date1));
}

// ==========================================
// 测试4: 产能变化分析
// ==========================================
#[test]
fn test_impact_summary_capacity_delta() {
    let engine = ImpactSummaryEngine::new();

    let date1 = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    // 产能从80吨增加到100吨
    let before_pools = vec![create_test_capacity_pool("H032", date1, 100.0, 120.0, 80.0)];

    let after_pools = vec![create_test_capacity_pool("H032", date1, 100.0, 120.0, 100.0)];

    let before_items = vec![];
    let after_items = vec![];
    let before_risks = vec![];
    let after_risks = vec![];
    let all_materials = vec![];
    let material_weights = HashMap::new();

    let impact = engine.generate_impact(
        &before_items,
        &after_items,
        &before_pools,
        &after_pools,
        &before_risks,
        &after_risks,
        &all_materials,
        &material_weights,
    );

    // 验证产能变化
    assert!(
        (impact.capacity_delta_t - 20.0).abs() < 0.01,
        "产能变化应该是20.0吨,实际: {}",
        impact.capacity_delta_t
    );

    // 验证产能变更详情
    assert_eq!(impact.capacity_changes.len(), 1);
    let change = &impact.capacity_changes[0];
    assert_eq!(change.machine_code, "H032");
    assert_eq!(change.date, date1);
    assert!((change.used_capacity_before_t - 80.0).abs() < 0.01);
    assert!((change.used_capacity_after_t - 100.0).abs() < 0.01);
    assert!((change.delta_t - 20.0).abs() < 0.01);
}

// ==========================================
// 测试5: 超限变化分析
// ==========================================
#[test]
fn test_impact_summary_overflow_delta() {
    let engine = ImpactSummaryEngine::new();

    let date1 = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    // before: used=100, limit=100, overflow=0
    // after: used=120, limit=100, overflow=20
    let before_pools = vec![create_test_capacity_pool("H032", date1, 100.0, 100.0, 100.0)];

    let after_pools = vec![create_test_capacity_pool("H032", date1, 100.0, 100.0, 120.0)];

    let before_items = vec![];
    let after_items = vec![];
    let before_risks = vec![];
    let after_risks = vec![];
    let all_materials = vec![];
    let material_weights = HashMap::new();

    let impact = engine.generate_impact(
        &before_items,
        &after_items,
        &before_pools,
        &after_pools,
        &before_risks,
        &after_risks,
        &all_materials,
        &material_weights,
    );

    // 验证超限变化: 从0增加到20
    assert!(
        (impact.overflow_delta_t - 20.0).abs() < 0.01,
        "超限变化应该是20.0吨,实际: {}",
        impact.overflow_delta_t
    );
}

// ==========================================
// 测试6: 风险等级变化分析
// ==========================================
#[test]
fn test_impact_summary_risk_level_change() {
    let engine = ImpactSummaryEngine::new();

    let date1 = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    // 风险从GREEN升级到YELLOW
    let before_risks = vec![create_test_risk_snapshot("H032", date1, RiskLevel::Green)];

    let after_risks = vec![create_test_risk_snapshot("H032", date1, RiskLevel::Yellow)];

    let before_items = vec![];
    let after_items = vec![];
    let before_pools = vec![];
    let after_pools = vec![];
    let all_materials = vec![];
    let material_weights = HashMap::new();

    let impact = engine.generate_impact(
        &before_items,
        &after_items,
        &before_pools,
        &after_pools,
        &before_risks,
        &after_risks,
        &all_materials,
        &material_weights,
    );

    // 验证风险等级变化
    assert_eq!(impact.risk_level_before, "GREEN");
    assert_eq!(impact.risk_level_after, "YELLOW");

    // 验证风险变更详情
    assert_eq!(impact.risk_changes.len(), 1);
    let risk_change = &impact.risk_changes[0];
    assert_eq!(risk_change.machine_code, "H032");
    assert_eq!(risk_change.date, date1);
    assert_eq!(risk_change.risk_before, "GREEN");
    assert_eq!(risk_change.risk_after, "YELLOW");
}

// ==========================================
// 测试7: 换辊影响分析
// ==========================================
#[test]
fn test_impact_summary_roll_campaign_impact() {
    let engine = ImpactSummaryEngine::new();

    let date1 = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    // before: 100吨材料
    let before_items = vec![
        create_test_plan_item("M001", date1, "H032", 1, 50.0),
        create_test_plan_item("M002", date1, "H032", 2, 50.0),
    ];

    // after: 120吨材料 (增加20吨,超过1吨阈值)
    let after_items = vec![
        create_test_plan_item("M001", date1, "H032", 1, 50.0),
        create_test_plan_item("M002", date1, "H032", 2, 50.0),
        create_test_plan_item("M003", date1, "H032", 3, 20.0), // 新增
    ];

    let before_pools = vec![];
    let after_pools = vec![];
    let before_risks = vec![];
    let after_risks = vec![];
    let all_materials = vec![];
    let material_weights = HashMap::from([
        ("M001".to_string(), 50.0),
        ("M002".to_string(), 50.0),
        ("M003".to_string(), 20.0),
    ]);

    let impact = engine.generate_impact(
        &before_items,
        &after_items,
        &before_pools,
        &after_pools,
        &before_risks,
        &after_risks,
        &all_materials,
        &material_weights,
    );

    // 验证换辊影响
    assert!(
        impact.roll_campaign_affected,
        "换辊窗口应该受到影响(吨位变化20吨 > 1吨阈值)"
    );
    assert!(impact.roll_tonnage_delta_t.is_some());
    assert!(
        (impact.roll_tonnage_delta_t.unwrap() - 20.0).abs() < 0.01,
        "换辊吨位变化应该是20.0吨"
    );
}

// ==========================================
// 测试8: 紧急单影响分析
// ==========================================
#[test]
fn test_impact_summary_urgent_material_impact() {
    let engine = ImpactSummaryEngine::new();

    let date1 = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2026, 1, 16).unwrap();

    // 移动3个材料: M001(L3), M002(L2), M003(L1)
    let before_items = vec![
        create_test_plan_item("M001", date1, "H032", 1, 50.0),
        create_test_plan_item("M002", date1, "H032", 2, 50.0),
        create_test_plan_item("M003", date1, "H032", 3, 30.0),
    ];

    let after_items = vec![
        create_test_plan_item("M001", date2, "H032", 1, 50.0), // 移动
        create_test_plan_item("M002", date2, "H032", 2, 50.0), // 移动
        create_test_plan_item("M003", date2, "H032", 3, 30.0), // 移动
    ];

    let before_pools = vec![];
    let after_pools = vec![];
    let before_risks = vec![];
    let after_risks = vec![];
    let all_materials = vec![
        create_test_material_state("M001", UrgentLevel::L3, false, SchedState::Ready),
        create_test_material_state("M002", UrgentLevel::L2, false, SchedState::Ready),
        create_test_material_state("M003", UrgentLevel::L1, false, SchedState::Ready),
    ];
    let material_weights = HashMap::from([
        ("M001".to_string(), 50.0),
        ("M002".to_string(), 50.0),
        ("M003".to_string(), 30.0),
    ]);

    let impact = engine.generate_impact(
        &before_items,
        &after_items,
        &before_pools,
        &after_pools,
        &before_risks,
        &after_risks,
        &all_materials,
        &material_weights,
    );

    // 验证紧急单影响
    assert_eq!(
        impact.urgent_material_affected, 2,
        "应该有2个紧急材料受影响(L2+L3)"
    );
    assert_eq!(impact.l3_critical_count, 1, "应该有1个L3红线材料");
}

// ==========================================
// 测试9: 锁定冲突检测
// ==========================================
#[test]
fn test_impact_summary_locked_conflicts() {
    let engine = ImpactSummaryEngine::new();

    let date1 = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2026, 1, 16).unwrap();

    // M001被锁定,但被移动了(冲突)
    let before_items = vec![
        create_test_plan_item("M001", date1, "H032", 1, 50.0),
        create_test_plan_item("M002", date1, "H032", 2, 50.0),
    ];

    let after_items = vec![
        create_test_plan_item("M001", date2, "H032", 1, 50.0), // 移动(冲突)
        create_test_plan_item("M002", date1, "H032", 2, 50.0), // 不变
    ];

    let before_pools = vec![];
    let after_pools = vec![];
    let before_risks = vec![];
    let after_risks = vec![];
    let all_materials = vec![
        create_test_material_state("M001", UrgentLevel::L2, true, SchedState::Ready), // locked
        create_test_material_state("M002", UrgentLevel::L1, false, SchedState::Ready),
    ];
    let material_weights = HashMap::from([
        ("M001".to_string(), 50.0),
        ("M002".to_string(), 50.0),
    ]);

    let impact = engine.generate_impact(
        &before_items,
        &after_items,
        &before_pools,
        &after_pools,
        &before_risks,
        &after_risks,
        &all_materials,
        &material_weights,
    );

    // 验证锁定冲突
    assert_eq!(impact.locked_conflicts.len(), 1, "应该检测到1个锁定冲突");
    assert_eq!(impact.locked_conflicts[0], "M001");
}

// ==========================================
// 测试10: 冻结冲突检测
// ==========================================
#[test]
fn test_impact_summary_frozen_conflicts() {
    let engine = ImpactSummaryEngine::new();

    let date1 = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2026, 1, 16).unwrap();

    // M001处于Locked状态,但被移动了(冲突)
    let before_items = vec![
        create_test_plan_item("M001", date1, "H032", 1, 50.0),
        create_test_plan_item("M002", date1, "H032", 2, 50.0),
    ];

    let after_items = vec![
        create_test_plan_item("M001", date2, "H032", 1, 50.0), // 移动(冲突)
        create_test_plan_item("M002", date1, "H032", 2, 50.0), // 不变
    ];

    let before_pools = vec![];
    let after_pools = vec![];
    let before_risks = vec![];
    let after_risks = vec![];
    let all_materials = vec![
        create_test_material_state("M001", UrgentLevel::L2, false, SchedState::Locked), // frozen
        create_test_material_state("M002", UrgentLevel::L1, false, SchedState::Ready),
    ];
    let material_weights = HashMap::from([
        ("M001".to_string(), 50.0),
        ("M002".to_string(), 50.0),
    ]);

    let impact = engine.generate_impact(
        &before_items,
        &after_items,
        &before_pools,
        &after_pools,
        &before_risks,
        &after_risks,
        &all_materials,
        &material_weights,
    );

    // 验证冻结冲突
    assert_eq!(impact.frozen_conflicts.len(), 1, "应该检测到1个冻结冲突");
    assert_eq!(impact.frozen_conflicts[0], "M001");
}

// ==========================================
// 测试11: 综合场景测试
// ==========================================
#[test]
fn test_impact_summary_comprehensive() {
    let engine = ImpactSummaryEngine::new();

    let date1 = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2026, 1, 21).unwrap();

    // 综合场景:
    // - M001移动(L3)
    // - M002挤出(L2)
    // - M003新增
    // - 产能从80增加到100
    // - 超限从0增加到5
    // - 风险从GREEN升级到YELLOW
    let before_items = vec![
        create_test_plan_item("M001", date1, "H032", 1, 50.0),
        create_test_plan_item("M002", date1, "H032", 2, 30.0),
    ];

    let after_items = vec![
        create_test_plan_item("M001", date2, "H032", 1, 50.0), // 移动到21日
        create_test_plan_item("M003", date1, "H032", 2, 50.0), // 新增
        // M002被挤出
    ];

    let before_pools = vec![create_test_capacity_pool("H032", date1, 100.0, 100.0, 80.0)];

    let after_pools = vec![create_test_capacity_pool("H032", date1, 100.0, 100.0, 105.0)];

    let before_risks = vec![create_test_risk_snapshot("H032", date1, RiskLevel::Green)];

    let after_risks = vec![create_test_risk_snapshot("H032", date1, RiskLevel::Yellow)];

    let all_materials = vec![
        create_test_material_state("M001", UrgentLevel::L3, false, SchedState::Ready),
        create_test_material_state("M002", UrgentLevel::L2, false, SchedState::Ready),
        create_test_material_state("M003", UrgentLevel::L1, false, SchedState::Ready),
    ];

    let material_weights = HashMap::from([
        ("M001".to_string(), 50.0),
        ("M002".to_string(), 30.0),
        ("M003".to_string(), 50.0),
    ]);

    let impact = engine.generate_impact(
        &before_items,
        &after_items,
        &before_pools,
        &after_pools,
        &before_risks,
        &after_risks,
        &all_materials,
        &material_weights,
    );

    // 验证材料影响
    assert_eq!(impact.moved_count, 1, "应该有1个材料被移动");
    assert_eq!(impact.squeezed_out_count, 1, "应该有1个材料被挤出");
    assert_eq!(impact.added_count, 1, "应该有1个材料被新增");
    assert_eq!(impact.material_changes.len(), 3, "应该有3个材料变更记录");

    // 验证产能影响
    assert!(
        (impact.capacity_delta_t - 25.0).abs() < 0.01,
        "产能变化应该是25.0吨(105-80)"
    );
    assert!(
        (impact.overflow_delta_t - 5.0).abs() < 0.01,
        "超限变化应该是5.0吨(5-0)"
    );

    // 验证风险影响
    assert_eq!(impact.risk_level_before, "GREEN");
    assert_eq!(impact.risk_level_after, "YELLOW");
    assert_eq!(impact.risk_changes.len(), 1);

    // 验证紧急单影响
    assert_eq!(
        impact.urgent_material_affected, 2,
        "应该有2个紧急材料受影响(L3移动+L2挤出)"
    );
    assert_eq!(impact.l3_critical_count, 1, "应该有1个L3红线材料");

    // 验证换辊影响(吨位从80增加到100,变化20吨 > 1吨阈值)
    assert!(impact.roll_campaign_affected, "换辊窗口应该受到影响");
    assert!(impact.roll_tonnage_delta_t.is_some());

    // 验证字段完整性
    assert!(!impact.material_changes.is_empty());
    assert!(!impact.capacity_changes.is_empty());
    assert!(!impact.risk_changes.is_empty());
}

// ==========================================
// 测试12: 空场景测试
// ==========================================
#[test]
fn test_impact_summary_no_changes() {
    let engine = ImpactSummaryEngine::new();

    let date1 = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    // before和after完全相同,没有任何变化
    let before_items = vec![
        create_test_plan_item("M001", date1, "H032", 1, 50.0),
        create_test_plan_item("M002", date1, "H032", 2, 50.0),
    ];

    let after_items = vec![
        create_test_plan_item("M001", date1, "H032", 1, 50.0),
        create_test_plan_item("M002", date1, "H032", 2, 50.0),
    ];

    let before_pools = vec![create_test_capacity_pool("H032", date1, 100.0, 120.0, 100.0)];
    let after_pools = vec![create_test_capacity_pool("H032", date1, 100.0, 120.0, 100.0)];

    let before_risks = vec![create_test_risk_snapshot("H032", date1, RiskLevel::Green)];
    let after_risks = vec![create_test_risk_snapshot("H032", date1, RiskLevel::Green)];

    let all_materials = vec![
        create_test_material_state("M001", UrgentLevel::L1, false, SchedState::Ready),
        create_test_material_state("M002", UrgentLevel::L1, false, SchedState::Ready),
    ];
    let material_weights = HashMap::from([
        ("M001".to_string(), 50.0),
        ("M002".to_string(), 50.0),
    ]);

    let impact = engine.generate_impact(
        &before_items,
        &after_items,
        &before_pools,
        &after_pools,
        &before_risks,
        &after_risks,
        &all_materials,
        &material_weights,
    );

    // 验证没有任何变化
    assert_eq!(impact.moved_count, 0);
    assert_eq!(impact.squeezed_out_count, 0);
    assert_eq!(impact.added_count, 0);
    assert_eq!(impact.material_changes.len(), 0);
    assert!(impact.capacity_delta_t.abs() < 0.01);
    assert!(impact.overflow_delta_t.abs() < 0.01);
    assert_eq!(impact.capacity_changes.len(), 0);
    assert_eq!(impact.risk_level_before, "GREEN");
    assert_eq!(impact.risk_level_after, "GREEN");
    assert_eq!(impact.risk_changes.len(), 0);
    assert_eq!(impact.urgent_material_affected, 0);
    assert_eq!(impact.l3_critical_count, 0);
    assert!(impact.locked_conflicts.is_empty());
    assert!(impact.frozen_conflicts.is_empty());
}

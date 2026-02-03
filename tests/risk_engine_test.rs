// ==========================================
// RiskEngine 引擎集成测试
// ==========================================
// 测试目标: 验证风险快照生成和风险等级评估
// 覆盖范围: GREEN/YELLOW/ORANGE/RED四级风险评估
// ==========================================

use chrono::{NaiveDate, Utc};
use hot_rolling_aps::domain::capacity::CapacityPool;
use hot_rolling_aps::domain::material::MaterialState;
use hot_rolling_aps::domain::plan::PlanItem;
use hot_rolling_aps::domain::types::{RiskLevel, RushLevel, SchedState, UrgentLevel};
use hot_rolling_aps::engine::RiskEngine;
use std::collections::HashMap;

// ==========================================
// 测试辅助函数
// ==========================================

/// 创建测试用的产能池
fn create_test_capacity_pool(
    machine_code: &str,
    target: f64,
    limit: f64,
) -> CapacityPool {
    CapacityPool {
        version_id: "V001".to_string(),
        machine_code: machine_code.to_string(),
        plan_date: NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        target_capacity_t: target,
        limit_capacity_t: limit,
        used_capacity_t: 0.0,
        overflow_t: 0.0,
        frozen_capacity_t: 0.0,
        accumulated_tonnage_t: 0.0,
        roll_campaign_id: None,
    }
}

/// 创建测试用的PlanItem
fn create_test_plan_item(
    material_id: &str,
    weight: f64,
) -> PlanItem {
    PlanItem {
        version_id: "V001".to_string(),
        material_id: material_id.to_string(),
        machine_code: "H032".to_string(),
        plan_date: NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        seq_no: 1,
        weight_t: weight,
        source_type: "CALC".to_string(),
        locked_in_plan: false,
        force_release_in_plan: false,
        violation_flags: None,
        urgent_level: None,
        sched_state: None,
        assign_reason: None,
        steel_grade: None,
    }
}

/// 创建测试用的MaterialState
fn create_test_material_state(
    material_id: &str,
    urgent_level: UrgentLevel,
    sched_state: SchedState,
    machine_code: &str,
) -> MaterialState {
    MaterialState {
        material_id: material_id.to_string(),
        sched_state,
        lock_flag: false,
        force_release_flag: false,
        urgent_level,
        urgent_reason: None,
        rush_level: RushLevel::L0,
        rolling_output_age_days: 7,
        ready_in_days: 0,
        earliest_sched_date: Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
        stock_age_days: 5,
        scheduled_date: Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
        scheduled_machine_code: Some(machine_code.to_string()),
        seq_no: None,
        manual_urgent_flag: false,
        user_confirmed: false,
        user_confirmed_at: None,
        user_confirmed_by: None,
        user_confirmed_reason: None,
        in_frozen_zone: false,
        last_calc_version_id: None,
        updated_at: Utc::now(),
        updated_by: Some("TEST".to_string()),
    }
}

// ==========================================
// 测试用例 1: GREEN - 正常状态
// ==========================================

#[test]
fn test_risk_engine_green_level() {
    println!("\n=== 测试：GREEN - 正常状态 ===");

    let engine = RiskEngine::new();
    let pool = create_test_capacity_pool("H032", 100.0, 120.0);

    // 创建已排产材料（50吨，利用率50%）
    let scheduled_items = vec![
        create_test_plan_item("MAT001", 30.0),
        create_test_plan_item("MAT002", 20.0),
    ];

    // 创建材料状态（无紧急材料）
    let all_materials = vec![
        create_test_material_state("MAT001", UrgentLevel::L0, SchedState::Scheduled, "H032"),
        create_test_material_state("MAT002", UrgentLevel::L0, SchedState::Scheduled, "H032"),
    ];

    // 材料重量映射
    let mut material_weights = HashMap::new();
    material_weights.insert("MAT001".to_string(), 30.0);
    material_weights.insert("MAT002".to_string(), 20.0);

    // 生成快照
    let snapshot = engine.generate_snapshot(
        "V001",
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        &pool,
        &scheduled_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    println!("✓ 快照生成完成");
    println!("  - 风险等级: {:?}", snapshot.risk_level);
    println!("  - 已用产能: {} 吨", snapshot.used_capacity_t);
    println!("  - 利用率: {:.1}%", snapshot.used_capacity_t / pool.target_capacity_t * 100.0);

    // 验证
    assert_eq!(snapshot.risk_level, RiskLevel::Green, "风险等级应该是GREEN");
    assert_eq!(snapshot.used_capacity_t, 50.0, "已用产能应该是50吨");
    assert_eq!(snapshot.overflow_t, 0.0, "不应该有超限吨位");
    assert_eq!(snapshot.l3_count, 0, "不应该有L3材料");
    assert_eq!(snapshot.l2_count, 0, "不应该有L2材料");

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 2: YELLOW - 接近目标产能
// ==========================================

#[test]
fn test_risk_engine_yellow_level_high_utilization() {
    println!("\n=== 测试：YELLOW - 接近目标产能 ===");

    let engine = RiskEngine::new();
    let pool = create_test_capacity_pool("H032", 100.0, 120.0);

    // 创建已排产材料（95吨，利用率95%）
    let scheduled_items = vec![
        create_test_plan_item("MAT001", 50.0),
        create_test_plan_item("MAT002", 45.0),
    ];

    let all_materials = vec![];
    let mut material_weights = HashMap::new();
    material_weights.insert("MAT001".to_string(), 50.0);
    material_weights.insert("MAT002".to_string(), 45.0);

    // 生成快照
    let snapshot = engine.generate_snapshot(
        "V001",
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        &pool,
        &scheduled_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    println!("✓ 快照生成完成");
    println!("  - 风险等级: {:?}", snapshot.risk_level);
    println!("  - 利用率: {:.1}%", snapshot.used_capacity_t / pool.target_capacity_t * 100.0);
    println!("  - 风险原因: {}", snapshot.risk_reason);

    // 验证
    assert_eq!(snapshot.risk_level, RiskLevel::Yellow, "风险等级应该是YELLOW");
    assert_eq!(snapshot.used_capacity_t, 95.0, "已用产能应该是95吨");
    assert!(
        snapshot.risk_reason.contains("接近目标产能"),
        "风险原因应该包含'接近目标产能'"
    );

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 3: YELLOW - L2紧急材料较多
// ==========================================

#[test]
fn test_risk_engine_yellow_level_l2_materials() {
    println!("\n=== 测试：YELLOW - L2紧急材料较多 ===");

    let engine = RiskEngine::new();
    let pool = create_test_capacity_pool("H032", 100.0, 120.0);

    let scheduled_items = vec![create_test_plan_item("MAT001", 50.0)];

    // 创建6个L2紧急材料（>=5触发YELLOW）
    let all_materials = vec![
        create_test_material_state("MAT001", UrgentLevel::L2, SchedState::Scheduled, "H032"),
        create_test_material_state("MAT002", UrgentLevel::L2, SchedState::Ready, "H032"),
        create_test_material_state("MAT003", UrgentLevel::L2, SchedState::Ready, "H032"),
        create_test_material_state("MAT004", UrgentLevel::L2, SchedState::Ready, "H032"),
        create_test_material_state("MAT005", UrgentLevel::L2, SchedState::Ready, "H032"),
        create_test_material_state("MAT006", UrgentLevel::L2, SchedState::Ready, "H032"),
    ];

    let mut material_weights = HashMap::new();
    material_weights.insert("MAT001".to_string(), 10.0);

    // 生成快照
    let snapshot = engine.generate_snapshot(
        "V001",
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        &pool,
        &scheduled_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    println!("✓ 快照生成完成");
    println!("  - 风险等级: {:?}", snapshot.risk_level);
    println!("  - L2材料数量: {}", snapshot.l2_count);
    println!("  - 风险原因: {}", snapshot.risk_reason);

    // 验证
    assert_eq!(snapshot.risk_level, RiskLevel::Yellow, "风险等级应该是YELLOW");
    assert_eq!(snapshot.l2_count, 6, "L2材料数量应该是6");
    assert!(
        snapshot.risk_reason.contains("L2紧急材料较多"),
        "风险原因应该包含'L2紧急材料较多'"
    );

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 4: ORANGE - 超限轻微
// ==========================================

#[test]
fn test_risk_engine_orange_level_overflow() {
    println!("\n=== 测试：ORANGE - 超限轻微 ===");

    let engine = RiskEngine::new();
    let pool = create_test_capacity_pool("H032", 100.0, 120.0);

    // 创建已排产材料（125吨，超限5吨）
    let scheduled_items = vec![
        create_test_plan_item("MAT001", 60.0),
        create_test_plan_item("MAT002", 65.0),
    ];

    let all_materials = vec![];
    let mut material_weights = HashMap::new();

    // 生成快照
    let snapshot = engine.generate_snapshot(
        "V001",
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        &pool,
        &scheduled_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    println!("✓ 快照生成完成");
    println!("  - 风险等级: {:?}", snapshot.risk_level);
    println!("  - 超限吨位: {} 吨", snapshot.overflow_t);
    println!("  - 风险原因: {}", snapshot.risk_reason);

    // 验证
    assert_eq!(snapshot.risk_level, RiskLevel::Orange, "风险等级应该是ORANGE");
    assert_eq!(snapshot.used_capacity_t, 125.0, "已用产能应该是125吨");
    assert_eq!(snapshot.overflow_t, 5.0, "超限吨位应该是5吨");
    assert!(
        snapshot.risk_reason.contains("超限轻微"),
        "风险原因应该包含'超限轻微'"
    );

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 5: ORANGE - L2紧急材料过多
// ==========================================

#[test]
fn test_risk_engine_orange_level_many_l2_materials() {
    println!("\n=== 测试：ORANGE - L2紧急材料过多 ===");

    let engine = RiskEngine::new();
    let pool = create_test_capacity_pool("H032", 100.0, 120.0);

    let scheduled_items = vec![create_test_plan_item("MAT001", 50.0)];

    // 创建12个L2紧急材料（>=10触发ORANGE）
    let mut all_materials = vec![];
    for i in 1..=12 {
        all_materials.push(create_test_material_state(
            &format!("MAT{:03}", i),
            UrgentLevel::L2,
            SchedState::Ready,
            "H032",
        ));
    }

    let mut material_weights = HashMap::new();

    // 生成快照
    let snapshot = engine.generate_snapshot(
        "V001",
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        &pool,
        &scheduled_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    println!("✓ 快照生成完成");
    println!("  - 风险等级: {:?}", snapshot.risk_level);
    println!("  - L2材料数量: {}", snapshot.l2_count);
    println!("  - 风险原因: {}", snapshot.risk_reason);

    // 验证
    assert_eq!(snapshot.risk_level, RiskLevel::Orange, "风险等级应该是ORANGE");
    assert_eq!(snapshot.l2_count, 12, "L2材料数量应该是12");
    assert!(
        snapshot.risk_reason.contains("L2紧急材料过多"),
        "风险原因应该包含'L2紧急材料过多'"
    );

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 6: RED - 超限严重
// ==========================================

#[test]
fn test_risk_engine_red_level_severe_overflow() {
    println!("\n=== 测试：RED - 超限严重 ===");

    let engine = RiskEngine::new();
    let pool = create_test_capacity_pool("H032", 100.0, 120.0);

    // 创建已排产材料（140吨，超限20吨，超限率16.7%>10%）
    let scheduled_items = vec![
        create_test_plan_item("MAT001", 70.0),
        create_test_plan_item("MAT002", 70.0),
    ];

    let all_materials = vec![];
    let mut material_weights = HashMap::new();

    // 生成快照
    let snapshot = engine.generate_snapshot(
        "V001",
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        &pool,
        &scheduled_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    println!("✓ 快照生成完成");
    println!("  - 风险等级: {:?}", snapshot.risk_level);
    println!("  - 超限吨位: {} 吨", snapshot.overflow_t);
    println!("  - 超限比例: {:.1}%", snapshot.overflow_t / pool.limit_capacity_t * 100.0);
    println!("  - 风险原因: {}", snapshot.risk_reason);

    // 验证
    assert_eq!(snapshot.risk_level, RiskLevel::Red, "风险等级应该是RED");
    assert_eq!(snapshot.used_capacity_t, 140.0, "已用产能应该是140吨");
    assert_eq!(snapshot.overflow_t, 20.0, "超限吨位应该是20吨");
    assert!(
        snapshot.risk_reason.contains("超限严重"),
        "风险原因应该包含'超限严重'"
    );

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 7: RED - L3红线材料过多
// ==========================================

#[test]
fn test_risk_engine_red_level_many_l3_materials() {
    println!("\n=== 测试：RED - L3红线材料过多 ===");

    let engine = RiskEngine::new();
    let pool = create_test_capacity_pool("H032", 100.0, 120.0);

    let scheduled_items = vec![create_test_plan_item("MAT001", 50.0)];

    // 创建6个L3红线材料（>=5触发RED）
    let mut all_materials = vec![];
    for i in 1..=6 {
        all_materials.push(create_test_material_state(
            &format!("MAT{:03}", i),
            UrgentLevel::L3,
            SchedState::Ready,
            "H032",
        ));
    }

    let mut material_weights = HashMap::new();

    // 生成快照
    let snapshot = engine.generate_snapshot(
        "V001",
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        &pool,
        &scheduled_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    println!("✓ 快照生成完成");
    println!("  - 风险等级: {:?}", snapshot.risk_level);
    println!("  - L3材料数量: {}", snapshot.l3_count);
    println!("  - 风险原因: {}", snapshot.risk_reason);

    // 验证
    assert_eq!(snapshot.risk_level, RiskLevel::Red, "风险等级应该是RED");
    assert_eq!(snapshot.l3_count, 6, "L3材料数量应该是6");
    assert!(
        snapshot.risk_reason.contains("L3红线材料过多"),
        "风险原因应该包含'L3红线材料过多'"
    );

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 8: RED - 换辊硬停止
// ==========================================

#[test]
fn test_risk_engine_red_level_roll_hard_stop() {
    println!("\n=== 测试：RED - 换辊硬停止 ===");

    let engine = RiskEngine::new();
    let pool = create_test_capacity_pool("H032", 100.0, 120.0);

    let scheduled_items = vec![create_test_plan_item("MAT001", 50.0)];
    let all_materials = vec![];
    let mut material_weights = HashMap::new();

    // 生成快照（带换辊硬停止状态）
    let snapshot = engine.generate_snapshot(
        "V001",
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        &pool,
        &scheduled_items,
        &all_materials,
        &material_weights,
        Some("HARD_STOP"),
        0.1,
    );

    println!("✓ 快照生成完成");
    println!("  - 风险等级: {:?}", snapshot.risk_level);
    println!("  - 换辊状态: {:?}", snapshot.roll_status);
    println!("  - 风险原因: {}", snapshot.risk_reason);

    // 验证
    assert_eq!(snapshot.risk_level, RiskLevel::Red, "风险等级应该是RED");
    assert_eq!(
        snapshot.roll_status.as_deref(),
        Some("HARD_STOP"),
        "换辊状态应该是HARD_STOP"
    );
    assert!(
        snapshot.risk_reason.contains("换辊硬停止"),
        "风险原因应该包含'换辊硬停止'"
    );

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 9: 快照字段完整性验证
// ==========================================

#[test]
fn test_risk_engine_snapshot_fields_completeness() {
    println!("\n=== 测试：快照字段完整性验证 ===");

    let engine = RiskEngine::new();
    let pool = create_test_capacity_pool("H032", 100.0, 120.0);

    let scheduled_items = vec![create_test_plan_item("MAT001", 50.0)];
    let all_materials = vec![
        create_test_material_state("MAT001", UrgentLevel::L2, SchedState::Scheduled, "H032"),
    ];

    let mut material_weights = HashMap::new();
    material_weights.insert("MAT001".to_string(), 50.0);

    // 生成快照
    let snapshot = engine.generate_snapshot(
        "V001",
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        &pool,
        &scheduled_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    println!("✓ 快照生成完成");

    // 验证所有关键字段
    assert!(!snapshot.snapshot_id.is_empty(), "snapshot_id不应该为空");
    assert_eq!(snapshot.version_id, "V001");
    assert_eq!(snapshot.machine_code, "H032");
    assert_eq!(snapshot.snapshot_date, NaiveDate::from_ymd_opt(2026, 1, 20).unwrap());
    assert_eq!(snapshot.target_capacity_t, 100.0);
    assert_eq!(snapshot.limit_capacity_t, 120.0);
    assert!(snapshot.used_capacity_t >= 0.0);
    assert!(snapshot.overflow_t >= 0.0);
    assert!(snapshot.l3_count >= 0);
    assert!(snapshot.l2_count >= 0);
    assert!(!snapshot.risk_reason.is_empty(), "risk_reason不应该为空");

    println!("  - 所有关键字段验证通过");
    println!("=== 测试通过 ===\n");
}

// ==========================================
// CapacityFiller 引擎集成测试
// ==========================================
// 测试目标: 验证产能池填充逻辑
// 覆盖范围: 产能约束、冻结区保护、锁定材料处理
// ==========================================

use chrono::{NaiveDate, Utc};
use hot_rolling_aps::domain::capacity::CapacityPool;
use hot_rolling_aps::domain::material::{MaterialMaster, MaterialState};
use hot_rolling_aps::domain::plan::PlanItem;
use hot_rolling_aps::domain::types::{RushLevel, SchedState, UrgentLevel};
use hot_rolling_aps::engine::CapacityFiller;

// ==========================================
// 测试辅助函数
// ==========================================

/// 创建测试用的产能池
fn create_test_capacity_pool(
    target_capacity: f64,
    limit_capacity: f64,
) -> CapacityPool {
    CapacityPool {
        version_id: "V001".to_string(),
        machine_code: "H032".to_string(),
        plan_date: NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        target_capacity_t: target_capacity,
        limit_capacity_t: limit_capacity,
        used_capacity_t: 0.0,
        overflow_t: 0.0,
        frozen_capacity_t: 0.0,
        accumulated_tonnage_t: 0.0,
        roll_campaign_id: None,
    }
}

/// 创建测试用的材料
fn create_test_material(
    material_id: &str,
    weight_t: f64,
    sched_state: SchedState,
) -> (MaterialMaster, MaterialState) {
    let master = MaterialMaster {
        material_id: material_id.to_string(),
        manufacturing_order_id: Some("MO001".to_string()),
        contract_no: Some("CT001".to_string()),
        due_date: Some(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap()),
        next_machine_code: Some("H032".to_string()),
        rework_machine_code: None,
        current_machine_code: Some("H032".to_string()),
        width_mm: Some(1200.0),
        thickness_mm: Some(2.0),
        length_m: Some(10.0),
        weight_t: Some(weight_t),
        available_width_mm: Some(1180.0),
        steel_mark: Some("Q235B".to_string()),
        slab_id: Some("SLAB001".to_string()),
        material_status_code_src: Some("READY".to_string()),
        status_updated_at: None,
        output_age_days_raw: Some(3),
        stock_age_days: Some(5),
        contract_nature: Some("NORMAL".to_string()),
        weekly_delivery_flag: Some("N".to_string()),
        export_flag: Some("0".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let state = MaterialState {
        material_id: material_id.to_string(),
        sched_state,
        lock_flag: sched_state == SchedState::Locked,
        force_release_flag: false,
        urgent_level: UrgentLevel::L0,
        urgent_reason: None,
        rush_level: RushLevel::L0,
        rolling_output_age_days: 7,
        ready_in_days: 0,
        earliest_sched_date: Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
        stock_age_days: 5,
        scheduled_date: Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
        scheduled_machine_code: Some("H032".to_string()),
        seq_no: None,
        manual_urgent_flag: false,
        in_frozen_zone: false,
        last_calc_version_id: None,
        updated_at: Utc::now(),
        updated_by: Some("TEST".to_string()),
    };

    (master, state)
}

// ==========================================
// 测试用例 1: 基本填充功能
// ==========================================

#[test]
fn test_capacity_filler_basic_fill() {
    println!("\n=== 测试：基本填充功能 ===");

    let filler = CapacityFiller::new();
    let mut capacity_pool = create_test_capacity_pool(100.0, 120.0);

    // 创建候选材料（总共80吨，不超过target）
    let candidates = vec![
        create_test_material("MAT001", 20.0, SchedState::Ready),
        create_test_material("MAT002", 30.0, SchedState::Ready),
        create_test_material("MAT003", 30.0, SchedState::Ready),
    ];

    let frozen_items = vec![];
    let version_id = "V001";

    // 执行填充
    let (plan_items, skipped_materials) = filler.fill_single_day(
        &mut capacity_pool,
        candidates,
        frozen_items,
        version_id,
    );

    println!("✓ 填充完成");
    println!("  - 已排材料数: {}", plan_items.len());
    println!("  - 跳过材料数: {}", skipped_materials.len());
    println!("  - 已用产能: {} 吨", capacity_pool.used_capacity_t);
    println!("  - 超限吨位: {} 吨", capacity_pool.overflow_t);

    // 验证
    assert_eq!(plan_items.len(), 3, "应该排入3个材料");
    assert_eq!(skipped_materials.len(), 0, "不应该有跳过的材料");
    assert_eq!(capacity_pool.used_capacity_t, 80.0, "已用产能应该是80吨");
    assert_eq!(capacity_pool.overflow_t, 0.0, "不应该有超限吨位");

    // 验证序号
    assert_eq!(plan_items[0].sequence_no(), 1);
    assert_eq!(plan_items[1].sequence_no(), 2);
    assert_eq!(plan_items[2].sequence_no(), 3);

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 2: 填充至limit
// ==========================================

#[test]
fn test_capacity_filler_fill_to_limit() {
    println!("\n=== 测试：填充至limit ===");

    let filler = CapacityFiller::new();
    let mut capacity_pool = create_test_capacity_pool(100.0, 120.0);

    // 创建候选材料（总共130吨，超过limit但允许填充到limit）
    let candidates = vec![
        create_test_material("MAT001", 40.0, SchedState::Ready),
        create_test_material("MAT002", 40.0, SchedState::Ready),
        create_test_material("MAT003", 30.0, SchedState::Ready),
        create_test_material("MAT004", 20.0, SchedState::Ready), // 这个会被跳过
    ];

    let frozen_items = vec![];
    let version_id = "V001";

    // 执行填充
    let (plan_items, skipped_materials) = filler.fill_single_day(
        &mut capacity_pool,
        candidates,
        frozen_items,
        version_id,
    );

    println!("✓ 填充完成");
    println!("  - 已排材料数: {}", plan_items.len());
    println!("  - 跳过材料数: {}", skipped_materials.len());
    println!("  - 已用产能: {} 吨", capacity_pool.used_capacity_t);
    println!("  - 目标产能: {} 吨", capacity_pool.target_capacity_t);
    println!("  - 上限产能: {} 吨", capacity_pool.limit_capacity_t);

    // 验证
    assert_eq!(plan_items.len(), 3, "应该排入3个材料（到达limit）");
    assert_eq!(skipped_materials.len(), 1, "应该跳过1个材料");
    assert_eq!(capacity_pool.used_capacity_t, 110.0, "已用产能应该是110吨");
    assert_eq!(capacity_pool.overflow_t, 0.0, "未超过limit不应该有overflow");

    // 验证跳过原因
    let (_, _, skip_reason) = &skipped_materials[0];
    assert!(
        skip_reason.contains("CAPACITY_LIMIT_EXCEEDED"),
        "跳过原因应该是产能限制"
    );

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 3: 冻结区材料优先
// ==========================================

#[test]
fn test_capacity_filler_frozen_zone_priority() {
    println!("\n=== 测试：冻结区材料优先 ===");

    let filler = CapacityFiller::new();
    let mut capacity_pool = create_test_capacity_pool(100.0, 120.0);

    // 创建冻结区材料
    let frozen_items = vec![
        PlanItem {
            version_id: "V001".to_string(),
            material_id: "FROZEN001".to_string(),
            machine_code: "H032".to_string(),
            plan_date: NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            seq_no: 1,
            weight_t: 20.0,
            source_type: "FROZEN".to_string(),
            locked_in_plan: true,
            force_release_in_plan: false,
            violation_flags: None,
            urgent_level: None,
            sched_state: None,
            assign_reason: None,
            steel_grade: None,
        },
        PlanItem {
            version_id: "V001".to_string(),
            material_id: "FROZEN002".to_string(),
            machine_code: "H032".to_string(),
            plan_date: NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            seq_no: 2,
            weight_t: 30.0,
            source_type: "FROZEN".to_string(),
            locked_in_plan: true,
            force_release_in_plan: false,
            violation_flags: None,
            urgent_level: None,
            sched_state: None,
            assign_reason: None,
            steel_grade: None,
        },
    ];

    // 创建计算区候选材料
    let candidates = vec![
        create_test_material("MAT001", 30.0, SchedState::Ready),
        create_test_material("MAT002", 40.0, SchedState::Ready),
    ];

    let version_id = "V001";

    // 执行填充
    let (plan_items, skipped_materials) = filler.fill_single_day(
        &mut capacity_pool,
        candidates,
        frozen_items,
        version_id,
    );

    println!("✓ 填充完成");
    println!("  - 已排材料数: {}", plan_items.len());
    println!("  - 冻结区材料: 2");
    println!("  - 计算区材料: {}", plan_items.len() - 2);

    // 验证
    assert_eq!(plan_items.len(), 4, "应该排入4个材料（2冻结+2计算）");
    assert_eq!(skipped_materials.len(), 0, "不应该有跳过的材料");

    // 验证冻结区材料在前
    assert_eq!(plan_items[0].material_id, "FROZEN001");
    assert_eq!(plan_items[1].material_id, "FROZEN002");
    assert_eq!(plan_items[2].material_id, "MAT001");
    assert_eq!(plan_items[3].material_id, "MAT002");

    // 验证序号连续
    assert_eq!(plan_items[0].sequence_no(), 1);
    assert_eq!(plan_items[1].sequence_no(), 2);
    assert_eq!(plan_items[2].sequence_no(), 3);
    assert_eq!(plan_items[3].sequence_no(), 4);

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 4: 锁定材料强制添加
// ==========================================

#[test]
fn test_capacity_filler_locked_material_forced() {
    println!("\n=== 测试：锁定材料强制添加 ===");

    let filler = CapacityFiller::new();
    let mut capacity_pool = create_test_capacity_pool(100.0, 120.0);

    // 创建候选材料（包含锁定材料，即使超过limit也要添加）
    let candidates = vec![
        create_test_material("MAT001", 60.0, SchedState::Ready),
        create_test_material("MAT002", 50.0, SchedState::Ready),
        create_test_material("MAT003_LOCKED", 30.0, SchedState::Locked), // 锁定材料
    ];

    let frozen_items = vec![];
    let version_id = "V001";

    // 执行填充
    let (plan_items, skipped_materials) = filler.fill_single_day(
        &mut capacity_pool,
        candidates,
        frozen_items,
        version_id,
    );

    println!("✓ 填充完成");
    println!("  - 已排材料数: {}", plan_items.len());
    println!("  - 已用产能: {} 吨", capacity_pool.used_capacity_t);
    println!("  - 上限产能: {} 吨", capacity_pool.limit_capacity_t);
    println!("  - 超限吨位: {} 吨", capacity_pool.overflow_t);

    // 验证：锁定材料应该被添加，即使超过limit
    assert_eq!(plan_items.len(), 3, "应该排入3个材料（包括锁定材料）");
    assert_eq!(
        capacity_pool.used_capacity_t, 140.0,
        "已用产能应该是140吨（超过limit）"
    );
    assert_eq!(
        capacity_pool.overflow_t, 20.0,
        "超限吨位应该是20吨"
    );

    // 验证锁定材料的assign_reason
    let locked_item = plan_items.iter().find(|item| item.material_id == "MAT003_LOCKED");
    assert!(locked_item.is_some(), "应该找到锁定材料");
    assert_eq!(
        locked_item.unwrap().assign_reason.as_deref(),
        Some("LOCKED_MATERIAL")
    );

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 5: 超限材料被跳过
// ==========================================

#[test]
fn test_capacity_filler_skip_over_limit() {
    println!("\n=== 测试：超限材料被跳过 ===");

    let filler = CapacityFiller::new();
    let mut capacity_pool = create_test_capacity_pool(100.0, 120.0);

    // 创建候选材料（MAT003会导致超限被跳过，但MAT004可以添加）
    let candidates = vec![
        create_test_material("MAT001", 50.0, SchedState::Ready),
        create_test_material("MAT002", 50.0, SchedState::Ready),
        create_test_material("MAT003", 30.0, SchedState::Ready), // 100+30=130>120，会超限，应该被跳过
        create_test_material("MAT004", 10.0, SchedState::Ready), // 100+10=110<=120，可以添加
    ];

    let frozen_items = vec![];
    let version_id = "V001";

    // 执行填充
    let (plan_items, skipped_materials) = filler.fill_single_day(
        &mut capacity_pool,
        candidates,
        frozen_items,
        version_id,
    );

    println!("✓ 填充完成");
    println!("  - 已排材料数: {}", plan_items.len());
    println!("  - 跳过材料数: {}", skipped_materials.len());
    println!("  - 已用产能: {} 吨", capacity_pool.used_capacity_t);

    // 验证：MAT001, MAT002, MAT004被排入，MAT003被跳过
    assert_eq!(plan_items.len(), 3, "应该排入3个材料（MAT001, MAT002, MAT004）");
    assert_eq!(skipped_materials.len(), 1, "应该跳过1个材料（MAT003）");
    assert_eq!(capacity_pool.used_capacity_t, 110.0, "已用产能应该是110吨");

    // 验证跳过的材料是MAT003
    assert_eq!(skipped_materials[0].0.material_id, "MAT003");

    // 验证排入的材料是MAT001, MAT002, MAT004
    assert_eq!(plan_items[0].material_id, "MAT001");
    assert_eq!(plan_items[1].material_id, "MAT002");
    assert_eq!(plan_items[2].material_id, "MAT004");

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 6: overflow_t计算
// ==========================================

#[test]
fn test_capacity_filler_overflow_calculation() {
    println!("\n=== 测试：overflow_t计算 ===");

    let filler = CapacityFiller::new();
    let mut capacity_pool = create_test_capacity_pool(100.0, 120.0);

    // 创建候选材料（包含锁定材料导致超限）
    let candidates = vec![
        create_test_material("MAT001", 60.0, SchedState::Ready),
        create_test_material("MAT002", 60.0, SchedState::Ready),
        create_test_material("MAT003_LOCKED", 20.0, SchedState::Locked), // 锁定材料导致超限
    ];

    let frozen_items = vec![];
    let version_id = "V001";

    // 执行填充
    let (plan_items, skipped_materials) = filler.fill_single_day(
        &mut capacity_pool,
        candidates,
        frozen_items,
        version_id,
    );

    println!("✓ 填充完成");
    println!("  - 已用产能: {} 吨", capacity_pool.used_capacity_t);
    println!("  - 上限产能: {} 吨", capacity_pool.limit_capacity_t);
    println!("  - 超限吨位: {} 吨", capacity_pool.overflow_t);

    // 验证 overflow_t 计算
    assert_eq!(
        capacity_pool.used_capacity_t, 140.0,
        "已用产能应该是140吨"
    );
    assert_eq!(
        capacity_pool.overflow_t, 20.0,
        "超限吨位应该是 140 - 120 = 20吨"
    );

    println!("=== 测试通过 ===\n");
}

// ==========================================
// 测试用例 7: 空候选材料列表
// ==========================================

#[test]
fn test_capacity_filler_empty_candidates() {
    println!("\n=== 测试：空候选材料列表 ===");

    let filler = CapacityFiller::new();
    let mut capacity_pool = create_test_capacity_pool(100.0, 120.0);

    let candidates = vec![];
    let frozen_items = vec![];
    let version_id = "V001";

    // 执行填充
    let (_plan_items, _skipped_materials) = filler.fill_single_day(
        &mut capacity_pool,
        candidates,
        frozen_items,
        version_id,
    );

    println!("✓ 填充完成");
    println!("  - 已排材料数: {}", _plan_items.len());
    println!("  - 已用产能: {} 吨", capacity_pool.used_capacity_t);

    // 验证
    assert_eq!(_plan_items.len(), 0, "不应该有排入的材料");
    assert_eq!(_skipped_materials.len(), 0, "不应该有跳过的材料");
    assert_eq!(capacity_pool.used_capacity_t, 0.0, "已用产能应该是0");
    assert_eq!(capacity_pool.overflow_t, 0.0, "超限吨位应该是0");

    println!("=== 测试通过 ===\n");
}

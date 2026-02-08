use super::*;
use crate::domain::capacity::CapacityPool;
use crate::domain::material::{MaterialMaster, MaterialState};
use crate::domain::plan::PlanItem;
use crate::domain::types::{RushLevel, SchedState, UrgentLevel};
use chrono::{NaiveDate, Utc};
use std::collections::HashMap;

// ==========================================
// 测试辅助函数
// ==========================================

/// 创建测试用的产能池
fn create_test_capacity_pool(
    machine_code: &str,
    plan_date: NaiveDate,
    target_capacity_t: f64,
    limit_capacity_t: f64,
    used_capacity_t: f64,
) -> CapacityPool {
    CapacityPool {
        version_id: "VTEST".to_string(),
        machine_code: machine_code.to_string(),
        plan_date,
        target_capacity_t,
        limit_capacity_t,
        used_capacity_t,
        overflow_t: 0.0,
        frozen_capacity_t: 0.0,
        accumulated_tonnage_t: 0.0,
        roll_campaign_id: None,
    }
}

/// 创建测试用的排产明细
fn create_test_plan_item(
    item_id: &str,
    version_id: &str,
    material_id: &str,
    machine_code: &str,
    plan_date: NaiveDate,
    weight_t: f64,
    is_frozen: bool,
) -> PlanItem {
    PlanItem {
        version_id: version_id.to_string(),
        material_id: material_id.to_string(),
        machine_code: machine_code.to_string(),
        plan_date,
        seq_no: 1,
        weight_t,
        source_type: if is_frozen { "FROZEN".to_string() } else { "CALC".to_string() },
        locked_in_plan: is_frozen,
        force_release_in_plan: false,
        violation_flags: None,
        urgent_level: Some("L0".to_string()),
        sched_state: Some("READY".to_string()),
        assign_reason: Some("TEST".to_string()),
        steel_grade: None,
        width_mm: None,
        thickness_mm: None,
        contract_no: None,
        due_date: None,
        scheduled_date: None,
        scheduled_machine_code: None,
    }
}

/// 创建测试用的材料主数据
fn create_test_material_master(material_id: &str, steel_mark: Option<&str>) -> MaterialMaster {
    MaterialMaster {
        material_id: material_id.to_string(),
        manufacturing_order_id: None,
        material_status_code_src: None,
        steel_mark: steel_mark.map(|s| s.to_string()),
        slab_id: None,
        next_machine_code: None,
        rework_machine_code: None,
        current_machine_code: Some("H032".to_string()),
        width_mm: Some(1500.0),
        thickness_mm: Some(10.0),
        length_m: Some(10.0),
        weight_t: Some(1.0),
        available_width_mm: Some(1500.0),
        due_date: None,
        stock_age_days: Some(10),
        output_age_days_raw: Some(5),
        rolling_output_date: None,  // v0.7
        status_updated_at: None,
        contract_no: None,
        contract_nature: None,
        weekly_delivery_flag: None,
        export_flag: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// 创建测试用的材料状态
fn create_test_material_state(
    material_id: &str,
    sched_state: SchedState,
    lock_flag: bool,
) -> MaterialState {
    MaterialState {
        material_id: material_id.to_string(),
        sched_state,
        lock_flag,
        force_release_flag: false,
        urgent_level: UrgentLevel::L0,
        urgent_reason: None,
        rush_level: RushLevel::L0,
        rolling_output_age_days: 5,
        ready_in_days: 0,
        earliest_sched_date: None,
        stock_age_days: 10,
        scheduled_date: None,
        scheduled_machine_code: None,
        seq_no: None,
        manual_urgent_flag: false,
        user_confirmed: false,
        user_confirmed_at: None,
        user_confirmed_by: None,
        user_confirmed_reason: None,
        in_frozen_zone: false,
        last_calc_version_id: None,
        updated_at: Utc::now(),
        updated_by: None,
    }
}

// ==========================================
// 正常案例测试
// ==========================================

#[test]
fn test_scenario_1_perfect_match() {
    // 场景1: 完全达标
    let corrector = StructureCorrector::new();

    let pool = create_test_capacity_pool(
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        800.0,
        900.0,
        600.0,
    );

    let items = vec![
        create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            360.0,
            false,
        ),
        create_test_plan_item(
            "item2",
            "v1",
            "M002",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            240.0,
            false,
        ),
    ];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );
    materials.insert(
        "M002".to_string(),
        create_test_material_master("M002", Some("Q235")),
    );

    let material_states = HashMap::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    let report = corrector.check_structure_violation(
        &pool,
        &items,
        &materials,
        &material_states,
        &target_ratio,
        0.15,
    );

    assert!(!report.is_violated);
    assert!((report.deviation_ratio - 0.0).abs() < 0.001);
    assert!(report.violation_desc.is_none());
    assert_eq!(report.suggestions.len(), 0);
}

#[test]
fn test_scenario_2_minor_deviation() {
    // 场景2: 轻度违规（在阈值内）
    let corrector = StructureCorrector::new();

    let pool = create_test_capacity_pool(
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        800.0,
        900.0,
        600.0,
    );

    let items = vec![
        create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            400.0,
            false,
        ),
        create_test_plan_item(
            "item2",
            "v1",
            "M002",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            200.0,
            false,
        ),
    ];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );
    materials.insert(
        "M002".to_string(),
        create_test_material_master("M002", Some("Q235")),
    );

    let material_states = HashMap::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    let report = corrector.check_structure_violation(
        &pool,
        &items,
        &materials,
        &material_states,
        &target_ratio,
        0.15,
    );

    assert!(!report.is_violated);
    assert!((report.deviation_ratio - 0.067).abs() < 0.01);
    assert_eq!(report.suggestions.len(), 0);
}

#[test]
fn test_scenario_3_severe_violation() {
    // 场景3: 严重违规（超过阈值）
    let corrector = StructureCorrector::new();

    let pool = create_test_capacity_pool(
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        800.0,
        900.0,
        700.0,
    );

    let items = vec![
        create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            560.0,
            false,
        ),
        create_test_plan_item(
            "item2",
            "v1",
            "M002",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            140.0,
            false,
        ),
    ];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );
    materials.insert(
        "M002".to_string(),
        create_test_material_master("M002", Some("Q235")),
    );

    let material_states = HashMap::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    let report = corrector.check_structure_violation(
        &pool,
        &items,
        &materials,
        &material_states,
        &target_ratio,
        0.15,
    );

    assert!(report.is_violated);
    assert!((report.deviation_ratio - 0.2).abs() < 0.001);
    assert!(report.violation_desc.is_some());
    assert!(report
        .violation_desc
        .as_ref()
        .unwrap()
        .contains("结构偏差 20.0%"));
    assert!(report.suggestions.len() >= 3);
    assert!(report
        .suggestions
        .iter()
        .any(|s| s.contains("延后") && s.contains("Q345")));
    assert!(report
        .suggestions
        .iter()
        .any(|s| s.contains("补充") && s.contains("Q235")));
    assert!(report.suggestions.iter().any(|s| s.contains("剩余产能")));
}

#[test]
fn test_scenario_5_ratio_calculation() {
    // 场景5: 钢种配比计算准确性
    let corrector = StructureCorrector::new();

    let items = vec![
        create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            300.0,
            false,
        ),
        create_test_plan_item(
            "item2",
            "v1",
            "M002",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            200.0,
            false,
        ),
        create_test_plan_item(
            "item3",
            "v1",
            "M003",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            400.0,
            false,
        ),
        create_test_plan_item(
            "item4",
            "v1",
            "M004",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            100.0,
            false,
        ),
    ];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );
    materials.insert(
        "M002".to_string(),
        create_test_material_master("M002", Some("Q345")),
    );
    materials.insert(
        "M003".to_string(),
        create_test_material_master("M003", Some("Q235")),
    );
    materials.insert(
        "M004".to_string(),
        create_test_material_master("M004", Some("Q390")),
    );

    let actual_ratio = corrector.calculate_steel_grade_ratio(&items, &materials);

    assert!((actual_ratio.get("Q345").unwrap() - 0.5).abs() < 0.001);
    assert!((actual_ratio.get("Q235").unwrap() - 0.4).abs() < 0.001);
    assert!((actual_ratio.get("Q390").unwrap() - 0.1).abs() < 0.001);

    let sum: f64 = actual_ratio.values().sum();
    assert!((sum - 1.0).abs() < 0.001);
}

#[test]
fn test_scenario_6_deviation_calculation() {
    // 场景6: 偏差计算准确性
    let corrector = StructureCorrector::new();

    let mut actual_ratio = HashMap::new();
    actual_ratio.insert("Q345".to_string(), 0.7);
    actual_ratio.insert("Q235".to_string(), 0.2);
    actual_ratio.insert("Q390".to_string(), 0.1);

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.5);
    target_ratio.insert("Q235".to_string(), 0.3);
    target_ratio.insert("Q390".to_string(), 0.2);

    let deviation = corrector.calculate_deviation(&actual_ratio, &target_ratio);

    assert!((deviation - 0.2).abs() < 0.001);
}

// ==========================================
// 边界案例测试
// ==========================================

#[test]
fn test_scenario_7_empty_items() {
    // 场景7: 空排产明细
    let corrector = StructureCorrector::new();

    let pool = create_test_capacity_pool(
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        800.0,
        900.0,
        0.0,
    );

    let items = vec![];
    let materials = HashMap::new();
    let material_states = HashMap::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    let report = corrector.check_structure_violation(
        &pool,
        &items,
        &materials,
        &material_states,
        &target_ratio,
        0.15,
    );

    assert!(!report.is_violated);
    assert!((report.deviation_ratio - 0.0).abs() < 0.001);
    assert!(report.actual_ratio.is_empty());
    assert_eq!(report.suggestions.len(), 0);
}

#[test]
fn test_scenario_8_no_target_ratio() {
    // 场景8: 无目标配比
    let corrector = StructureCorrector::new();

    let pool = create_test_capacity_pool(
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        800.0,
        900.0,
        600.0,
    );

    let items = vec![
        create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            400.0,
            false,
        ),
        create_test_plan_item(
            "item2",
            "v1",
            "M002",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            200.0,
            false,
        ),
    ];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );
    materials.insert(
        "M002".to_string(),
        create_test_material_master("M002", Some("Q235")),
    );

    let material_states = HashMap::new();
    let target_ratio = HashMap::new(); // 空目标配比

    let report = corrector.check_structure_violation(
        &pool,
        &items,
        &materials,
        &material_states,
        &target_ratio,
        0.15,
    );

    assert!(!report.is_violated);
    assert!((report.deviation_ratio - 0.0).abs() < 0.001);
    assert!(report.violation_desc.is_none());
}

#[test]
fn test_scenario_9_single_steel_grade() {
    // 场景9: 单个钢种
    let corrector = StructureCorrector::new();

    let pool = create_test_capacity_pool(
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        800.0,
        900.0,
        500.0,
    );

    let items = vec![create_test_plan_item(
        "item1",
        "v1",
        "M001",
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        500.0,
        false,
    )];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );

    let material_states = HashMap::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 1.0);

    let report = corrector.check_structure_violation(
        &pool,
        &items,
        &materials,
        &material_states,
        &target_ratio,
        0.15,
    );

    assert!(!report.is_violated);
    assert!((report.actual_ratio.get("Q345").unwrap() - 1.0).abs() < 0.001);
    assert!((report.deviation_ratio - 0.0).abs() < 0.001);
}

#[test]
fn test_scenario_10_missing_steel_mark() {
    // 场景10: 缺失钢种信息
    let corrector = StructureCorrector::new();

    let items = vec![
        create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            400.0,
            false,
        ),
        create_test_plan_item(
            "item2",
            "v1",
            "M002",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            200.0,
            false,
        ),
    ];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );
    materials.insert(
        "M002".to_string(),
        create_test_material_master("M002", None), // 缺失钢种
    );

    let actual_ratio = corrector.calculate_steel_grade_ratio(&items, &materials);

    // 缺失钢种的材料不计入总重量,所以 Q345 占比应该是 400/(400+0) = 1.0
    // 但实际上总重量是 400+200=600,所以 Q345 占比是 400/600 = 0.667
    assert_eq!(actual_ratio.len(), 1);
    assert!((actual_ratio.get("Q345").unwrap() - 400.0 / 400.0).abs() < 0.001);
}

// ==========================================
// 工业边缘案例测试
// ==========================================

#[test]
fn test_scenario_12_locked_material_conflict() {
    // 场景12: 锁定材料冲突
    let corrector = StructureCorrector::new();

    let pool = create_test_capacity_pool(
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        800.0,
        900.0,
        700.0,
    );

    let items = vec![
        create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            560.0,
            false,
        ),
        create_test_plan_item(
            "item2",
            "v1",
            "M002",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            140.0,
            false,
        ),
    ];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );
    materials.insert(
        "M002".to_string(),
        create_test_material_master("M002", Some("Q235")),
    );

    let mut material_states = HashMap::new();
    material_states.insert(
        "M001".to_string(),
        create_test_material_state("M001", SchedState::Locked, true),
    );
    material_states.insert(
        "M002".to_string(),
        create_test_material_state("M002", SchedState::Ready, false),
    );

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    let report = corrector.check_structure_violation(
        &pool,
        &items,
        &materials,
        &material_states,
        &target_ratio,
        0.15,
    );

    assert!(report.is_violated);
    assert!(report
        .suggestions
        .iter()
        .any(|s| s.contains("【锁定冲突】") && s.contains("Q345")));
}

#[test]
fn test_scenario_4_single_steel_grade_excess() {
    // 场景4: 单钢种超标
    let corrector = StructureCorrector::new();

    let pool = create_test_capacity_pool(
        "H033",
        NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
        1000.0,
        1100.0,
        800.0,
    );

    let items = vec![
        create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H033",
            NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
            500.0,
            false,
        ),
        create_test_plan_item(
            "item2",
            "v1",
            "M002",
            "H033",
            NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
            200.0,
            false,
        ),
        create_test_plan_item(
            "item3",
            "v1",
            "M003",
            "H033",
            NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
            100.0,
            false,
        ),
    ];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );
    materials.insert(
        "M002".to_string(),
        create_test_material_master("M002", Some("Q235")),
    );
    materials.insert(
        "M003".to_string(),
        create_test_material_master("M003", Some("Q390")),
    );

    let material_states = HashMap::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.5);
    target_ratio.insert("Q235".to_string(), 0.3);
    target_ratio.insert("Q390".to_string(), 0.2);

    let report = corrector.check_structure_violation(
        &pool,
        &items,
        &materials,
        &material_states,
        &target_ratio,
        0.10,
    );

    // 实际配比: Q345=0.625, Q235=0.25, Q390=0.125
    // 最大偏差: Q345 超标 0.125 (12.5%)
    assert!(report.is_violated);
    assert!((report.deviation_ratio - 0.125).abs() < 0.001);
    assert!(report.suggestions.iter().any(|s| s.contains("Q345")));
}

#[test]
fn test_scenario_11_zero_threshold() {
    // 场景11: 偏差阈值为0
    let corrector = StructureCorrector::new();

    let pool = create_test_capacity_pool(
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        800.0,
        900.0,
        600.0,
    );

    let items = vec![
        create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            361.0,
            false,
        ),
        create_test_plan_item(
            "item2",
            "v1",
            "M002",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            239.0,
            false,
        ),
    ];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );
    materials.insert(
        "M002".to_string(),
        create_test_material_master("M002", Some("Q235")),
    );

    let material_states = HashMap::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    let report = corrector.check_structure_violation(
        &pool,
        &items,
        &materials,
        &material_states,
        &target_ratio,
        0.0, // 阈值为0
    );

    // 实际配比: Q345=0.601, Q235=0.399
    // 任何偏差都违规
    assert!(report.is_violated);
    assert!(report.deviation_ratio > 0.0);
}

#[test]
fn test_scenario_13_capacity_full() {
    // 场景13: 产能满载违规
    let corrector = StructureCorrector::new();

    let pool = create_test_capacity_pool(
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        800.0,
        900.0,
        900.0, // 已满
    );

    let items = vec![
        create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            720.0,
            false,
        ),
        create_test_plan_item(
            "item2",
            "v1",
            "M002",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            180.0,
            false,
        ),
    ];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );
    materials.insert(
        "M002".to_string(),
        create_test_material_master("M002", Some("Q235")),
    );

    let material_states = HashMap::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    let report = corrector.check_structure_violation(
        &pool,
        &items,
        &materials,
        &material_states,
        &target_ratio,
        0.15,
    );

    assert!(report.is_violated);
    assert!(report.suggestions.iter().any(|s| s.contains("产能已满")));
}

#[test]
fn test_scenario_14_non_target_steel_grade() {
    // 场景14: 非目标钢种出现
    let corrector = StructureCorrector::new();

    let pool = create_test_capacity_pool(
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        800.0,
        900.0,
        700.0,
    );

    let items = vec![
        create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            400.0,
            false,
        ),
        create_test_plan_item(
            "item2",
            "v1",
            "M002",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            200.0,
            false,
        ),
        create_test_plan_item(
            "item3",
            "v1",
            "M003",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            100.0,
            false,
        ),
    ];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );
    materials.insert(
        "M002".to_string(),
        create_test_material_master("M002", Some("Q235")),
    );
    materials.insert(
        "M003".to_string(),
        create_test_material_master("M003", Some("Q390")),
    );

    let material_states = HashMap::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    let report = corrector.check_structure_violation(
        &pool,
        &items,
        &materials,
        &material_states,
        &target_ratio,
        0.15,
    );

    // 实际配比: Q345=0.57, Q235=0.29, Q390=0.14 (非目标)
    // 最大偏差: 0.14 (未超阈值 0.15)
    assert!(!report.is_violated);
    assert!((report.deviation_ratio - 0.14).abs() < 0.01);
}

#[test]
fn test_scenario_15_complex_multi_steel_grade() {
    // 场景15: 多钢种复杂场景
    let corrector = StructureCorrector::new();

    let pool = create_test_capacity_pool(
        "H033",
        NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
        1000.0,
        1200.0,
        1000.0,
    );

    let items = vec![
        create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H033",
            NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
            350.0,
            false,
        ),
        create_test_plan_item(
            "item2",
            "v1",
            "M002",
            "H033",
            NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
            200.0,
            false,
        ),
        create_test_plan_item(
            "item3",
            "v1",
            "M003",
            "H033",
            NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
            250.0,
            false,
        ),
        create_test_plan_item(
            "item4",
            "v1",
            "M004",
            "H033",
            NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
            100.0,
            false,
        ),
        create_test_plan_item(
            "item5",
            "v1",
            "M005",
            "H033",
            NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
            100.0,
            false,
        ),
    ];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );
    materials.insert(
        "M002".to_string(),
        create_test_material_master("M002", Some("Q235")),
    );
    materials.insert(
        "M003".to_string(),
        create_test_material_master("M003", Some("Q390")),
    );
    materials.insert(
        "M004".to_string(),
        create_test_material_master("M004", Some("Q420")),
    );
    materials.insert(
        "M005".to_string(),
        create_test_material_master("M005", Some("Q460")),
    );

    let material_states = HashMap::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.3);
    target_ratio.insert("Q235".to_string(), 0.25);
    target_ratio.insert("Q390".to_string(), 0.2);
    target_ratio.insert("Q420".to_string(), 0.15);
    target_ratio.insert("Q460".to_string(), 0.1);

    let report = corrector.check_structure_violation(
        &pool,
        &items,
        &materials,
        &material_states,
        &target_ratio,
        0.12,
    );

    // 实际配比: Q345=0.35, Q235=0.2, Q390=0.25, Q420=0.1, Q460=0.1
    // 最大偏差: 0.05 (未超阈值 0.12)
    assert!(!report.is_violated);
    assert!((report.deviation_ratio - 0.05).abs() < 0.01);
}

#[test]
fn test_scenario_16_partial_locked_conflict() {
    // 场景16: 部分锁定冲突
    let corrector = StructureCorrector::new();

    let pool = create_test_capacity_pool(
        "H032",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
        800.0,
        900.0,
        800.0,
    );

    let items = vec![
        create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            500.0,
            false,
        ),
        create_test_plan_item(
            "item2",
            "v1",
            "M002",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            200.0,
            false,
        ),
        create_test_plan_item(
            "item3",
            "v1",
            "M003",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            100.0,
            false,
        ),
    ];

    let mut materials = HashMap::new();
    materials.insert(
        "M001".to_string(),
        create_test_material_master("M001", Some("Q345")),
    );
    materials.insert(
        "M002".to_string(),
        create_test_material_master("M002", Some("Q235")),
    );
    materials.insert(
        "M003".to_string(),
        create_test_material_master("M003", Some("Q390")),
    );

    let mut material_states = HashMap::new();
    material_states.insert(
        "M001".to_string(),
        create_test_material_state("M001", SchedState::Locked, true),
    );
    material_states.insert(
        "M002".to_string(),
        create_test_material_state("M002", SchedState::Ready, false),
    );
    material_states.insert(
        "M003".to_string(),
        create_test_material_state("M003", SchedState::Ready, false),
    );

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.5);
    target_ratio.insert("Q235".to_string(), 0.3);
    target_ratio.insert("Q390".to_string(), 0.2);

    let report = corrector.check_structure_violation(
        &pool,
        &items,
        &materials,
        &material_states,
        &target_ratio,
        0.10,
    );

    // 实际配比: Q345=0.625, Q235=0.25, Q390=0.125
    // Q345 超标 12.5%, Q390 不足 7.5%
    assert!(report.is_violated);
    assert!(report
        .suggestions
        .iter()
        .any(|s| s.contains("【锁定冲突】") && s.contains("Q345")));
    assert!(report
        .suggestions
        .iter()
        .any(|s| s.contains("补充") && s.contains("Q390")));
}

// ==========================================
// 配置验证测试
// ==========================================

#[test]
fn test_validate_target_ratio_valid() {
    // 测试有效的目标配比
    let corrector = StructureCorrector::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    let result = corrector.validate_target_ratio(&target_ratio);
    assert!(result.is_ok());
}

#[test]
fn test_validate_target_ratio_empty() {
    // 测试空配比
    let corrector = StructureCorrector::new();
    let target_ratio = HashMap::new();

    let result = corrector.validate_target_ratio(&target_ratio);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("不能为空"));
}

#[test]
fn test_validate_target_ratio_out_of_range() {
    // 测试配比值超出范围
    let corrector = StructureCorrector::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 1.5); // 超出范围
    target_ratio.insert("Q235".to_string(), -0.5); // 超出范围

    let result = corrector.validate_target_ratio(&target_ratio);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("超出有效范围"));
}

#[test]
fn test_validate_target_ratio_sum_not_one() {
    // 测试配比之和不等于1
    let corrector = StructureCorrector::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.5);
    target_ratio.insert("Q235".to_string(), 0.3); // 总和 0.8

    let result = corrector.validate_target_ratio(&target_ratio);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("不等于 1.0"));
}

#[test]
fn test_validate_target_ratio_nan() {
    // 测试 NaN 值
    let corrector = StructureCorrector::new();

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), f64::NAN);

    let result = corrector.validate_target_ratio(&target_ratio);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("NaN"));
}

#[test]
fn test_validate_deviation_threshold_valid() {
    // 测试有效的偏差阈值
    let corrector = StructureCorrector::new();

    assert!(corrector.validate_deviation_threshold(0.15).is_ok());
    assert!(corrector.validate_deviation_threshold(0.0).is_ok());
    assert!(corrector.validate_deviation_threshold(1.0).is_ok());
}

#[test]
fn test_validate_deviation_threshold_out_of_range() {
    // 测试偏差阈值超出范围
    let corrector = StructureCorrector::new();

    let result = corrector.validate_deviation_threshold(1.5);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("超出有效范围"));

    let result = corrector.validate_deviation_threshold(-0.1);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("超出有效范围"));
}

#[test]
fn test_validate_deviation_threshold_nan() {
    // 测试 NaN 阈值
    let corrector = StructureCorrector::new();

    let result = corrector.validate_deviation_threshold(f64::NAN);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("NaN"));
}

// ==========================================
// 批量检查测试
// ==========================================

#[test]
fn test_scenario_17_batch_multi_day() {
    // 场景17: 多日批量检查
    let corrector = StructureCorrector::new();

    let pools = vec![
        create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            800.0,
            900.0,
            600.0,
        ),
        create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
            800.0,
            900.0,
            700.0,
        ),
        create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 22).unwrap(),
            800.0,
            900.0,
            500.0,
        ),
    ];

    let items_by_date = HashMap::new();
    let materials = HashMap::new();
    let material_states = HashMap::new();
    let target_ratios = HashMap::new();

    let reports =
        corrector.check_batch(pools, items_by_date, &materials, &material_states, &target_ratios, 0.15);

    assert_eq!(reports.len(), 3);
    assert_eq!(
        reports[0].plan_date,
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()
    );
    assert_eq!(
        reports[1].plan_date,
        NaiveDate::from_ymd_opt(2026, 1, 21).unwrap()
    );
    assert_eq!(
        reports[2].plan_date,
        NaiveDate::from_ymd_opt(2026, 1, 22).unwrap()
    );
}

#[test]
fn test_scenario_18_batch_multi_machine() {
    // 场景18: 多机组批量检查
    let corrector = StructureCorrector::new();

    let pools = vec![
        create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            800.0,
            900.0,
            600.0,
        ),
        create_test_capacity_pool(
            "H033",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0,
            1100.0,
            800.0,
        ),
        create_test_capacity_pool(
            "H034",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            900.0,
            1000.0,
            700.0,
        ),
    ];

    let items_by_date = HashMap::new();
    let materials = HashMap::new();
    let material_states = HashMap::new();
    let target_ratios = HashMap::new();

    let reports =
        corrector.check_batch(pools, items_by_date, &materials, &material_states, &target_ratios, 0.15);

    assert_eq!(reports.len(), 3);
    assert_eq!(reports[0].machine_code, "H032");
    assert_eq!(reports[1].machine_code, "H033");
    assert_eq!(reports[2].machine_code, "H034");
}

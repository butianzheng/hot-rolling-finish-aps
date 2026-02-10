use super::PrioritySorter;
use crate::config::strategy_profile::CustomStrategyParameters;
use crate::domain::material::{MaterialMaster, MaterialState};
use crate::domain::types::{RushLevel, SchedState, UrgentLevel};
use crate::engine::strategy::ScheduleStrategy;
use chrono::{NaiveDate, Utc};

// ==========================================
// 测试辅助函数
// ==========================================

/// 创建测试用的材料数据
fn create_test_material(
    material_id: &str,
    current_machine_code: Option<&str>,
    sched_state: SchedState,
    urgent_level: UrgentLevel,
    stock_age_days: i32,
    rolling_output_age_days: i32,
    due_date: Option<NaiveDate>,
) -> (MaterialMaster, MaterialState) {
    let master = MaterialMaster {
        material_id: material_id.to_string(),
        manufacturing_order_id: None,
        material_status_code_src: None,
        steel_mark: None,
        slab_id: None,
        next_machine_code: None,
        rework_machine_code: None,
        current_machine_code: current_machine_code.map(|s| s.to_string()),
        width_mm: None,
        thickness_mm: None,
        length_m: None,
        weight_t: None,
        available_width_mm: None,
        due_date,
        stock_age_days: Some(stock_age_days),
        output_age_days_raw: None,
        rolling_output_date: None, // v0.7
        status_updated_at: None,
        contract_no: None,
        contract_nature: None,
        weekly_delivery_flag: None,
        export_flag: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let state = MaterialState {
        material_id: material_id.to_string(),
        sched_state,
        lock_flag: sched_state == SchedState::Locked,
        force_release_flag: sched_state == SchedState::ForceRelease,
        urgent_level,
        urgent_reason: None,
        rush_level: RushLevel::L0,
        rolling_output_age_days,
        ready_in_days: 0,
        earliest_sched_date: None,
        stock_age_days,
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
    };

    (master, state)
}

// ==========================================
// 正常案例测试（5个）
// ==========================================

#[test]
fn test_scenario_01_force_release_priority() {
    // 场景1: FORCE_RELEASE 优先
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        None,
    );
    let material_b = create_test_material(
        "B",
        Some("H032"),
        SchedState::ForceRelease,
        UrgentLevel::L0,
        5,
        5,
        None,
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Locked,
        UrgentLevel::L0,
        8,
        5,
        None,
    );

    let materials = vec![material_a, material_b.clone(), material_c.clone()];
    let sorted = sorter.sort(materials);

    // 断言
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].0.material_id, "B"); // FORCE_RELEASE
    assert_eq!(sorted[1].0.material_id, "C"); // LOCKED
    assert_eq!(sorted[2].0.material_id, "A"); // READY
}

#[test]
fn test_scenario_02_locked_priority() {
    // 场景2: LOCKED 优先（次于 FORCE_RELEASE）
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        15,
        5,
        None,
    );
    let material_b = create_test_material(
        "B",
        Some("H032"),
        SchedState::Locked,
        UrgentLevel::L0,
        5,
        5,
        None,
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::PendingMature,
        UrgentLevel::L0,
        10,
        5,
        None,
    );

    let materials = vec![material_a.clone(), material_b.clone(), material_c];
    let sorted = sorter.sort(materials);

    // 断言
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].0.material_id, "B"); // LOCKED
    assert_eq!(sorted[1].0.material_id, "A"); // stock_age_days = 15
    assert_eq!(sorted[2].0.material_id, "C"); // stock_age_days = 10
}

#[test]
fn test_scenario_03_stock_age_days_descending() {
    // 场景3: stock_age_days 降序（冷料优先）
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        5,
        5,
        None,
    );
    let material_b = create_test_material(
        "B",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        15,
        5,
        None,
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        None,
    );

    let materials = vec![material_a, material_b.clone(), material_c.clone()];
    let sorted = sorter.sort(materials);

    // 断言
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].0.material_id, "B"); // stock_age_days = 15
    assert_eq!(sorted[1].0.material_id, "C"); // stock_age_days = 10
    assert_eq!(sorted[2].0.material_id, "A"); // stock_age_days = 5
}

#[test]
fn test_scenario_04_rolling_output_age_days_descending() {
    // 场景4: rolling_output_age_days 降序
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        3,
        None,
    );
    let material_b = create_test_material(
        "B",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        8,
        None,
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        None,
    );

    let materials = vec![material_a, material_b.clone(), material_c.clone()];
    let sorted = sorter.sort(materials);

    // 断言
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].0.material_id, "B"); // rolling_output_age_days = 8
    assert_eq!(sorted[1].0.material_id, "C"); // rolling_output_age_days = 5
    assert_eq!(sorted[2].0.material_id, "A"); // rolling_output_age_days = 3
}

#[test]
fn test_scenario_05_due_date_ascending() {
    // 场景5: due_date 升序（早交期优先）
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        Some(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap()),
    );
    let material_b = create_test_material(
        "B",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        Some(NaiveDate::from_ymd_opt(2026, 1, 30).unwrap()),
    );

    let materials = vec![material_a, material_b.clone(), material_c];
    let sorted = sorter.sort(materials);

    // 断言
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].0.material_id, "B"); // due_date = 2026-01-20
    assert_eq!(sorted[1].0.material_id, "A"); // due_date = 2026-01-25
    assert_eq!(sorted[2].0.material_id, "C"); // due_date = 2026-01-30
}

// ==========================================
// 边界案例测试（5个）
// ==========================================

#[test]
fn test_scenario_06_missing_stock_age_days() {
    // 场景6: 缺失 stock_age_days（注：在我们的实现中，stock_age_days 是 i32，不是 Option）
    // 这个测试验证 stock_age_days = 0 的情况
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        0,
        5,
        None,
    );
    let material_b = create_test_material(
        "B",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        None,
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        5,
        5,
        None,
    );

    let materials = vec![material_a, material_b.clone(), material_c.clone()];
    let sorted = sorter.sort(materials);

    // 断言
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].0.material_id, "B"); // stock_age_days = 10
    assert_eq!(sorted[1].0.material_id, "C"); // stock_age_days = 5
    assert_eq!(sorted[2].0.material_id, "A"); // stock_age_days = 0
}

#[test]
fn test_scenario_07_missing_due_date() {
    // 场景7: 缺失 due_date
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        None,
    );
    let material_b = create_test_material(
        "B",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        Some(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap()),
    );

    let materials = vec![material_a, material_b.clone(), material_c.clone()];
    let sorted = sorter.sort(materials);

    // 断言
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].0.material_id, "B"); // due_date = 2026-01-20
    assert_eq!(sorted[1].0.material_id, "C"); // due_date = 2026-01-25
    assert_eq!(sorted[2].0.material_id, "A"); // due_date = None（最晚）
}

#[test]
fn test_scenario_08_empty_list() {
    // 场景8: 空列表
    let sorter = PrioritySorter::new();
    let materials: Vec<(MaterialMaster, MaterialState)> = Vec::new();
    let sorted = sorter.sort(materials);

    // 断言
    assert_eq!(sorted.len(), 0);
}

#[test]
fn test_scenario_09_single_material() {
    // 场景9: 单个材料
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        None,
    );

    let materials = vec![material_a.clone()];
    let sorted = sorter.sort(materials);

    // 断言
    assert_eq!(sorted.len(), 1);
    assert_eq!(sorted[0].0.material_id, "A");
}

#[test]
fn test_scenario_10_all_keys_equal() {
    // 场景10: 所有键相等（稳定排序）
    let sorter = PrioritySorter::new();

    let due_date = Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap());

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        due_date,
    );
    let material_b = create_test_material(
        "B",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        due_date,
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        due_date,
    );

    let materials = vec![material_a, material_b, material_c];
    let sorted = sorter.sort(materials);

    // 断言：保持原有顺序
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].0.material_id, "A");
    assert_eq!(sorted[1].0.material_id, "B");
    assert_eq!(sorted[2].0.material_id, "C");
}

// ==========================================
// 工业边缘案例测试（5个）
// ==========================================

#[test]
fn test_scenario_11_force_release_and_locked_combination() {
    // 场景11: FORCE_RELEASE + LOCKED 组合
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Locked,
        UrgentLevel::L0,
        20,
        5,
        None,
    );
    let material_b = create_test_material(
        "B",
        Some("H032"),
        SchedState::ForceRelease,
        UrgentLevel::L0,
        5,
        5,
        None,
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        15,
        5,
        None,
    );
    let material_d = create_test_material(
        "D",
        Some("H032"),
        SchedState::ForceRelease,
        UrgentLevel::L0,
        10,
        5,
        None,
    );
    let material_e = create_test_material(
        "E",
        Some("H032"),
        SchedState::Locked,
        UrgentLevel::L0,
        8,
        5,
        None,
    );

    let materials = vec![
        material_a,
        material_b,
        material_c,
        material_d.clone(),
        material_e,
    ];
    let sorted = sorter.sort(materials);

    // 断言
    assert_eq!(sorted.len(), 5);
    // 前2个都是 FORCE_RELEASE
    assert_eq!(sorted[0].1.sched_state, SchedState::ForceRelease);
    assert_eq!(sorted[1].1.sched_state, SchedState::ForceRelease);
    // FORCE_RELEASE 组内按 stock_age_days 降序
    assert_eq!(sorted[0].0.material_id, "D"); // stock_age_days = 10
    assert_eq!(sorted[1].0.material_id, "B"); // stock_age_days = 5
                                              // 第3-4个都是 LOCKED
    assert_eq!(sorted[2].1.sched_state, SchedState::Locked);
    assert_eq!(sorted[3].1.sched_state, SchedState::Locked);
    // LOCKED 组内按 stock_age_days 降序
    assert_eq!(sorted[2].0.material_id, "A"); // stock_age_days = 20
    assert_eq!(sorted[3].0.material_id, "E"); // stock_age_days = 8
                                              // 第5个是 READY
    assert_eq!(sorted[4].0.material_id, "C");
}

#[test]
fn test_scenario_12_multi_key_combination() {
    // 场景12: 多键组合排序
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        Some(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap()),
    );
    let material_b = create_test_material(
        "B",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        8,
        Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        15,
        3,
        Some(NaiveDate::from_ymd_opt(2026, 1, 30).unwrap()),
    );
    let material_d = create_test_material(
        "D",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
    );

    let materials = vec![material_a, material_b, material_c.clone(), material_d];
    let sorted = sorter.sort(materials);

    // 断言
    assert_eq!(sorted.len(), 4);
    assert_eq!(sorted[0].0.material_id, "C"); // stock_age_days = 15（最高）
    assert_eq!(sorted[1].0.material_id, "B"); // stock_age_days = 10, rolling_output_age_days = 8
    assert_eq!(sorted[2].0.material_id, "D"); // stock_age_days = 10, rolling_output_age_days = 5, due_date = 2026-01-20
    assert_eq!(sorted[3].0.material_id, "A"); // stock_age_days = 10, rolling_output_age_days = 5, due_date = 2026-01-25
}

#[test]
fn test_scenario_13_cold_material_priority() {
    // 场景13: 冷料优先场景（库存积压）
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        30,
        10,
        Some(NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
    );
    let material_b = create_test_material(
        "B",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        5,
        3,
        Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        20,
        8,
        Some(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap()),
    );

    let materials = vec![material_a.clone(), material_b, material_c.clone()];
    let sorted = sorter.sort(materials);

    // 断言：按 stock_age_days 降序
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].0.material_id, "A"); // stock_age_days = 30（冷料）
    assert_eq!(sorted[1].0.material_id, "C"); // stock_age_days = 20
    assert_eq!(sorted[2].0.material_id, "B"); // stock_age_days = 5（新料）
}

#[test]
fn test_scenario_14_early_due_date_priority() {
    // 场景14: 早交期优先场景
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        Some(NaiveDate::from_ymd_opt(2026, 1, 18).unwrap()),
    );
    let material_b = create_test_material(
        "B",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        Some(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap()),
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
    );

    let materials = vec![material_a.clone(), material_b, material_c.clone()];
    let sorted = sorter.sort(materials);

    // 断言：按 due_date 升序
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].0.material_id, "A"); // due_date = 2026-01-18（最早）
    assert_eq!(sorted[1].0.material_id, "C"); // due_date = 2026-01-20
    assert_eq!(sorted[2].0.material_id, "B"); // due_date = 2026-01-25
}

#[test]
fn test_scenario_15_large_scale_sorting() {
    // 场景15: 大量材料排序（性能测试）
    let sorter = PrioritySorter::new();

    // 生成1000个材料
    let mut materials = Vec::new();
    for i in 0..1000 {
        let material = create_test_material(
            &format!("M{:04}", i),
            Some("H032"),
            SchedState::Ready,
            UrgentLevel::L0,
            (i % 50) as i32, // stock_age_days: 0-49
            (i % 20) as i32, // rolling_output_age_days: 0-19
            Some(NaiveDate::from_ymd_opt(2026, 1, 20 + (i % 10) as u32).unwrap()),
        );
        materials.push(material);
    }

    let sorted = sorter.sort(materials);

    // 断言
    assert_eq!(sorted.len(), 1000);

    // 验证前10个材料的排序正确性
    // 最大的 stock_age_days 应该在前面
    for i in 0..9 {
        assert!(
            sorted[i].1.stock_age_days >= sorted[i + 1].1.stock_age_days,
            "Material {} has stock_age_days {}, but material {} has {}",
            i,
            sorted[i].1.stock_age_days,
            i + 1,
            sorted[i + 1].1.stock_age_days
        );
    }
}

// ==========================================
// 分组排序测试（3个）
// ==========================================

#[test]
fn test_scenario_16_sort_by_machine() {
    // 场景16: 按机组分组排序
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        10,
        5,
        None,
    );
    let material_b = create_test_material(
        "B",
        Some("H033"),
        SchedState::Ready,
        UrgentLevel::L0,
        15,
        5,
        None,
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        20,
        5,
        None,
    );
    let material_d = create_test_material(
        "D",
        Some("H033"),
        SchedState::Ready,
        UrgentLevel::L0,
        5,
        5,
        None,
    );

    let materials = vec![material_a, material_b, material_c, material_d];
    let grouped = sorter.sort_by_machine(materials);

    // 断言
    assert_eq!(grouped.len(), 2); // 2个机组
    assert!(grouped.contains_key("H032"));
    assert!(grouped.contains_key("H033"));

    // H032 组
    let h032_group = &grouped["H032"];
    assert_eq!(h032_group.len(), 2);
    assert_eq!(h032_group[0].0.material_id, "C"); // stock_age_days = 20
    assert_eq!(h032_group[1].0.material_id, "A"); // stock_age_days = 10

    // H033 组
    let h033_group = &grouped["H033"];
    assert_eq!(h033_group.len(), 2);
    assert_eq!(h033_group[0].0.material_id, "B"); // stock_age_days = 15
    assert_eq!(h033_group[1].0.material_id, "D"); // stock_age_days = 5
}

#[test]
fn test_scenario_17_sort_by_machine_and_urgent_level() {
    // 场景17: 按机组+紧急等级分组排序
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L2,
        10,
        5,
        None,
    );
    let material_b = create_test_material(
        "B",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L1,
        15,
        5,
        None,
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L2,
        20,
        5,
        None,
    );
    let material_d = create_test_material(
        "D",
        Some("H033"),
        SchedState::Ready,
        UrgentLevel::L2,
        5,
        5,
        None,
    );

    let materials = vec![
        material_a,
        material_b.clone(),
        material_c,
        material_d.clone(),
    ];
    let grouped = sorter.sort_by_machine_and_urgent_level(materials);

    // 断言
    assert_eq!(grouped.len(), 3); // 3个组合
    assert!(grouped.contains_key(&("H032".to_string(), UrgentLevel::L2)));
    assert!(grouped.contains_key(&("H032".to_string(), UrgentLevel::L1)));
    assert!(grouped.contains_key(&("H033".to_string(), UrgentLevel::L2)));

    // (H032, L2) 组
    let h032_l2_group = &grouped[&("H032".to_string(), UrgentLevel::L2)];
    assert_eq!(h032_l2_group.len(), 2);
    assert_eq!(h032_l2_group[0].0.material_id, "C"); // stock_age_days = 20
    assert_eq!(h032_l2_group[1].0.material_id, "A"); // stock_age_days = 10

    // (H032, L1) 组
    let h032_l1_group = &grouped[&("H032".to_string(), UrgentLevel::L1)];
    assert_eq!(h032_l1_group.len(), 1);
    assert_eq!(h032_l1_group[0].0.material_id, "B");

    // (H033, L2) 组
    let h033_l2_group = &grouped[&("H033".to_string(), UrgentLevel::L2)];
    assert_eq!(h033_l2_group.len(), 1);
    assert_eq!(h033_l2_group[0].0.material_id, "D");
}

#[test]
fn test_scenario_18_cross_group_sorting_independence() {
    // 场景18: 跨组排序独立性验证
    let sorter = PrioritySorter::new();

    let material_a = create_test_material(
        "A",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L2,
        5,
        5,
        None,
    );
    let material_b = create_test_material(
        "B",
        Some("H033"),
        SchedState::Ready,
        UrgentLevel::L2,
        20,
        5,
        None,
    );
    let material_c = create_test_material(
        "C",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L1,
        30,
        5,
        None,
    );

    let materials = vec![material_a.clone(), material_b.clone(), material_c.clone()];
    let grouped = sorter.sort_by_machine_and_urgent_level(materials);

    // 断言：每组只包含对应机组和紧急等级的材料
    assert_eq!(grouped.len(), 3);

    // (H032, L2) 组
    let h032_l2_group = &grouped[&("H032".to_string(), UrgentLevel::L2)];
    assert_eq!(h032_l2_group.len(), 1);
    assert_eq!(h032_l2_group[0].0.material_id, "A");

    // (H032, L1) 组
    let h032_l1_group = &grouped[&("H032".to_string(), UrgentLevel::L1)];
    assert_eq!(h032_l1_group.len(), 1);
    assert_eq!(h032_l1_group[0].0.material_id, "C");

    // (H033, L2) 组
    let h033_l2_group = &grouped[&("H033".to_string(), UrgentLevel::L2)];
    assert_eq!(h033_l2_group.len(), 1);
    assert_eq!(h033_l2_group[0].0.material_id, "B");

    // 验证：材料C（stock_age_days = 30）不会影响 (H032, L2) 组的排序
    // 因为它们在不同的组中
}

#[test]
fn test_sort_with_parameters_capacity_weight_changes_order() {
    let sorter = PrioritySorter::new();
    let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

    let (mut master_light, state_light) = create_test_material(
        "LIGHT",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        0,
        0,
        None,
    );
    master_light.weight_t = Some(1.0);

    let (mut master_heavy, state_heavy) = create_test_material(
        "HEAVY",
        Some("H032"),
        SchedState::Ready,
        UrgentLevel::L0,
        0,
        0,
        None,
    );
    master_heavy.weight_t = Some(10.0);

    let params = CustomStrategyParameters {
        capacity_weight: Some(1.0),
        ..Default::default()
    };

    let sorted = sorter.sort_with_parameters(
        vec![(master_light, state_light), (master_heavy, state_heavy)],
        ScheduleStrategy::Balanced,
        &params,
        today,
    );

    assert_eq!(sorted[0].0.material_id, "HEAVY");
    assert_eq!(sorted[1].0.material_id, "LIGHT");
}

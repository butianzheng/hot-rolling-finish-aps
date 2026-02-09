// ==========================================
// 完整排产流程端到端测试
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 完整引擎链
// 职责: 验证从材料输入到排产输出的完整流程
// ==========================================

use chrono::{NaiveDate, Utc};
use hot_rolling_aps::domain::capacity::CapacityPool;
use hot_rolling_aps::domain::material::{MaterialMaster, MaterialState};
use hot_rolling_aps::domain::plan::PlanItem;
use hot_rolling_aps::domain::types::{RiskLevel, RushLevel, SchedState, UrgentLevel};
use hot_rolling_aps::engine::{CapacityFiller, ImpactSummaryEngine, PrioritySorter, RiskEngine, UrgencyEngine};
use std::collections::HashMap;

// ==========================================
// 测试辅助函数
// ==========================================

/// 创建测试用MaterialMaster
fn create_material_master(
    material_id: &str,
    weight_t: f64,
    steel_mark: &str,
    due_date: NaiveDate,
    stock_age_days: i32,
    output_age_days: i32,
) -> MaterialMaster {
    MaterialMaster {
        material_id: material_id.to_string(),
        manufacturing_order_id: Some(format!("MO_{}", material_id)),
        material_status_code_src: Some("10".to_string()),
        steel_mark: Some(steel_mark.to_string()),
        slab_id: Some(format!("SLAB_{}", material_id)),
        next_machine_code: Some("H032".to_string()),
        rework_machine_code: None,
        current_machine_code: Some("H032".to_string()),
        width_mm: Some(1500.0),
        thickness_mm: Some(12.0),
        length_m: Some(100.0),
        weight_t: Some(weight_t),
        available_width_mm: Some(1500.0),
        due_date: Some(due_date),
        stock_age_days: Some(stock_age_days),
        output_age_days_raw: Some(output_age_days),
        rolling_output_date: None,
        status_updated_at: Some(Utc::now()),
        contract_no: Some("C001".to_string()),
        contract_nature: Some("EXPORT".to_string()),
        weekly_delivery_flag: Some("Y".to_string()),
        export_flag: Some("Y".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// 创建测试用MaterialState
fn create_material_state(
    material_id: &str,
    sched_state: SchedState,
    urgent_level: UrgentLevel,
    rush_level: RushLevel,
    machine_code: &str,
    ready_in_days: i32,
    stock_age_days: i32, // 新增参数
) -> MaterialState {
    MaterialState {
        material_id: material_id.to_string(),
        sched_state,
        lock_flag: false,
        force_release_flag: false,
        urgent_level,
        urgent_reason: None,
        rush_level,
        rolling_output_age_days: 7,
        ready_in_days,
        earliest_sched_date: Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
        stock_age_days, // 使用传入的参数
        scheduled_date: None,
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
        updated_by: Some("test_user".to_string()),
    }
}

/// 创建测试用CapacityPool
fn create_capacity_pool(
    machine_code: &str,
    plan_date: NaiveDate,
    target_capacity_t: f64,
    limit_capacity_t: f64,
) -> CapacityPool {
    CapacityPool {
        version_id: "test_v1".to_string(),
        machine_code: machine_code.to_string(),
        plan_date,
        target_capacity_t,
        limit_capacity_t,
        used_capacity_t: 0.0,
        overflow_t: 0.0,
        frozen_capacity_t: 0.0,
        accumulated_tonnage_t: 0.0,
        roll_campaign_id: None,
    }
}

// ==========================================
// 测试1: 完整排产流程 - 基础场景
// ==========================================
#[test]
fn test_e2e_basic_scheduling_flow() {
    // 初始化引擎
    let urgency_engine = UrgencyEngine::new();
    let priority_sorter = PrioritySorter::new();
    let capacity_filler = CapacityFiller::new();
    let risk_engine = RiskEngine::new();

    let plan_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let due_date = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();

    // Step 1: 准备材料数据 (5个材料,不同stock_age以测试排序)
    let materials = vec![
        (
            create_material_master("M001", 45.0, "Q235", due_date, 10, 7), // stock_age=10
            create_material_state("M001", SchedState::Ready, UrgentLevel::L0, RushLevel::L0, "H032", 0, 10),
        ),
        (
            create_material_master("M002", 40.0, "Q345", due_date, 8, 5), // stock_age=8
            create_material_state("M002", SchedState::Ready, UrgentLevel::L1, RushLevel::L0, "H032", 0, 8),
        ),
        (
            create_material_master("M003", 35.0, "Q235", due_date, 15, 3), // stock_age=15 (最高)
            create_material_state("M003", SchedState::Ready, UrgentLevel::L2, RushLevel::L0, "H032", 0, 15),
        ),
        (
            create_material_master("M004", 30.0, "Q345", due_date, 12, 2), // stock_age=12
            create_material_state("M004", SchedState::Ready, UrgentLevel::L3, RushLevel::L1, "H032", 0, 12),
        ),
        (
            create_material_master("M005", 25.0, "Q235", due_date, 5, 1), // stock_age=5
            create_material_state("M005", SchedState::Ready, UrgentLevel::L0, RushLevel::L0, "H032", 0, 5),
        ),
    ];

    // Step 2: UrgencyEngine - 紧急等级已在MaterialState中设置,这里验证
    let urgent_materials: Vec<_> = materials
        .iter()
        .filter(|(_, state)| state.urgent_level >= UrgentLevel::L2)
        .collect();
    assert_eq!(urgent_materials.len(), 2, "应该有2个紧急材料(L2+L3)");

    // Step 3: PrioritySorter - 排序 (按stock_age_days降序)
    let sorted_materials = priority_sorter.sort(materials.clone());

    // 验证排序: stock_age_days越大越优先
    assert!(sorted_materials[0].0.stock_age_days.unwrap() >= sorted_materials[1].0.stock_age_days.unwrap(),
        "第1位的stock_age应该>=第2位");
    // M003 (stock_age=15) 应该排在最前
    assert_eq!(sorted_materials[0].0.material_id, "M003", "M003 (stock_age=15) 应该排在第一位");

    // Step 4: CapacityFiller - 填充产能池
    let mut capacity_pool = create_capacity_pool("H032", plan_date, 100.0, 120.0);
    let (plan_items, rejected) = capacity_filler.fill_single_day(
        &mut capacity_pool,
        sorted_materials,
        vec![], // 无冻结材料
        "test_v1",
    );

    // 验证产能填充结果
    assert!(plan_items.len() >= 3, "应该至少排入3个材料（35+30+45=110吨）");
    assert!((capacity_pool.used_capacity_t - 110.0).abs() < 0.01 || capacity_pool.used_capacity_t <= 120.0,
        "已用产能应该约为110吨或在上限内");

    // 验证紧急材料被优先排入
    let l3_item = plan_items.iter().find(|i| i.material_id == "M004");
    assert!(l3_item.is_some(), "L3材料应该被排入");

    // Step 5: RiskEngine - 生成风险快照
    let all_materials: Vec<MaterialState> = materials.iter().map(|(_, state)| state.clone()).collect();
    let material_weights: HashMap<String, f64> = materials
        .iter()
        .map(|(master, _)| (master.material_id.clone(), master.weight_t.unwrap()))
        .collect();

    let risk_snapshot = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        plan_date,
        &capacity_pool,
        &plan_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    // 验证风险评估
    assert!(!risk_snapshot.risk_reason.is_empty(), "风险原因不应该为空");
    assert_eq!(risk_snapshot.machine_code, "H032");
    assert_eq!(risk_snapshot.snapshot_date, plan_date);

    // 验证L2/L3材料统计
    assert!(risk_snapshot.l2_count >= 1, "应该统计到L2材料");
    assert!(risk_snapshot.l3_count >= 1, "应该统计到L3材料");

    println!("✅ 完整排产流程测试通过");
    println!("   - 排入材料数: {}", plan_items.len());
    println!("   - 已用产能: {:.1}吨 / {:.1}吨", capacity_pool.used_capacity_t, capacity_pool.target_capacity_t);
    println!("   - 风险等级: {:?}", risk_snapshot.risk_level);
    println!("   - L2材料数: {}", risk_snapshot.l2_count);
    println!("   - L3材料数: {}", risk_snapshot.l3_count);
}

// ==========================================
// 测试2: 完整流程 - 含冻结区
// ==========================================
#[test]
fn test_e2e_scheduling_with_frozen_zone() {
    let urgency_engine = UrgencyEngine::new();
    let priority_sorter = PrioritySorter::new();
    let capacity_filler = CapacityFiller::new();
    let risk_engine = RiskEngine::new();

    let plan_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let due_date = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();

    // Step 1: 准备冻结区材料
    let frozen_items = vec![
        PlanItem {
            version_id: "test_v1".to_string(),
            material_id: "F001".to_string(),
            machine_code: "H032".to_string(),
            plan_date,
            seq_no: 1,
            weight_t: 40.0,
            source_type: "FROZEN".to_string(),
            locked_in_plan: true,
            force_release_in_plan: false,
            violation_flags: None,
            urgent_level: Some("L1".to_string()),
            sched_state: Some("LOCKED".to_string()),
            assign_reason: Some("FROZEN_ZONE".to_string()),
            steel_grade: None,
            width_mm: None,
            thickness_mm: None,
            contract_no: None,
            due_date: None,
            scheduled_date: None,
            scheduled_machine_code: None,
        },
    ];

    // Step 2: 准备候选材料
    let materials = vec![
        (
            create_material_master("M001", 35.0, "Q235", due_date, 10, 7),
            create_material_state("M001", SchedState::Ready, UrgentLevel::L1, RushLevel::L0, "H032", 0, 10),
        ),
        (
            create_material_master("M002", 30.0, "Q345", due_date, 8, 5),
            create_material_state("M002", SchedState::Ready, UrgentLevel::L1, RushLevel::L0, "H032", 0, 8),
        ),
        (
            create_material_master("M003", 25.0, "Q235", due_date, 6, 3),
            create_material_state("M003", SchedState::Ready, UrgentLevel::L1, RushLevel::L0, "H032", 0, 6),
        ),
    ];

    // Step 3: 排序
    let sorted_materials = priority_sorter.sort(materials.clone());

    // Step 4: 填充产能池 (含冻结区)
    let mut capacity_pool = create_capacity_pool("H032", plan_date, 100.0, 150.0);
    let (plan_items, _rejected) = capacity_filler.fill_single_day(
        &mut capacity_pool,
        sorted_materials,
        frozen_items.clone(),
        "test_v1",
    );

    // 验证冻结区材料优先
    assert!(plan_items.len() >= 4, "应该包含冻结材料+候选材料");
    assert_eq!(plan_items[0].material_id, "F001", "冻结材料应该排在第一位");
    // 验证source_type: 通过is_frozen()方法
    assert!(plan_items[0].is_frozen(), "冻结材料的is_frozen应该返回true");

    // Step 5: 风险评估
    let mut all_materials: Vec<MaterialState> = materials.iter().map(|(_, state)| state.clone()).collect();
    // 添加冻结材料的状态
    all_materials.push(create_material_state("F001", SchedState::Locked, UrgentLevel::L1, RushLevel::L0, "H032", 0, 1));

    let mut material_weights: HashMap<String, f64> = materials
        .iter()
        .map(|(master, _)| (master.material_id.clone(), master.weight_t.unwrap()))
        .collect();
    material_weights.insert("F001".to_string(), 40.0);

    let risk_snapshot = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        plan_date,
        &capacity_pool,
        &plan_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    // 验证风险评估包含冻结区影响
    assert!(risk_snapshot.used_capacity_t >= 40.0, "已用产能应该包含冻结区");

    println!("✅ 冻结区排产流程测试通过");
    println!("   - 冻结材料: F001 排在第1位");
    println!("   - 总排入数: {}", plan_items.len());
    println!("   - 已用产能: {:.1}吨", capacity_pool.used_capacity_t);
}

// ==========================================
// 测试3: 完整流程 - 高负荷场景
// ==========================================
#[test]
fn test_e2e_scheduling_high_load_scenario() {
    let priority_sorter = PrioritySorter::new();
    let capacity_filler = CapacityFiller::new();
    let risk_engine = RiskEngine::new();

    let plan_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let due_date = NaiveDate::from_ymd_opt(2026, 1, 22).unwrap();

    // Step 1: 准备大量材料 (总吨位超过产能)
    let materials: Vec<_> = (1..=10)
        .map(|i| {
            let material_id = format!("M{:03}", i);
            let urgent_level = if i <= 3 {
                UrgentLevel::L3
            } else if i <= 6 {
                UrgentLevel::L2
            } else {
                UrgentLevel::L1
            };
            (
                create_material_master(&material_id, 20.0, "Q235", due_date, 10, 5),
                create_material_state(&material_id, SchedState::Ready, urgent_level, RushLevel::L0, "H032", 0, 10),
            )
        })
        .collect();

    // Step 2: 排序
    let sorted_materials = priority_sorter.sort(materials.clone());

    // 验证L3材料排在最前
    assert_eq!(sorted_materials[0].1.urgent_level, UrgentLevel::L3);
    assert_eq!(sorted_materials[1].1.urgent_level, UrgentLevel::L3);
    assert_eq!(sorted_materials[2].1.urgent_level, UrgentLevel::L3);

    // Step 3: 填充产能池 (100吨目标,120吨上限)
    let mut capacity_pool = create_capacity_pool("H032", plan_date, 100.0, 120.0);
    let (plan_items, rejected) = capacity_filler.fill_single_day(
        &mut capacity_pool,
        sorted_materials,
        vec![],
        "test_v1",
    );

    // 验证填充结果
    assert!(plan_items.len() <= 6, "应该只排入不超过6个材料(120吨上限)");
    assert!(rejected.len() >= 4, "应该有至少4个材料被拒绝");
    assert!(capacity_pool.used_capacity_t <= capacity_pool.limit_capacity_t + 0.01, "不应超过上限(非locked)");

    // Step 4: 风险评估 (高负荷场景)
    let all_materials: Vec<MaterialState> = materials.iter().map(|(_, state)| state.clone()).collect();
    let material_weights: HashMap<String, f64> = materials
        .iter()
        .map(|(master, _)| (master.material_id.clone(), master.weight_t.unwrap()))
        .collect();

    let risk_snapshot = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        plan_date,
        &capacity_pool,
        &plan_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    // 验证高负荷风险
    assert!(
        risk_snapshot.risk_level >= RiskLevel::Yellow,
        "高负荷场景应该触发至少YELLOW风险,实际: {:?}",
        risk_snapshot.risk_level
    );
    assert!(risk_snapshot.l3_count >= 3, "应该统计到至少3个L3材料");

    println!("✅ 高负荷排产流程测试通过");
    println!("   - 排入材料: {} / 10", plan_items.len());
    println!("   - 拒绝材料: {}", rejected.len());
    println!("   - 已用产能: {:.1}吨 / {:.1}吨上限", capacity_pool.used_capacity_t, capacity_pool.limit_capacity_t);
    println!("   - 风险等级: {:?}", risk_snapshot.risk_level);
    println!("   - L3红线数: {}", risk_snapshot.l3_count);
}

// ==========================================
// 测试4: 完整流程 - 含影响分析
// ==========================================
#[test]
fn test_e2e_scheduling_with_impact_analysis() {
    let priority_sorter = PrioritySorter::new();
    let capacity_filler = CapacityFiller::new();
    let risk_engine = RiskEngine::new();
    let impact_engine = ImpactSummaryEngine::new();

    let plan_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let due_date = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();

    // Step 1: 准备初始排产 (before)
    let materials_before = vec![
        (
            create_material_master("M001", 40.0, "Q235", due_date, 10, 7),
            create_material_state("M001", SchedState::Ready, UrgentLevel::L1, RushLevel::L0, "H032", 0, 10),
        ),
        (
            create_material_master("M002", 40.0, "Q345", due_date, 8, 5),
            create_material_state("M002", SchedState::Ready, UrgentLevel::L2, RushLevel::L0, "H032", 0, 8),
        ),
    ];

    let sorted_before = priority_sorter.sort(materials_before.clone());

    let mut pool_before = create_capacity_pool("H032", plan_date, 100.0, 120.0);
    let (plan_items_before, _) = capacity_filler.fill_single_day(
        &mut pool_before,
        sorted_before,
        vec![],
        "test_v1",
    );

    let all_materials_before: Vec<MaterialState> = materials_before.iter().map(|(_, s)| s.clone()).collect();
    let material_weights: HashMap<String, f64> = materials_before
        .iter()
        .map(|(m, _)| (m.material_id.clone(), m.weight_t.unwrap()))
        .collect();

    let risk_before = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        plan_date,
        &pool_before,
        &plan_items_before,
        &all_materials_before,
        &material_weights,
        None,
        0.1,
    );

    // Step 2: 准备调整后排产 (after) - 增加一个L3紧急材料
    let mut materials_after = materials_before.clone();
    materials_after.push((
        create_material_master("M003", 30.0, "Q235", due_date, 2, 1),
        create_material_state("M003", SchedState::Ready, UrgentLevel::L3, RushLevel::L1, "H032", 0, 2),
    ));

    let sorted_after = priority_sorter.sort(materials_after.clone());

    let mut pool_after = create_capacity_pool("H032", plan_date, 100.0, 120.0);
    let (plan_items_after, _) = capacity_filler.fill_single_day(
        &mut pool_after,
        sorted_after,
        vec![],
        "test_v1",
    );

    let all_materials_after: Vec<MaterialState> = materials_after.iter().map(|(_, s)| s.clone()).collect();
    let mut material_weights_after = material_weights.clone();
    material_weights_after.insert("M003".to_string(), 30.0);

    let risk_after = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        plan_date,
        &pool_after,
        &plan_items_after,
        &all_materials_after,
        &material_weights_after,
        None,
        0.1,
    );

    // Step 3: 生成影响摘要
    let impact_summary = impact_engine.generate_impact(
        &plan_items_before,
        &plan_items_after,
        &[pool_before.clone()],
        &[pool_after.clone()],
        &[risk_before.clone()],
        &[risk_after.clone()],
        &all_materials_after,
        &material_weights_after,
    );

    // 验证影响分析
    assert_eq!(impact_summary.added_count, 1, "应该新增1个材料");
    assert!(
        impact_summary.capacity_delta_t > 0.0,
        "产能变化应该为正值"
    );
    assert!(impact_summary.urgent_material_affected >= 1, "应该影响紧急材料");
    assert!(impact_summary.l3_critical_count >= 1, "应该有L3材料受影响");

    println!("✅ 含影响分析的排产流程测试通过");
    println!("   - 新增材料数: {}", impact_summary.added_count);
    println!("   - 产能变化: {:.1}吨", impact_summary.capacity_delta_t);
    println!("   - 风险变化: {} → {}", impact_summary.risk_level_before, impact_summary.risk_level_after);
    println!("   - L3红线影响: {}", impact_summary.l3_critical_count);
}

// ==========================================
// 测试5: 多日排产流程
// ==========================================
#[test]
fn test_e2e_multi_day_scheduling() {
    let priority_sorter = PrioritySorter::new();
    let capacity_filler = CapacityFiller::new();
    let risk_engine = RiskEngine::new();

    let base_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let due_date = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();

    // 准备材料 (10个材料,需要分配到2天)
    let materials: Vec<_> = (1..=10)
        .map(|i| {
            let material_id = format!("M{:03}", i);
            (
                create_material_master(&material_id, 25.0, "Q235", due_date, 10, 5),
                create_material_state(&material_id, SchedState::Ready, UrgentLevel::L1, RushLevel::L0, "H032", 0, 10),
            )
        })
        .collect();

    let sorted_materials = priority_sorter.sort(materials.clone());

    // Day 1: 排产第一天
    let mut pool_day1 = create_capacity_pool("H032", base_date, 100.0, 120.0);
    let (plan_items_day1, remaining_day1) = capacity_filler.fill_single_day(
        &mut pool_day1,
        sorted_materials.clone(),
        vec![],
        "test_v1",
    );

    assert!(plan_items_day1.len() >= 4, "第1天应该排入至少4个材料");
    assert!(pool_day1.used_capacity_t >= 100.0, "第1天应该达到目标产能");

    // Day 2: 排产第二天 (使用第1天剩余材料)
    let date_day2 = base_date + chrono::Duration::days(1);
    let mut pool_day2 = create_capacity_pool("H032", date_day2, 100.0, 120.0);

    // 从剩余材料中提取
    let remaining_materials: Vec<_> = remaining_day1.into_iter().map(|(m, s, _)| (m, s)).collect();

    let (plan_items_day2, _remaining_day2) = capacity_filler.fill_single_day(
        &mut pool_day2,
        remaining_materials,
        vec![],
        "test_v1",
    );

    // 验证多日排产
    let total_scheduled = plan_items_day1.len() + plan_items_day2.len();
    assert!(total_scheduled >= 8, "2天应该排入至少8个材料");

    // 生成每日风险快照
    let all_materials: Vec<MaterialState> = materials.iter().map(|(_, s)| s.clone()).collect();
    let material_weights: HashMap<String, f64> = materials
        .iter()
        .map(|(m, _)| (m.material_id.clone(), m.weight_t.unwrap()))
        .collect();

    let risk_day1 = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        base_date,
        &pool_day1,
        &plan_items_day1,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    let risk_day2 = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        date_day2,
        &pool_day2,
        &plan_items_day2,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    println!("✅ 多日排产流程测试通过");
    println!("   - Day1: {}个材料, {:.1}吨, 风险={:?}",
        plan_items_day1.len(), pool_day1.used_capacity_t, risk_day1.risk_level);
    println!("   - Day2: {}个材料, {:.1}吨, 风险={:?}",
        plan_items_day2.len(), pool_day2.used_capacity_t, risk_day2.risk_level);
    println!("   - 总计: {}个材料排产完成", total_scheduled);
}

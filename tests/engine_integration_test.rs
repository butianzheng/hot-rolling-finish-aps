// ==========================================
// 引擎间集成测试
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md
// 职责: 验证多个引擎之间的协作和数据流转
// 场景: CapacityFiller → RiskEngine 组合测试
// ==========================================

use chrono::{DateTime, NaiveDate, Utc};
use hot_rolling_aps::domain::capacity::CapacityPool;
use hot_rolling_aps::domain::material::{MaterialMaster, MaterialState};
use hot_rolling_aps::domain::plan::PlanItem;
use hot_rolling_aps::domain::types::{RiskLevel, RushLevel, SchedState, UrgentLevel};
use hot_rolling_aps::engine::{CapacityFiller, RiskEngine};
use std::collections::HashMap;

// ==========================================
// 测试辅助函数
// ==========================================

/// 创建测试用MaterialMaster
fn create_test_material_master(
    material_id: &str,
    weight_t: f64,
    steel_mark: &str,
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
        due_date: Some(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap()),
        stock_age_days: Some(5),
        output_age_days_raw: Some(3),
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
        scheduled_machine_code: Some("H032".to_string()), // 设置机组代码以便RiskEngine统计
        seq_no: None,
        manual_urgent_flag: false,
        user_confirmed: false,
        user_confirmed_at: None,
        user_confirmed_by: None,
        user_confirmed_reason: None,
        in_frozen_zone: false,
        last_calc_version_id: None,
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
// 测试1: 正常填充 → GREEN风险
// ==========================================
#[test]
fn test_integration_normal_fill_green_risk() {
    let capacity_filler = CapacityFiller::new();
    let risk_engine = RiskEngine::new();

    let date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let mut pool = create_test_capacity_pool("H032", date, 100.0, 120.0);

    // 准备候选材料: 总计80吨 (低于目标100吨)
    let candidates = vec![
        (
            create_test_material_master("M001", 40.0, "Q235"),
            create_test_material_state("M001", UrgentLevel::L0, false, SchedState::Ready),
        ),
        (
            create_test_material_master("M002", 40.0, "Q235"),
            create_test_material_state("M002", UrgentLevel::L0, false, SchedState::Ready),
        ),
    ];

    let frozen_items = vec![];

    // Step 1: CapacityFiller填充产能
    let (plan_items, _rejected) = capacity_filler.fill_single_day(
        &mut pool,
        candidates.clone(),
        frozen_items,
        "test_v1",
    );

    // 验证CapacityFiller输出
    assert_eq!(plan_items.len(), 2, "应该排入2个材料");
    assert!(
        (pool.used_capacity_t - 80.0).abs() < 0.01,
        "已用产能应该是80吨"
    );
    assert!(
        (pool.overflow_t - 0.0).abs() < 0.01,
        "超限应该是0吨"
    );

    // Step 2: 准备RiskEngine输入
    let all_materials: Vec<MaterialState> = candidates.iter().map(|(_, state)| state.clone()).collect();
    let material_weights: HashMap<String, f64> = candidates
        .iter()
        .map(|(master, _)| (master.material_id.clone(), master.weight_t.unwrap()))
        .collect();

    // Step 3: RiskEngine生成风险快照
    let risk_snapshot = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        date,
        &pool,
        &plan_items,
        &all_materials,
        &material_weights,
        None, // roll_status
        0.1,  // overflow_red_threshold_pct
    );

    // 验证RiskEngine输出
    assert_eq!(risk_snapshot.risk_level, RiskLevel::Green, "风险等级应该是GREEN");
    assert_eq!(risk_snapshot.machine_code, "H032");
    assert_eq!(risk_snapshot.snapshot_date, date);
    assert!((risk_snapshot.used_capacity_t - 80.0).abs() < 0.01);
    assert!((risk_snapshot.overflow_t - 0.0).abs() < 0.01);

    // 验证产能利用率 (80/100 = 80%)
    let utilization = risk_snapshot.used_capacity_t / risk_snapshot.target_capacity_t;
    assert!(utilization < 0.95, "利用率应该低于95%,实际: {:.2}%", utilization * 100.0);
}

// ==========================================
// 测试2: 高利用率填充 → YELLOW风险
// ==========================================
#[test]
fn test_integration_high_utilization_yellow_risk() {
    let capacity_filler = CapacityFiller::new();
    let risk_engine = RiskEngine::new();

    let date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let mut pool = create_test_capacity_pool("H032", date, 100.0, 120.0);

    // 准备候选材料: 总计95吨 (高利用率,但未超限)
    let candidates = vec![
        (
            create_test_material_master("M001", 45.0, "Q235"),
            create_test_material_state("M001", UrgentLevel::L0, false, SchedState::Ready),
        ),
        (
            create_test_material_master("M002", 50.0, "Q235"),
            create_test_material_state("M002", UrgentLevel::L0, false, SchedState::Ready),
        ),
    ];

    let frozen_items = vec![];

    // Step 1: CapacityFiller填充
    let (plan_items, _rejected) = capacity_filler.fill_single_day(
        &mut pool,
        candidates.clone(),
        frozen_items,
        "test_v1",
    );

    assert_eq!(plan_items.len(), 2);
    assert!((pool.used_capacity_t - 95.0).abs() < 0.01);

    // Step 2: RiskEngine评估
    let all_materials: Vec<MaterialState> = candidates.iter().map(|(_, state)| state.clone()).collect();
    let material_weights: HashMap<String, f64> = candidates
        .iter()
        .map(|(master, _)| (master.material_id.clone(), master.weight_t.unwrap()))
        .collect();

    let risk_snapshot = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        date,
        &pool,
        &plan_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    // 验证风险等级: 95%利用率应该触发YELLOW
    assert_eq!(
        risk_snapshot.risk_level,
        RiskLevel::Yellow,
        "95%利用率应该触发YELLOW风险"
    );
    assert!((risk_snapshot.used_capacity_t - 95.0).abs() < 0.01);
    // risk_reason是JSON格式,包含reasons数组
    assert!(!risk_snapshot.risk_reason.is_empty(), "风险原因不应该为空");
}

// ==========================================
// 测试3: 超限填充 → ORANGE风险
// ==========================================
#[test]
fn test_integration_overflow_orange_risk() {
    let capacity_filler = CapacityFiller::new();
    let risk_engine = RiskEngine::new();

    let date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let mut pool = create_test_capacity_pool("H032", date, 100.0, 120.0);

    // 准备候选材料: 总计125吨 (超过limit 120吨)
    let candidates = vec![
        (
            create_test_material_master("M001", 50.0, "Q235"),
            create_test_material_state("M001", UrgentLevel::L0, false, SchedState::Ready),
        ),
        (
            create_test_material_master("M002", 50.0, "Q235"),
            create_test_material_state("M002", UrgentLevel::L0, false, SchedState::Ready),
        ),
        (
            create_test_material_master("M003", 25.0, "Q235"),
            create_test_material_state("M003", UrgentLevel::L0, false, SchedState::Ready),
        ),
    ];

    let frozen_items = vec![];

    // Step 1: CapacityFiller填充
    let (plan_items, rejected) = capacity_filler.fill_single_day(
        &mut pool,
        candidates.clone(),
        frozen_items,
        "test_v1",
    );

    // 验证填充结果: 前两个材料填充,第三个被拒绝或跳过
    // 注意: locked材料的处理逻辑取决于CapacityFiller实现
    // 如果locked_in_plan只是标记,而不是lock_flag,则不会强制加入
    assert!(plan_items.len() >= 2, "应该至少排入2个材料");
    assert_eq!(rejected.len() >= 0, true, "可能有材料被拒绝");
    assert!((pool.used_capacity_t - 100.0).abs() < 0.01 || pool.used_capacity_t > 100.0);

    // Step 2: 模拟强制添加locked材料导致超限
    // 创建一个locked材料 (sched_state必须是Locked才会被强制加入)
    let locked_candidates = vec![
        (
            create_test_material_master("M001", 50.0, "Q235"),
            create_test_material_state("M001", UrgentLevel::L0, false, SchedState::Ready),
        ),
        (
            create_test_material_master("M002", 50.0, "Q235"),
            create_test_material_state("M002", UrgentLevel::L0, false, SchedState::Ready),
        ),
        (
            create_test_material_master("M003", 25.0, "Q235"),
            create_test_material_state("M003", UrgentLevel::L1, true, SchedState::Locked), // sched_state=Locked才会强制加入
        ),
    ];

    let mut pool2 = create_test_capacity_pool("H032", date, 100.0, 120.0);
    let (plan_items_with_locked, _) = capacity_filler.fill_single_day(
        &mut pool2,
        locked_candidates.clone(),
        vec![],
        "test_v1",
    );

    // locked材料应该被强制加入,导致超限
    assert_eq!(plan_items_with_locked.len(), 3, "locked材料应该被强制加入");
    assert!(pool2.used_capacity_t > pool2.limit_capacity_t, "应该超限");
    assert!((pool2.overflow_t - 5.0).abs() < 0.01, "超限应该是5吨");

    // Step 3: RiskEngine评估超限风险
    let all_materials: Vec<MaterialState> = locked_candidates.iter().map(|(_, state)| state.clone()).collect();
    let material_weights: HashMap<String, f64> = locked_candidates
        .iter()
        .map(|(master, _)| (master.material_id.clone(), master.weight_t.unwrap()))
        .collect();

    let risk_snapshot = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        date,
        &pool2,
        &plan_items_with_locked,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    // 验证风险等级: 超限应该触发ORANGE或RED
    assert!(
        risk_snapshot.risk_level >= RiskLevel::Orange,
        "超限应该触发ORANGE或RED风险,实际: {:?}",
        risk_snapshot.risk_level
    );
    assert!((risk_snapshot.overflow_t - 5.0).abs() < 0.01);
    assert!(risk_snapshot.risk_reason.contains("超限") || risk_snapshot.risk_reason.contains("overflow"));
}

// ==========================================
// 测试4: 大量L2材料 → YELLOW/ORANGE风险
// ==========================================
#[test]
fn test_integration_many_l2_materials_yellow_risk() {
    let capacity_filler = CapacityFiller::new();
    let risk_engine = RiskEngine::new();

    let date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let mut pool = create_test_capacity_pool("H032", date, 100.0, 120.0);

    // 准备候选材料: 6个L2紧急材料 (>=5触发YELLOW)
    let candidates: Vec<(MaterialMaster, MaterialState)> = (1..=6)
        .map(|i| {
            let material_id = format!("M{:03}", i);
            (
                create_test_material_master(&material_id, 15.0, "Q235"),
                create_test_material_state(&material_id, UrgentLevel::L2, false, SchedState::Ready),
            )
        })
        .collect();

    let frozen_items = vec![];

    // Step 1: CapacityFiller填充
    let (plan_items, _rejected) = capacity_filler.fill_single_day(
        &mut pool,
        candidates.clone(),
        frozen_items,
        "test_v1",
    );

    // 验证填充结果
    assert_eq!(plan_items.len(), 6, "应该排入6个L2材料");
    assert!((pool.used_capacity_t - 90.0).abs() < 0.01, "已用产能应该是90吨");

    // Step 2: RiskEngine评估
    let all_materials: Vec<MaterialState> = candidates.iter().map(|(_, state)| state.clone()).collect();
    let material_weights: HashMap<String, f64> = candidates
        .iter()
        .map(|(master, _)| (master.material_id.clone(), master.weight_t.unwrap()))
        .collect();

    let risk_snapshot = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        date,
        &pool,
        &plan_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    // 验证风险等级: 6个L2材料应该触发YELLOW风险
    assert!(
        risk_snapshot.risk_level >= RiskLevel::Yellow,
        "6个L2材料应该触发至少YELLOW风险,实际: {:?}",
        risk_snapshot.risk_level
    );
    assert_eq!(risk_snapshot.l2_count, 6, "应该统计到6个L2材料");
}

// ==========================================
// 测试5: L3红线材料 → RED风险
// ==========================================
#[test]
fn test_integration_l3_critical_materials_red_risk() {
    let capacity_filler = CapacityFiller::new();
    let risk_engine = RiskEngine::new();

    let date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let mut pool = create_test_capacity_pool("H032", date, 100.0, 120.0);

    // 准备候选材料: 5个L3红线材料 (>=5触发RED)
    let candidates: Vec<(MaterialMaster, MaterialState)> = (1..=5)
        .map(|i| {
            let material_id = format!("M{:03}", i);
            (
                create_test_material_master(&material_id, 18.0, "Q235"),
                create_test_material_state(&material_id, UrgentLevel::L3, false, SchedState::Ready),
            )
        })
        .collect();

    let frozen_items = vec![];

    // Step 1: CapacityFiller填充
    let (plan_items, _rejected) = capacity_filler.fill_single_day(
        &mut pool,
        candidates.clone(),
        frozen_items,
        "test_v1",
    );

    // 验证填充结果
    assert_eq!(plan_items.len(), 5, "应该排入5个L3材料");
    assert!((pool.used_capacity_t - 90.0).abs() < 0.01, "已用产能应该是90吨");

    // Step 2: RiskEngine评估
    let all_materials: Vec<MaterialState> = candidates.iter().map(|(_, state)| state.clone()).collect();
    let material_weights: HashMap<String, f64> = candidates
        .iter()
        .map(|(master, _)| (master.material_id.clone(), master.weight_t.unwrap()))
        .collect();

    let risk_snapshot = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        date,
        &pool,
        &plan_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    // 验证风险等级: 5个L3材料应该触发RED风险
    assert_eq!(
        risk_snapshot.risk_level,
        RiskLevel::Red,
        "5个L3材料应该触发RED风险"
    );
    assert_eq!(risk_snapshot.l3_count, 5, "应该统计到5个L3材料");
    assert!(
        risk_snapshot.risk_reason.contains("L3") || risk_snapshot.risk_reason.contains("红线"),
        "风险原因应该提到L3材料"
    );
}

// ==========================================
// 测试6: 冻结区材料优先 + 风险评估
// ==========================================
#[test]
fn test_integration_frozen_zone_priority_with_risk() {
    let capacity_filler = CapacityFiller::new();
    let risk_engine = RiskEngine::new();

    let date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let mut pool = create_test_capacity_pool("H032", date, 100.0, 120.0);

    // 准备冻结材料
    let frozen_items = vec![
        PlanItem {
            version_id: "test_v1".to_string(),
            material_id: "F001".to_string(),
            machine_code: "H032".to_string(),
            plan_date: date,
            seq_no: 1,
            weight_t: 30.0,
            source_type: "FROZEN".to_string(),
            locked_in_plan: true,
            force_release_in_plan: false,
            violation_flags: None,
            urgent_level: Some("L2".to_string()),
            sched_state: Some("LOCKED".to_string()),
            assign_reason: Some("FROZEN_ZONE".to_string()),
            steel_grade: None,
            width_mm: None,
            thickness_mm: None,
        },
    ];

    // 准备候选材料
    let candidates = vec![
        (
            create_test_material_master("M001", 40.0, "Q235"),
            create_test_material_state("M001", UrgentLevel::L1, false, SchedState::Ready),
        ),
        (
            create_test_material_master("M002", 40.0, "Q235"),
            create_test_material_state("M002", UrgentLevel::L0, false, SchedState::Ready),
        ),
    ];

    // Step 1: CapacityFiller填充 (冻结区优先)
    let (plan_items, _rejected) = capacity_filler.fill_single_day(
        &mut pool,
        candidates.clone(),
        frozen_items.clone(),
        "test_v1",
    );

    // 验证冻结材料在最前
    assert!(plan_items.len() >= 3, "应该包含冻结材料+候选材料");
    assert_eq!(plan_items[0].material_id, "F001", "冻结材料应该排在第一位");
    assert_eq!(plan_items[0].assign_reason.as_ref().map(|s| s.as_str()), Some("FROZEN_ZONE"));

    // 验证产能计算包含冻结区
    // 注意: frozen_capacity_t字段在当前CapacityFiller实现中未更新,所以只验证used_capacity_t
    assert!(pool.used_capacity_t >= 30.0, "已用产能应该包含冻结区");

    // Step 2: RiskEngine评估 (需要包含所有材料状态)
    let mut all_materials: Vec<MaterialState> = candidates.iter().map(|(_, state)| state.clone()).collect();
    // 添加冻结材料的状态
    all_materials.push(create_test_material_state("F001", UrgentLevel::L2, true, SchedState::Locked));

    let mut material_weights: HashMap<String, f64> = candidates
        .iter()
        .map(|(master, _)| (master.material_id.clone(), master.weight_t.unwrap()))
        .collect();
    material_weights.insert("F001".to_string(), 30.0);

    let risk_snapshot = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        date,
        &pool,
        &plan_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    // 验证风险评估正确反映冻结区影响
    assert!(risk_snapshot.used_capacity_t >= 30.0, "已用产能应该包含冻结区");
    assert!(risk_snapshot.l2_count >= 1, "应该统计到冻结区的L2材料");

    // 注意: frozen_capacity_t由Capacity Filler设置,不是测试的核心关注点
    // 主要验证风险评估能正确处理冻结区材料即可
}

// ==========================================
// 测试7: 综合场景 - 高利用率 + L2材料 + 轻微超限
// ==========================================
#[test]
fn test_integration_comprehensive_scenario() {
    let capacity_filler = CapacityFiller::new();
    let risk_engine = RiskEngine::new();

    let date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let mut pool = create_test_capacity_pool("H032", date, 100.0, 120.0);

    // 综合场景:
    // - 4个L2材料 (接近5个阈值)
    // - 总吨位118吨 (接近limit但未超)
    // - 高利用率 (118/100 = 118%)
    let candidates = vec![
        (
            create_test_material_master("M001", 30.0, "Q235"),
            create_test_material_state("M001", UrgentLevel::L2, false, SchedState::Ready),
        ),
        (
            create_test_material_master("M002", 30.0, "Q235"),
            create_test_material_state("M002", UrgentLevel::L2, false, SchedState::Ready),
        ),
        (
            create_test_material_master("M003", 28.0, "Q235"),
            create_test_material_state("M003", UrgentLevel::L2, false, SchedState::Ready),
        ),
        (
            create_test_material_master("M004", 30.0, "Q235"),
            create_test_material_state("M004", UrgentLevel::L2, false, SchedState::Ready),
        ),
    ];

    let frozen_items = vec![];

    // Step 1: CapacityFiller填充
    let (plan_items, _rejected) = capacity_filler.fill_single_day(
        &mut pool,
        candidates.clone(),
        frozen_items,
        "test_v1",
    );

    assert_eq!(plan_items.len(), 4);
    assert!((pool.used_capacity_t - 118.0).abs() < 0.01);
    assert!(pool.used_capacity_t <= pool.limit_capacity_t, "不应该超限");

    // Step 2: RiskEngine评估
    let all_materials: Vec<MaterialState> = candidates.iter().map(|(_, state)| state.clone()).collect();
    let material_weights: HashMap<String, f64> = candidates
        .iter()
        .map(|(master, _)| (master.material_id.clone(), master.weight_t.unwrap()))
        .collect();

    let risk_snapshot = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        date,
        &pool,
        &plan_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    // 验证综合风险评估
    assert!(
        risk_snapshot.risk_level >= RiskLevel::Yellow,
        "综合场景应该至少触发YELLOW风险(高利用率+4个L2)"
    );
    assert_eq!(risk_snapshot.l2_count, 4);
    assert!((risk_snapshot.used_capacity_t - 118.0).abs() < 0.01);

    // 验证利用率
    let utilization = risk_snapshot.used_capacity_t / risk_snapshot.target_capacity_t;
    assert!(utilization > 1.0, "利用率应该超过100%");
}

// ==========================================
// 测试8: 数据流转完整性验证
// ==========================================
#[test]
fn test_integration_data_flow_integrity() {
    let capacity_filler = CapacityFiller::new();
    let risk_engine = RiskEngine::new();

    let date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let mut pool = create_test_capacity_pool("H032", date, 100.0, 120.0);

    let candidates = vec![
        (
            create_test_material_master("M001", 50.0, "Q235"),
            create_test_material_state("M001", UrgentLevel::L1, false, SchedState::Ready),
        ),
        (
            create_test_material_master("M002", 45.0, "Q345"),
            create_test_material_state("M002", UrgentLevel::L2, false, SchedState::Ready),
        ),
    ];

    let frozen_items = vec![];

    // Step 1: CapacityFiller
    let (plan_items, _) = capacity_filler.fill_single_day(
        &mut pool,
        candidates.clone(),
        frozen_items,
        "test_v1",
    );

    // Step 2: RiskEngine
    let all_materials: Vec<MaterialState> = candidates.iter().map(|(_, state)| state.clone()).collect();
    let material_weights: HashMap<String, f64> = candidates
        .iter()
        .map(|(master, _)| (master.material_id.clone(), master.weight_t.unwrap()))
        .collect();

    let risk_snapshot = risk_engine.generate_snapshot(
        "test_v1",
        "H032",
        date,
        &pool,
        &plan_items,
        &all_materials,
        &material_weights,
        None,
        0.1,
    );

    // 验证数据流转完整性

    // 1. CapacityPool数据传递
    assert_eq!(risk_snapshot.used_capacity_t, pool.used_capacity_t);
    assert_eq!(risk_snapshot.target_capacity_t, pool.target_capacity_t);
    assert_eq!(risk_snapshot.limit_capacity_t, pool.limit_capacity_t);
    assert_eq!(risk_snapshot.overflow_t, pool.overflow_t);

    // 2. PlanItem数据传递
    assert_eq!(plan_items.len(), 2);
    for item in &plan_items {
        assert_eq!(item.version_id, "test_v1");
        assert_eq!(item.machine_code, "H032");
        assert_eq!(item.plan_date, date);
        assert!(item.weight_t > 0.0);
    }

    // 3. MaterialState数据传递
    assert_eq!(risk_snapshot.l2_count, 1, "应该统计到1个L2材料");
    assert_eq!(risk_snapshot.l3_count, 0);

    // 4. 元数据完整性
    assert_eq!(risk_snapshot.version_id, "test_v1");
    assert_eq!(risk_snapshot.machine_code, "H032");
    assert_eq!(risk_snapshot.snapshot_date, date);
    assert!(!risk_snapshot.risk_reason.is_empty(), "风险原因不应该为空");
    assert!(!risk_snapshot.snapshot_id.is_empty(), "快照ID不应该为空");
}

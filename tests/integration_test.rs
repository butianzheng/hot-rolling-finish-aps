// ==========================================
// 引擎集成测试
// ==========================================
// 依据: docs/integration_test_plan.md
// 测试范围: Eligibility → Urgency → Priority → Capacity Filler → Structure
// ==========================================

mod helpers;

use chrono::NaiveDate;
use helpers::mock_config::MockConfig;
use helpers::test_data_builder::*;
use hot_rolling_aps::domain::types::{SchedState, UrgentLevel};
use hot_rolling_aps::engine::ScheduleOrchestrator;
use std::collections::HashMap;
use std::sync::Arc;

// ==========================================
// 场景1: 正常排产流程（基准场景）
// ==========================================

#[tokio::test]
async fn test_scenario_1_normal_schedule() {
    // 创建配置
    let config = Arc::new(MockConfig::default());
    let orchestrator = ScheduleOrchestrator::new(config);

    // 构建测试数据
    let today = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    let materials = vec![
        MaterialBuilder::new("M001")
            .steel_mark("Q345")
            .weight(200.0)
            .output_age_days(5)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 2, 15).unwrap()) // 修改为26天后,避免触发N2
            .build(),
        MaterialBuilder::new("M002")
            .steel_mark("Q345")
            .weight(180.0)
            .output_age_days(4)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 2, 5).unwrap()) // 修改为16天后,避免触发N2(7天)
            .build(),
        MaterialBuilder::new("M003")
            .steel_mark("Q235")
            .weight(150.0)
            .output_age_days(6)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 2, 10).unwrap()) // 修改为21天后,避免触发N2
            .build(),
        MaterialBuilder::new("M004")
            .steel_mark("Q235")
            .weight(120.0)
            .output_age_days(3)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 2, 20).unwrap()) // 修改为31天后,避免触发N2
            .build(),
        MaterialBuilder::new("M005")
            .steel_mark("Q390")
            .weight(100.0)
            .output_age_days(7)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 2, 25).unwrap()) // 修改为36天后,避免触发N2
            .build(),
    ];

    let states = vec![
        MaterialStateBuilder::new("M001")
            .stock_age_days(10)
            .build(),
        MaterialStateBuilder::new("M002")
            .stock_age_days(8)
            .build(),
        MaterialStateBuilder::new("M003")
            .stock_age_days(12)
            .build(),
        MaterialStateBuilder::new("M004")
            .stock_age_days(5)
            .build(),
        MaterialStateBuilder::new("M005")
            .stock_age_days(15)
            .build(),
    ];

    let mut capacity_pool = create_capacity_pool("H032", today, 800.0, 900.0, 0.0);

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    // 执行排产
    let result = orchestrator
        .execute_single_day_schedule(
            materials,
            states,
            &mut capacity_pool,
            vec![],
            &target_ratio,
            0.15,
            today,
            "v1",
        )
        .await
        .unwrap();

    // 断言
    assert_eq!(result.eligible_materials.len(), 5, "所有材料应该适温");
    assert_eq!(result.blocked_materials.len(), 0, "没有材料被阻断");
    assert_eq!(result.plan_items.len(), 5, "所有材料应该被排产");
    assert_eq!(
        result.updated_capacity_pool.used_capacity_t, 750.0,
        "总重量应为750t"
    );
    assert!(
        !result.structure_report.is_violated,
        "结构配比应该达标"
    );

    // 验证紧急等级
    for (_, (level, _)) in &result.urgent_levels {
        assert_eq!(*level, UrgentLevel::L0, "所有材料应为L0等级");
    }

    // 验证排序（按库存天数降序）
    assert_eq!(result.sorted_materials[0].0.material_id, "M005"); // 15天
    assert_eq!(result.sorted_materials[1].0.material_id, "M003"); // 12天
    assert_eq!(result.sorted_materials[2].0.material_id, "M001"); // 10天
}

// ==========================================
// 场景2: 冻结区保护场景
// ==========================================

#[tokio::test]
async fn test_scenario_2_frozen_zone_protection() {
    let config = Arc::new(MockConfig::default());
    let orchestrator = ScheduleOrchestrator::new(config);

    let today = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    // 冻结区材料
    let frozen_items = vec![
        PlanItemBuilder::new("v1", "F001", "H032", today)
            .seq_no(1)
            .weight(200.0)
            .frozen()
            .build(),
        PlanItemBuilder::new("v1", "F002", "H032", today)
            .seq_no(2)
            .weight(100.0)
            .frozen()
            .build(),
    ];

    // 候选材料
    let materials = vec![
        MaterialBuilder::new("M001")
            .steel_mark("Q345")
            .weight(250.0)
            .output_age_days(5)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap())
            .build(),
        MaterialBuilder::new("M002")
            .steel_mark("Q235")
            .weight(200.0)
            .output_age_days(4)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 26).unwrap())
            .build(),
        MaterialBuilder::new("M003")
            .steel_mark("Q390")
            .weight(150.0)
            .output_age_days(6)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 27).unwrap())
            .build(),
    ];

    let states = vec![
        MaterialStateBuilder::new("M001")
            .stock_age_days(10)
            .build(),
        MaterialStateBuilder::new("M002")
            .stock_age_days(8)
            .build(),
        MaterialStateBuilder::new("M003")
            .stock_age_days(12)
            .build(),
    ];

    let mut capacity_pool = create_capacity_pool("H032", today, 800.0, 900.0, 300.0);

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    // 执行排产
    let result = orchestrator
        .execute_single_day_schedule(
            materials,
            states,
            &mut capacity_pool,
            frozen_items,
            &target_ratio,
            0.15,
            today,
            "v1",
        )
        .await
        .unwrap();

    // 断言
    assert_eq!(result.plan_items.len(), 5, "应有5个排产项（2冻结+3计算）");
    assert_eq!(
        result.plan_items[0].material_id, "F001",
        "冻结材料应保持序号1"
    );
    assert_eq!(
        result.plan_items[1].material_id, "F002",
        "冻结材料应保持序号2"
    );
    assert!(result.plan_items[0].is_frozen(), "F001应标记为冻结");
    assert!(result.plan_items[1].is_frozen(), "F002应标记为冻结");
    assert_eq!(result.skipped_materials.len(), 0, "所有候选材料都应被排产（总重600t≤剩余600t）");
}

// ==========================================
// 场景3: 非适温材料阻断场景
// ==========================================

#[tokio::test]
async fn test_scenario_3_immature_material_blocked() {
    let config = Arc::new(MockConfig::winter()); // 冬季 min_temp_days=3
    let orchestrator = ScheduleOrchestrator::new(config);

    let today = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    let materials = vec![
        MaterialBuilder::new("M001")
            .steel_mark("Q345")
            .weight(200.0)
            .output_age_days(5) // 适温
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap())
            .build(),
        MaterialBuilder::new("M002")
            .steel_mark("Q345")
            .weight(180.0)
            .output_age_days(2) // 冷料 (ready_in_days=1)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 26).unwrap())
            .build(),
        MaterialBuilder::new("M003")
            .steel_mark("Q235")
            .weight(150.0)
            .output_age_days(1) // 冷料 (ready_in_days=2)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 27).unwrap())
            .build(),
        MaterialBuilder::new("M004")
            .steel_mark("Q235")
            .weight(120.0)
            .output_age_days(4) // 适温
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 28).unwrap())
            .build(),
        MaterialBuilder::new("M005")
            .steel_mark("Q390")
            .weight(100.0)
            .output_age_days(0) // 冷料 (ready_in_days=3)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 30).unwrap())
            .build(),
    ];

    let states = vec![
        MaterialStateBuilder::new("M001")
            .stock_age_days(10)
            .build(),
        MaterialStateBuilder::new("M002")
            .stock_age_days(8)
            .build(),
        MaterialStateBuilder::new("M003")
            .stock_age_days(12)
            .build(),
        MaterialStateBuilder::new("M004")
            .stock_age_days(5)
            .build(),
        MaterialStateBuilder::new("M005")
            .stock_age_days(15)
            .build(),
    ];

    let mut capacity_pool = create_capacity_pool("H032", today, 800.0, 900.0, 0.0);

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    // 执行排产
    let result = orchestrator
        .execute_single_day_schedule(
            materials,
            states,
            &mut capacity_pool,
            vec![],
            &target_ratio,
            0.15,
            today,
            "v1",
        )
        .await
        .unwrap();

    // 断言
    assert_eq!(result.eligible_materials.len(), 2, "只有2个材料适温");
    assert_eq!(result.blocked_materials.len(), 3, "3个材料被阻断（冷料）");
    assert_eq!(result.plan_items.len(), 2, "只排产2个适温材料");
    assert_eq!(
        result.updated_capacity_pool.used_capacity_t, 320.0,
        "总重量应为320t"
    );

    // 验证阻断材料
    let blocked_ids: Vec<&str> = result
        .blocked_materials
        .iter()
        .map(|(m, _, _)| m.material_id.as_str())
        .collect();
    assert!(blocked_ids.contains(&"M002"));
    assert!(blocked_ids.contains(&"M003"));
    assert!(blocked_ids.contains(&"M005"));
}

// ==========================================
// 场景4: 产能满载场景
// ==========================================

#[tokio::test]
async fn test_scenario_4_capacity_full() {
    let config = Arc::new(MockConfig::default());
    let orchestrator = ScheduleOrchestrator::new(config);

    let today = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    let materials = vec![
        MaterialBuilder::new("M001")
            .steel_mark("Q345")
            .weight(200.0)
            .output_age_days(5)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap())
            .build(),
        MaterialBuilder::new("M002")
            .steel_mark("Q345")
            .weight(180.0)
            .output_age_days(4)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 26).unwrap())
            .build(),
        MaterialBuilder::new("M003")
            .steel_mark("Q235")
            .weight(150.0)
            .output_age_days(6)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 27).unwrap())
            .build(),
        MaterialBuilder::new("M004")
            .steel_mark("Q235")
            .weight(200.0)
            .output_age_days(3)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 28).unwrap())
            .build(),
        MaterialBuilder::new("M005")
            .steel_mark("Q345")
            .weight(220.0)
            .output_age_days(7)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 30).unwrap())
            .build(),
        MaterialBuilder::new("M006")
            .steel_mark("Q235")
            .weight(150.0)
            .output_age_days(5)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 2, 1).unwrap())
            .build(),
    ];

    let states = vec![
        MaterialStateBuilder::new("M001")
            .stock_age_days(20)
            .build(),
        MaterialStateBuilder::new("M002")
            .stock_age_days(18)
            .build(),
        MaterialStateBuilder::new("M003")
            .stock_age_days(15)
            .build(),
        MaterialStateBuilder::new("M004")
            .stock_age_days(12)
            .build(),
        MaterialStateBuilder::new("M005")
            .stock_age_days(10)
            .build(),
        MaterialStateBuilder::new("M006")
            .stock_age_days(8)
            .build(),
    ];

    let mut capacity_pool = create_capacity_pool("H032", today, 800.0, 900.0, 0.0);

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    // 执行排产
    let result = orchestrator
        .execute_single_day_schedule(
            materials,
            states,
            &mut capacity_pool,
            vec![],
            &target_ratio,
            0.15,
            today,
            "v1",
        )
        .await
        .unwrap();

    // 断言
    assert!(
        result.updated_capacity_pool.used_capacity_t >= 880.0
            && result.updated_capacity_pool.used_capacity_t <= 900.0,
        "应填充至接近limit (880-900t)，实际: {}t",
        result.updated_capacity_pool.used_capacity_t
    );
    assert!(
        result.skipped_materials.len() >= 1,
        "至少有1个材料被跳过"
    );

    // 验证跳过原因
    for (_, _, reason) in &result.skipped_materials {
        assert!(
            reason.contains("CAPACITY_LIMIT_EXCEEDED"),
            "跳过原因应为产能超限"
        );
    }
}

// ==========================================
// 场景5: 锁定材料冲突场景
// ==========================================

#[tokio::test]
async fn test_scenario_5_locked_material_conflict() {
    let config = Arc::new(MockConfig::default());
    let orchestrator = ScheduleOrchestrator::new(config);

    let today = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    let materials = vec![
        MaterialBuilder::new("M001")
            .steel_mark("Q345")
            .weight(300.0)
            .output_age_days(5)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap())
            .build(),
        MaterialBuilder::new("M002")
            .steel_mark("Q345")
            .weight(280.0)
            .output_age_days(4)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 26).unwrap())
            .build(),
        MaterialBuilder::new("M003")
            .steel_mark("Q235")
            .weight(250.0)
            .output_age_days(6)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 27).unwrap())
            .build(),
        MaterialBuilder::new("M004")
            .steel_mark("Q235")
            .weight(150.0)
            .output_age_days(3)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 28).unwrap())
            .build(),
        MaterialBuilder::new("M005")
            .steel_mark("Q390")
            .weight(100.0)
            .output_age_days(7)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 30).unwrap())
            .build(),
    ];

    let states = vec![
        MaterialStateBuilder::new("M001")
            .locked()
            .stock_age_days(20)
            .build(),
        MaterialStateBuilder::new("M002")
            .locked()
            .stock_age_days(18)
            .build(),
        MaterialStateBuilder::new("M003")
            .locked()
            .stock_age_days(15)
            .build(),
        MaterialStateBuilder::new("M004")
            .stock_age_days(12)
            .build(),
        MaterialStateBuilder::new("M005")
            .stock_age_days(10)
            .build(),
    ];

    let mut capacity_pool = create_capacity_pool("H032", today, 800.0, 900.0, 0.0);

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    // 执行排产
    let result = orchestrator
        .execute_single_day_schedule(
            materials,
            states,
            &mut capacity_pool,
            vec![],
            &target_ratio,
            0.15,
            today,
            "v1",
        )
        .await
        .unwrap();

    // 断言
    assert_eq!(result.plan_items.len(), 3, "应排产3个锁定材料");
    assert_eq!(
        result.updated_capacity_pool.used_capacity_t, 830.0,
        "锁定材料总重830t"
    );

    // 验证所有排产的材料都是锁定状态
    for item in &result.plan_items {
        let state = result
            .eligible_materials
            .iter()
            .find(|(m, _)| m.material_id == item.material_id)
            .map(|(_, s)| s)
            .unwrap();
        assert_eq!(
            state.sched_state,
            SchedState::Locked,
            "排产的材料应为锁定状态"
        );
    }

    // 注意: Locked 和 in_frozen_zone 是不同概念
    // Locked: 材料被锁定,必须排产
    // in_frozen_zone: 材料在冻结区,不可调整
    // 根据业务规范,只有 in_frozen_zone=true 才会自动抬升到 L2
    // 因此不验证 Locked 材料的紧急等级
}

// ==========================================
// 场景6: 结构违规场景
// ==========================================

#[tokio::test]
async fn test_scenario_6_structure_violation() {
    let config = Arc::new(MockConfig::default());
    let orchestrator = ScheduleOrchestrator::new(config);

    let today = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    let materials = vec![
        MaterialBuilder::new("M001")
            .steel_mark("Q345")
            .weight(350.0)
            .output_age_days(5)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap())
            .build(),
        MaterialBuilder::new("M002")
            .steel_mark("Q345")
            .weight(300.0)
            .output_age_days(4)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 26).unwrap())
            .build(),
        MaterialBuilder::new("M003")
            .steel_mark("Q235")
            .weight(100.0)
            .output_age_days(6)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 27).unwrap())
            .build(),
        MaterialBuilder::new("M004")
            .steel_mark("Q235")
            .weight(50.0)
            .output_age_days(3)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 28).unwrap())
            .build(),
    ];

    let states = vec![
        MaterialStateBuilder::new("M001")
            .stock_age_days(20)
            .build(),
        MaterialStateBuilder::new("M002")
            .stock_age_days(18)
            .build(),
        MaterialStateBuilder::new("M003")
            .stock_age_days(15)
            .build(),
        MaterialStateBuilder::new("M004")
            .stock_age_days(12)
            .build(),
    ];

    let mut capacity_pool = create_capacity_pool("H032", today, 800.0, 900.0, 0.0);

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    // 执行排产
    let result = orchestrator
        .execute_single_day_schedule(
            materials,
            states,
            &mut capacity_pool,
            vec![],
            &target_ratio,
            0.15,
            today,
            "v1",
        )
        .await
        .unwrap();

    // 断言
    assert_eq!(result.plan_items.len(), 4, "应排产4个材料");
    assert_eq!(
        result.updated_capacity_pool.used_capacity_t, 800.0,
        "总重量应为800t"
    );

    // 验证结构违规
    assert!(
        result.structure_report.is_violated,
        "应检测到结构违规"
    );
    assert!(
        result.structure_report.deviation_ratio > 0.15,
        "偏差应超过阈值15%"
    );

    // 验证违规描述
    assert!(result.structure_report.violation_desc.is_some());
    let desc = result.structure_report.violation_desc.as_ref().unwrap();
    assert!(desc.contains("Q345"), "违规描述应包含Q345");
    assert!(desc.contains("Q235"), "违规描述应包含Q235");

    // 验证建议
    assert!(
        result
            .structure_report
            .suggestions
            .iter()
            .any(|s| s.contains("延后") && s.contains("Q345")),
        "应建议延后Q345"
    );
    assert!(
        result
            .structure_report
            .suggestions
            .iter()
            .any(|s| s.contains("补充") && s.contains("Q235")),
        "应建议补充Q235"
    );
}

// ==========================================
// 场景7: 紧急等级抬升场景
// ==========================================

#[tokio::test]
async fn test_scenario_7_urgent_level_escalation() {
    let config = Arc::new(MockConfig::with_n1_n2(3, 7));
    let orchestrator = ScheduleOrchestrator::new(config);

    let today = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    let materials = vec![
        MaterialBuilder::new("M001")
            .steel_mark("Q345")
            .weight(200.0)
            .output_age_days(5)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 19).unwrap()) // 超期
            .build(),
        MaterialBuilder::new("M002")
            .steel_mark("Q345")
            .weight(180.0)
            .output_age_days(4)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 22).unwrap()) // 临期N1
            .build(),
        MaterialBuilder::new("M003")
            .steel_mark("Q235")
            .weight(150.0)
            .output_age_days(6)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 26).unwrap()) // 临期N2
            .build(),
        MaterialBuilder::new("M004")
            .steel_mark("Q235")
            .weight(120.0)
            .output_age_days(3)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 30).unwrap()) // 正常
            .build(),
        MaterialBuilder::new("M005")
            .steel_mark("Q390")
            .weight(100.0)
            .output_age_days(7)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 2, 5).unwrap()) // 正常
            .build(),
    ];

    let states = vec![
        MaterialStateBuilder::new("M001")
            .stock_age_days(20)
            .build(),
        MaterialStateBuilder::new("M002")
            .stock_age_days(18)
            .build(),
        MaterialStateBuilder::new("M003")
            .stock_age_days(15)
            .build(),
        MaterialStateBuilder::new("M004")
            .stock_age_days(12)
            .build(),
        MaterialStateBuilder::new("M005")
            .stock_age_days(10)
            .build(),
    ];

    let mut capacity_pool = create_capacity_pool("H032", today, 800.0, 900.0, 0.0);

    let mut target_ratio = HashMap::new();
    target_ratio.insert("Q345".to_string(), 0.6);
    target_ratio.insert("Q235".to_string(), 0.4);

    // 执行排产
    let result = orchestrator
        .execute_single_day_schedule(
            materials,
            states,
            &mut capacity_pool,
            vec![],
            &target_ratio,
            0.15,
            today,
            "v1",
        )
        .await
        .unwrap();

    // 断言紧急等级
    assert_eq!(
        result.urgent_levels.get("M001").unwrap().0,
        UrgentLevel::L3,
        "M001应为L3（超期）"
    );
    assert_eq!(
        result.urgent_levels.get("M002").unwrap().0,
        UrgentLevel::L2,
        "M002应为L2（临期N1）"
    );
    assert_eq!(
        result.urgent_levels.get("M003").unwrap().0,
        UrgentLevel::L1,
        "M003应为L1（临期N2）"
    );
    assert_eq!(
        result.urgent_levels.get("M004").unwrap().0,
        UrgentLevel::L0,
        "M004应为L0（正常）"
    );
    assert_eq!(
        result.urgent_levels.get("M005").unwrap().0,
        UrgentLevel::L0,
        "M005应为L0（正常）"
    );

    // 验证排序（L3 > L2 > L1 > L0）
    assert_eq!(
        result.sorted_materials[0].0.material_id, "M001",
        "L3应排第一"
    );
    assert_eq!(
        result.sorted_materials[1].0.material_id, "M002",
        "L2应排第二"
    );
    assert_eq!(
        result.sorted_materials[2].0.material_id, "M003",
        "L1应排第三"
    );
}

// ==========================================
// 场景8: 多机组并行排产场景
// ==========================================

#[tokio::test]
async fn test_scenario_8_multi_machine_parallel() {
    let config = Arc::new(MockConfig::default());
    let orchestrator = ScheduleOrchestrator::new(config);

    let today = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    // H032 机组材料
    let materials_h032 = vec![
        MaterialBuilder::new("M001")
            .steel_mark("Q345")
            .weight(200.0)
            .output_age_days(5)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap())
            .build(),
        MaterialBuilder::new("M002")
            .steel_mark("Q235")
            .weight(150.0)
            .output_age_days(4)
            .machine("H032")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 26).unwrap())
            .build(),
    ];

    let states_h032 = vec![
        MaterialStateBuilder::new("M001")
            .stock_age_days(10)
            .build(),
        MaterialStateBuilder::new("M002")
            .stock_age_days(8)
            .build(),
    ];

    let mut capacity_pool_h032 = create_capacity_pool("H032", today, 800.0, 900.0, 0.0);

    // H033 机组材料
    let materials_h033 = vec![
        MaterialBuilder::new("M003")
            .steel_mark("Q345")
            .weight(300.0)
            .output_age_days(6)
            .machine("H033")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap())
            .build(),
        MaterialBuilder::new("M004")
            .steel_mark("Q390")
            .weight(250.0)
            .output_age_days(5)
            .machine("H033")
            .due_date(NaiveDate::from_ymd_opt(2026, 1, 26).unwrap())
            .build(),
    ];

    let states_h033 = vec![
        MaterialStateBuilder::new("M003")
            .stock_age_days(12)
            .build(),
        MaterialStateBuilder::new("M004")
            .stock_age_days(10)
            .build(),
    ];

    let mut capacity_pool_h033 = create_capacity_pool("H033", today, 1000.0, 1100.0, 0.0);

    let mut target_ratio_h032 = HashMap::new();
    target_ratio_h032.insert("Q345".to_string(), 0.6);
    target_ratio_h032.insert("Q235".to_string(), 0.4);

    let mut target_ratio_h033 = HashMap::new();
    target_ratio_h033.insert("Q345".to_string(), 0.5);
    target_ratio_h033.insert("Q390".to_string(), 0.5);

    // 执行 H032 排产
    let result_h032 = orchestrator
        .execute_single_day_schedule(
            materials_h032,
            states_h032,
            &mut capacity_pool_h032,
            vec![],
            &target_ratio_h032,
            0.15,
            today,
            "v1",
        )
        .await
        .unwrap();

    // 执行 H033 排产
    let result_h033 = orchestrator
        .execute_single_day_schedule(
            materials_h033,
            states_h033,
            &mut capacity_pool_h033,
            vec![],
            &target_ratio_h033,
            0.15,
            today,
            "v1",
        )
        .await
        .unwrap();

    // 断言 H032
    assert_eq!(result_h032.plan_items.len(), 2, "H032应排产2个材料");
    assert!(
        result_h032
            .plan_items
            .iter()
            .all(|item| item.machine_code == "H032"),
        "H032的排产项应属于H032"
    );

    // 断言 H033
    assert_eq!(result_h033.plan_items.len(), 2, "H033应排产2个材料");
    assert!(
        result_h033
            .plan_items
            .iter()
            .all(|item| item.machine_code == "H033"),
        "H033的排产项应属于H033"
    );

    // 验证独立性
    assert_eq!(
        result_h032.updated_capacity_pool.machine_code, "H032",
        "H032产能池应保持独立"
    );
    assert_eq!(
        result_h033.updated_capacity_pool.machine_code, "H033",
        "H033产能池应保持独立"
    );
}

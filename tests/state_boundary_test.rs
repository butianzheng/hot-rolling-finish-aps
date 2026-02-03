// ==========================================
// 状态边界测试
// ==========================================
// 测试范围:
// 1. MaterialState 修改后正确持久化
// 2. Orchestrator 更新的 urgent_level/rush_level 被保存
// 3. 验证 Master Spec 的状态边界规则
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - State Boundary Rules
// - material_state 是唯一事实层
// - 每个决策必须可追踪
// ==========================================

mod test_helpers;

use chrono::{NaiveDate, Utc};
use hot_rolling_aps::domain::material::{MaterialMaster, MaterialState};
use hot_rolling_aps::domain::types::{RushLevel, SchedState, UrgentLevel};
use hot_rolling_aps::repository::{MaterialMasterRepository, MaterialStateRepository};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

// ==========================================
// 测试辅助函数
// ==========================================

/// 创建测试数据库并返回连接
fn setup_test_db() -> (tempfile::NamedTempFile, String, Arc<Mutex<Connection>>) {
    let (temp_file, db_path) = test_helpers::create_test_db().expect("创建测试数据库失败");
    let conn = test_helpers::open_test_connection(&db_path).expect("打开数据库失败");
    test_helpers::insert_test_config(&conn).expect("插入配置失败");

    (temp_file, db_path, Arc::new(Mutex::new(conn)))
}

/// 创建测试材料主数据
fn create_test_material_master(material_id: &str) -> MaterialMaster {
    MaterialMaster {
        material_id: material_id.to_string(),
        manufacturing_order_id: Some("MO001".to_string()),
        material_status_code_src: Some("READY".to_string()),
        steel_mark: Some("Q235".to_string()),
        slab_id: Some("SLAB001".to_string()),
        next_machine_code: Some("H032".to_string()),
        rework_machine_code: None,
        current_machine_code: Some("H032".to_string()),
        width_mm: Some(1500.0),
        thickness_mm: Some(3.0),
        length_m: Some(50.0),
        weight_t: Some(2.5),
        available_width_mm: Some(1480.0),
        due_date: Some(NaiveDate::from_ymd_opt(2026, 2, 15).unwrap()),
        stock_age_days: Some(10),
        output_age_days_raw: Some(5),
        status_updated_at: None,
        contract_no: Some("CONTRACT001".to_string()),
        contract_nature: Some("A".to_string()),
        weekly_delivery_flag: Some("Y".to_string()),
        export_flag: Some("N".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// 创建测试材料状态（初始状态，无紧急等级）
fn create_test_material_state(material_id: &str) -> MaterialState {
    MaterialState {
        material_id: material_id.to_string(),
        sched_state: SchedState::Ready,
        lock_flag: false,
        force_release_flag: false,
        urgent_level: UrgentLevel::L0, // 初始为 L0
        urgent_reason: None,           // 初始无原因
        rush_level: RushLevel::L0,     // 初始无催料
        rolling_output_age_days: 5,
        ready_in_days: 0,
        earliest_sched_date: Some(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap()),
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
        updated_by: Some("test".to_string()),
    }
}

// ==========================================
// 测试用例
// ==========================================

/// 测试: MaterialState 批量插入后可以正确读取
#[test]
fn test_material_state_persistence_roundtrip() {
    let (_temp_file, db_path, _conn) = setup_test_db();

    // 创建 Repository
    let state_repo =
        MaterialStateRepository::new(&db_path).expect("创建 MaterialStateRepository 失败");
    let master_repo =
        MaterialMasterRepository::new(&db_path).expect("创建 MaterialMasterRepository 失败");

    // 准备测试数据
    let material = create_test_material_master("MAT001");
    let state = create_test_material_state("MAT001");

    // 插入材料主数据
    master_repo
        .batch_insert_material_master(vec![material])
        .expect("插入材料主数据失败");

    // 插入初始状态
    state_repo
        .batch_insert_material_state(vec![state.clone()])
        .expect("插入材料状态失败");

    // 读取并验证
    let loaded_state = state_repo
        .find_by_id("MAT001")
        .expect("读取材料状态失败")
        .expect("材料状态不存在");

    assert_eq!(loaded_state.material_id, "MAT001");
    assert_eq!(loaded_state.urgent_level, UrgentLevel::L0);
    assert!(loaded_state.urgent_reason.is_none());
    assert_eq!(loaded_state.rush_level, RushLevel::L0);
}

/// 测试: MaterialState 更新后 urgent_level/rush_level 被正确保存
#[test]
fn test_material_state_urgency_update_persistence() {
    let (_temp_file, db_path, _conn) = setup_test_db();

    // 创建 Repository
    let state_repo =
        MaterialStateRepository::new(&db_path).expect("创建 MaterialStateRepository 失败");
    let master_repo =
        MaterialMasterRepository::new(&db_path).expect("创建 MaterialMasterRepository 失败");

    // 准备测试数据
    let material = create_test_material_master("MAT002");
    let mut state = create_test_material_state("MAT002");

    // 插入材料主数据
    master_repo
        .batch_insert_material_master(vec![material])
        .expect("插入材料主数据失败");

    // 插入初始状态
    state_repo
        .batch_insert_material_state(vec![state.clone()])
        .expect("插入材料状态失败");

    // 模拟 Orchestrator 更新紧急等级
    state.urgent_level = UrgentLevel::L2;
    state.urgent_reason = Some("交期临近 N1 窗口".to_string());
    state.rush_level = RushLevel::L1;
    state.updated_at = Utc::now();

    // 使用 batch_insert 更新（INSERT OR REPLACE）
    state_repo
        .batch_insert_material_state(vec![state.clone()])
        .expect("更新材料状态失败");

    // 读取并验证更新后的状态
    let loaded_state = state_repo
        .find_by_id("MAT002")
        .expect("读取材料状态失败")
        .expect("材料状态不存在");

    // 验证关键字段被正确持久化
    assert_eq!(
        loaded_state.urgent_level,
        UrgentLevel::L2,
        "urgent_level 应该被更新为 L2"
    );
    assert_eq!(
        loaded_state.urgent_reason,
        Some("交期临近 N1 窗口".to_string()),
        "urgent_reason 应该被持久化"
    );
    assert_eq!(
        loaded_state.rush_level,
        RushLevel::L1,
        "rush_level 应该被更新为 L1"
    );
}

/// 测试: 批量更新多个材料状态
#[test]
fn test_batch_material_state_update() {
    let (_temp_file, db_path, _conn) = setup_test_db();

    // 创建 Repository
    let state_repo =
        MaterialStateRepository::new(&db_path).expect("创建 MaterialStateRepository 失败");
    let master_repo =
        MaterialMasterRepository::new(&db_path).expect("创建 MaterialMasterRepository 失败");

    // 准备多个测试材料
    let materials: Vec<MaterialMaster> = (1..=5)
        .map(|i| create_test_material_master(&format!("MAT10{}", i)))
        .collect();

    let states: Vec<MaterialState> = (1..=5)
        .map(|i| create_test_material_state(&format!("MAT10{}", i)))
        .collect();

    // 插入材料主数据和初始状态
    master_repo
        .batch_insert_material_master(materials)
        .expect("批量插入材料主数据失败");
    state_repo
        .batch_insert_material_state(states.clone())
        .expect("批量插入材料状态失败");

    // 模拟 Orchestrator 批量更新
    let updated_states: Vec<MaterialState> = states
        .into_iter()
        .enumerate()
        .map(|(i, mut s)| {
            // 根据索引分配不同的紧急等级
            s.urgent_level = match i {
                0 => UrgentLevel::L3,
                1 => UrgentLevel::L2,
                2 => UrgentLevel::L1,
                _ => UrgentLevel::L0,
            };
            s.urgent_reason = Some(format!("测试原因_{}", i));
            s.rush_level = if i < 2 { RushLevel::L2 } else { RushLevel::L0 };
            s.updated_at = Utc::now();
            s
        })
        .collect();

    // 批量更新
    let updated_count = state_repo
        .batch_insert_material_state(updated_states)
        .expect("批量更新材料状态失败");
    assert_eq!(updated_count, 5, "应该更新 5 条记录");

    // 验证每个材料的状态
    let s1 = state_repo.find_by_id("MAT101").unwrap().unwrap();
    assert_eq!(s1.urgent_level, UrgentLevel::L3);
    assert_eq!(s1.rush_level, RushLevel::L2);

    let s2 = state_repo.find_by_id("MAT102").unwrap().unwrap();
    assert_eq!(s2.urgent_level, UrgentLevel::L2);
    assert_eq!(s2.rush_level, RushLevel::L2);

    let s3 = state_repo.find_by_id("MAT103").unwrap().unwrap();
    assert_eq!(s3.urgent_level, UrgentLevel::L1);
    assert_eq!(s3.rush_level, RushLevel::L0);

    let s4 = state_repo.find_by_id("MAT104").unwrap().unwrap();
    assert_eq!(s4.urgent_level, UrgentLevel::L0);
}

/// 测试: 状态更新不影响其他字段
#[test]
fn test_state_update_preserves_other_fields() {
    let (_temp_file, db_path, _conn) = setup_test_db();

    // 创建 Repository
    let state_repo =
        MaterialStateRepository::new(&db_path).expect("创建 MaterialStateRepository 失败");
    let master_repo =
        MaterialMasterRepository::new(&db_path).expect("创建 MaterialMasterRepository 失败");

    // 准备测试数据，设置特定的字段值
    let material = create_test_material_master("MAT003");
    let mut state = create_test_material_state("MAT003");
    state.lock_flag = true;
    state.manual_urgent_flag = true;
    state.stock_age_days = 25;
    state.rolling_output_age_days = 15;

    // 插入
    master_repo
        .batch_insert_material_master(vec![material])
        .expect("插入材料主数据失败");
    state_repo
        .batch_insert_material_state(vec![state.clone()])
        .expect("插入材料状态失败");

    // 更新紧急等级（模拟 Orchestrator 行为）
    state.urgent_level = UrgentLevel::L3;
    state.urgent_reason = Some("手动标记紧急".to_string());
    state.rush_level = RushLevel::L2;
    state.updated_at = Utc::now();

    state_repo
        .batch_insert_material_state(vec![state])
        .expect("更新材料状态失败");

    // 验证其他字段未受影响
    let loaded = state_repo.find_by_id("MAT003").unwrap().unwrap();

    // 验证更新的字段
    assert_eq!(loaded.urgent_level, UrgentLevel::L3);
    assert_eq!(loaded.rush_level, RushLevel::L2);

    // 验证原有字段保持不变
    assert!(loaded.lock_flag, "lock_flag 应该保持为 true");
    assert!(
        loaded.manual_urgent_flag,
        "manual_urgent_flag 应该保持为 true"
    );
    assert_eq!(loaded.stock_age_days, 25, "stock_age_days 应该保持为 25");
    assert_eq!(
        loaded.rolling_output_age_days, 15,
        "rolling_output_age_days 应该保持为 15"
    );
}

/// 测试: 紧急原因字段可以被正确序列化和反序列化
#[test]
fn test_urgent_reason_serialization() {
    let (_temp_file, db_path, _conn) = setup_test_db();

    let state_repo =
        MaterialStateRepository::new(&db_path).expect("创建 MaterialStateRepository 失败");
    let master_repo =
        MaterialMasterRepository::new(&db_path).expect("创建 MaterialMasterRepository 失败");

    let material = create_test_material_master("MAT004");
    let mut state = create_test_material_state("MAT004");

    // 设置包含中文和特殊字符的原因
    let reason = r#"{"level":"L2","reason":"交期临近","due_date":"2026-02-15","days_to_due":5}"#;
    state.urgent_level = UrgentLevel::L2;
    state.urgent_reason = Some(reason.to_string());

    master_repo
        .batch_insert_material_master(vec![material])
        .expect("插入材料主数据失败");
    state_repo
        .batch_insert_material_state(vec![state])
        .expect("插入材料状态失败");

    let loaded = state_repo.find_by_id("MAT004").unwrap().unwrap();

    assert_eq!(loaded.urgent_reason, Some(reason.to_string()));

    // 验证 JSON 可以被正确解析
    let parsed: serde_json::Value =
        serde_json::from_str(loaded.urgent_reason.as_ref().unwrap()).expect("JSON 解析失败");
    assert_eq!(parsed["level"], "L2");
    assert_eq!(parsed["days_to_due"], 5);
}

/// 测试: 验证 Master Spec 红线 - material_state 是唯一事实层
/// 即：material_state 的修改必须通过 Repository 持久化，不能只在内存中修改
#[test]
fn test_master_spec_state_is_source_of_truth() {
    let (_temp_file, db_path, _conn) = setup_test_db();

    let state_repo =
        MaterialStateRepository::new(&db_path).expect("创建 MaterialStateRepository 失败");
    let master_repo =
        MaterialMasterRepository::new(&db_path).expect("创建 MaterialMasterRepository 失败");

    let material = create_test_material_master("MAT005");
    let state = create_test_material_state("MAT005");

    master_repo
        .batch_insert_material_master(vec![material])
        .expect("插入材料主数据失败");
    state_repo
        .batch_insert_material_state(vec![state.clone()])
        .expect("插入材料状态失败");

    // 模拟"只在内存中修改"的错误做法
    let mut memory_state = state.clone();
    memory_state.urgent_level = UrgentLevel::L3;
    // 注意：这里故意不调用 batch_insert_material_state

    // 从数据库读取 - 应该是原始值
    let db_state = state_repo.find_by_id("MAT005").unwrap().unwrap();

    // 验证数据库中的状态仍然是原始值
    assert_eq!(
        db_state.urgent_level,
        UrgentLevel::L0,
        "数据库中的状态不应该被内存修改影响"
    );

    // 内存中的修改与数据库不同
    assert_ne!(
        memory_state.urgent_level, db_state.urgent_level,
        "内存修改与数据库状态应该不同（未持久化）"
    );

    // 只有持久化后，数据库才会反映修改
    state_repo
        .batch_insert_material_state(vec![memory_state.clone()])
        .expect("持久化材料状态失败");

    let final_db_state = state_repo.find_by_id("MAT005").unwrap().unwrap();
    assert_eq!(
        final_db_state.urgent_level,
        UrgentLevel::L3,
        "持久化后，数据库应该反映修改"
    );
}

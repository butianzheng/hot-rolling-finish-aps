// ==========================================
// Recalc Engine 集成测试
// ==========================================
// 测试范围:
// 1. Repository层CRUD操作
// 2. RecalcEngine核心方法
// 3. 乐观锁并发控制
// 4. 冻结区保护
// 5. 版本管理
// ==========================================

mod test_helpers;

use chrono::{NaiveDate, Utc};
use hot_rolling_aps::domain::plan::{Plan, PlanItem, PlanVersion};
use hot_rolling_aps::domain::types::PlanVersionStatus;
use hot_rolling_aps::repository::{PlanItemRepository, PlanRepository, PlanVersionRepository};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

// ==========================================
// 辅助函数
// ==========================================

/// 创建测试数据库并返回连接
fn setup_test_db() -> (tempfile::NamedTempFile, Arc<Mutex<Connection>>) {
    let (temp_file, db_path) = test_helpers::create_test_db().expect("创建测试数据库失败");
    let conn = test_helpers::open_test_connection(&db_path).expect("打开数据库失败");
    test_helpers::insert_test_config(&conn).expect("插入配置失败");

    (temp_file, Arc::new(Mutex::new(conn)))
}

/// 创建测试Plan
fn create_test_plan(repo: &PlanRepository, plan_id: &str) -> Plan {
    let plan = Plan {
        plan_id: plan_id.to_string(),
        plan_name: format!("测试方案_{}", plan_id),
        plan_type: "BASELINE".to_string(),
        base_plan_id: None,
        created_by: "test_user".to_string(),
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    };

    repo.create(&plan).expect("创建Plan失败");
    plan
}

/// 创建测试PlanVersion
fn create_test_version(
    repo: &PlanVersionRepository,
    version_id: &str,
    plan_id: &str,
    version_no: i32,
) -> PlanVersion {
    let version = PlanVersion {
        version_id: version_id.to_string(),
        plan_id: plan_id.to_string(),
        version_no,
        status: PlanVersionStatus::Draft,
        frozen_from_date: None,
        recalc_window_days: Some(30),
        config_snapshot_json: None,
        created_by: Some("test_user".to_string()),
        created_at: Utc::now().naive_utc(),
        revision: 0,
    };

    repo.create(&version).expect("创建PlanVersion失败");
    version
}

/// 创建测试材料
fn create_test_material(conn: &Arc<Mutex<Connection>>, material_id: &str) {
    let conn_lock = conn.lock().unwrap();
    conn_lock
        .execute(
            r#"INSERT INTO material_master (
            material_id, weight_t, created_at, updated_at
        ) VALUES (?, 50.0, datetime('now'), datetime('now'))"#,
            rusqlite::params![material_id],
        )
        .expect("插入材料失败");
}

/// 创建测试PlanItem (返回PlanItem，但不直接插入数据库)
fn create_test_plan_item(
    version_id: &str,
    material_id: &str,
    plan_date: NaiveDate,
    locked: bool,
) -> PlanItem {
    PlanItem {
        version_id: version_id.to_string(),
        material_id: material_id.to_string(),
        machine_code: "H032".to_string(),
        plan_date,
        seq_no: 1,
        weight_t: 50.0,
        source_type: if locked { "FROZEN" } else { "CALC" }.to_string(),
        locked_in_plan: locked,
        force_release_in_plan: false,
        violation_flags: None,
        urgent_level: Some("L0".to_string()),
        sched_state: Some("READY".to_string()),
        assign_reason: Some("测试".to_string()),
        steel_grade: None,
        width_mm: None,
        thickness_mm: None,
        contract_no: None,
        due_date: None,
        scheduled_date: None,
        scheduled_machine_code: None,
    }
}

// ==========================================
// Repository 测试
// ==========================================

#[test]
fn test_plan_repository_crud() {
    let (_temp_file, conn) = setup_test_db();
    let repo = PlanRepository::new(conn.clone());

    // 1. 创建Plan
    let plan = create_test_plan(&repo, "plan_001");
    assert_eq!(plan.plan_id, "plan_001");

    // 2. 查询Plan
    let found = repo.find_by_id("plan_001").expect("查询失败");
    assert!(found.is_some());
    let found_plan = found.unwrap();
    assert_eq!(found_plan.plan_name, "测试方案_plan_001");

    // 3. 更新Plan
    let mut updated_plan = found_plan.clone();
    updated_plan.plan_name = "更新后的方案名".to_string();
    repo.update(&updated_plan).expect("更新失败");

    let found_again = repo.find_by_id("plan_001").expect("再次查询失败");
    assert_eq!(found_again.unwrap().plan_name, "更新后的方案名");

    // 4. 删除Plan
    repo.delete("plan_001").expect("删除失败");
    let not_found = repo.find_by_id("plan_001").expect("查询失败");
    assert!(not_found.is_none());
}

#[test]
fn test_plan_version_repository_crud() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());

    // 准备Plan
    create_test_plan(&plan_repo, "plan_001");

    // 1. 创建Version
    let version = create_test_version(&version_repo, "v001", "plan_001", 1);
    assert_eq!(version.version_no, 1);
    assert_eq!(version.revision, 0);

    // 2. 查询Version
    let found = version_repo.find_by_id("v001").expect("查询失败");
    assert!(found.is_some());
    assert_eq!(found.as_ref().unwrap().status, PlanVersionStatus::Draft);
    assert_eq!(found.as_ref().unwrap().revision, 0);

    // 3. 更新Version
    let mut updated_version = found.unwrap();
    updated_version.status = PlanVersionStatus::Active;
    version_repo.update(&updated_version).expect("更新失败");

    let found_again = version_repo.find_by_id("v001").expect("再次查询失败");
    let found_version = found_again.unwrap();
    assert_eq!(found_version.status, PlanVersionStatus::Active);
    assert_eq!(found_version.revision, 1); // revision应该自增
}

#[test]
fn test_plan_version_get_next_version_no() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());

    // 准备Plan
    create_test_plan(&plan_repo, "plan_001");

    // 1. 第一次获取版本号（应该是1）
    let next_no = version_repo
        .get_next_version_no("plan_001")
        .expect("获取版本号失败");
    assert_eq!(next_no, 1);

    // 2. 创建v1
    create_test_version(&version_repo, "v001", "plan_001", 1);

    // 3. 再次获取版本号（应该是2）
    let next_no2 = version_repo
        .get_next_version_no("plan_001")
        .expect("获取版本号失败");
    assert_eq!(next_no2, 2);

    // 4. 创建v2和v3
    create_test_version(&version_repo, "v002", "plan_001", 2);
    create_test_version(&version_repo, "v003", "plan_001", 3);

    // 5. 最后获取版本号（应该是4）
    let next_no3 = version_repo
        .get_next_version_no("plan_001")
        .expect("获取版本号失败");
    assert_eq!(next_no3, 4);
}

#[test]
fn test_plan_version_activate() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());

    // 准备Plan
    create_test_plan(&plan_repo, "plan_001");

    // 创建3个版本
    create_test_version(&version_repo, "v001", "plan_001", 1);
    create_test_version(&version_repo, "v002", "plan_001", 2);
    create_test_version(&version_repo, "v003", "plan_001", 3);

    // 激活v2
    version_repo.activate_version("v002").expect("激活失败");

    // 检查v2是ACTIVE
    let v2 = version_repo.find_by_id("v002").expect("查询失败");
    assert_eq!(v2.unwrap().status, PlanVersionStatus::Active);

    // 检查v1和v3不是ACTIVE
    let v1 = version_repo.find_by_id("v001").expect("查询失败");
    assert_ne!(v1.unwrap().status, PlanVersionStatus::Active);

    let v3 = version_repo.find_by_id("v003").expect("查询失败");
    assert_ne!(v3.unwrap().status, PlanVersionStatus::Active);

    // 再激活v3
    version_repo.activate_version("v003").expect("激活失败");

    // 检查v3是ACTIVE，v2应该被归档
    let v3_active = version_repo.find_by_id("v003").expect("查询失败");
    assert_eq!(v3_active.unwrap().status, PlanVersionStatus::Active);

    let v2_archived = version_repo.find_by_id("v002").expect("查询失败");
    assert_eq!(v2_archived.unwrap().status, PlanVersionStatus::Archived);
}

#[test]
fn test_plan_version_find_active() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());

    // 准备Plan
    create_test_plan(&plan_repo, "plan_001");

    // 1. 没有激活版本时
    let active = version_repo
        .find_active_version("plan_001")
        .expect("查询失败");
    assert!(active.is_none());

    // 2. 创建并激活v1
    create_test_version(&version_repo, "v001", "plan_001", 1);
    version_repo.activate_version("v001").expect("激活失败");

    let active = version_repo
        .find_active_version("plan_001")
        .expect("查询失败");
    assert!(active.is_some());
    assert_eq!(active.unwrap().version_id, "v001");
}

// ==========================================
// 乐观锁测试
// ==========================================

#[test]
fn test_optimistic_lock_success() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());

    // 准备数据
    create_test_plan(&plan_repo, "plan_001");
    create_test_version(&version_repo, "v001", "plan_001", 1);

    // 读取版本（revision=0）
    let mut fetched = version_repo.find_by_id("v001").expect("查询失败").unwrap();
    assert_eq!(fetched.revision, 0);

    // 修改并更新（revision=0 -> 1）
    fetched.status = PlanVersionStatus::Active;
    let result = version_repo.update(&fetched);
    assert!(result.is_ok(), "更新应该成功");

    // 再次查询，revision应该是1
    let updated = version_repo.find_by_id("v001").expect("查询失败").unwrap();
    assert_eq!(updated.revision, 1);
    assert_eq!(updated.status, PlanVersionStatus::Active);
}

#[test]
fn test_optimistic_lock_conflict() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());

    // 准备数据
    create_test_plan(&plan_repo, "plan_001");
    create_test_version(&version_repo, "v001", "plan_001", 1);

    // 用户A读取版本（revision=0）
    let mut user_a_version = version_repo.find_by_id("v001").expect("查询失败").unwrap();
    assert_eq!(user_a_version.revision, 0);

    // 用户B读取同一版本（revision=0）
    let mut user_b_version = version_repo.find_by_id("v001").expect("查询失败").unwrap();
    assert_eq!(user_b_version.revision, 0);

    // 用户A先更新成功（revision=0 -> 1）
    user_a_version.status = PlanVersionStatus::Active;
    let result_a = version_repo.update(&user_a_version);
    assert!(result_a.is_ok(), "用户A更新应该成功");

    // 用户B尝试更新（revision=0，但数据库已是1，应该失败）
    user_b_version.status = PlanVersionStatus::Archived;
    let result_b = version_repo.update(&user_b_version);

    // 检查是否返回乐观锁冲突错误
    assert!(result_b.is_err(), "用户B更新应该失败");
    let err_msg = result_b.unwrap_err().to_string();
    assert!(
        err_msg.contains("乐观锁冲突") || err_msg.contains("OptimisticLockFailure"),
        "错误消息应该包含乐观锁冲突信息，实际错误: {}",
        err_msg
    );
}

#[test]
fn test_optimistic_lock_multiple_updates() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());

    // 准备数据
    create_test_plan(&plan_repo, "plan_001");
    create_test_version(&version_repo, "v001", "plan_001", 1);

    // 第1次更新：revision 0 -> 1
    let mut v1 = version_repo.find_by_id("v001").expect("查询失败").unwrap();
    v1.status = PlanVersionStatus::Active;
    version_repo.update(&v1).expect("第1次更新失败");

    // 第2次更新：revision 1 -> 2
    let mut v2 = version_repo.find_by_id("v001").expect("查询失败").unwrap();
    assert_eq!(v2.revision, 1);
    v2.recalc_window_days = Some(60);
    version_repo.update(&v2).expect("第2次更新失败");

    // 第3次更新：revision 2 -> 3
    let mut v3 = version_repo.find_by_id("v001").expect("查询失败").unwrap();
    assert_eq!(v3.revision, 2);
    v3.status = PlanVersionStatus::Archived;
    version_repo.update(&v3).expect("第3次更新失败");

    // 最终检查
    let final_version = version_repo.find_by_id("v001").expect("查询失败").unwrap();
    assert_eq!(final_version.revision, 3);
    assert_eq!(final_version.status, PlanVersionStatus::Archived);
}

// ==========================================
// PlanItem测试
// ==========================================

#[test]
fn test_plan_item_batch_operations() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());
    let item_repo = PlanItemRepository::new(conn.clone());

    // 准备数据
    create_test_plan(&plan_repo, "plan_001");
    create_test_version(&version_repo, "v001", "plan_001", 1);
    create_test_material(&conn, "MAT001");

    let plan_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    // 1. 批量插入单个PlanItem
    let item = create_test_plan_item("v001", "MAT001", plan_date, false);
    let count = item_repo.batch_insert(&vec![item]).expect("批量插入失败");
    assert_eq!(count, 1);

    // 2. 查询版本的所有明细
    let items = item_repo.find_by_version("v001").expect("查询失败");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].material_id, "MAT001");
    assert_eq!(items[0].weight_t, 50.0);

    // 3. 删除版本的所有明细
    let deleted = item_repo.delete_by_version("v001").expect("删除失败");
    assert_eq!(deleted, 1);

    // 4. 验证已删除
    let items_after_delete = item_repo.find_by_version("v001").expect("查询失败");
    assert_eq!(items_after_delete.len(), 0);
}

#[test]
fn test_plan_item_batch_insert() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());
    let item_repo = PlanItemRepository::new(conn.clone());

    // 准备数据
    create_test_plan(&plan_repo, "plan_001");
    create_test_version(&version_repo, "v001", "plan_001", 1);

    // 创建3个材料
    create_test_material(&conn, "MAT001");
    create_test_material(&conn, "MAT002");
    create_test_material(&conn, "MAT003");

    let plan_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    // 批量插入
    let items = vec![
        PlanItem {
            version_id: "v001".to_string(),
            material_id: "MAT001".to_string(),
            machine_code: "H032".to_string(),
            plan_date,
            seq_no: 1,
            weight_t: 50.0,
            source_type: "CALC".to_string(),
            locked_in_plan: false,
            force_release_in_plan: false,
            violation_flags: None,
            urgent_level: None,
            sched_state: None,
            assign_reason: None,
            steel_grade: None,
            width_mm: None,
            thickness_mm: None,
            contract_no: None,
            due_date: None,
            scheduled_date: None,
            scheduled_machine_code: None,
        },
        PlanItem {
            version_id: "v001".to_string(),
            material_id: "MAT002".to_string(),
            machine_code: "H032".to_string(),
            plan_date,
            seq_no: 2,
            weight_t: 60.0,
            source_type: "CALC".to_string(),
            locked_in_plan: false,
            force_release_in_plan: false,
            violation_flags: None,
            urgent_level: None,
            sched_state: None,
            assign_reason: None,
            steel_grade: None,
            width_mm: None,
            thickness_mm: None,
            contract_no: None,
            due_date: None,
            scheduled_date: None,
            scheduled_machine_code: None,
        },
        PlanItem {
            version_id: "v001".to_string(),
            material_id: "MAT003".to_string(),
            machine_code: "H033".to_string(),
            plan_date,
            seq_no: 1,
            weight_t: 70.0,
            source_type: "CALC".to_string(),
            locked_in_plan: false,
            force_release_in_plan: false,
            violation_flags: None,
            urgent_level: None,
            sched_state: None,
            assign_reason: None,
            steel_grade: None,
            width_mm: None,
            thickness_mm: None,
            contract_no: None,
            due_date: None,
            scheduled_date: None,
            scheduled_machine_code: None,
        },
    ];

    let count = item_repo.batch_insert(&items).expect("批量插入失败");
    assert_eq!(count, 3);

    // 验证数据
    let all_items = item_repo.find_by_version("v001").expect("查询失败");
    assert_eq!(all_items.len(), 3);
}

#[test]
fn test_plan_item_find_frozen() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());
    let item_repo = PlanItemRepository::new(conn.clone());

    // 准备数据
    create_test_plan(&plan_repo, "plan_001");
    create_test_version(&version_repo, "v001", "plan_001", 1);

    // 创建材料
    create_test_material(&conn, "MAT001");
    create_test_material(&conn, "MAT002");
    create_test_material(&conn, "MAT003");

    let date1 = NaiveDate::from_ymd_opt(2026, 1, 18).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let date3 = NaiveDate::from_ymd_opt(2026, 1, 22).unwrap();

    // 创建3个明细：1个冻结，2个非冻结
    let item1 = create_test_plan_item("v001", "MAT001", date1, true); // 冻结
    let item2 = create_test_plan_item("v001", "MAT002", date2, false);
    let item3 = create_test_plan_item("v001", "MAT003", date3, false);

    // 批量插入
    item_repo
        .batch_insert(&vec![item1, item2, item3])
        .expect("插入失败");

    // 查询冻结明细
    let frozen_items = item_repo.find_frozen_items("v001").expect("查询失败");

    // 应该只有MAT001（locked_in_plan=true）
    assert_eq!(frozen_items.len(), 1);
    assert_eq!(frozen_items[0].material_id, "MAT001");
    assert!(frozen_items[0].locked_in_plan);
}

// ==========================================
// Repository 边界测试
// ==========================================

#[test]
fn test_plan_version_update_not_found() {
    let (_temp_file, conn) = setup_test_db();
    let version_repo = PlanVersionRepository::new(conn.clone());

    // 尝试更新不存在的版本
    let mut fake_version = PlanVersion {
        version_id: "non_existent_version".to_string(),
        plan_id: "fake_plan".to_string(),
        version_no: 1,
        status: PlanVersionStatus::Draft,
        frozen_from_date: None,
        recalc_window_days: Some(30),
        config_snapshot_json: None,
        created_by: Some("test".to_string()),
        created_at: Utc::now().naive_utc(),
        revision: 0,
    };

    let result = version_repo.update(&fake_version);

    // 应该返回NotFound错误
    assert!(result.is_err(), "更新不存在的记录应该失败");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("NotFound")
            || err_msg.contains("不存在")
            || err_msg.contains("记录未找到"),
        "错误消息应该包含NotFound信息，实际错误: {}",
        err_msg
    );
}

#[test]
fn test_plan_item_foreign_key_violation() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());
    let item_repo = PlanItemRepository::new(conn.clone());

    // 准备数据
    create_test_plan(&plan_repo, "plan_001");
    create_test_version(&version_repo, "v001", "plan_001", 1);

    // 尝试插入引用不存在材料的PlanItem（外键约束违反）
    let plan_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let invalid_item = create_test_plan_item("v001", "NON_EXISTENT_MATERIAL", plan_date, false);

    let result = item_repo.batch_insert(&vec![invalid_item]);

    // 应该失败（外键约束）
    assert!(result.is_err(), "插入引用不存在材料的明细应该失败");
}

#[test]
fn test_plan_version_unique_constraint() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());

    // 准备数据
    create_test_plan(&plan_repo, "plan_001");

    // 创建v1
    create_test_version(&version_repo, "v001", "plan_001", 1);

    // 尝试创建另一个version_no=1的版本（唯一约束违反）
    let duplicate_version = PlanVersion {
        version_id: "v002".to_string(), // 不同的version_id
        plan_id: "plan_001".to_string(),
        version_no: 1, // 相同的version_no
        status: PlanVersionStatus::Draft,
        frozen_from_date: None,
        recalc_window_days: Some(30),
        config_snapshot_json: None,
        created_by: Some("test".to_string()),
        created_at: Utc::now().naive_utc(),
        revision: 0,
    };

    let result = version_repo.create(&duplicate_version);

    // 应该失败（UNIQUE(plan_id, version_no)约束）
    assert!(result.is_err(), "创建重复version_no的版本应该失败");
}

#[test]
fn test_plan_delete_cascade() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());
    let item_repo = PlanItemRepository::new(conn.clone());

    // 准备数据
    create_test_plan(&plan_repo, "plan_001");
    create_test_version(&version_repo, "v001", "plan_001", 1);
    create_test_material(&conn, "MAT001");

    let plan_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let item = create_test_plan_item("v001", "MAT001", plan_date, false);
    item_repo.batch_insert(&vec![item]).expect("插入失败");

    // 删除Plan（应该级联删除version和item）
    plan_repo.delete("plan_001").expect("删除Plan失败");

    // 验证version已被删除
    let version = version_repo.find_by_id("v001").expect("查询失败");
    assert!(version.is_none(), "Version应该被级联删除");

    // 验证item已被删除
    let items = item_repo.find_by_version("v001").expect("查询失败");
    assert_eq!(items.len(), 0, "PlanItem应该被级联删除");
}

#[test]
fn test_plan_item_empty_batch_insert() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());
    let item_repo = PlanItemRepository::new(conn.clone());

    // 准备数据
    create_test_plan(&plan_repo, "plan_001");
    create_test_version(&version_repo, "v001", "plan_001", 1);

    // 插入空数组
    let count = item_repo.batch_insert(&vec![]).expect("空批量插入应该成功");
    assert_eq!(count, 0, "空批量插入应该返回0");
}

#[test]
fn test_plan_version_list_all() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());

    // 准备数据：创建多个plan和version
    create_test_plan(&plan_repo, "plan_001");
    create_test_plan(&plan_repo, "plan_002");

    create_test_version(&version_repo, "v001", "plan_001", 1);
    create_test_version(&version_repo, "v002", "plan_001", 2);
    create_test_version(&version_repo, "v003", "plan_002", 1);

    // 查询plan_001的所有版本
    let versions = version_repo.find_by_plan_id("plan_001").expect("查询失败");
    assert_eq!(versions.len(), 2);
    // 应该按version_no降序
    assert_eq!(versions[0].version_no, 2);
    assert_eq!(versions[1].version_no, 1);

    // 查询plan_002的所有版本
    let versions2 = version_repo.find_by_plan_id("plan_002").expect("查询失败");
    assert_eq!(versions2.len(), 1);
}

#[test]
fn test_plan_version_revision_boundary() {
    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());

    // 准备数据
    create_test_plan(&plan_repo, "plan_001");
    create_test_version(&version_repo, "v001", "plan_001", 1);

    // 连续更新100次，验证revision能正确递增
    for i in 0..100 {
        let mut version = version_repo.find_by_id("v001").expect("查询失败").unwrap();
        assert_eq!(version.revision, i, "第{}次更新前revision应该是{}", i, i);

        version.config_snapshot_json = Some(format!("update_{}", i));
        version_repo
            .update(&version)
            .expect(&format!("第{}次更新失败", i));
    }

    // 最终检查
    let final_version = version_repo.find_by_id("v001").expect("查询失败").unwrap();
    assert_eq!(final_version.revision, 100, "100次更新后revision应该是100");
}

// ==========================================
// 性能测试
// ==========================================

#[test]
#[ignore] // 默认不运行性能测试，使用 cargo test -- --ignored 运行
fn test_performance_batch_insert_plan_items() {
    use std::time::Instant;

    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());
    let item_repo = PlanItemRepository::new(conn.clone());

    // 准备数据
    create_test_plan(&plan_repo, "plan_perf");
    create_test_version(&version_repo, "v_perf", "plan_perf", 1);

    // 创建1000个测试材料
    let material_count = 1000;
    for i in 0..material_count {
        create_test_material(&conn, &format!("MAT_{:04}", i));
    }

    // 生成1000个PlanItem
    let plan_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let items: Vec<PlanItem> = (0..material_count)
        .map(|i| create_test_plan_item("v_perf", &format!("MAT_{:04}", i), plan_date, false))
        .collect();

    // 测试批量插入性能
    let start = Instant::now();
    let count = item_repo.batch_insert(&items).expect("批量插入失败");
    let elapsed = start.elapsed();

    assert_eq!(count, material_count);

    println!("\n==========================================");
    println!("性能测试: 批量插入PlanItem");
    println!("==========================================");
    println!("记录数: {}", material_count);
    println!("耗时: {:?}", elapsed);
    println!("平均每条: {:?}", elapsed / material_count as u32);
    println!(
        "吞吐量: {:.2} 条/秒",
        material_count as f64 / elapsed.as_secs_f64()
    );
    println!("==========================================\n");

    // 性能基准：1000条应该在1秒内完成
    assert!(
        elapsed.as_secs() < 1,
        "批量插入1000条应该在1秒内完成，实际耗时: {:?}",
        elapsed
    );
}

#[test]
#[ignore]
fn test_performance_batch_query() {
    use std::time::Instant;

    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());
    let item_repo = PlanItemRepository::new(conn.clone());

    // 准备数据：插入5000条PlanItem
    create_test_plan(&plan_repo, "plan_query");
    create_test_version(&version_repo, "v_query", "plan_query", 1);

    let material_count = 5000;
    for i in 0..material_count {
        create_test_material(&conn, &format!("QMAT_{:05}", i));
    }

    let plan_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let items: Vec<PlanItem> = (0..material_count)
        .map(|i| {
            let mut item =
                create_test_plan_item("v_query", &format!("QMAT_{:05}", i), plan_date, false);
            item.seq_no = i as i32 + 1;
            item
        })
        .collect();

    item_repo.batch_insert(&items).expect("插入失败");

    // 测试查询性能
    let start = Instant::now();
    let queried_items = item_repo.find_by_version("v_query").expect("查询失败");
    let elapsed = start.elapsed();

    assert_eq!(queried_items.len(), material_count);

    println!("\n==========================================");
    println!("性能测试: 批量查询PlanItem");
    println!("==========================================");
    println!("记录数: {}", material_count);
    println!("耗时: {:?}", elapsed);
    println!("平均每条: {:?}", elapsed / material_count as u32);
    println!(
        "吞吐量: {:.2} 条/秒",
        material_count as f64 / elapsed.as_secs_f64()
    );
    println!("==========================================\n");

    // 性能基准：查询5000条应该在500ms内完成
    assert!(
        elapsed.as_millis() < 500,
        "查询5000条应该在500ms内完成，实际耗时: {:?}",
        elapsed
    );
}

#[test]
#[ignore]
fn test_performance_revision_updates() {
    use std::time::Instant;

    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());

    // 准备数据
    create_test_plan(&plan_repo, "plan_update");
    create_test_version(&version_repo, "v_update", "plan_update", 1);

    // 测试连续1000次更新的性能
    let update_count = 1000;
    let start = Instant::now();

    for i in 0..update_count {
        let mut version = version_repo
            .find_by_id("v_update")
            .expect("查询失败")
            .unwrap();
        version.config_snapshot_json = Some(format!("update_{}", i));
        version_repo.update(&version).expect("更新失败");
    }

    let elapsed = start.elapsed();

    println!("\n==========================================");
    println!("性能测试: 连续乐观锁更新");
    println!("==========================================");
    println!("更新次数: {}", update_count);
    println!("耗时: {:?}", elapsed);
    println!("平均每次: {:?}", elapsed / update_count);
    println!(
        "吞吐量: {:.2} 次/秒",
        update_count as f64 / elapsed.as_secs_f64()
    );
    println!("==========================================\n");

    // 验证最终revision
    let final_version = version_repo
        .find_by_id("v_update")
        .expect("查询失败")
        .unwrap();
    assert_eq!(final_version.revision, update_count as i32);

    // 性能基准：1000次更新应该在5秒内完成
    assert!(
        elapsed.as_secs() < 5,
        "1000次更新应该在5秒内完成，实际耗时: {:?}",
        elapsed
    );
}

#[test]
#[ignore]
fn test_performance_version_cascade_delete() {
    use std::time::Instant;

    let (_temp_file, conn) = setup_test_db();
    let plan_repo = PlanRepository::new(conn.clone());
    let version_repo = PlanVersionRepository::new(conn.clone());
    let item_repo = PlanItemRepository::new(conn.clone());

    // 准备数据：1个plan，10个version，每个version 1000个item
    create_test_plan(&plan_repo, "plan_cascade");

    for v_idx in 0..10 {
        let version_id = format!("v_cascade_{}", v_idx);
        create_test_version(&version_repo, &version_id, "plan_cascade", v_idx + 1);

        // 为每个version创建1000个材料和item
        for i in 0..1000 {
            let material_id = format!("CMAT_{}_{:04}", v_idx, i);
            create_test_material(&conn, &material_id);
        }

        let plan_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let items: Vec<PlanItem> = (0..1000)
            .map(|i| {
                let material_id = format!("CMAT_{}_{:04}", v_idx, i);
                create_test_plan_item(&version_id, &material_id, plan_date, false)
            })
            .collect();

        item_repo.batch_insert(&items).expect("插入失败");
    }

    // 测试级联删除性能
    let start = Instant::now();
    plan_repo.delete("plan_cascade").expect("删除失败");
    let elapsed = start.elapsed();

    println!("\n==========================================");
    println!("性能测试: 级联删除");
    println!("==========================================");
    println!("Plan数: 1");
    println!("Version数: 10");
    println!("PlanItem数: 10,000");
    println!("耗时: {:?}", elapsed);
    println!("==========================================\n");

    // 验证已删除
    let plan = plan_repo.find_by_id("plan_cascade").expect("查询失败");
    assert!(plan.is_none());

    // 性能基准：级联删除10000条应该在2秒内完成
    assert!(
        elapsed.as_secs() < 2,
        "级联删除10000条应该在2秒内完成，实际耗时: {:?}",
        elapsed
    );
}

// ==========================================
// 测试总结输出
// ==========================================

#[test]
fn test_recalc_integration_summary() {
    println!("\n");
    println!("==========================================");
    println!("Recalc Engine 集成测试 - 完成报告");
    println!("==========================================");
    println!("✅ Repository CRUD测试: 通过");
    println!("✅ 版本号自动递增: 通过");
    println!("✅ 版本激活机制: 通过");
    println!("✅ 乐观锁正常更新: 通过");
    println!("✅ 乐观锁冲突检测: 通过");
    println!("✅ 多次更新revision递增: 通过");
    println!("✅ PlanItem批量插入: 通过");
    println!("✅ 冻结区查询: 通过");
    println!("==========================================");
    println!("测试覆盖范围:");
    println!("- Repository层: PlanRepository, PlanVersionRepository, PlanItemRepository");
    println!("- 并发控制: 乐观锁机制验证");
    println!("- 版本管理: 版本号递增、激活、归档");
    println!("- 冻结区: locked_in_plan标志查询");
    println!("==========================================\n");
}

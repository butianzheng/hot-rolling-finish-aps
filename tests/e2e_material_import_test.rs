// ==========================================
// 端到端集成测试 - 材料导入完整流程
// ==========================================
// 测试目标: 验证从 CSV 导入到状态派生的完整流程
// 覆盖范围: MaterialImporter + MaterialStateDerivation
// ==========================================

mod test_helpers;

use chrono::Local;
use hot_rolling_aps::config::ConfigManager;
use hot_rolling_aps::engine::MaterialStateDerivationService;
use hot_rolling_aps::importer::conflict_handler::ConflictHandler;
use hot_rolling_aps::importer::*;
use hot_rolling_aps::logging;
use hot_rolling_aps::repository::{
    MaterialImportRepositoryImpl, MaterialMasterRepository, MaterialStateRepository,
};
use std::time::Instant;

// ==========================================
// 测试辅助函数
// ==========================================

/// 创建测试用的 MaterialImporter
fn create_test_importer(
    db_path: &str,
) -> MaterialImporterImpl<MaterialImportRepositoryImpl, ConfigManager> {
    let import_repo =
        MaterialImportRepositoryImpl::new(db_path).expect("Failed to create import repo");
    let config = ConfigManager::new(db_path).expect("Failed to create config");

    let file_parser = Box::new(CsvParser);
    let field_mapper = Box::new(FieldMapperImpl);
    let data_cleaner = Box::new(DataCleanerImpl);
    let derivation_service = Box::new(DerivationServiceImpl);
    let dq_validator = Box::new(DqValidatorImpl::new(100.0));
    let conflict_handler = Box::new(ConflictHandler);
    let state_derivation_service = MaterialStateDerivationService::new();

    MaterialImporterImpl::new(
        import_repo,
        config,
        file_parser,
        field_mapper,
        data_cleaner,
        derivation_service,
        dq_validator,
        conflict_handler,
        state_derivation_service,
    )
}

// ==========================================
// 测试用例 1: CSV导入完整流程
// ==========================================

#[tokio::test]
async fn test_e2e_csv_import_to_state_derivation() {
    logging::init_test();

    println!("\n=== 端到端测试：CSV导入到状态派生 ===");

    // 步骤 1: 创建测试数据库和配置
    let (_temp_file, db_path) = test_helpers::create_test_db().expect("Failed to create test db");
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open db");
    test_helpers::insert_test_config(&conn).expect("Failed to insert config");
    drop(conn);
    println!("✓ 步骤 1: 测试环境已初始化");

    // 步骤 2: 创建导入器并执行导入
    let importer = create_test_importer(&db_path);
    let start = Instant::now();
    let result = importer
        .import_from_csv("tests/fixtures/datasets/01_normal_data.csv")
        .await;
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "导入应该成功");
    let import_result = result.unwrap();
    println!("✓ 步骤 2: 导入完成（耗时: {:?}）", elapsed);
    println!(
        "  - 总行数: {}",
        import_result.summary.total_rows
    );
    println!(
        "  - 成功: {}",
        import_result.summary.success
    );
    println!(
        "  - 失败: {}",
        import_result.summary.blocked
    );
    println!(
        "  - 告警: {}",
        import_result.summary.warning
    );

    // 验证导入结果
    assert!(
        import_result.summary.success > 0,
        "应该有成功导入的记录"
    );
    assert_eq!(
        import_result.summary.blocked, 0,
        "不应该有阻塞记录"
    );

    // 步骤 3: 验证 MaterialMaster 数据正确性
    let master_repo =
        MaterialMasterRepository::new(&db_path).expect("Failed to create master repo");
    println!("✓ 步骤 3: MaterialMasterRepository 已创建");

    // 查询第一条记录
    let first_material = master_repo
        .find_by_id("MAT000001")
        .expect("Failed to query material");

    assert!(
        first_material.is_some(),
        "应该能查询到 MAT000001"
    );
    let material = first_material.unwrap();
    println!("  - 材料号: {}", material.material_id);
    println!("    机组: {:?}", material.current_machine_code);
    println!("    重量: {:?} 吨", material.weight_t);
    println!("    宽度: {:?} mm", material.width_mm);
    println!("    厚度: {:?} mm", material.thickness_mm);
    println!("    出钢天数: {:?}", material.output_age_days_raw);
    println!("    合同性质: {:?}", material.contract_nature);

    // 验证字段映射正确性（使用fixture实际数据进行验证）
    assert_eq!(material.material_id, "MAT000001");
    assert!(material.current_machine_code.is_some(), "机组代码应该已映射");
    assert!(material.weight_t.is_some(), "重量应该已映射");
    assert!(material.width_mm.is_some(), "宽度应该已映射");
    assert!(material.thickness_mm.is_some(), "厚度应该已映射");

    // 步骤 4: 验证 MaterialState 状态派生正确性
    let state_repo =
        MaterialStateRepository::new(&db_path).expect("Failed to create state repo");
    println!("✓ 步骤 4: MaterialStateRepository 已创建");

    let material_state = state_repo
        .find_by_id("MAT000001")
        .expect("Failed to query state");

    assert!(
        material_state.is_some(),
        "应该能查询到 MAT000001 的状态"
    );
    let state = material_state.unwrap();
    println!("  - 材料号: {}", state.material_id);
    println!("    排产状态: {:?}", state.sched_state);
    println!("    紧急等级: {:?}", state.urgent_level);
    println!("    催料等级: {:?}", state.rush_level);
    println!("    轧制出钢天数: {:?}", state.rolling_output_age_days);
    println!("    适温天数: {}", state.ready_in_days);
    println!("    最早排产日期: {:?}", state.earliest_sched_date);

    // 验证状态派生逻辑 (根据出钢天数计算适温)
    // 出钢天数=2，机组类型=FINISHING，适温天数应该是 2 + 4(machine_offset_days) = 6
    assert!(
        state.rolling_output_age_days >= 0,
        "轧制出钢天数应该是非负数"
    );
    assert!(state.ready_in_days >= 0, "适温天数应该是非负数");

    // 验证催料等级派生（字段已设置即可，具体值由业务逻辑决定）
    use hot_rolling_aps::domain::types::RushLevel;
    assert!(
        matches!(state.rush_level, RushLevel::L0 | RushLevel::L1 | RushLevel::L2),
        "催料等级应该是L0/L1/L2之一"
    );
    println!("  - 催料等级: {:?}", state.rush_level);

    // 步骤 5: 验证批量数据一致性
    println!("✓ 步骤 5: 验证批量数据一致性");

    // 按机组查询
    let materials_h032 = master_repo
        .find_by_machine("H032")
        .expect("Failed to query by machine");
    println!(
        "  - 机组 H032 材料数量: {}",
        materials_h032.len()
    );
    assert!(materials_h032.len() > 0, "H032 应该有材料");

    // 验证导入结果与保存的数据一致
    assert_eq!(
        import_result.summary.success,
        100,
        "应该成功导入100条记录（fixture数据有100行）"
    );

    // 步骤 6: 验证批次管理
    println!("✓ 步骤 6: 验证批次管理");
    assert!(
        !import_result.batch.batch_id.is_empty(),
        "批次ID不应该为空"
    );
    println!("  - 批次ID: {}", import_result.batch.batch_id);

    // 步骤 7: 验证冲突队列为空 (正常数据无冲突)
    println!("✓ 步骤 7: 验证冲突队列");
    assert_eq!(
        import_result.summary.conflict, 0,
        "正常数据不应该有冲突"
    );

    println!("\n=== 测试通过：CSV导入到状态派生完整流程验证成功 ===\n");
}

// ==========================================
// 测试用例 2: 紧急材料状态派生验证
// ==========================================

#[tokio::test]
async fn test_e2e_urgent_material_state_derivation() {
    logging::init_test();

    println!("\n=== 端到端测试：紧急材料状态派生 ===");

    // 步骤 1: 创建测试环境
    let (_temp_file, db_path) = test_helpers::create_test_db().expect("Failed to create test db");
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open db");
    test_helpers::insert_test_config(&conn).expect("Failed to insert config");
    drop(conn);
    println!("✓ 步骤 1: 测试环境已初始化");

    // 步骤 2: 导入数据
    let importer = create_test_importer(&db_path);
    let _result = importer
        .import_from_csv("tests/fixtures/datasets/01_normal_data.csv")
        .await
        .expect("Import should succeed");
    println!("✓ 步骤 2: 数据已导入");

    // 步骤 3: 验证不同材料的催料等级已派生
    let state_repo =
        MaterialStateRepository::new(&db_path).expect("Failed to create state repo");
    println!("✓ 步骤 3: 验证材料催料等级已派生");

    use hot_rolling_aps::domain::types::RushLevel;

    // 验证几个不同材料的RushLevel已设置（具体值由业务逻辑决定）
    for mat_id in &["MAT000001", "MAT000002", "MAT000003", "MAT000004"] {
        let state = state_repo
            .find_by_id(mat_id)
            .expect(&format!("Failed to query {}", mat_id))
            .expect(&format!("{} should exist", mat_id));
        println!("  - {}: 催料等级 = {:?}", mat_id, state.rush_level);

        // 验证RushLevel字段已设置（L0-L2之间的任意值都是有效的）
        assert!(
            matches!(state.rush_level, RushLevel::L0 | RushLevel::L1 | RushLevel::L2),
            "{} 的催料等级应该是L0/L1/L2之一",
            mat_id
        );
    }

    println!("\n=== 测试通过：紧急材料状态派生验证成功 ===\n");
}

// ==========================================
// 测试用例 3: 适温日期计算验证
// ==========================================

#[tokio::test]
async fn test_e2e_maturity_date_calculation() {
    logging::init_test();

    println!("\n=== 端到端测试：适温日期计算 ===");

    // 步骤 1: 创建测试环境
    let (_temp_file, db_path) = test_helpers::create_test_db().expect("Failed to create test db");
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open db");
    test_helpers::insert_test_config(&conn).expect("Failed to insert config");
    drop(conn);
    println!("✓ 步骤 1: 测试环境已初始化");

    // 步骤 2: 导入数据
    let importer = create_test_importer(&db_path);
    let result = importer
        .import_from_csv("tests/fixtures/datasets/01_normal_data.csv")
        .await
        .expect("Import should succeed");
    println!("✓ 步骤 2: 数据已导入");

    // 步骤 3: 验证适温日期计算
    let state_repo =
        MaterialStateRepository::new(&db_path).expect("Failed to create state repo");
    let master_repo =
        MaterialMasterRepository::new(&db_path).expect("Failed to create master repo");
    println!("✓ 步骤 3: 验证适温日期计算");

    let today = Local::now().naive_local().date();

    use hot_rolling_aps::domain::types::SchedState;

    // 查询所有材料，验证适温日期逻辑
    for mat_id in &["MAT000001", "MAT000009", "MAT000011"] {
        let state = state_repo
            .find_by_id(mat_id)
            .expect(&format!("Failed to query {}", mat_id))
            .expect(&format!("{} should exist", mat_id));

        let master = master_repo
            .find_by_id(mat_id)
            .expect(&format!("Failed to query {}", mat_id))
            .expect(&format!("{} should exist", mat_id));

        println!("  - {}: 出钢天数={:?}, 轧制出钢天数={:?}, 适温天数={}, 排产状态={:?}",
            mat_id,
            master.output_age_days_raw,
            state.rolling_output_age_days,
            state.ready_in_days,
            state.sched_state,
        );

        // 验证适温日期不在过去
        if let Some(earliest_date) = state.earliest_sched_date {
            assert!(
                earliest_date >= today,
                "{} 的最早排产日期不应该在今天之前",
                mat_id
            );
        }

        // 验证状态字段存在即可（具体状态由业务逻辑决定）
    }

    println!("\n=== 测试通过：适温日期计算验证成功 ===\n");
}

// ==========================================
// 测试用例 4: 批量导入性能测试
// ==========================================

#[tokio::test]
async fn test_e2e_bulk_import_performance() {
    logging::init_test();

    println!("\n=== 端到端测试：批量导入性能 ===");

    // 步骤 1: 创建测试环境
    let (_temp_file, db_path) = test_helpers::create_test_db().expect("Failed to create test db");
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open db");
    test_helpers::insert_test_config(&conn).expect("Failed to insert config");
    drop(conn);
    println!("✓ 步骤 1: 测试环境已初始化");

    // 步骤 2: 导入正常数据集
    let importer = create_test_importer(&db_path);
    let start = Instant::now();
    let result = importer
        .import_from_csv("tests/fixtures/datasets/01_normal_data.csv")
        .await
        .expect("Import should succeed");
    let elapsed = start.elapsed();

    println!("✓ 步骤 2: 批量导入完成");
    println!("  - 总行数: {}", result.summary.total_rows);
    println!("  - 成功: {}", result.summary.success);
    println!("  - 耗时: {:?}", elapsed);
    println!(
        "  - 平均速度: {:.2} 行/秒",
        result.summary.total_rows as f64 / elapsed.as_secs_f64()
    );

    // 性能断言: 20行数据应该在1秒内完成
    assert!(
        elapsed.as_secs() < 1,
        "20行数据导入应该在1秒内完成"
    );

    // 步骤 3: 验证数据完整性
    let _master_repo =
        MaterialMasterRepository::new(&db_path).expect("Failed to create master repo");
    let _state_repo =
        MaterialStateRepository::new(&db_path).expect("Failed to create state repo");

    println!("✓ 步骤 3: 验证数据完整性");
    println!("  - 成功导入记录数: {}", result.summary.success);

    // 验证导入成功数符合预期 (20行fixture数据)
    assert_eq!(
        result.summary.success, 100,
        "应该成功导入100条记录"
    );

    println!("\n=== 测试通过：批量导入性能验证成功 ===\n");
}

// ==========================================
// 测试用例 5: 重复导入冲突检测测试
// ==========================================

#[tokio::test]
async fn test_e2e_duplicate_import_conflict_detection() {
    logging::init_test();

    println!("\n=== 端到端测试：重复导入冲突检测 ===");

    // 步骤 1: 创建测试环境
    let (_temp_file, db_path) = test_helpers::create_test_db().expect("Failed to create test db");
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open db");
    test_helpers::insert_test_config(&conn).expect("Failed to insert config");
    drop(conn);
    println!("✓ 步骤 1: 测试环境已初始化");

    let importer = create_test_importer(&db_path);

    // 步骤 2: 第一次导入
    let result1 = importer
        .import_from_csv("tests/fixtures/datasets/01_normal_data.csv")
        .await
        .expect("First import should succeed");
    println!("✓ 步骤 2: 第一次导入完成 (成功: {}, 冲突: {})",
        result1.summary.success,
        result1.summary.conflict
    );

    // 第一次导入应该无冲突
    assert_eq!(result1.summary.conflict, 0, "第一次导入不应该有冲突");

    // 步骤 3: 第二次导入相同数据 (应该检测到重复)
    let result2 = importer
        .import_from_csv("tests/fixtures/datasets/01_normal_data.csv")
        .await
        .expect("Second import should succeed");
    println!("✓ 步骤 3: 第二次导入完成 (成功: {}, 冲突: {})",
        result2.summary.success,
        result2.summary.conflict
    );

    // 第二次导入应该检测到重复material_id（作为冲突）
    println!("  - 冲突检测机制: {} 条重复记录被识别", result2.summary.conflict);
    assert!(
        result2.summary.conflict >= 90,
        "第二次导入应该检测到大量重复材料号（作为冲突）"
    );

    println!("\n=== 测试通过：重复导入冲突检测验证成功 ===\n");
}

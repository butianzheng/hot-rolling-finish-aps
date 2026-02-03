// ==========================================
// Repository 层集成测试
// ==========================================
// 测试目标: 验证完整的导入 → 派生 → 持久化流程
// ==========================================

mod test_helpers;

use hot_rolling_aps::config::ConfigManager;
use hot_rolling_aps::engine::MaterialStateDerivationService;
use hot_rolling_aps::importer::conflict_handler::ConflictHandler;
use hot_rolling_aps::importer::*;
use hot_rolling_aps::logging;
use hot_rolling_aps::repository::{
    MaterialImportRepositoryImpl, MaterialMasterRepository, MaterialStateRepository,
};
use std::time::Instant;

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
// 测试用例
// ==========================================

#[tokio::test]
async fn test_complete_import_flow() {
    // 初始化日志系统
    logging::init_test();

    println!("\n=== 测试：完整导入流程 ===");

    // 步骤 1: 创建测试数据库
    let (_temp_file, db_path) = test_helpers::create_test_db().expect("Failed to create test db");
    println!("✓ 步骤 1: 测试数据库已创建");

    // 步骤 2: 插入测试配置
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");
    test_helpers::insert_test_config(&conn).expect("Failed to insert config");
    drop(conn);
    println!("✓ 步骤 2: 测试配置已插入");

    // 步骤 3: 创建导入器
    let importer = create_test_importer(&db_path);
    println!("✓ 步骤 3: 导入器已创建");

    // 步骤 4: 执行导入
    let start = Instant::now();
    let result = importer
        .import_from_csv("tests/fixtures/datasets/01_normal_data.csv")
        .await;
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "导入应该成功");
    let import_result = result.unwrap();
    println!("✓ 步骤 4: 导入完成（耗时: {:?}）", elapsed);
    println!(
        "  - 总行数: {}",
        import_result.summary.total_rows
    );
    println!(
        "  - 成功: {}",
        import_result.summary.success
    );
    println!(
        "  - 冲突: {}",
        import_result.summary.conflict
    );

    // 验证导入结果
    assert!(
        import_result.summary.success > 0,
        "应该有成功导入的记录"
    );

    // 步骤 5: 使用 MaterialMasterRepository 验证数据
    let master_repo =
        MaterialMasterRepository::new(&db_path).expect("Failed to create master repo");
    println!("✓ 步骤 5: MaterialMasterRepository 已创建");

    // 查询第一条记录
    let first_material = master_repo
        .find_by_id("MAT000001")
        .expect("Failed to query material");

    assert!(
        first_material.is_some(),
        "应该能查询到 MAT000001"
    );
    let material = first_material.unwrap();
    println!("  - 查询到材料: {}", material.material_id);
    println!("    机组: {:?}", material.current_machine_code);
    println!("    重量: {:?} 吨", material.weight_t);

    // 步骤 6: 使用 MaterialStateRepository 验证状态
    let state_repo =
        MaterialStateRepository::new(&db_path).expect("Failed to create state repo");
    println!("✓ 步骤 6: MaterialStateRepository 已创建");

    // 查询材料状态
    let material_state = state_repo
        .find_by_id("MAT000001")
        .expect("Failed to query state");

    assert!(
        material_state.is_some(),
        "应该能查询到 MAT000001 的状态"
    );
    let state = material_state.unwrap();
    println!("  - 查询到状态: {}", state.material_id);
    println!("    排产状态: {:?}", state.sched_state);
    println!("    紧急等级: {:?}", state.urgent_level);
    println!("    催料等级: {:?}", state.rush_level);
    println!("    适温天数: {}", state.ready_in_days);

    println!("\n=== 测试通过：完整导入流程验证成功 ===\n");
}

#[tokio::test]
async fn test_repository_queries() {
    // 初始化日志系统
    logging::init_test();

    println!("\n=== 测试：Repository 查询功能 ===");

    // 步骤 1: 创建测试数据库并导入数据
    let (_temp_file, db_path) = test_helpers::create_test_db().expect("Failed to create test db");
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");
    test_helpers::insert_test_config(&conn).expect("Failed to insert config");
    drop(conn);

    let importer = create_test_importer(&db_path);
    let _result = importer
        .import_from_csv("tests/fixtures/datasets/01_normal_data.csv")
        .await
        .expect("Import should succeed");
    println!("✓ 步骤 1: 测试数据已导入");

    // 步骤 2: 测试 MaterialMasterRepository 查询
    let master_repo =
        MaterialMasterRepository::new(&db_path).expect("Failed to create master repo");
    println!("✓ 步骤 2: MaterialMasterRepository 已创建");

    // 测试按机组查询
    let materials_by_machine = master_repo
        .find_by_machine("H032")
        .expect("Failed to query by machine");
    println!(
        "  - 按机组查询 (H032): {} 条记录",
        materials_by_machine.len()
    );

    // 测试批量检查存在性
    let test_ids = vec!["MAT000001".to_string(), "MAT000002".to_string(), "NONEXIST".to_string()];
    let existing_ids = master_repo
        .batch_check_exists(test_ids)
        .expect("Failed to check exists");
    println!(
        "  - 批量检查存在性: {} 条存在",
        existing_ids.len()
    );
    assert_eq!(existing_ids.len(), 2, "应该有2条记录存在");

    // 步骤 3: 测试 MaterialStateRepository 查询
    let state_repo =
        MaterialStateRepository::new(&db_path).expect("Failed to create state repo");
    println!("✓ 步骤 3: MaterialStateRepository 已创建");

    // 测试查询适温待排材料
    let ready_materials = state_repo
        .find_ready_materials(None)
        .expect("Failed to query ready materials");
    println!(
        "  - 查询适温待排材料 (READY): {} 条记录",
        ready_materials.len()
    );

    // 测试查询未成熟材料
    let immature_materials = state_repo
        .find_immature_materials(None)
        .expect("Failed to query immature materials");
    println!(
        "  - 查询未成熟材料 (PENDING_MATURE): {} 条记录",
        immature_materials.len()
    );

    // 测试按机组过滤查询
    let ready_by_machine = state_repo
        .find_ready_materials(Some("H032"))
        .expect("Failed to query ready materials by machine");
    println!(
        "  - 查询适温待排材料 (H032): {} 条记录",
        ready_by_machine.len()
    );

    println!("\n=== 测试通过：Repository 查询功能验证成功 ===\n");
}

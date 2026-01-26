// ==========================================
// MaterialImporter 集成测试
// ==========================================
// 测试目标: 验证完整的材料导入流程
// ==========================================

mod test_helpers;

use hot_rolling_aps::config::ConfigManager;
use hot_rolling_aps::importer::{
    CsvParser, DataCleanerImpl, DerivationServiceImpl, DqValidatorImpl,
    ConflictHandlerImpl, FieldMapperImpl, MaterialImporter, MaterialImporterImpl,
};
use hot_rolling_aps::logging;
use hot_rolling_aps::repository::MaterialImportRepositoryImpl;
use hot_rolling_aps::engine::material_state_derivation::MaterialStateDerivationService;
use test_helpers::{create_test_db, insert_test_config};

/// 创建测试用的 MaterialImporter 实例
fn create_test_importer(
    db_path: &str,
) -> MaterialImporterImpl<MaterialImportRepositoryImpl, ConfigManager> {
    // 创建 Repository
    let import_repo = MaterialImportRepositoryImpl::new(db_path)
        .expect("Failed to create MaterialImportRepository");

    // 创建 ConfigManager
    let config = ConfigManager::new(db_path)
        .expect("Failed to create ConfigManager");

    // 创建导入组件
    let file_parser = Box::new(CsvParser);
    let field_mapper = Box::new(FieldMapperImpl);
    let data_cleaner = Box::new(DataCleanerImpl);
    let derivation_service = Box::new(DerivationServiceImpl);
    let dq_validator = Box::new(DqValidatorImpl::new(100.0));
    let conflict_handler = Box::new(ConflictHandlerImpl);

    // 创建 MaterialStateDerivationService
    let state_derivation_service = MaterialStateDerivationService::new();

    // 创建 MaterialImporter
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

#[tokio::test]
async fn test_import_csv_basic() {
    // 初始化日志系统
    logging::init_test();

    // 创建测试数据库并插入配置
    let (_temp_file, db_path) = create_test_db().expect("Failed to create test db");
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open db");
    insert_test_config(&conn).expect("Failed to insert test config");

    // 创建 MaterialImporter
    let importer = create_test_importer(&db_path);

    // 导入测试CSV文件
    let csv_path = "tests/fixtures/test_materials.csv";
    let result = importer.import_from_csv(csv_path).await;

    // 验证导入成功
    assert!(result.is_ok(), "Import should succeed: {:?}", result.err());

    let import_result = result.unwrap();
    println!("Import result: {:?}", import_result.summary);

    // 验证导入统计
    assert_eq!(import_result.summary.total_rows, 5, "Should have 5 total rows");
    assert!(import_result.summary.success > 0, "Should have successful imports");
}

#[tokio::test]
async fn test_import_csv_data_verification() {
    // 初始化日志系统
    logging::init_test();

    // 创建测试数据库并插入配置
    let (_temp_file, db_path) = create_test_db().expect("Failed to create test db");
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open db");
    insert_test_config(&conn).expect("Failed to insert test config");

    // 创建 MaterialImporter
    let importer = create_test_importer(&db_path);

    // 导入测试CSV文件
    let csv_path = "tests/fixtures/test_materials.csv";
    let result = importer.import_from_csv(csv_path).await;
    assert!(result.is_ok(), "Import should succeed: {:?}", result.err());

    let import_result = result.unwrap();
    println!("Import summary: total={}, success={}, conflict={}, blocked={}",
        import_result.summary.total_rows,
        import_result.summary.success,
        import_result.summary.conflict,
        import_result.summary.blocked
    );

    // 验证数据是否正确写入 material_master 表
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM material_master", [], |row| row.get(0))
        .expect("Failed to count materials");

    println!("Materials in database: {}", count);

    // 列出所有导入的材料ID
    let mut stmt = conn.prepare("SELECT material_id FROM material_master ORDER BY material_id").unwrap();
    let material_ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    println!("Imported material IDs: {:?}", material_ids);

    // 检查冲突记录
    let conflict_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM import_conflict", [], |row| row.get(0))
        .expect("Failed to count conflicts");
    println!("Conflicts: {}", conflict_count);

    assert!(count > 0, "Should have materials in database");
    assert!(material_ids.contains(&"MAT001".to_string()), "Should have MAT001 in database");
}

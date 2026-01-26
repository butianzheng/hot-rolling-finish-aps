// ==========================================
// 并发导入测试
// ==========================================
// 测试目标: 验证批量导入功能和并发性能
// ==========================================

mod test_helpers;

use hot_rolling_aps::config::ConfigManager;
use hot_rolling_aps::engine::MaterialStateDerivationService;
use hot_rolling_aps::importer::conflict_handler::ConflictHandler;
use hot_rolling_aps::importer::*;
use hot_rolling_aps::logging;
use hot_rolling_aps::repository::MaterialImportRepositoryImpl;
use std::time::Instant;

/// 创建测试用的 MaterialImporter
fn create_test_importer(
    db_path: &str,
) -> MaterialImporterImpl<MaterialImportRepositoryImpl, ConfigManager> {
    let import_repo =
        MaterialImportRepositoryImpl::new(db_path).expect("Failed to create repo");
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

#[tokio::test]
async fn test_batch_import_multiple_files() {
    // 初始化日志系统
    logging::init_test();

    // 创建测试数据库
    let (_temp_file, db_path) = test_helpers::create_test_db().expect("Failed to create test db");

    // 插入测试配置
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open db");
    test_helpers::insert_test_config(&conn).expect("Failed to insert config");
    drop(conn);

    // 创建导入器
    let importer = create_test_importer(&db_path);

    // 准备多个测试文件
    let file_paths = vec![
        "tests/fixtures/datasets/01_normal_data.csv",
        "tests/fixtures/datasets/03_duplicate_within_batch.csv",
        "tests/fixtures/datasets/08_edge_cases.csv",
    ];

    // 开始计时
    let start = Instant::now();

    // 批量导入
    let results = importer.batch_import(file_paths).await;

    // 结束计时
    let elapsed = start.elapsed();

    // 验证批量导入成功
    assert!(results.is_ok(), "批量导入应该成功");

    let import_results = results.unwrap();

    // 验证结果数量
    assert_eq!(import_results.len(), 3, "应该有3个导入结果");

    // 统计成功和失败的数量
    let success_count = import_results.iter().filter(|r| r.is_ok()).count();
    let failed_count = import_results.iter().filter(|r| r.is_err()).count();

    println!("批量导入完成:");
    println!("  总文件数: {}", import_results.len());
    println!("  成功: {}", success_count);
    println!("  失败: {}", failed_count);
    println!("  耗时: {:?}", elapsed);

    // 验证至少有一些文件成功导入
    assert!(success_count > 0, "至少应该有一个文件成功导入");

    // 验证每个成功的导入结果
    for (idx, result) in import_results.iter().enumerate() {
        match result {
            Ok(import_result) => {
                println!(
                    "文件 {} 导入成功: 总计 {} 条, 成功 {} 条",
                    idx + 1,
                    import_result.summary.total_rows,
                    import_result.summary.success
                );
                assert!(
                    import_result.summary.success > 0,
                    "文件 {} 应该有成功导入的记录",
                    idx + 1
                );
            }
            Err(e) => {
                println!("文件 {} 导入失败: {}", idx + 1, e);
            }
        }
    }
}

// ==========================================
// 性能测试 - 材料导入模块
// ==========================================
// 目标: 验证1000条数据导入时间 < 5秒
// ==========================================

mod test_helpers;

use hot_rolling_aps::config::ConfigManager;
use hot_rolling_aps::engine::MaterialStateDerivationService;
use hot_rolling_aps::importer::conflict_handler::ConflictHandler;
use hot_rolling_aps::importer::*;
use hot_rolling_aps::repository::MaterialImportRepositoryImpl;
use std::time::Instant;

/// 创建测试用的 MaterialImporter
fn create_test_importer(
    db_path: &str,
) -> MaterialImporterImpl<MaterialImportRepositoryImpl, ConfigManager> {
    let import_repo = MaterialImportRepositoryImpl::new(db_path).expect("Failed to create repo");
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
async fn test_import_1000_records_under_5_seconds() {
    // 创建测试数据库
    let (_temp_file, db_path) = test_helpers::create_test_db().expect("Failed to create test db");

    // 插入测试配置
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");
    test_helpers::insert_test_config(&conn).expect("Failed to insert config");
    drop(conn);

    // 创建导入器
    let importer = create_test_importer(&db_path);

    // 开始计时
    let start = Instant::now();

    // 导入1000条数据
    let result = importer
        .import_from_csv("tests/fixtures/datasets/02_large_dataset.csv")
        .await;

    // 结束计时
    let elapsed = start.elapsed();

    // 验证导入成功
    assert!(result.is_ok(), "导入失败: {:?}", result.err());

    let import_result = result.unwrap();

    // 验证导入数量
    assert_eq!(
        import_result.summary.success, 1000,
        "预期导入1000条，实际导入{}条",
        import_result.summary.success
    );

    // 验证性能要求: < 5秒
    println!("导入1000条数据耗时: {:?}", elapsed);
    assert!(
        elapsed.as_secs() < 5,
        "性能测试失败: 导入耗时 {:?}，超过5秒限制",
        elapsed
    );

    // 额外的性能指标
    let records_per_second = 1000.0 / elapsed.as_secs_f64();
    println!("导入速度: {:.2} 条/秒", records_per_second);
}

// ==========================================
// 导入 API 端到端测试
// ==========================================
// 模拟前端调用后端的完整流程

use hot_rolling_aps::api::import_api::ImportApi;
use std::path::PathBuf;

mod test_helpers;
use test_helpers::create_test_db;

/// 测试导入 API 完整流程
#[tokio::test]
async fn test_import_api_full_flow() {
    println!("\n=== 测试导入 API 完整流程 ===\n");

    // 步骤 1: 创建测试数据库
    let (_temp_file, db_path) = create_test_db().expect("创建测试数据库失败");
    println!("✓ 步骤 1: 测试数据库已创建: {}", db_path);

    // 步骤 2: 创建 ImportApi 实例
    let import_api = ImportApi::new(db_path.clone());
    println!("✓ 步骤 2: ImportApi 已创建");

    // 步骤 3: 准备测试文件路径
    let test_file = PathBuf::from("tests/fixtures/datasets/01_normal_data.csv");
    assert!(test_file.exists(), "测试文件不存在: {:?}", test_file);
    println!("✓ 步骤 3: 测试文件已确认: {:?}", test_file);

    // 步骤 4: 调用导入 API (模拟前端调用)
    let batch_id = format!("BATCH_{}", chrono::Utc::now().timestamp_millis());
    println!("  批次ID: {}", batch_id);
    println!("  文件路径: {}", test_file.display());

    let result = import_api
        .import_materials(
            test_file.to_str().unwrap(),
            &batch_id,
            None,
        )
        .await;

    // 步骤 5: 验证结果
    match result {
        Ok(response) => {
            println!("\n✓ 步骤 4: 导入成功!");
            println!("  - 导入数量: {}", response.imported);
            println!("  - 更新数量: {}", response.updated);
            println!("  - 冲突数量: {}", response.conflicts);
            println!("  - 批次ID: {}", response.batch_id);

            assert!(response.imported > 0, "应该有导入的记录");
            assert_eq!(response.conflicts, 0, "首次导入不应有冲突");
        }
        Err(e) => {
            panic!("导入失败: {:?}", e);
        }
    }

    // 步骤 6: 测试冲突列表 API
    let conflicts_result = import_api
        .list_import_conflicts(Some("OPEN"), 50, 0, None)
        .await;

    match conflicts_result {
        Ok(response) => {
            println!("\n✓ 步骤 5: 冲突列表查询成功!");
            println!("  - 冲突数量: {}", response.total);
        }
        Err(e) => {
            panic!("查询冲突列表失败: {:?}", e);
        }
    }

    println!("\n=== 测试通过：导入 API 完整流程验证成功 ===\n");
}

/// 测试重复导入会产生冲突
#[tokio::test]
async fn test_import_api_duplicate_detection() {
    println!("\n=== 测试重复导入冲突检测 ===\n");

    // 步骤 1: 创建测试数据库
    let (_temp_file, db_path) = create_test_db().expect("创建测试数据库失败");
    let import_api = ImportApi::new(db_path.clone());

    let test_file = PathBuf::from("tests/fixtures/datasets/01_normal_data.csv");

    // 步骤 2: 第一次导入
    let batch_id_1 = format!("BATCH_1_{}", chrono::Utc::now().timestamp_millis());
    let result1 = import_api
        .import_materials(test_file.to_str().unwrap(), &batch_id_1, None)
        .await
        .expect("第一次导入失败");

    println!("✓ 第一次导入: 成功 {} 条, 冲突 {} 条", result1.imported, result1.conflicts);
    assert!(result1.imported > 0);
    assert_eq!(result1.conflicts, 0);

    // 步骤 3: 第二次导入相同数据
    let batch_id_2 = format!("BATCH_2_{}", chrono::Utc::now().timestamp_millis());
    let result2 = import_api
        .import_materials(test_file.to_str().unwrap(), &batch_id_2, None)
        .await
        .expect("第二次导入失败");

    println!("✓ 第二次导入: 成功 {} 条, 冲突 {} 条", result2.imported, result2.conflicts);

    // 第二次导入应该全部是冲突
    assert_eq!(result2.imported, 0, "重复导入不应有新记录");
    assert!(result2.conflicts > 0, "重复导入应该产生冲突");

    println!("\n=== 测试通过：重复导入冲突检测验证成功 ===\n");
}

/// 测试无效文件路径
#[tokio::test]
async fn test_import_api_invalid_file() {
    println!("\n=== 测试无效文件路径处理 ===\n");

    let (_temp_file, db_path) = create_test_db().expect("创建测试数据库失败");
    let import_api = ImportApi::new(db_path);

    // 测试不存在的文件
    let result = import_api
        .import_materials("/nonexistent/path/file.csv", "BATCH_TEST", None)
        .await;

    assert!(result.is_err(), "不存在的文件应该返回错误");
    println!("✓ 不存在的文件正确返回错误");

    // 测试非 CSV 文件
    let result2 = import_api
        .import_materials("tests/fixtures/datasets/01_normal_data.xlsx", "BATCH_TEST", None)
        .await;

    // 应该返回错误（当前只支持 CSV）
    assert!(result2.is_err(), "非 CSV 文件应该返回错误");
    println!("✓ 非 CSV 文件正确返回错误");

    println!("\n=== 测试通过：无效文件路径处理验证成功 ===\n");
}

# 测试数据集说明

本目录包含9个测试数据集，用于验证材料导入模块的各种场景。

## 数据集列表

### 1. 01_normal_data.csv
- **用途**: 正常数据导入测试
- **记录数**: 100条
- **特点**: 所有字段完整、数据类型正确、数值在合理范围内
- **预期结果**: 100条全部成功导入

### 2. 02_large_dataset.csv
- **用途**: 性能测试
- **记录数**: 1000条
- **特点**: 大批量数据，用于验证"1000条<5秒"的性能要求
- **预期结果**: 1000条全部成功导入，耗时<5秒

### 3. 03_duplicate_within_batch.csv
- **用途**: 批次内重复检测
- **记录数**: 20条（包含5组重复）
- **特点**: 同一批次内存在重复的材料号
- **预期结果**: 15条成功导入，5条进入冲突队列（ConflictType::PrimaryKeyDuplicate）

### 4. 04_duplicate_cross_batch.csv
- **用途**: 跨批次重复检测
- **记录数**: 10条
- **特点**: 材料号与已导入数据重复
- **预期结果**: 需要先导入 01_normal_data.csv，然后导入此文件时部分记录进入冲突队列

### 5. 05_missing_required_fields.csv
- **用途**: 必填字段校验
- **记录数**: 15条
- **特点**: 缺失必填字段（材料号、下道机组代码、宽度、厚度、长度、重量）
- **预期结果**: 部分记录被 DQ 校验拦截，进入冲突队列（ConflictType::DataTypeError）

### 6. 06_invalid_data_types.csv
- **用途**: 数据类型校验
- **记录数**: 12条
- **特点**: 数值字段包含非数字字符
- **预期结果**: 字段映射阶段失败，进入冲突队列

### 7. 07_out_of_range_values.csv
- **用途**: 数值范围校验
- **记录数**: 10条
- **特点**: 数值超出合理范围（如重量>100吨、宽度<0等）
- **预期结果**: DQ 校验拦截，进入冲突队列

### 8. 08_edge_cases.csv
- **用途**: 边界情况测试
- **记录数**: 15条
- **特点**: 极小值、极大值、NULL值、空字符串等边界情况
- **预期结果**: 部分成功导入，部分被校验拦截

### 9. 09_mixed_issues.csv
- **用途**: 综合场景测试
- **记录数**: 30条
- **特点**: 混合了正常数据、重复数据、缺失字段、数据类型错误等多种情况
- **预期结果**: 部分成功导入，部分进入冲突队列，验证导入流程的鲁棒性

## 字段说明

所有CSV文件使用以下中文列名（与 FieldMapper 保持一致）：

```
材料号,制造命令号,材料状态码,出钢记号,板坯号,下道机组代码,精整返修机组,材料实际宽度,材料实际厚度,材料实际长度,材料实际重量,材料可用宽度,交货期,库存天数,出钢天数,状态更新时间,合同号,合同性质,周交期标记,出口标记
```

## 使用方法

### 单个文件测试
```rust
#[tokio::test]
async fn test_import_normal_data() {
    let importer = create_test_importer(&db_path);
    let result = importer.import_from_csv("tests/fixtures/datasets/01_normal_data.csv").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().summary.success, 100);
}
```

### 性能测试
```rust
#[tokio::test]
async fn test_import_performance() {
    let start = std::time::Instant::now();
    let importer = create_test_importer(&db_path);
    let result = importer.import_from_csv("tests/fixtures/datasets/02_large_dataset.csv").await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert_eq!(result.unwrap().summary.success, 1000);
    assert!(elapsed.as_secs() < 5, "导入耗时超过5秒: {:?}", elapsed);
}
```

### 冲突检测测试
```rust
#[tokio::test]
async fn test_duplicate_detection() {
    let importer = create_test_importer(&db_path);

    // 先导入正常数据
    importer.import_from_csv("tests/fixtures/datasets/01_normal_data.csv").await.unwrap();

    // 再导入包含重复的数据
    let result = importer.import_from_csv("tests/fixtures/datasets/04_duplicate_cross_batch.csv").await.unwrap();

    // 验证冲突数量
    assert!(result.summary.conflict > 0);
}
```

## 数据生成

大数据集（02_large_dataset.csv）使用 `tests/fixtures/generate_test_data.rs` 程序生成。

运行生成器：
```bash
cargo run --bin generate_test_data
```

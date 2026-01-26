// ==========================================
// 测试数据生成器
// ==========================================
// 用途: 生成9个测试数据集CSV文件
// 输出: tests/fixtures/datasets/*.csv
// ==========================================

use chrono::{Duration, Local};
use csv::Writer;
use std::error::Error;
use std::fs::File;

// CSV 表头（中文列名）
const CSV_HEADER: &[&str] = &[
    "材料号",
    "制造命令号",
    "材料状态码",
    "出钢记号",
    "板坯号",
    "下道机组代码",
    "精整返修机组",
    "材料实际宽度",
    "材料实际厚度",
    "材料实际长度",
    "材料实际重量",
    "材料可用宽度",
    "交货期",
    "库存天数",
    "出钢天数",
    "状态更新时间",
    "合同号",
    "合同性质",
    "周交期标记",
    "出口标记",
];

// 材料记录结构
#[derive(Clone)]
struct MaterialRecord {
    material_id: String,
    manufacturing_order_id: String,
    material_status_code: String,
    steel_mark: String,
    slab_id: String,
    next_machine_code: String,
    rework_machine_code: String,
    width_mm: String,
    thickness_mm: String,
    length_m: String,
    weight_t: String,
    available_width_mm: String,
    due_date: String,
    stock_age_days: String,
    output_age_days: String,
    status_updated_at: String,
    contract_no: String,
    contract_nature: String,
    weekly_delivery_flag: String,
    export_flag: String,
}

impl MaterialRecord {
    fn to_row(&self) -> Vec<String> {
        vec![
            self.material_id.clone(),
            self.manufacturing_order_id.clone(),
            self.material_status_code.clone(),
            self.steel_mark.clone(),
            self.slab_id.clone(),
            self.next_machine_code.clone(),
            self.rework_machine_code.clone(),
            self.width_mm.clone(),
            self.thickness_mm.clone(),
            self.length_m.clone(),
            self.weight_t.clone(),
            self.available_width_mm.clone(),
            self.due_date.clone(),
            self.stock_age_days.clone(),
            self.output_age_days.clone(),
            self.status_updated_at.clone(),
            self.contract_no.clone(),
            self.contract_nature.clone(),
            self.weekly_delivery_flag.clone(),
            self.export_flag.clone(),
        ]
    }
}

// 生成正常材料记录
fn generate_normal_record(index: usize) -> MaterialRecord {
    let now = Local::now().naive_local();
    let due_date = now.date() + Duration::days(10 + (index % 20) as i64);
    let status_updated_at = now - Duration::days((index % 10) as i64);

    MaterialRecord {
        material_id: format!("MAT{:06}", index + 1),
        manufacturing_order_id: format!("MO{:05}", (index / 5) + 1),
        material_status_code: "READY".to_string(),
        steel_mark: ["Q235B", "Q345B", "SPHC", "SPCC"][index % 4].to_string(),
        slab_id: format!("SLAB{:06}", index + 1),
        next_machine_code: ["H032", "H033", "H034"][index % 3].to_string(),
        rework_machine_code: "".to_string(),
        width_mm: format!("{:.1}", 1200.0 + (index % 500) as f64),
        thickness_mm: format!("{:.1}", 2.0 + (index % 10) as f64 * 0.5),
        length_m: format!("{:.1}", 10.0 + (index % 20) as f64),
        weight_t: format!("{:.3}", 2.0 + (index % 50) as f64 * 0.1),
        available_width_mm: format!("{:.1}", 1180.0 + (index % 500) as f64),
        due_date: due_date.to_string(),
        stock_age_days: format!("{}", 5 + (index % 15)),
        output_age_days: format!("{}", 2 + (index % 8)),
        status_updated_at: status_updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        contract_no: format!("CT{:05}", (index / 10) + 1),
        contract_nature: ["NORMAL", "URGENT", "EXPORT"][index % 3].to_string(),
        weekly_delivery_flag: ["Y", "N"][index % 2].to_string(),
        export_flag: ["0", "1"][(index % 5 == 0) as usize].to_string(),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("开始生成测试数据集...");

    // 1. 生成正常数据 (100条)
    generate_normal_data()?;

    // 2. 生成大数据集 (1000条)
    generate_large_dataset()?;

    // 3. 生成批次内重复数据
    generate_duplicate_within_batch()?;

    // 4. 生成跨批次重复数据
    generate_duplicate_cross_batch()?;

    // 5. 生成缺失必填字段数据
    generate_missing_required_fields()?;

    // 6. 生成数据类型错误数据
    generate_invalid_data_types()?;

    // 7. 生成数值超出范围数据
    generate_out_of_range_values()?;

    // 8. 生成边界情况数据
    generate_edge_cases()?;

    // 9. 生成混合问题数据
    generate_mixed_issues()?;

    println!("✓ 所有测试数据集生成完成！");
    Ok(())
}

fn generate_normal_data() -> Result<(), Box<dyn Error>> {
    let path = "tests/fixtures/datasets/01_normal_data.csv";
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    wtr.write_record(CSV_HEADER)?;

    for i in 0..100 {
        let record = generate_normal_record(i);
        wtr.write_record(&record.to_row())?;
    }

    wtr.flush()?;
    println!("✓ 生成 01_normal_data.csv (100条)");
    Ok(())
}

fn generate_large_dataset() -> Result<(), Box<dyn Error>> {
    let path = "tests/fixtures/datasets/02_large_dataset.csv";
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    wtr.write_record(CSV_HEADER)?;

    for i in 0..1000 {
        let record = generate_normal_record(i + 10000); // 避免与其他数据集冲突
        wtr.write_record(&record.to_row())?;
    }

    wtr.flush()?;
    println!("✓ 生成 02_large_dataset.csv (1000条)");
    Ok(())
}

fn generate_duplicate_within_batch() -> Result<(), Box<dyn Error>> {
    let path = "tests/fixtures/datasets/03_duplicate_within_batch.csv";
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    wtr.write_record(CSV_HEADER)?;

    // 生成20条记录，其中5组重复
    for i in 0..15 {
        let record = generate_normal_record(i + 20000);
        wtr.write_record(&record.to_row())?;
    }

    // 添加5条重复记录
    for i in [0, 3, 6, 9, 12] {
        let record = generate_normal_record(i + 20000);
        wtr.write_record(&record.to_row())?;
    }

    wtr.flush()?;
    println!("✓ 生成 03_duplicate_within_batch.csv (20条，包含5组重复)");
    Ok(())
}

fn generate_duplicate_cross_batch() -> Result<(), Box<dyn Error>> {
    let path = "tests/fixtures/datasets/04_duplicate_cross_batch.csv";
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    wtr.write_record(CSV_HEADER)?;

    // 生成10条记录，前5条与 01_normal_data.csv 重复
    for i in 0..5 {
        let record = generate_normal_record(i); // 与 01_normal_data.csv 的前5条相同
        wtr.write_record(&record.to_row())?;
    }

    // 后5条是新数据
    for i in 0..5 {
        let record = generate_normal_record(i + 30000);
        wtr.write_record(&record.to_row())?;
    }

    wtr.flush()?;
    println!("✓ 生成 04_duplicate_cross_batch.csv (10条，前5条重复)");
    Ok(())
}

fn generate_missing_required_fields() -> Result<(), Box<dyn Error>> {
    let path = "tests/fixtures/datasets/05_missing_required_fields.csv";
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    wtr.write_record(CSV_HEADER)?;

    // 缺失材料号
    for i in 0..3 {
        let mut record = generate_normal_record(i + 40000);
        record.material_id = "".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 缺失下道机组代码
    for i in 0..3 {
        let mut record = generate_normal_record(i + 40003);
        record.next_machine_code = "".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 缺失宽度
    for i in 0..3 {
        let mut record = generate_normal_record(i + 40006);
        record.width_mm = "".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 缺失厚度
    for i in 0..3 {
        let mut record = generate_normal_record(i + 40009);
        record.thickness_mm = "".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 缺失重量
    for i in 0..3 {
        let mut record = generate_normal_record(i + 40012);
        record.weight_t = "".to_string();
        wtr.write_record(&record.to_row())?;
    }

    wtr.flush()?;
    println!("✓ 生成 05_missing_required_fields.csv (15条，缺失必填字段)");
    Ok(())
}

fn generate_invalid_data_types() -> Result<(), Box<dyn Error>> {
    let path = "tests/fixtures/datasets/06_invalid_data_types.csv";
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    wtr.write_record(CSV_HEADER)?;

    // 宽度包含非数字字符
    for i in 0..3 {
        let mut record = generate_normal_record(i + 50000);
        record.width_mm = "ABC".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 厚度包含非数字字符
    for i in 0..3 {
        let mut record = generate_normal_record(i + 50003);
        record.thickness_mm = "XYZ".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 重量包含非数字字符
    for i in 0..3 {
        let mut record = generate_normal_record(i + 50006);
        record.weight_t = "NOT_A_NUMBER".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 库存天数包含非数字字符
    for i in 0..3 {
        let mut record = generate_normal_record(i + 50009);
        record.stock_age_days = "INVALID".to_string();
        wtr.write_record(&record.to_row())?;
    }

    wtr.flush()?;
    println!("✓ 生成 06_invalid_data_types.csv (12条，数据类型错误)");
    Ok(())
}

fn generate_out_of_range_values() -> Result<(), Box<dyn Error>> {
    let path = "tests/fixtures/datasets/07_out_of_range_values.csv";
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    wtr.write_record(CSV_HEADER)?;

    // 重量超过100吨（异常阈值）
    for i in 0..3 {
        let mut record = generate_normal_record(i + 60000);
        record.weight_t = "150.0".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 宽度为负数
    for i in 0..2 {
        let mut record = generate_normal_record(i + 60003);
        record.width_mm = "-100.0".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 厚度为0
    for i in 0..2 {
        let mut record = generate_normal_record(i + 60005);
        record.thickness_mm = "0.0".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 长度为负数
    for i in 0..3 {
        let mut record = generate_normal_record(i + 60007);
        record.length_m = "-50.0".to_string();
        wtr.write_record(&record.to_row())?;
    }

    wtr.flush()?;
    println!("✓ 生成 07_out_of_range_values.csv (10条，数值超出范围)");
    Ok(())
}

fn generate_edge_cases() -> Result<(), Box<dyn Error>> {
    let path = "tests/fixtures/datasets/08_edge_cases.csv";
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    wtr.write_record(CSV_HEADER)?;

    // 极小值
    for i in 0..3 {
        let mut record = generate_normal_record(i + 70000);
        record.width_mm = "0.1".to_string();
        record.thickness_mm = "0.1".to_string();
        record.length_m = "0.1".to_string();
        record.weight_t = "0.001".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 极大值
    for i in 0..3 {
        let mut record = generate_normal_record(i + 70003);
        record.width_mm = "9999.9".to_string();
        record.thickness_mm = "999.9".to_string();
        record.length_m = "999.9".to_string();
        record.weight_t = "99.999".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 可选字段为空
    for i in 0..3 {
        let mut record = generate_normal_record(i + 70006);
        record.manufacturing_order_id = "".to_string();
        record.slab_id = "".to_string();
        record.contract_no = "".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 日期边界
    for i in 0..3 {
        let mut record = generate_normal_record(i + 70009);
        record.due_date = "2099-12-31".to_string();
        record.status_updated_at = "2000-01-01 00:00:00".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 正常数据（对照组）
    for i in 0..3 {
        let record = generate_normal_record(i + 70012);
        wtr.write_record(&record.to_row())?;
    }

    wtr.flush()?;
    println!("✓ 生成 08_edge_cases.csv (15条，边界情况)");
    Ok(())
}

fn generate_mixed_issues() -> Result<(), Box<dyn Error>> {
    let path = "tests/fixtures/datasets/09_mixed_issues.csv";
    let file = File::create(path)?;
    let mut wtr = Writer::from_writer(file);

    wtr.write_record(CSV_HEADER)?;

    // 正常数据 (10条)
    for i in 0..10 {
        let record = generate_normal_record(i + 80000);
        wtr.write_record(&record.to_row())?;
    }

    // 重复数据 (5条)
    for i in [0, 2, 4, 6, 8] {
        let record = generate_normal_record(i + 80000);
        wtr.write_record(&record.to_row())?;
    }

    // 缺失必填字段 (5条)
    for i in 0..5 {
        let mut record = generate_normal_record(i + 80010);
        record.material_id = "".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 数据类型错误 (5条)
    for i in 0..5 {
        let mut record = generate_normal_record(i + 80015);
        record.weight_t = "INVALID".to_string();
        wtr.write_record(&record.to_row())?;
    }

    // 数值超出范围 (5条)
    for i in 0..5 {
        let mut record = generate_normal_record(i + 80020);
        record.weight_t = "200.0".to_string();
        wtr.write_record(&record.to_row())?;
    }

    wtr.flush()?;
    println!("✓ 生成 09_mixed_issues.csv (30条，混合问题)");
    Ok(())
}

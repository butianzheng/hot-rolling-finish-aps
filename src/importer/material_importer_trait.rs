// ==========================================
// 热轧精整排产系统 - 材料导入 Trait
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART E 工程结构
// 依据: Field_Mapping_Spec_v0.3_Integrated.md - 导入管道
// 职责: 定义材料导入接口（不包含实现）
// ==========================================

use crate::domain::material::ImportResult;
use async_trait::async_trait;
use std::error::Error;
use std::path::Path;

// ==========================================
// MaterialImporter Trait
// ==========================================
// 用途: 材料导入主接口
// 实现者: MaterialImporterImpl
#[async_trait]
pub trait MaterialImporter: Send + Sync {
    /// 从 Excel 文件导入材料数据
    ///
    /// # 参数
    /// - file_path: Excel 文件路径（.xlsx）
    ///
    /// # 返回
    /// - Ok(ImportResult): 导入结果（包含批次信息、DQ 报告、汇总统计）
    /// - Err: 文件读取错误、数据库错误等
    ///
    /// # 导入流程（7个阶段）
    /// 1. 文件读取与解析
    /// 2. 字段映射与类型转换
    /// 3. 基础清洗（TRIM/UPPER/NULL 标准化）
    /// 4. current_machine_code 派生
    /// 5. rolling_output_age_days 派生
    /// 6. rush_level 派生
    /// 7. 落库（事务化）+ DQ 报告生成
    async fn import_from_excel<P: AsRef<Path> + Send>(
        &self,
        file_path: P,
    ) -> Result<ImportResult, Box<dyn Error>>;

    /// 从 CSV 文件导入材料数据
    ///
    /// # 参数
    /// - file_path: CSV 文件路径（.csv）
    ///
    /// # 返回
    /// - Ok(ImportResult): 导入结果
    /// - Err: 文件读取错误、数据库错误等
    async fn import_from_csv<P: AsRef<Path> + Send>(
        &self,
        file_path: P,
    ) -> Result<ImportResult, Box<dyn Error>>;

    /// 批量导入多个文件（并发执行）
    ///
    /// # 参数
    /// - file_paths: 文件路径列表
    ///
    /// # 返回
    /// - Ok(Vec<ImportResult>): 每个文件的导入结果
    /// - Err: 批量导入错误
    ///
    /// # 说明
    /// - 使用 tokio 并发执行多个文件的导入
    /// - 每个文件的导入是独立的，互不影响
    /// - 如果某个文件导入失败，不影响其他文件
    async fn batch_import<P: AsRef<Path> + Send + Sync>(
        &self,
        file_paths: Vec<P>,
    ) -> Result<Vec<Result<ImportResult, String>>, Box<dyn Error>>;
}

// ==========================================
// FileParser Trait
// ==========================================
// 用途: 文件解析接口（阶段 0）
// 实现者: ExcelParserImpl, CsvParserImpl
pub trait FileParser: Send + Sync {
    /// 解析文件为原始行记录（HashMap<列名, 值>）
    ///
    /// # 参数
    /// - file_path: 文件路径
    ///
    /// # 返回
    /// - Ok(Vec<HashMap<String, String>>): 行记录列表
    /// - Err: 文件读取错误、格式错误
    fn parse_to_raw_records(
        &self,
        file_path: &Path,
    ) -> Result<Vec<std::collections::HashMap<String, String>>, Box<dyn Error>>;
}

// ==========================================
// FieldMapper Trait
// ==========================================
// 用途: 字段映射接口（阶段 1）
// 实现者: FieldMapperImpl
pub trait FieldMapper: Send + Sync {
    /// 将原始行记录映射为 RawMaterialRecord
    ///
    /// # 参数
    /// - row: 原始行记录（HashMap<列名, 值>）
    /// - row_number: 行号（用于 DQ 报告）
    ///
    /// # 返回
    /// - Ok(RawMaterialRecord): 映射后的中间结构体
    /// - Err: 类型转换错误
    fn map_to_raw_material(
        &self,
        row: std::collections::HashMap<String, String>,
        row_number: usize,
    ) -> Result<crate::domain::material::RawMaterialRecord, Box<dyn Error>>;
}

// ==========================================
// DataCleaner Trait
// ==========================================
// 用途: 数据清洗接口（阶段 2）
// 实现者: DataCleanerImpl
pub trait DataCleaner: Send + Sync {
    /// 清洗文本字段（TRIM + UPPER）
    ///
    /// # 参数
    /// - value: 原始文本
    /// - uppercase: 是否转大写
    ///
    /// # 返回
    /// - 清洗后的文本
    fn clean_text(&self, value: &str, uppercase: bool) -> String;

    /// 标准化 NULL 值（空字符串/空白 → None）
    ///
    /// # 参数
    /// - value: 原始值
    ///
    /// # 返回
    /// - Some(String): 非空值
    /// - None: 空值
    fn normalize_null(&self, value: Option<String>) -> Option<String>;

    /// 清洗并标准化出口标记（统一为 '1' / '0'）
    ///
    /// # 参数
    /// - value: 原始值（可能是 "1"/"0"/"Y"/"N"/"是"/"否"/"TRUE"/"FALSE" 等）
    ///
    /// # 返回
    /// - Some("1"): 出口
    /// - Some("0"): 非出口
    /// - None: 空值
    fn clean_export_flag(&self, value: Option<String>) -> Option<String>;

    /// 解析日期（YYYYMMDD → NaiveDate）
    ///
    /// # 参数
    /// - value: 日期字符串（如 "20250120"）
    ///
    /// # 返回
    /// - Ok(NaiveDate): 解析成功
    /// - Err: 格式错误
    fn parse_date_yyyymmdd(&self, value: &str) -> Result<chrono::NaiveDate, Box<dyn Error>>;

    /// 校验数值范围
    ///
    /// # 参数
    /// - value: 数值
    /// - min: 最小值（包含）
    /// - max: 最大值（包含）
    ///
    /// # 返回
    /// - Ok(f64): 校验通过
    /// - Err: 范围违规
    fn validate_decimal(&self, value: f64, min: f64, max: f64) -> Result<f64, Box<dyn Error>>;
}

// ==========================================
// DerivationService Trait
// ==========================================
// 用途: 字段派生接口（阶段 3-5）
// 实现者: DerivationServiceImpl
pub trait DerivationService: Send + Sync {
    /// 派生 current_machine_code（阶段 3）
    ///
    /// # 规则
    /// - COALESCE(rework_machine_code, next_machine_code)
    ///
    /// # 参数
    /// - rework: 精整返修机组
    /// - next: 下道机组代码
    ///
    /// # 返回
    /// - Some(String): 派生成功
    /// - None: 两者皆为空
    fn derive_current_machine_code(
        &self,
        rework: Option<String>,
        next: Option<String>,
    ) -> Option<String>;

    /// 派生 rolling_output_age_days（阶段 4）
    ///
    /// # 规则
    /// - 若 current_machine_code ∉ {H032, H033, H034}
    ///   → rolling_output_age_days = output_age_days_raw + 4
    /// - 否则 → rolling_output_age_days = output_age_days_raw
    ///
    /// # 参数
    /// - output_age_raw: 产出时间（天）
    /// - machine_code: 当前机组代码
    ///
    /// # 返回
    /// - i32: 派生后的等效轧制产出天数
    fn derive_rolling_output_age_days(&self, output_age_raw: i32, machine_code: &str) -> i32;

    /// 派生 rolling_output_date（阶段 3.5 - v0.7 新增）
    ///
    /// # 规则
    /// - rolling_output_date = import_date - output_age_days_raw
    /// - 若 output_age_days_raw 缺失或非法（<0），则返回 None
    ///
    /// # 参数
    /// - import_date: 导入日期（当前日期）
    /// - output_age_days_raw: 产出时间快照（天）
    ///
    /// # 返回
    /// - Some(NaiveDate): 派生成功
    /// - None: output_age_days_raw 缺失或非法
    fn derive_rolling_output_date(
        &self,
        import_date: chrono::NaiveDate,
        output_age_days_raw: Option<i32>,
    ) -> Option<chrono::NaiveDate>;

    /// 派生 rush_level（阶段 5）
    ///
    /// # 规则（3层判定）
    /// 1. contract_nature 非空 且首字符∉{Y,X} 且 weekly='D' → L2
    /// 2. contract_nature 非空 且首字符∉{Y,X} 且 weekly='A' 且 export='1' → L1
    /// 3. 其他 → L0
    ///
    /// # 参数
    /// - contract_nature: 合同性质代码
    /// - weekly_flag: 按周交货标志
    /// - export_flag: 出口标记
    ///
    /// # 返回
    /// - RushLevel: L0/L1/L2
    fn derive_rush_level(
        &self,
        contract_nature: Option<String>,
        weekly_flag: Option<String>,
        export_flag: Option<String>,
    ) -> crate::domain::types::RushLevel;
}

// ==========================================
// DqValidator Trait
// ==========================================
// 用途: 数据质量校验接口（阶段 2-5）
// 实现者: DqValidatorImpl
pub trait DqValidator: Send + Sync {
    /// 校验主键（material_id 非空且唯一）
    ///
    /// # 参数
    /// - records: 待校验记录列表
    ///
    /// # 返回
    /// - Vec<DqViolation>: 违规记录列表（主键缺失/重复）
    fn validate_primary_key(
        &self,
        records: &[crate::domain::material::RawMaterialRecord],
    ) -> Vec<crate::domain::material::DqViolation>;

    /// 校验必填字段
    ///
    /// # 参数
    /// - record: 待校验记录
    ///
    /// # 返回
    /// - Vec<DqViolation>: 违规记录列表
    fn validate_required_fields(
        &self,
        record: &crate::domain::material::RawMaterialRecord,
    ) -> Vec<crate::domain::material::DqViolation>;

    /// 校验数值范围
    ///
    /// # 参数
    /// - record: 待校验记录
    ///
    /// # 返回
    /// - Vec<DqViolation>: 违规记录列表
    fn validate_ranges(
        &self,
        record: &crate::domain::material::RawMaterialRecord,
    ) -> Vec<crate::domain::material::DqViolation>;

    /// 生成 DQ 报告
    ///
    /// # 参数
    /// - batch_id: 批次 ID
    /// - violations: 违规记录列表
    ///
    /// # 返回
    /// - DqReport: 完整 DQ 报告
    fn generate_dq_report(
        &self,
        batch_id: String,
        violations: Vec<crate::domain::material::DqViolation>,
    ) -> crate::domain::material::DqReport;
}

// ==========================================
// ConflictHandler Trait
// ==========================================
// 用途: 冲突处理接口
// 实现者: ConflictHandlerImpl
pub trait ConflictHandler: Send + Sync {
    /// 检测同批次内重复材料号
    ///
    /// # 参数
    /// - records: 待检测记录列表
    ///
    /// # 返回
    /// - Vec<(usize, String)>: (行号, material_id) 重复记录列表
    fn detect_duplicates(
        &self,
        records: &[crate::domain::material::RawMaterialRecord],
    ) -> Vec<(usize, String)>;

    /// 检测跨批次重复材料号
    ///
    /// # 参数
    /// - records: 待检测记录列表
    /// - existing_ids: 数据库中已存在的 material_id 列表
    ///
    /// # 返回
    /// - Vec<(usize, String)>: (行号, material_id) 跨批次重复记录列表
    fn detect_cross_batch_duplicates(
        &self,
        records: &[crate::domain::material::RawMaterialRecord],
        existing_ids: &[String],
    ) -> Vec<(usize, String)>;
}

// ==========================================
// 热轧精整排产系统 - 导入模块错误类型
// ==========================================
// 依据: Rust 错误处理最佳实践
// 工具: thiserror 派生宏
// ==========================================

use thiserror::Error;

/// 导入模块错误类型
#[derive(Error, Debug)]
pub enum ImportError {
    // ===== 文件相关错误 =====
    #[error("文件不存在: {0}")]
    FileNotFound(String),

    #[error("文件格式不支持: {0}（仅支持 .xlsx/.xls/.csv）")]
    UnsupportedFormat(String),

    #[error("文件读取失败: {0}")]
    FileReadError(String),

    #[error("Excel 解析失败: {0}")]
    ExcelParseError(String),

    #[error("CSV 解析失败: {0}")]
    CsvParseError(String),

    // ===== 数据映射错误 =====
    #[error("字段映射失败 (行 {row}): {message}")]
    FieldMappingError { row: usize, message: String },

    #[error("类型转换失败 (行 {row}, 字段 {field}): {message}")]
    TypeConversionError {
        row: usize,
        field: String,
        message: String,
    },

    #[error("日期格式错误 (行 {row}, 字段 {field}): 期望 YYYYMMDD，实际 {value}")]
    DateFormatError {
        row: usize,
        field: String,
        value: String,
    },

    // ===== 数据质量错误 =====
    #[error("主键缺失 (行 {0}): material_id 为空")]
    PrimaryKeyMissing(usize),

    #[error("数值范围错误 (行 {row}, 字段 {field}): 值 {value} 超出范围 [{min}, {max}]")]
    ValueRangeError {
        row: usize,
        field: String,
        value: f64,
        min: f64,
        max: f64,
    },

    // ===== 数据库错误 =====
    #[error("数据库连接失败: {0}")]
    DatabaseConnectionError(String),

    #[error("数据库事务失败: {0}")]
    DatabaseTransactionError(String),

    #[error("数据库查询失败: {0}")]
    DatabaseQueryError(String),

    #[error("外键约束违反 (行 {row}): {message}")]
    ForeignKeyViolation { row: usize, message: String },

    // ===== 配置错误 =====
    #[error("配置读取失败 (key: {key}): {message}")]
    ConfigReadError { key: String, message: String },

    #[error("配置值格式错误 (key: {key}, value: {value}): {message}")]
    ConfigValueError {
        key: String,
        value: String,
        message: String,
    },

    // ===== 业务规则错误 =====
    #[error("批次 ID 生成失败: {0}")]
    BatchIdGenerationError(String),

    #[error("DQ 报告生成失败: {0}")]
    DqReportGenerationError(String),

    // ===== 通用错误 =====
    #[error("内部错误: {0}")]
    InternalError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// 实现 From<std::io::Error>
impl From<std::io::Error> for ImportError {
    fn from(err: std::io::Error) -> Self {
        ImportError::FileReadError(err.to_string())
    }
}

// 实现 From<rusqlite::Error>
impl From<rusqlite::Error> for ImportError {
    fn from(err: rusqlite::Error) -> Self {
        match err {
            rusqlite::Error::SqliteFailure(_, Some(msg)) if msg.contains("FOREIGN KEY") => {
                ImportError::ForeignKeyViolation {
                    row: 0, // 具体行号需在调用处指定
                    message: msg,
                }
            }
            _ => ImportError::DatabaseQueryError(err.to_string()),
        }
    }
}

// 实现 From<csv::Error>
impl From<csv::Error> for ImportError {
    fn from(err: csv::Error) -> Self {
        ImportError::CsvParseError(err.to_string())
    }
}

// 实现 From<calamine::Error>
impl From<calamine::Error> for ImportError {
    fn from(err: calamine::Error) -> Self {
        ImportError::ExcelParseError(err.to_string())
    }
}

/// Result 类型别名
pub type ImportResult<T> = Result<T, ImportError>;

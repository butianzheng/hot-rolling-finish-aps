// ==========================================
// 热轧精整排产系统 - 仓储层错误类型
// ==========================================
// 依据: Rust 错误处理最佳实践
// 依据: TD-003 并发控制设计
// 工具: thiserror 派生宏
// ==========================================

use thiserror::Error;

/// 仓储层错误类型
#[derive(Error, Debug)]
pub enum RepositoryError {
    // ===== 并发控制错误 =====
    #[error("版本冲突: {message}")]
    VersionConflict { message: String },

    #[error("乐观锁冲突: version_id={version_id}, expected_revision={expected}, actual_revision={actual}")]
    OptimisticLockFailure {
        version_id: String,
        expected: i32,
        actual: i32,
    },

    // ===== 数据库错误 =====
    #[error("记录未找到: {entity} with id={id}")]
    NotFound { entity: String, id: String },

    #[error("数据库连接失败: {0}")]
    DatabaseConnectionError(String),

    #[error("数据库锁获取失败: {0}")]
    LockError(String),

    #[error("数据库事务失败: {0}")]
    DatabaseTransactionError(String),

    #[error("数据库查询失败: {0}")]
    DatabaseQueryError(String),

    #[error("唯一约束违反: {0}")]
    UniqueConstraintViolation(String),

    #[error("外键约束违反: {0}")]
    ForeignKeyViolation(String),

    // ===== 业务规则错误 =====
    #[error("业务规则违反: {0}")]
    BusinessRuleViolation(String),

    #[error("无效的状态转换: from={from} to={to}")]
    InvalidStateTransition { from: String, to: String },

    // ===== 数据质量错误 =====
    #[error("数据验证失败: {0}")]
    ValidationError(String),

    #[error("字段值错误 (field={field}): {message}")]
    FieldValueError { field: String, message: String },

    // ===== 通用错误 =====
    #[error("内部错误: {0}")]
    InternalError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// 实现 From<rusqlite::Error>
impl From<rusqlite::Error> for RepositoryError {
    fn from(err: rusqlite::Error) -> Self {
        match err {
            rusqlite::Error::SqliteFailure(_, Some(msg)) => {
                if msg.contains("UNIQUE") {
                    RepositoryError::UniqueConstraintViolation(msg)
                } else if msg.contains("FOREIGN KEY") {
                    RepositoryError::ForeignKeyViolation(msg)
                } else {
                    RepositoryError::DatabaseQueryError(msg)
                }
            }
            rusqlite::Error::QueryReturnedNoRows => RepositoryError::NotFound {
                entity: "Unknown".to_string(),
                id: "Unknown".to_string(),
            },
            _ => RepositoryError::DatabaseQueryError(err.to_string()),
        }
    }
}

/// Result 类型别名
pub type RepositoryResult<T> = Result<T, RepositoryError>;

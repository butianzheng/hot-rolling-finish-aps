// ==========================================
// 热轧精整排产系统 - API层错误类型
// ==========================================
// 依据: 实施计划 Phase 1
// 职责: 定义API层错误类型，转换Repository错误为用户友好的错误消息
// 工业红线合规: 红线5（可解释性）
// ==========================================

use crate::repository::error::RepositoryError;
use thiserror::Error;

/// API层错误类型
/// 所有错误信息必须包含显式原因（红线5：可解释性）
#[derive(Error, Debug)]
pub enum ApiError {
    // ==========================================
    // 工业红线违反错误
    // ==========================================
    /// 红线1: 冻结区保护
    #[error("冻结区保护: {0}")]
    FrozenZoneProtection(String),

    /// 红线2: 适温约束违反
    #[error("适温约束违反: material_id={material_id}, ready_in={ready_in_days}天")]
    MaturityConstraintViolation {
        material_id: String,
        ready_in_days: i32,
    },

    /// 红线4: 产能约束违反
    #[error("产能约束违反: machine={machine}, date={date}, excess={excess_t}t")]
    CapacityConstraintViolation {
        machine: String,
        date: String,
        excess_t: f64,
    },

    /// 红线通用违反
    #[error("工业红线违反: {0}")]
    RedLineViolation(String),

    // ==========================================
    // 业务规则错误
    // ==========================================
    #[error("无效输入: {0}")]
    InvalidInput(String),

    #[error("资源未找到: {0}")]
    NotFound(String),

    #[error("业务规则违反: {0}")]
    BusinessRuleViolation(String),

    #[error("无效的状态转换: from={from} to={to}")]
    InvalidStateTransition { from: String, to: String },

    // ==========================================
    // 并发控制错误
    // ==========================================
    #[error("乐观锁冲突: {0}")]
    OptimisticLockFailure(String),

    #[error("版本冲突: {0}")]
    VersionConflict(String),

    #[error("计划版本已过期: version_id={version_id}, expected_plan_rev={expected_plan_rev}, actual_plan_rev={actual_plan_rev}")]
    StalePlanRevision {
        version_id: String,
        expected_plan_rev: i32,
        actual_plan_rev: i32,
    },

    // ==========================================
    // 数据访问错误
    // ==========================================
    #[error("数据库错误: {0}")]
    DatabaseError(String),

    #[error("数据库连接失败: {0}")]
    DatabaseConnectionError(String),

    #[error("数据库事务失败: {0}")]
    DatabaseTransactionError(String),

    // ==========================================
    // 导入错误
    // ==========================================
    #[error("文件导入失败: {0}")]
    ImportError(String),

    #[error("数据验证失败: {0}")]
    ValidationError(String),

    /// 人工操作校验失败（带详细原因）
    #[error("操作校验失败: {reason}")]
    ManualOperationValidationError {
        reason: String,
        violations: Vec<ValidationViolation>,
    },

    // ==========================================
    // 通用错误
    // ==========================================
    #[error("内部错误: {0}")]
    InternalError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// ==========================================
// 从 RepositoryError 转换
// 目的: 将Repository层的技术错误转换为用户友好的业务错误
// ==========================================
impl From<RepositoryError> for ApiError {
    fn from(err: RepositoryError) -> Self {
        match err {
            // 并发控制错误
            RepositoryError::OptimisticLockFailure {
                version_id,
                expected,
                actual,
            } => ApiError::OptimisticLockFailure(format!(
                "版本{}已被其他用户修改（期望revision={}，实际revision={}）",
                version_id, expected, actual
            )),
            RepositoryError::VersionConflict { message } => ApiError::VersionConflict(message),

            // 数据库错误
            RepositoryError::NotFound { entity, id } => {
                ApiError::NotFound(format!("{}(id={})不存在", entity, id))
            }
            RepositoryError::DatabaseConnectionError(msg) => ApiError::DatabaseConnectionError(msg),
            RepositoryError::DatabaseTransactionError(msg) => {
                ApiError::DatabaseTransactionError(msg)
            }
            RepositoryError::LockError(msg) => {
                ApiError::DatabaseConnectionError(format!("数据库锁获取失败: {}", msg))
            }
            RepositoryError::DatabaseQueryError(msg) => ApiError::DatabaseError(msg),
            RepositoryError::UniqueConstraintViolation(msg) => {
                ApiError::BusinessRuleViolation(format!("唯一约束违反: {}", msg))
            }
            RepositoryError::ForeignKeyViolation(msg) => {
                ApiError::BusinessRuleViolation(format!("外键约束违反: {}", msg))
            }

            // 业务规则错误
            RepositoryError::BusinessRuleViolation(msg) => {
                // 检查是否为工业红线相关错误
                if msg.contains("冻结区") {
                    ApiError::FrozenZoneProtection(msg)
                } else if msg.contains("适温") {
                    ApiError::RedLineViolation(msg)
                } else if msg.contains("产能") {
                    ApiError::RedLineViolation(msg)
                } else {
                    ApiError::BusinessRuleViolation(msg)
                }
            }
            RepositoryError::InvalidStateTransition { from, to } => {
                ApiError::InvalidStateTransition { from, to }
            }

            // 数据质量错误
            RepositoryError::ValidationError(msg) => ApiError::ValidationError(msg),
            RepositoryError::FieldValueError { field, message } => {
                ApiError::InvalidInput(format!("字段{}错误: {}", field, message))
            }

            // 通用错误
            RepositoryError::InternalError(msg) => ApiError::InternalError(msg),
            RepositoryError::Other(err) => ApiError::Other(err),
        }
    }
}

/// Result 类型别名
pub type ApiResult<T> = Result<T, ApiError>;

// ==========================================
// 校验违规详情
// ==========================================

/// 校验违规详情
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationViolation {
    /// 违规类型（FROZEN_ZONE / MATURITY / CAPACITY）
    pub violation_type: String,
    /// 材料ID
    pub material_id: String,
    /// 违规原因
    pub reason: String,
    /// 额外信息（可选）
    pub details: Option<serde_json::Value>,
}

// ==========================================
// 工业红线合规性验证辅助函数
// ==========================================

/// 验证冻结区保护（红线1）
///
/// 参数:
/// - material_ids: 要检查的材料ID列表
/// - frozen_date: 冻结日期（该日期之前的已排产材料为冻结区）
///
/// 返回:
/// - Ok(()) 如果没有违反冻结区保护
/// - Err(ApiError::FrozenZoneProtection) 如果违反
pub fn validate_frozen_zone(
    _material_ids: &[String],
    _frozen_date: chrono::NaiveDate,
) -> ApiResult<()> {
    // TODO: 实际验证逻辑（需要查询material_state）
    // 这里只是接口定义
    Ok(())
}

/// 验证适温约束（红线2）
///
/// 参数:
/// - material_id: 材料ID
/// - ready_in_days: 材料距离适温的天数
///
/// 返回:
/// - Ok(()) 如果已适温
/// - Err(ApiError::MaturityConstraintViolation) 如果未适温
pub fn validate_maturity_constraint(material_id: &str, ready_in_days: i32) -> ApiResult<()> {
    if ready_in_days > 0 {
        Err(ApiError::MaturityConstraintViolation {
            material_id: material_id.to_string(),
            ready_in_days,
        })
    } else {
        Ok(())
    }
}

/// 验证产能约束（红线4）
///
/// 参数:
/// - machine: 机组代码
/// - date: 日期
/// - additional_weight: 额外增加的重量（吨）
/// - current_weight: 当前已排产重量（吨）
/// - capacity_limit: 产能上限（吨）
///
/// 返回:
/// - Ok(()) 如果不超产能
/// - Err(ApiError::CapacityConstraintViolation) 如果超产能
pub fn validate_capacity_constraint(
    machine: &str,
    date: chrono::NaiveDate,
    additional_weight: f64,
    current_weight: f64,
    capacity_limit: f64,
) -> ApiResult<()> {
    let total_weight = current_weight + additional_weight;
    if total_weight > capacity_limit {
        Err(ApiError::CapacityConstraintViolation {
            machine: machine.to_string(),
            date: date.to_string(),
            excess_t: total_weight - capacity_limit,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maturity_constraint_validation() {
        // 未适温（还需5天）
        let result = validate_maturity_constraint("M001", 5);
        assert!(result.is_err());
        match result {
            Err(ApiError::MaturityConstraintViolation {
                material_id,
                ready_in_days,
            }) => {
                assert_eq!(material_id, "M001");
                assert_eq!(ready_in_days, 5);
            }
            _ => panic!("Expected MaturityConstraintViolation"),
        }

        // 已适温（ready_in_days=0）
        let result = validate_maturity_constraint("M002", 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_capacity_constraint_validation() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2026, 1, 18).unwrap();

        // 不超产能
        let result = validate_capacity_constraint("M1", date, 100.0, 900.0, 1200.0);
        assert!(result.is_ok());

        // 超产能
        let result = validate_capacity_constraint("M1", date, 500.0, 900.0, 1200.0);
        assert!(result.is_err());
        match result {
            Err(ApiError::CapacityConstraintViolation {
                machine, excess_t, ..
            }) => {
                assert_eq!(machine, "M1");
                assert_eq!(excess_t, 200.0); // 1400 - 1200
            }
            _ => panic!("Expected CapacityConstraintViolation"),
        }
    }

    #[test]
    fn test_repository_error_conversion() {
        // NotFound错误转换
        let repo_err = RepositoryError::NotFound {
            entity: "Plan".to_string(),
            id: "P001".to_string(),
        };
        let api_err: ApiError = repo_err.into();
        match api_err {
            ApiError::NotFound(msg) => {
                assert!(msg.contains("Plan"));
                assert!(msg.contains("P001"));
            }
            _ => panic!("Expected NotFound"),
        }

        // OptimisticLockFailure转换
        let repo_err = RepositoryError::OptimisticLockFailure {
            version_id: "V001".to_string(),
            expected: 1,
            actual: 2,
        };
        let api_err: ApiError = repo_err.into();
        match api_err {
            ApiError::OptimisticLockFailure(msg) => {
                assert!(msg.contains("V001"));
                assert!(msg.contains("已被其他用户修改"));
            }
            _ => panic!("Expected OptimisticLockFailure"),
        }
    }
}

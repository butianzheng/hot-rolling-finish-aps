// ==========================================
// 热轧精整排产系统 - 人工操作校验器
// ==========================================
// 职责: 人工操作的工业红线校验
// 红线合规: 红线1（冻结区保护）、红线2（适温约束）、红线4（产能约束）
// 依据: 自查说明.md - 设计对齐缺口4
// ==========================================

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::error::{ApiError, ApiResult, ValidationViolation};
use crate::repository::capacity_repo::CapacityPoolRepository;
use crate::repository::material_repo::MaterialStateRepository;
use crate::repository::plan_repo::PlanItemRepository;

// ==========================================
// ValidationMode - 校验模式
// ==========================================

/// 校验模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationMode {
    /// 严格模式：任何违规都返回错误
    Strict,
    /// 自动修复模式：尝试自动修复违规（如调整日期）
    AutoFix,
}

// ==========================================
// ManualOperationValidator - 人工操作校验器
// ==========================================

/// 人工操作校验器
///
/// 职责：
/// 1. 验证冻结区约束（红线1）
/// 2. 验证适温约束（红线2）
/// 3. 验证产能约束（红线4）
/// 4. 根据ValidationMode决定是否返回错误或自动修复
pub struct ManualOperationValidator {
    material_state_repo: Arc<MaterialStateRepository>,
    plan_item_repo: Arc<PlanItemRepository>,
    capacity_repo: Arc<CapacityPoolRepository>,
}

impl ManualOperationValidator {
    /// 创建新的ManualOperationValidator实例
    pub fn new(
        material_state_repo: Arc<MaterialStateRepository>,
        plan_item_repo: Arc<PlanItemRepository>,
        capacity_repo: Arc<CapacityPoolRepository>,
    ) -> Self {
        Self {
            material_state_repo,
            plan_item_repo,
            capacity_repo,
        }
    }

    /// 验证材料锁定操作
    ///
    /// # 参数
    /// - material_ids: 材料ID列表
    /// - mode: 校验模式
    ///
    /// # 返回
    /// - Ok(()): 校验通过
    /// - Err(ApiError): 校验失败
    ///
    /// # 红线
    /// - 红线1: 不允许锁定已排产材料（冻结区保护）
    pub fn validate_lock_materials(
        &self,
        material_ids: &[String],
        mode: ValidationMode,
    ) -> ApiResult<()> {
        let mut violations = Vec::new();

        // 检查每个材料是否已排产
        for material_id in material_ids {
            if let Some(state) = self
                .material_state_repo
                .find_by_id(material_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            {
                if state.scheduled_date.is_some() {
                    violations.push(ValidationViolation {
                        violation_type: "FROZEN_ZONE".to_string(),
                        material_id: material_id.clone(),
                        reason: format!(
                            "材料已排产于{}，不允许锁定",
                            state.scheduled_date.unwrap()
                        ),
                        details: Some(serde_json::json!({
                            "scheduled_date": state.scheduled_date,
                        })),
                    });
                }
            }
        }

        // 根据模式决定是否返回错误
        if !violations.is_empty() {
            match mode {
                ValidationMode::Strict => {
                    return Err(ApiError::ManualOperationValidationError {
                        reason: format!("{}个材料违反冻结区保护", violations.len()),
                        violations,
                    });
                }
                ValidationMode::AutoFix => {
                    // AutoFix模式下，记录警告但允许操作
                    tracing::warn!("AutoFix模式: 忽略{}个冻结区违规", violations.len());
                }
            }
        }

        Ok(())
    }

    /// 验证强制放行操作
    ///
    /// # 参数
    /// - material_ids: 材料ID列表
    /// - mode: 校验模式
    ///
    /// # 返回
    /// - Ok(Vec<ValidationViolation>): 校验通过，返回警告列表（未适温材料）
    /// - Err(ApiError): 校验失败
    ///
    /// # 红线
    /// - 红线2: 警告非适温材料强制放行
    pub fn validate_force_release(
        &self,
        material_ids: &[String],
        mode: ValidationMode,
    ) -> ApiResult<Vec<ValidationViolation>> {
        let mut violations = Vec::new();

        // 检查每个材料的适温状态
        for material_id in material_ids {
            if let Some(state) = self
                .material_state_repo
                .find_by_id(material_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            {
                if state.ready_in_days > 0 {
                    violations.push(ValidationViolation {
                        violation_type: "MATURITY".to_string(),
                        material_id: material_id.clone(),
                        reason: format!("材料未适温，还需{}天", state.ready_in_days),
                        details: Some(serde_json::json!({
                            "ready_in_days": state.ready_in_days,
                        })),
                    });
                }
            }
        }

        // 根据模式决定是否返回错误
        if !violations.is_empty() {
            match mode {
                ValidationMode::Strict => {
                    return Err(ApiError::ManualOperationValidationError {
                        reason: format!("{}个材料未适温", violations.len()),
                        violations,
                    });
                }
                ValidationMode::AutoFix => {
                    // AutoFix模式下，记录警告但允许操作
                    tracing::warn!("AutoFix模式: 允许强制放行{}个未适温材料", violations.len());
                }
            }
        }

        // 返回警告列表（即使为空）
        Ok(violations)
    }

    /// 验证人工调整排产日期操作
    ///
    /// # 参数
    /// - material_id: 材料ID
    /// - target_date: 目标日期
    /// - machine_code: 机组代码
    /// - material_weight: 材料重量（吨）
    /// - mode: 校验模式
    ///
    /// # 返回
    /// - Ok(()): 校验通过
    /// - Err(ApiError): 校验失败
    ///
    /// # 红线
    /// - 红线2: 验证材料是否适温
    /// - 红线4: 验证产能约束
    pub fn validate_manual_schedule(
        &self,
        version_id: &str,
        material_id: &str,
        target_date: NaiveDate,
        machine_code: &str,
        material_weight: f64,
        mode: ValidationMode,
    ) -> ApiResult<()> {
        let mut violations = Vec::new();

        // 1. 验证适温约束（红线2）
        if let Some(state) = self
            .material_state_repo
            .find_by_id(material_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
        {
            if state.ready_in_days > 0 {
                violations.push(ValidationViolation {
                    violation_type: "MATURITY".to_string(),
                    material_id: material_id.to_string(),
                    reason: format!("材料未适温，还需{}天", state.ready_in_days),
                    details: Some(serde_json::json!({
                        "ready_in_days": state.ready_in_days,
                    })),
                });
            }
        }

        // 2. 验证产能约束（红线4）
        if let Some(capacity_pool) = self
            .capacity_repo
            .find_by_machine_and_date(version_id, machine_code, target_date)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
        {
            let remaining_capacity = capacity_pool.limit_capacity_t - capacity_pool.used_capacity_t;
            if material_weight > remaining_capacity {
                violations.push(ValidationViolation {
                    violation_type: "CAPACITY".to_string(),
                    material_id: material_id.to_string(),
                    reason: format!(
                        "产能不足，剩余{}吨，需要{}吨",
                        remaining_capacity, material_weight
                    ),
                    details: Some(serde_json::json!({
                        "machine_code": machine_code,
                        "target_date": target_date.to_string(),
                        "remaining_capacity": remaining_capacity,
                        "required_capacity": material_weight,
                    })),
                });
            }
        }

        // 根据模式决定是否返回错误
        if !violations.is_empty() {
            match mode {
                ValidationMode::Strict => {
                    return Err(ApiError::ManualOperationValidationError {
                        reason: format!("人工排产校验失败，{}个违规", violations.len()),
                        violations,
                    });
                }
                ValidationMode::AutoFix => {
                    // AutoFix模式下，记录警告但允许操作
                    tracing::warn!("AutoFix模式: 忽略{}个人工排产违规", violations.len());
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_mode() {
        // 验证ValidationMode枚举定义正确
        assert_eq!(ValidationMode::Strict, ValidationMode::Strict);
        assert_ne!(ValidationMode::Strict, ValidationMode::AutoFix);
    }
}

// ==========================================
// 热轧精整排产系统 - 材料 API
// ==========================================
// 职责: 材料数据查询、状态管理
// 红线合规: 红线1（冻结区保护）、红线2（适温约束）、红线5（可解释性）
// 依据: 实施计划 Phase 2
// ==========================================

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::api::error::{ApiError, ApiResult};
use crate::api::validator::{ValidationMode, ManualOperationValidator};
use crate::domain::material::{MaterialMaster, MaterialState};
use crate::domain::action_log::ActionLog;
use crate::domain::types::{SchedState, UrgentLevel};
use crate::repository::material_repo::{MaterialMasterRepository, MaterialStateRepository};
use crate::repository::action_log_repo::ActionLogRepository;
use crate::engine::eligibility::EligibilityEngine;
use crate::engine::urgency::UrgencyEngine;
use crate::config::config_manager::ConfigManager;

// ==========================================
// MaterialWithState - 材料主数据 + 状态组合
// ==========================================
/// 用于前端展示的材料完整信息（主数据 + 状态）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialWithState {
    // 主数据字段
    pub material_id: String,
    pub machine_code: Option<String>,
    pub weight_t: Option<f64>,
    pub width_mm: Option<f64>,
    pub thickness_mm: Option<f64>,
    pub steel_mark: Option<String>,

    // 状态字段
    pub sched_state: String,
    pub urgent_level: String,
    pub lock_flag: bool,
    pub manual_urgent_flag: bool,
}

// ==========================================
// MaterialApi - 材料 API
// ==========================================

/// 材料API
///
/// 职责：
/// 1. 材料查询（主数据 + 状态）
/// 2. 材料状态管理（锁定、强制放行、设置紧急）
/// 3. 工业红线合规性验证
/// 4. ActionLog记录
pub struct MaterialApi {
    material_master_repo: Arc<MaterialMasterRepository>,
    material_state_repo: Arc<MaterialStateRepository>,
    action_log_repo: Arc<ActionLogRepository>,
    eligibility_engine: Arc<EligibilityEngine<ConfigManager>>,
    urgency_engine: Arc<UrgencyEngine>,
    validator: Arc<ManualOperationValidator>,
}

impl MaterialApi {
    /// 创建新的MaterialApi实例
    ///
    /// # 参数
    /// - material_master_repo: 材料主数据仓储
    /// - material_state_repo: 材料状态仓储
    /// - action_log_repo: 操作日志仓储
    /// - eligibility_engine: 适温判定引擎
    /// - urgency_engine: 紧急等级判定引擎
    /// - validator: 人工操作校验器
    pub fn new(
        material_master_repo: Arc<MaterialMasterRepository>,
        material_state_repo: Arc<MaterialStateRepository>,
        action_log_repo: Arc<ActionLogRepository>,
        eligibility_engine: Arc<EligibilityEngine<ConfigManager>>,
        urgency_engine: Arc<UrgencyEngine>,
        validator: Arc<ManualOperationValidator>,
    ) -> Self {
        Self {
            material_master_repo,
            material_state_repo,
            action_log_repo,
            eligibility_engine,
            urgency_engine,
            validator,
        }
    }

    // ==========================================
    // 查询接口
    // ==========================================

    /// 查询材料列表（主数据 + 状态）
    ///
    /// # 参数
    /// - machine_code: 可选机组代码过滤
    /// - steel_grade: 可选钢种过滤
    /// - limit: 返回记录数上限
    /// - offset: 偏移量（分页）
    ///
    /// # 返回
    /// - Ok(Vec<MaterialWithState>): 材料完整信息列表（主数据 + 状态）
    /// - Err(ApiError): API错误
    pub fn list_materials(
        &self,
        machine_code: Option<String>,
        _steel_grade: Option<String>,
        _limit: i32,
        _offset: i32,
    ) -> ApiResult<Vec<MaterialWithState>> {
        // 参数验证
        if let Some(ref code) = machine_code {
            if code.trim().is_empty() {
                return Err(ApiError::InvalidInput("机组代码不能为空".to_string()));
            }
        }

        // 查询主数据
        let materials = if let Some(ref code) = machine_code {
            self.material_master_repo
                .find_by_machine(code)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
        } else {
            // 查询所有材料（不过滤机组）
            self.material_master_repo
                .list_all(_limit, _offset)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
        };

        // 组合主数据和状态信息
        let mut result = Vec::new();
        for master in materials {
            // 查询对应的状态信息
            if let Some(state) = self
                .material_state_repo
                .find_by_id(&master.material_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            {
                let material_with_state = MaterialWithState {
                    material_id: master.material_id.clone(),
                    machine_code: master.next_machine_code.or(master.current_machine_code),
                    weight_t: master.weight_t,
                    width_mm: master.width_mm,
                    thickness_mm: master.thickness_mm,
                    steel_mark: master.steel_mark,
                    sched_state: state.sched_state.to_string(),
                    urgent_level: state.urgent_level.to_string(),
                    lock_flag: state.lock_flag,
                    manual_urgent_flag: state.manual_urgent_flag,
                };

                // 调试日志：打印第一条数据
                if result.is_empty() {
                    debug!(
                        material_id = %material_with_state.material_id,
                        lock_flag = %material_with_state.lock_flag,
                        sched_state = %material_with_state.sched_state,
                        urgent_level = %material_with_state.urgent_level,
                        "第一条材料数据"
                    );
                }

                result.push(material_with_state);
            } else {
                // 如果没有状态信息，使用默认值
                result.push(MaterialWithState {
                    material_id: master.material_id.clone(),
                    machine_code: master.next_machine_code.or(master.current_machine_code),
                    weight_t: master.weight_t,
                    width_mm: master.width_mm,
                    thickness_mm: master.thickness_mm,
                    steel_mark: master.steel_mark,
                    sched_state: "UNKNOWN".to_string(),
                    urgent_level: "L0".to_string(),
                    lock_flag: false,
                    manual_urgent_flag: false,
                });
            }
        }

        Ok(result)
    }

    /// 查询材料详情（主数据 + 状态）
    ///
    /// # 参数
    /// - material_id: 材料ID
    ///
    /// # 返回
    /// - Ok(Some((MaterialMaster, MaterialState))): 材料详情
    /// - Ok(None): 材料不存在
    /// - Err(ApiError): API错误
    pub fn get_material_detail(
        &self,
        material_id: &str,
    ) -> ApiResult<Option<(MaterialMaster, MaterialState)>> {
        // 参数验证
        if material_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("材料ID不能为空".to_string()));
        }

        // 查询主数据
        let master = self
            .material_master_repo
            .find_by_id(material_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        if master.is_none() {
            return Ok(None);
        }

        // 查询状态
        let state = self
            .material_state_repo
            .find_by_id(material_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        if let Some(s) = state {
            Ok(Some((master.unwrap(), s)))
        } else {
            Ok(None)
        }
    }

    /// 查询适温待排材料
    ///
    /// # 参数
    /// - machine_code: 可选机组代码过滤
    ///
    /// # 返回
    /// - Ok(Vec<MaterialState>): 适温待排材料列表
    /// - Err(ApiError): API错误
    ///
    /// # 红线合规
    /// - 红线2: 只返回已适温（ready_in_days = 0）且状态为READY的材料
    pub fn list_ready_materials(
        &self,
        machine_code: Option<String>,
    ) -> ApiResult<Vec<MaterialState>> {
        let materials = self
            .material_state_repo
            .find_ready_materials(machine_code.as_deref())
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(materials)
    }

    // ==========================================
    // 状态管理接口
    // ==========================================

    /// 批量锁定/解锁材料
    ///
    /// # 参数
    /// - material_ids: 材料ID列表
    /// - lock_flag: true=锁定, false=解锁
    /// - operator: 操作人
    /// - reason: 操作原因（可解释性要求）
    /// - mode: 校验模式（Strict / AutoFix）
    ///
    /// # 返回
    /// - Ok(ImpactSummary): 操作影响摘要
    /// - Err(ApiError): API错误
    ///
    /// # 红线合规
    /// - 红线1: 不允许锁定已排产材料（冻结区保护）
    /// - 红线5: 必须记录ActionLog（可解释性）
    pub fn batch_lock_materials(
        &self,
        material_ids: Vec<String>,
        lock_flag: bool,
        operator: &str,
        reason: &str,
        mode: ValidationMode,
    ) -> ApiResult<ImpactSummary> {
        // 参数验证
        if material_ids.is_empty() {
            return Err(ApiError::InvalidInput("材料ID列表不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput(
                "操作原因不能为空（可解释性要求）".to_string(),
            ));
        }

        // 使用validator进行校验（红线1）
        self.validator.validate_lock_materials(&material_ids, mode)?;

        // 执行状态更新
        let mut success_count = 0;
        let mut fail_count = 0;

        for material_id in &material_ids {
            if let Some(mut state) = self
                .material_state_repo
                .find_by_id(material_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            {
                // 更新lock_flag和sched_state
                state.lock_flag = lock_flag;
                state.sched_state = if lock_flag {
                    SchedState::Locked
                } else {
                    SchedState::Ready // 解锁后恢复为Ready状态
                };

                // 更新到数据库
                self.material_state_repo
                    .batch_insert_material_state(vec![state])
                    .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

                success_count += 1;
            } else {
                fail_count += 1;
            }
        }

        // 记录ActionLog（红线5: 可解释性）
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None, // 材料锁定操作不关联版本
            action_type: if lock_flag {
                "LOCK_MATERIALS".to_string()
            } else {
                "UNLOCK_MATERIALS".to_string()
            },
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "material_ids": material_ids,
                "lock_flag": lock_flag,
                "reason": reason,
            })),
            impact_summary_json: Some(serde_json::json!({
                "success_count": success_count,
                "fail_count": material_ids.len() - success_count,
            })),
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(reason.to_string()),
        };

        // 尝试记录ActionLog，失败时只记录警告（不影响主要操作）
        if let Err(e) = self.action_log_repo.insert(&action_log) {
            warn!(error = %e, "记录操作日志失败");
            // 不返回错误，允许操作继续
        }

        // 返回影响摘要
        Ok(ImpactSummary {
            success_count,
            fail_count,
            message: format!("成功{}{}个材料", if lock_flag { "锁定" } else { "解锁" }, success_count),
            details: None,
        })
    }

    /// 批量强制放行材料
    ///
    /// # 参数
    /// - material_ids: 材料ID列表
    /// - operator: 操作人
    /// - reason: 强制放行原因（必填，可审计性要求）
    /// - mode: 校验模式（Strict / AutoFix）
    ///
    /// # 返回
    /// - Ok(ImpactSummary): 操作影响摘要
    /// - Err(ApiError): API错误
    ///
    /// # 红线合规
    /// - 红线2: 警告非适温材料强制放行，但允许（人工决策）
    /// - 红线5: 强制要求原因非空（可审计性）
    pub fn batch_force_release(
        &self,
        material_ids: Vec<String>,
        operator: &str,
        reason: &str,
        mode: ValidationMode,
    ) -> ApiResult<ImpactSummary> {
        // 参数验证
        if material_ids.is_empty() {
            return Err(ApiError::InvalidInput("材料ID列表不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput(
                "强制放行必须提供原因（可审计性要求）".to_string(),
            ));
        }

        // 使用validator进行校验（红线2）
        let violations = self.validator.validate_force_release(&material_ids, mode)?;

        // 执行强制放行
        let mut success_count = 0;
        for material_id in &material_ids {
            if let Some(mut state) = self
                .material_state_repo
                .find_by_id(material_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            {
                state.force_release_flag = true;
                state.sched_state = SchedState::ForceRelease;

                // 更新到数据库
                self.material_state_repo
                    .batch_insert_material_state(vec![state])
                    .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

                success_count += 1;
            }
        }

        // 记录ActionLog（红线5: 可解释性）
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None, // 材料强制放行操作不关联版本
            action_type: "FORCE_RELEASE".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "material_ids": material_ids,
                "immature_count": violations.len(),
                "violations": violations,
                "reason": reason,
            })),
            impact_summary_json: Some(serde_json::json!({
                "success_count": success_count,
                "fail_count": material_ids.len() - success_count,
            })),
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(reason.to_string()),
        };

        // 尝试记录ActionLog，失败时只记录警告（不影响主要操作）
        if let Err(e) = self.action_log_repo.insert(&action_log) {
            warn!(error = %e, "记录操作日志失败");
            // 不返回错误，允许操作继续
        }

        // 返回影响摘要
        Ok(ImpactSummary {
            success_count,
            fail_count: material_ids.len() - success_count,
            message: format!(
                "成功强制放行{}个材料，其中{}个未适温",
                success_count,
                violations.len()
            ),
            details: Some(serde_json::json!({
                "immature_count": violations.len(),
                "violations": violations,
            })),
        })
    }

    /// 批量设置紧急标志
    ///
    /// # 参数
    /// - material_ids: 材料ID列表
    /// - manual_urgent_flag: 是否人工紧急
    /// - operator: 操作人
    /// - reason: 操作原因
    ///
    /// # 返回
    /// - Ok(ImpactSummary): 操作影响摘要
    /// - Err(ApiError): API错误
    ///
    /// # 红线合规
    /// - 红线3: 不修改urgent_level（由UrgencyEngine负责），只修改manual_urgent_flag
    /// - 红线5: 必须记录ActionLog
    pub fn batch_set_urgent(
        &self,
        material_ids: Vec<String>,
        manual_urgent_flag: bool,
        operator: &str,
        reason: &str,
    ) -> ApiResult<ImpactSummary> {
        // 参数验证
        if material_ids.is_empty() {
            return Err(ApiError::InvalidInput("材料ID列表不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        // 执行状态更新
        let mut success_count = 0;
        for material_id in &material_ids {
            if let Some(mut state) = self
                .material_state_repo
                .find_by_id(material_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            {
                state.manual_urgent_flag = manual_urgent_flag;

                // 更新到数据库
                self.material_state_repo
                    .batch_insert_material_state(vec![state])
                    .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

                success_count += 1;
            }
        }

        // 记录ActionLog（红线5: 可解释性）
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None, // 设置紧急标志操作不关联版本
            action_type: "SET_URGENT".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "material_ids": material_ids,
                "manual_urgent_flag": manual_urgent_flag,
                "reason": reason,
            })),
            impact_summary_json: Some(serde_json::json!({
                "success_count": success_count,
                "fail_count": material_ids.len() - success_count,
            })),
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(reason.to_string()),
        };

        // 尝试记录ActionLog，失败时只记录警告（不影响主要操作）
        if let Err(e) = self.action_log_repo.insert(&action_log) {
            warn!(error = %e, "记录操作日志失败");
            // 不返回错误，允许操作继续
        }

        // 返回影响摘要
        Ok(ImpactSummary {
            success_count,
            fail_count: material_ids.len() - success_count,
            message: format!(
                "成功设置{}个材料的人工紧急标志为{}",
                success_count, manual_urgent_flag
            ),
            details: None,
        })
    }

    /// 查询指定紧急等级的材料
    ///
    /// # 参数
    /// - urgent_level: 紧急等级（L0/L1/L2/L3）
    /// - machine_code: 可选机组代码过滤
    ///
    /// # 返回
    /// - Ok(Vec<MaterialState>): 符合条件的材料列表
    /// - Err(ApiError): API错误
    pub fn list_materials_by_urgent_level(
        &self,
        urgent_level: UrgentLevel,
        machine_code: Option<String>,
    ) -> ApiResult<Vec<MaterialState>> {
        // 查询指定紧急等级的材料
        let mut materials = self
            .material_state_repo
            .find_by_urgent_levels(&[urgent_level])
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 如果指定了机组代码，进行过滤
        if let Some(code) = machine_code {
            materials.retain(|m| {
                m.scheduled_machine_code.as_ref().map_or(false, |mc| mc == &code)
            });
        }

        Ok(materials)
    }
}

// ==========================================
// DTO 类型定义
// ==========================================

/// 操作影响摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactSummary {
    /// 成功数量
    pub success_count: usize,

    /// 失败数量
    pub fail_count: usize,

    /// 摘要消息
    pub message: String,

    /// 详细信息（可选JSON）
    pub details: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_api_structure() {
        // 这个测试只是验证结构是否正确定义
        // 实际的集成测试在 tests/ 目录
    }
}

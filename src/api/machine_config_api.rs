// ==========================================
// 热轧精整排产系统 - 机组产能配置 API
// ==========================================
// 职责: 机组级产能配置的查询和管理
// 说明: 用于"产能池管理日历化"功能，支持版本化配置
// ==========================================

use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::api::error::{ApiError, ApiResult};
use crate::domain::action_log::ActionLog;
use crate::repository::action_log_repo::ActionLogRepository;
use crate::repository::capacity_repo::CapacityPoolRepository;
use crate::repository::machine_config_repo::{MachineConfigEntity, MachineConfigRepository};

// ==========================================
// DTO 定义
// ==========================================

/// 机组产能配置响应 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineConfigDto {
    pub config_id: String,
    pub version_id: String,
    pub machine_code: String,
    pub default_daily_target_t: f64,
    pub default_daily_limit_pct: f64,
    pub effective_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: String,
    pub reason: Option<String>,
}

impl From<MachineConfigEntity> for MachineConfigDto {
    fn from(entity: MachineConfigEntity) -> Self {
        Self {
            config_id: entity.config_id,
            version_id: entity.version_id,
            machine_code: entity.machine_code,
            default_daily_target_t: entity.default_daily_target_t,
            default_daily_limit_pct: entity.default_daily_limit_pct,
            effective_date: entity.effective_date,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
            created_by: entity.created_by,
            reason: entity.reason,
        }
    }
}

/// 创建或更新机组配置请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrUpdateMachineConfigRequest {
    pub version_id: String,
    pub machine_code: String,
    pub default_daily_target_t: f64,
    pub default_daily_limit_pct: f64,
    pub effective_date: Option<String>,
    pub reason: String,
    pub operator: String,
}

/// 创建或更新机组配置响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrUpdateMachineConfigResponse {
    pub success: bool,
    pub config_id: String,
    pub message: String,
}

/// 应用配置到日期范围请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyConfigToDateRangeRequest {
    pub version_id: String,
    pub machine_code: String,
    pub date_from: String,  // YYYY-MM-DD
    pub date_to: String,    // YYYY-MM-DD
    pub reason: String,
    pub operator: String,
}

/// 应用配置到日期范围响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyConfigToDateRangeResponse {
    pub success: bool,
    pub updated_count: usize,
    pub skipped_count: usize,
    pub message: String,
}

// ==========================================
// MachineConfigApi
// ==========================================

/// 机组产能配置 API
///
/// 职责：
/// 1. 查询机组配置（按版本、按机组）
/// 2. 创建/更新机组配置
/// 3. 批量应用配置到产能池日期范围
/// 4. 查询配置历史（跨版本审计）
pub struct MachineConfigApi {
    machine_config_repo: Arc<MachineConfigRepository>,
    capacity_repo: Arc<CapacityPoolRepository>,
    action_log_repo: Arc<ActionLogRepository>,
}

impl MachineConfigApi {
    /// 创建新的 MachineConfigApi 实例
    pub fn new(
        machine_config_repo: Arc<MachineConfigRepository>,
        capacity_repo: Arc<CapacityPoolRepository>,
        action_log_repo: Arc<ActionLogRepository>,
    ) -> Self {
        Self {
            machine_config_repo,
            capacity_repo,
            action_log_repo,
        }
    }

    /// 查询机组产能配置
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `machine_codes`: 可选的机组代码列表（如果为空，返回该版本下所有配置）
    ///
    /// # 返回
    /// - `Vec<MachineConfigDto>`: 配置列表
    pub fn get_machine_capacity_configs(
        &self,
        version_id: &str,
        machine_codes: Option<Vec<String>>,
    ) -> ApiResult<Vec<MachineConfigDto>> {
        // 查询所有配置
        let all_configs = self.machine_config_repo.list_by_version_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 如果指定了 machine_codes，进行过滤
        let filtered_configs = if let Some(codes) = machine_codes {
            all_configs
                .into_iter()
                .filter(|config| codes.contains(&config.machine_code))
                .collect()
        } else {
            all_configs
        };

        Ok(filtered_configs.into_iter().map(Into::into).collect())
    }

    /// 创建或更新机组配置
    ///
    /// # 参数
    /// - `request`: 创建或更新请求
    ///
    /// # 返回
    /// - `CreateOrUpdateMachineConfigResponse`: 响应
    ///
    /// # 说明
    /// - 如果 (version_id, machine_code) 已存在，则更新；否则创建
    /// - 自动记录 ActionLog
    pub fn create_or_update_machine_config(
        &self,
        request: CreateOrUpdateMachineConfigRequest,
    ) -> ApiResult<CreateOrUpdateMachineConfigResponse> {
        // 参数验证
        self.validate_create_or_update_request(&request)?;

        // 创建实体
        let entity = MachineConfigEntity::new(
            request.version_id.clone(),
            request.machine_code.clone(),
            request.default_daily_target_t,
            request.default_daily_limit_pct,
            request.operator.clone(),
            Some(request.reason.clone()),
            request.effective_date.clone(),
        );

        let config_id = entity.config_id.clone();

        // 执行 upsert
        self.machine_config_repo.upsert(&entity)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录 ActionLog
        let action_desc = format!(
            "机组配置 [{}] 更新: 目标产能={:.3}t/天, 极限产能={:.2}%, 原因: {}",
            request.machine_code,
            request.default_daily_target_t,
            request.default_daily_limit_pct * 100.0,
            request.reason
        );

        self.log_action(
            &request.operator,
            "UPDATE_MACHINE_CONFIG",
            &action_desc,
        )?;

        Ok(CreateOrUpdateMachineConfigResponse {
            success: true,
            config_id,
            message: format!("机组配置 [{}] 已成功更新", request.machine_code),
        })
    }

    /// 批量应用机组配置到产能池日期范围
    ///
    /// # 参数
    /// - `request`: 应用配置请求
    ///
    /// # 返回
    /// - `ApplyConfigToDateRangeResponse`: 响应
    ///
    /// # 说明
    /// - 将机组配置应用到指定日期范围的 capacity_pool 记录
    /// - 跳过已有 used_capacity_t > 0 的记录（避免覆盖实际已用产能）
    /// - 自动记录 ActionLog
    pub fn apply_machine_config_to_dates(
        &self,
        request: ApplyConfigToDateRangeRequest,
    ) -> ApiResult<ApplyConfigToDateRangeResponse> {
        // 参数验证
        self.validate_apply_request(&request)?;

        // 查询机组配置
        let config = self.machine_config_repo
            .find_by_key(&request.version_id, &request.machine_code)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| {
                ApiError::NotFound(format!(
                    "未找到机组 [{}] 在版本 [{}] 下的配置",
                    request.machine_code, request.version_id
                ))
            })?;

        // 解析日期范围
        let date_from = chrono::NaiveDate::parse_from_str(&request.date_from, "%Y-%m-%d")
            .map_err(|_| ApiError::InvalidInput("起始日期格式错误（应为 YYYY-MM-DD）".to_string()))?;
        let date_to = chrono::NaiveDate::parse_from_str(&request.date_to, "%Y-%m-%d")
            .map_err(|_| ApiError::InvalidInput("结束日期格式错误（应为 YYYY-MM-DD）".to_string()))?;

        if date_from > date_to {
            return Err(ApiError::InvalidInput("起始日期不能晚于结束日期".to_string()));
        }

        // 计算产能值
        let target_capacity_t = config.default_daily_target_t;
        let limit_capacity_t = config.default_daily_target_t * config.default_daily_limit_pct;

        // 批量更新产能池
        let mut updated_count = 0;
        let mut skipped_count = 0;
        let mut current_date = date_from;

        while current_date <= date_to {
            // 查询当前日期的产能池记录
            let pool_opt = self.capacity_repo
                .find_by_machine_and_date(&request.version_id, &request.machine_code, current_date)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

            match pool_opt {
                Some(pool) if pool.used_capacity_t > 1e-6 => {
                    // 跳过已有已用产能的记录（避免覆盖实际数据）
                    skipped_count += 1;
                }
                Some(mut pool) => {
                    // 更新产能值
                    pool.target_capacity_t = target_capacity_t;
                    pool.limit_capacity_t = limit_capacity_t;

                    self.capacity_repo.upsert_single(&pool)
                        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

                    updated_count += 1;
                }
                None => {
                    // 如果不存在，创建新记录
                    // 注意：这里可能需要从 capacity_repo 的 Entity 定义中创建
                    // 暂时跳过，因为通常产能池记录应该由版本创建时初始化
                    skipped_count += 1;
                }
            }

            current_date = current_date
                .succ_opt()
                .ok_or_else(|| ApiError::InternalError("日期递增失败".to_string()))?;
        }

        // 记录 ActionLog
        let action_desc = format!(
            "应用机组配置 [{}] 到日期范围 [{}~{}]: 更新 {} 条, 跳过 {} 条, 原因: {}",
            request.machine_code,
            request.date_from,
            request.date_to,
            updated_count,
            skipped_count,
            request.reason
        );

        self.log_action(
            &request.operator,
            "APPLY_MACHINE_CONFIG_TO_DATES",
            &action_desc,
        )?;

        Ok(ApplyConfigToDateRangeResponse {
            success: true,
            updated_count,
            skipped_count,
            message: format!(
                "已成功应用配置到 {} 条记录，跳过 {} 条已有数据的记录",
                updated_count, skipped_count
            ),
        })
    }

    /// 查询机组配置历史（跨版本）
    ///
    /// # 参数
    /// - `machine_code`: 机组代码
    /// - `limit`: 可选的限制条数
    ///
    /// # 返回
    /// - `Vec<MachineConfigDto>`: 历史配置列表（按创建时间倒序）
    pub fn get_machine_config_history(
        &self,
        machine_code: &str,
        limit: Option<usize>,
    ) -> ApiResult<Vec<MachineConfigDto>> {
        let history = self.machine_config_repo
            .list_history_by_machine(machine_code, limit)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(history.into_iter().map(Into::into).collect())
    }

    // ==========================================
    // 私有辅助方法
    // ==========================================

    /// 验证创建或更新请求
    fn validate_create_or_update_request(&self, request: &CreateOrUpdateMachineConfigRequest) -> ApiResult<()> {
        if request.version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("version_id 不能为空".to_string()));
        }

        if request.machine_code.trim().is_empty() {
            return Err(ApiError::InvalidInput("machine_code 不能为空".to_string()));
        }

        if request.default_daily_target_t <= 0.0 {
            return Err(ApiError::InvalidInput("default_daily_target_t 必须大于 0".to_string()));
        }

        if request.default_daily_limit_pct < 1.0 {
            return Err(ApiError::InvalidInput("default_daily_limit_pct 必须 >= 1.0 (100%)".to_string()));
        }

        if request.reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("reason 不能为空（必须填写配置原因）".to_string()));
        }

        if request.operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("operator 不能为空".to_string()));
        }

        // 验证生效日期格式（如果提供）
        if let Some(ref date_str) = request.effective_date {
            chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .map_err(|_| ApiError::InvalidInput("effective_date 格式错误（应为 YYYY-MM-DD）".to_string()))?;
        }

        Ok(())
    }

    /// 验证应用配置请求
    fn validate_apply_request(&self, request: &ApplyConfigToDateRangeRequest) -> ApiResult<()> {
        if request.version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("version_id 不能为空".to_string()));
        }

        if request.machine_code.trim().is_empty() {
            return Err(ApiError::InvalidInput("machine_code 不能为空".to_string()));
        }

        if request.date_from.trim().is_empty() || request.date_to.trim().is_empty() {
            return Err(ApiError::InvalidInput("date_from 和 date_to 不能为空".to_string()));
        }

        if request.reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("reason 不能为空".to_string()));
        }

        if request.operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("operator 不能为空".to_string()));
        }

        Ok(())
    }

    /// 记录操作日志
    fn log_action(&self, operator: &str, action_type: &str, description: &str) -> ApiResult<()> {
        let log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None,
            action_type: action_type.to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: None,
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(description.to_string()),
        };

        self.action_log_repo.insert(&log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

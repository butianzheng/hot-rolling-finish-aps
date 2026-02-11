// ==========================================
// 热轧精整排产系统 - 换辊管理 API
// ==========================================
// 职责: 换辊窗口查询、管理
// 依据: Engine_Specs_v0.3_Integrated.md - 7. Roll Campaign Engine
// ==========================================

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::error::{ApiError, ApiResult};
use crate::config::{config_keys, ConfigManager};
use crate::domain::action_log::ActionLog;
use crate::domain::roller::{RollerCampaign, RollerCampaignMonitor};
use crate::repository::action_log_repo::ActionLogRepository;
use crate::repository::roll_campaign_plan_repo::{
    RollCampaignPlanEntity, RollCampaignPlanRepository,
};
use crate::repository::roller_repo::RollerCampaignRepository;

// ==========================================
// RollerApi - 换辊管理 API
// ==========================================

/// 换辊管理API
///
/// 职责：
/// 1. 换辊窗口查询（按版本、按机组）
/// 2. 换辊窗口管理（创建、结束）
/// 3. 累计吨位更新
/// 4. ActionLog记录
pub struct RollerApi {
    roller_repo: Arc<RollerCampaignRepository>,
    roll_plan_repo: Arc<RollCampaignPlanRepository>,
    action_log_repo: Arc<ActionLogRepository>,
    config_manager: Arc<ConfigManager>,
}

impl RollerApi {
    /// 创建新的RollerApi实例
    pub fn new(
        roller_repo: Arc<RollerCampaignRepository>,
        roll_plan_repo: Arc<RollCampaignPlanRepository>,
        action_log_repo: Arc<ActionLogRepository>,
        config_manager: Arc<ConfigManager>,
    ) -> Self {
        Self {
            roller_repo,
            roll_plan_repo,
            action_log_repo,
            config_manager,
        }
    }

    fn normalize_datetime_str(value: &str) -> ApiResult<String> {
        let raw = value.trim();
        if raw.is_empty() {
            return Err(ApiError::InvalidInput("日期时间不能为空".to_string()));
        }

        // Common formats from UI (dayjs) and API usage.
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S") {
            return Ok(dt.format("%Y-%m-%d %H:%M:%S").to_string());
        }
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M") {
            return Ok(dt.format("%Y-%m-%d %H:%M:%S").to_string());
        }
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S") {
            return Ok(dt.format("%Y-%m-%d %H:%M:%S").to_string());
        }
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw) {
            return Ok(dt.naive_local().format("%Y-%m-%d %H:%M:%S").to_string());
        }

        Err(ApiError::InvalidInput(
            "日期时间格式错误（应为 YYYY-MM-DD HH:MM[:SS] 或 RFC3339）".to_string(),
        ))
    }

    fn format_datetime_for_ipc(value: &str) -> String {
        let raw = value.trim();
        if raw.is_empty() {
            return String::new();
        }

        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S") {
            return dt.format("%Y-%m-%dT%H:%M:%S").to_string();
        }
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M") {
            return dt.format("%Y-%m-%dT%H:%M:%S").to_string();
        }
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S") {
            return dt.format("%Y-%m-%dT%H:%M:%S").to_string();
        }
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M") {
            return dt.format("%Y-%m-%dT%H:%M:%S").to_string();
        }
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw) {
            return dt.naive_local().format("%Y-%m-%dT%H:%M:%S").to_string();
        }

        raw.replace(' ', "T")
    }

    /// 查询版本的所有换辊窗口
    ///
    /// # 参数
    /// - version_id: 版本ID
    ///
    /// # 返回
    /// - Ok(Vec<RollerCampaignInfo>): 换辊窗口列表
    /// - Err(ApiError): API错误
    pub fn list_campaigns(&self, version_id: &str) -> ApiResult<Vec<RollerCampaignInfo>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        let campaigns = self
            .roller_repo
            .find_by_version_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(campaigns
            .into_iter()
            .map(RollerCampaignInfo::from)
            .collect())
    }

    /// 查询机组当前进行中的换辊窗口
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - machine_code: 机组代码
    ///
    /// # 返回
    /// - Ok(Some(RollerCampaignInfo)): 进行中的换辊窗口
    /// - Ok(None): 未找到
    /// - Err(ApiError): API错误
    pub fn get_active_campaign(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> ApiResult<Option<RollerCampaignInfo>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if machine_code.trim().is_empty() {
            return Err(ApiError::InvalidInput("机组代码不能为空".to_string()));
        }

        let campaign = self
            .roller_repo
            .find_active_campaign(version_id, machine_code)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(campaign.map(RollerCampaignInfo::from))
    }

    /// 查询需要换辊的机组列表
    ///
    /// # 参数
    /// - version_id: 版本ID
    ///
    /// # 返回
    /// - Ok(Vec<RollerCampaignInfo>): 需要换辊的换辊窗口列表
    /// - Err(ApiError): API错误
    pub fn list_needs_roll_change(&self, version_id: &str) -> ApiResult<Vec<RollerCampaignInfo>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        let campaigns = self
            .roller_repo
            .find_needs_roll_change(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(campaigns
            .into_iter()
            .map(RollerCampaignInfo::from)
            .collect())
    }

    // ==========================================
    // 换辊时间监控/微调 (计划)
    // ==========================================

    pub fn list_campaign_plans(&self, version_id: &str) -> ApiResult<Vec<RollCampaignPlanInfo>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        let plans = self
            .roll_plan_repo
            .list_by_version_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(plans.into_iter().map(RollCampaignPlanInfo::from).collect())
    }

    pub fn upsert_campaign_plan(
        &self,
        version_id: &str,
        machine_code: &str,
        initial_start_at: &str,
        next_change_at: Option<&str>,
        downtime_minutes: Option<i32>,
        operator: &str,
        reason: &str,
    ) -> ApiResult<()> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if machine_code.trim().is_empty() {
            return Err(ApiError::InvalidInput("机组代码不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        let initial_start_at = Self::normalize_datetime_str(initial_start_at)?;
        let next_change_at = match next_change_at {
            Some(v) if !v.trim().is_empty() => Some(Self::normalize_datetime_str(v)?),
            _ => None,
        };

        if let Some(m) = downtime_minutes {
            if m <= 0 || m > 24 * 60 {
                return Err(ApiError::InvalidInput(
                    "停机时长需在 1~1440 分钟之间".to_string(),
                ));
            }
        }

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let entity = RollCampaignPlanEntity {
            version_id: version_id.to_string(),
            machine_code: machine_code.to_string(),
            initial_start_at: initial_start_at.clone(),
            next_change_at: next_change_at.clone(),
            downtime_minutes,
            updated_at: now.clone(),
            updated_by: Some(operator.to_string()),
        };

        self.roll_plan_repo
            .upsert(&entity)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog（审计）
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version_id.to_string()),
            action_type: "UPSERT_ROLL_CAMPAIGN_PLAN".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "machine_code": machine_code,
                "initial_start_at": initial_start_at,
                "next_change_at": next_change_at,
                "downtime_minutes": downtime_minutes,
                "reason": reason,
            })),
            impact_summary_json: None,
            machine_code: Some(machine_code.to_string()),
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!(
                "更新换辊计划: {} (停机 {} 分钟)",
                machine_code,
                downtime_minutes.unwrap_or(0)
            )),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// 创建新的换辊窗口
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - machine_code: 机组代码
    /// - campaign_no: 换辊批次号
    /// - start_date: 开始日期
    /// - suggest_threshold_t: 建议换辊阈值（可选）
    /// - hard_limit_t: 强制换辊阈值（可选）
    /// - operator: 操作人
    /// - reason: 操作原因
    ///
    /// # 返回
    /// - Ok(()): 成功
    /// - Err(ApiError): API错误
    pub fn create_campaign(
        &self,
        version_id: &str,
        machine_code: &str,
        campaign_no: i32,
        start_date: NaiveDate,
        suggest_threshold_t: Option<f64>,
        hard_limit_t: Option<f64>,
        operator: &str,
        reason: &str,
    ) -> ApiResult<()> {
        // 参数验证
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if machine_code.trim().is_empty() {
            return Err(ApiError::InvalidInput("机组代码不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        let default_suggest_threshold_t = self
            .config_manager
            .get_global_config_value(config_keys::ROLL_SUGGEST_THRESHOLD_T)
            .ok()
            .flatten()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| *v > 0.0)
            .unwrap_or(1500.0);

        let default_hard_limit_t = self
            .config_manager
            .get_global_config_value(config_keys::ROLL_HARD_LIMIT_T)
            .ok()
            .flatten()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| *v > 0.0)
            .unwrap_or(2500.0);

        let effective_suggest_threshold_t =
            suggest_threshold_t.unwrap_or(default_suggest_threshold_t);
        let effective_hard_limit_t = hard_limit_t.unwrap_or(default_hard_limit_t);

        if effective_suggest_threshold_t <= 0.0 {
            return Err(ApiError::InvalidInput("建议换辊阈值必须大于0".to_string()));
        }
        if effective_hard_limit_t <= 0.0 {
            return Err(ApiError::InvalidInput("强制换辊阈值必须大于0".to_string()));
        }
        if effective_hard_limit_t <= effective_suggest_threshold_t {
            return Err(ApiError::InvalidInput(
                "强制换辊阈值必须大于建议换辊阈值".to_string(),
            ));
        }

        // 创建换辊窗口
        let campaign = RollerCampaign::new(
            version_id.to_string(),
            machine_code.to_string(),
            campaign_no,
            start_date,
            Some(effective_suggest_threshold_t),
            Some(effective_hard_limit_t),
        );

        self.roller_repo
            .create(&campaign)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version_id.to_string()),
            action_type: "CREATE_ROLL_CAMPAIGN".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "machine_code": machine_code,
                "campaign_no": campaign_no,
                "start_date": start_date.to_string(),
                "suggest_threshold_t": effective_suggest_threshold_t,
                "hard_limit_t": effective_hard_limit_t,
                "reason": reason,
            })),
            impact_summary_json: None,
            machine_code: Some(machine_code.to_string()),
            date_range_start: Some(start_date),
            date_range_end: None,
            detail: Some(format!("创建换辊窗口: {}批次{}", machine_code, campaign_no)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// 结束换辊窗口
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - machine_code: 机组代码
    /// - campaign_no: 换辊批次号
    /// - end_date: 结束日期
    /// - operator: 操作人
    /// - reason: 操作原因
    ///
    /// # 返回
    /// - Ok(()): 成功
    /// - Err(ApiError): API错误
    pub fn close_campaign(
        &self,
        version_id: &str,
        machine_code: &str,
        campaign_no: i32,
        end_date: NaiveDate,
        operator: &str,
        reason: &str,
    ) -> ApiResult<()> {
        // 参数验证
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if machine_code.trim().is_empty() {
            return Err(ApiError::InvalidInput("机组代码不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        self.roller_repo
            .close_campaign(version_id, machine_code, campaign_no, end_date)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version_id.to_string()),
            action_type: "CLOSE_ROLL_CAMPAIGN".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "machine_code": machine_code,
                "campaign_no": campaign_no,
                "end_date": end_date.to_string(),
                "reason": reason,
            })),
            impact_summary_json: None,
            machine_code: Some(machine_code.to_string()),
            date_range_start: None,
            date_range_end: Some(end_date),
            detail: Some(format!("结束换辊窗口: {}批次{}", machine_code, campaign_no)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

// ==========================================
// DTO 类型定义
// ==========================================

/// 换辊时间监控计划（按版本+机组）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollCampaignPlanInfo {
    pub version_id: String,
    pub machine_code: String,
    pub initial_start_at: String,
    pub next_change_at: Option<String>,
    pub downtime_minutes: Option<i32>,
    pub updated_at: String,
    pub updated_by: Option<String>,
}

impl From<RollCampaignPlanEntity> for RollCampaignPlanInfo {
    fn from(v: RollCampaignPlanEntity) -> Self {
        Self {
            version_id: v.version_id,
            machine_code: v.machine_code,
            initial_start_at: RollerApi::format_datetime_for_ipc(&v.initial_start_at),
            next_change_at: v
                .next_change_at
                .as_deref()
                .map(RollerApi::format_datetime_for_ipc),
            downtime_minutes: v.downtime_minutes,
            updated_at: RollerApi::format_datetime_for_ipc(&v.updated_at),
            updated_by: v.updated_by,
        }
    }
}

/// 换辊窗口信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollerCampaignInfo {
    /// 版本ID
    pub version_id: String,

    /// 机组代码
    pub machine_code: String,

    /// 换辊批次号
    pub campaign_no: i32,

    /// 开始日期
    pub start_date: String,

    /// 结束日期
    pub end_date: Option<String>,

    /// 累计吨位
    pub cum_weight_t: f64,

    /// 建议换辊阈值
    pub suggest_threshold_t: f64,

    /// 强制换辊阈值
    pub hard_limit_t: f64,

    /// 换辊状态
    pub status: String,

    /// 是否进行中
    pub is_active: bool,

    /// 剩余可用吨位
    pub remaining_tonnage_t: f64,

    /// 辊使用率
    pub utilization_ratio: f64,

    /// 是否需要换辊（建议）
    pub should_change_roll: bool,

    /// 是否强制换辊
    pub is_hard_stop: bool,
}

impl From<RollerCampaign> for RollerCampaignInfo {
    fn from(campaign: RollerCampaign) -> Self {
        Self {
            version_id: campaign.version_id.clone(),
            machine_code: campaign.machine_code.clone(),
            campaign_no: campaign.campaign_no,
            start_date: campaign.start_date.to_string(),
            end_date: campaign.end_date.map(|d| d.to_string()),
            cum_weight_t: campaign.cum_weight_t,
            suggest_threshold_t: campaign.suggest_threshold_t,
            hard_limit_t: campaign.hard_limit_t,
            status: format!("{:?}", campaign.status),
            is_active: campaign.is_active(),
            remaining_tonnage_t: campaign.remaining_tonnage_t(),
            utilization_ratio: campaign.utilization_ratio(),
            should_change_roll: campaign.should_change_roll(),
            is_hard_stop: campaign.is_hard_stop(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roller_api_structure() {
        // 这个测试只是验证结构是否正确定义
        // 实际的集成测试在 tests/ 目录
    }

    #[test]
    fn test_roll_campaign_plan_info_datetime_for_ipc() {
        let dto = RollCampaignPlanInfo::from(RollCampaignPlanEntity {
            version_id: "v1".to_string(),
            machine_code: "H032".to_string(),
            initial_start_at: "2026-02-10 08:30:00".to_string(),
            next_change_at: Some("2026-02-11 09:40".to_string()),
            downtime_minutes: Some(45),
            updated_at: "2026-02-10 10:11:12".to_string(),
            updated_by: Some("tester".to_string()),
        });

        assert_eq!(dto.initial_start_at, "2026-02-10T08:30:00");
        assert_eq!(dto.next_change_at.as_deref(), Some("2026-02-11T09:40:00"));
        assert_eq!(dto.updated_at, "2026-02-10T10:11:12");
    }
}

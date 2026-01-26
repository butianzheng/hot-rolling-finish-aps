// ==========================================
// 热轧精整排产系统 - 换辊管理 API
// ==========================================
// 职责: 换辊窗口查询、管理
// 依据: Engine_Specs_v0.3_Integrated.md - 7. Roll Campaign Engine
// ==========================================

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use chrono::NaiveDate;

use crate::api::error::{ApiError, ApiResult};
use crate::domain::roller::{RollerCampaign, RollerCampaignMonitor};
use crate::domain::action_log::ActionLog;
use crate::repository::roller_repo::RollerCampaignRepository;
use crate::repository::action_log_repo::ActionLogRepository;

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
    action_log_repo: Arc<ActionLogRepository>,
}

impl RollerApi {
    /// 创建新的RollerApi实例
    pub fn new(
        roller_repo: Arc<RollerCampaignRepository>,
        action_log_repo: Arc<ActionLogRepository>,
    ) -> Self {
        Self {
            roller_repo,
            action_log_repo,
        }
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

        Ok(campaigns.into_iter().map(RollerCampaignInfo::from).collect())
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

        Ok(campaigns.into_iter().map(RollerCampaignInfo::from).collect())
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

        // 创建换辊窗口
        let campaign = RollerCampaign::new(
            version_id.to_string(),
            machine_code.to_string(),
            campaign_no,
            start_date,
            suggest_threshold_t,
            hard_limit_t,
        );

        self.roller_repo
            .create(&campaign)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: version_id.to_string(),
            action_type: "CREATE_ROLL_CAMPAIGN".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "machine_code": machine_code,
                "campaign_no": campaign_no,
                "start_date": start_date.to_string(),
                "suggest_threshold_t": suggest_threshold_t,
                "hard_limit_t": hard_limit_t,
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
            version_id: version_id.to_string(),
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
}

// ==========================================
// 热轧精整排产系统 - 驾驶舱 API
// ==========================================
// 职责: 封装 DecisionApi，提供驾驶舱聚合查询和操作日志查询
// 红线合规: 红线3（分层紧急度）
// 依据: 实施计划 Phase 4 + Master Spec PART G
// 架构: API 层 → Decision 层 (DecisionApi) → Use Case 层
// ==========================================

use std::sync::Arc;
use chrono::{NaiveDate, NaiveDateTime};

use crate::api::error::{ApiError, ApiResult};
use crate::domain::action_log::ActionLog;
use crate::repository::action_log_repo::ActionLogRepository;

// 导入 DecisionApi（P2 阶段完成的决策层）
use crate::decision::api::decision_api::DecisionApi;
use crate::decision::api::dto::{
    GetDecisionDaySummaryRequest, DecisionDaySummaryResponse,
    GetMachineBottleneckProfileRequest, MachineBottleneckProfileResponse,
    ListOrderFailureSetRequest, OrderFailureSetResponse,
    GetColdStockProfileRequest, ColdStockProfileResponse,
};

// ==========================================
// DashboardApi - 驾驶舱 API
// ==========================================

/// 驾驶舱API
///
/// 职责：
/// 1. 封装 DecisionApi，提供决策查询（D1-D6）
/// 2. 操作日志查询
/// 3. 聚合接口（如需要）
///
/// 架构说明：
/// - DashboardAPI 是前端驾驶舱的专用 API 层
/// - 内部委托给 DecisionApi（已通过 10/10 端到端测试）
/// - 复用 P2 阶段已验证的业务逻辑
pub struct DashboardApi {
    /// DecisionApi 实例（封装 D1-D6 决策逻辑）
    decision_api: Arc<dyn DecisionApi>,
    /// 操作日志 Repository（DecisionApi 不包含此功能）
    action_log_repo: Arc<ActionLogRepository>,
}

impl DashboardApi {
    /// 创建新的DashboardApi实例
    ///
    /// # 参数
    /// - decision_api: DecisionApi 实例（封装 D1-D6 决策用例）
    /// - action_log_repo: 操作日志 Repository
    pub fn new(
        decision_api: Arc<dyn DecisionApi>,
        action_log_repo: Arc<ActionLogRepository>,
    ) -> Self {
        Self {
            decision_api,
            action_log_repo,
        }
    }

    // ==========================================
    // 风险快照查询接口（向后兼容）
    // ==========================================
    // 注意：这些方法是为了向后兼容 Tauri 命令层
    // 实际的决策逻辑已迁移到 DecisionApi
    // TODO: 考虑在未来版本中移除这些方法，统一使用 DecisionApi
    // ==========================================

    /// 查询风险快照（按版本）
    ///
    /// # 参数
    /// - version_id: 版本ID
    ///
    /// # 返回
    /// - Ok(DecisionDaySummaryResponse): 日期风险摘要（使用 DecisionApi D1）
    /// - Err(ApiError): API错误
    ///
    /// # 向后兼容说明
    /// 此方法为向后兼容保留，内部委托给 DecisionApi D1
    pub fn list_risk_snapshots(&self, version_id: &str) -> ApiResult<DecisionDaySummaryResponse> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        // 委托给 D1: 查询 30 天的日期风险摘要
        let today = chrono::Local::now().date_naive();
        let date_from = today.to_string();
        let date_to = (today + chrono::Duration::days(30)).to_string();

        let request = GetDecisionDaySummaryRequest {
            version_id: version_id.to_string(),
            date_from,
            date_to,
            risk_level_filter: None,
            limit: None,
            sort_by: Some("risk_score".to_string()),
        };

        self.decision_api
            .get_decision_day_summary(request)
            .map_err(|e| ApiError::DatabaseError(e))
    }

    /// 查询风险快照（按日期）
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - snapshot_date: 快照日期
    ///
    /// # 返回
    /// - Ok(DecisionDaySummaryResponse): 指定日期的风险摘要
    /// - Err(ApiError): API错误
    ///
    /// # 向后兼容说明
    /// 此方法为向后兼容保留，内部委托给 DecisionApi D1
    pub fn get_risk_snapshot(
        &self,
        version_id: &str,
        snapshot_date: NaiveDate,
    ) -> ApiResult<DecisionDaySummaryResponse> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        // 委托给 D1: 查询指定日期的风险摘要
        let date_str = snapshot_date.to_string();

        let request = GetDecisionDaySummaryRequest {
            version_id: version_id.to_string(),
            date_from: date_str.clone(),
            date_to: date_str,
            risk_level_filter: None,
            limit: None,
            sort_by: Some("risk_score".to_string()),
        };

        self.decision_api
            .get_decision_day_summary(request)
            .map_err(|e| ApiError::DatabaseError(e))
    }

    // ==========================================
    // 决策查询接口 - 委托给 DecisionApi (D1-D6)
    // ==========================================

    /// D1: 哪天最危险（简化版本，向后兼容 Tauri 命令）
    ///
    /// 依据: Master Spec PART G - "哪天最危险"
    ///
    /// # 参数
    /// - version_id: 版本ID
    ///
    /// # 返回
    /// - Ok(DecisionDaySummaryResponse): 日期风险摘要响应（未来 30 天）
    /// - Err(ApiError): API错误
    ///
    /// # 向后兼容说明
    /// 此方法为向后兼容 Tauri 命令层保留，查询未来 30 天的风险摘要
    pub fn get_most_risky_date(
        &self,
        version_id: &str,
    ) -> ApiResult<DecisionDaySummaryResponse> {
        self.get_most_risky_date_full(version_id, None, None, None, Some(10))
    }

    /// D1: 哪天最危险（完整参数版本）
    ///
    /// 依据: Master Spec PART G - "哪天最危险"
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - date_from: 开始日期（可选，默认为今天）
    /// - date_to: 结束日期（可选，默认为 30 天后）
    /// - risk_level_filter: 风险等级过滤（可选）
    /// - limit: 返回记录数限制（可选）
    ///
    /// # 返回
    /// - Ok(DecisionDaySummaryResponse): 日期风险摘要响应
    /// - Err(ApiError): API错误
    pub fn get_most_risky_date_full(
        &self,
        version_id: &str,
        date_from: Option<&str>,
        date_to: Option<&str>,
        risk_level_filter: Option<Vec<String>>,
        limit: Option<u32>,
    ) -> ApiResult<DecisionDaySummaryResponse> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        // 默认日期范围：今天到 30 天后
        let today = chrono::Local::now().date_naive();
        let default_date_from = today.to_string();
        let default_date_to = (today + chrono::Duration::days(30)).to_string();

        let request = GetDecisionDaySummaryRequest {
            version_id: version_id.to_string(),
            date_from: date_from.unwrap_or(&default_date_from).to_string(),
            date_to: date_to.unwrap_or(&default_date_to).to_string(),
            risk_level_filter,
            limit,
            sort_by: Some("risk_score".to_string()), // 按风险分数降序
        };

        self.decision_api
            .get_decision_day_summary(request)
            .map_err(|e| ApiError::DatabaseError(e))
    }

    /// D2: 哪些紧急单无法完成（简化版本，向后兼容 Tauri 命令）
    ///
    /// 依据: Master Spec PART G - "哪些紧急单无法完成"
    ///
    /// # 参数
    /// - version_id: 版本ID（可选）
    ///
    /// # 返回
    /// - Ok(OrderFailureSetResponse): 订单失败集合响应
    /// - Err(ApiError): API错误
    ///
    /// # 向后兼容说明
    /// 此方法为向后兼容 Tauri 命令层保留
    pub fn get_unsatisfied_urgent_materials(
        &self,
        version_id: Option<&str>,
    ) -> ApiResult<OrderFailureSetResponse> {
        let version_id = version_id.ok_or_else(|| {
            ApiError::InvalidInput("版本ID不能为空".to_string())
        })?;

        self.get_unsatisfied_urgent_materials_full(version_id, None, None, Some(100))
    }

    /// D2: 哪些紧急单无法完成（完整参数版本）
    ///
    /// 依据: Master Spec PART G - "哪些紧急单无法完成"
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - fail_type_filter: 失败类型过滤（可选）
    /// - urgency_level_filter: 紧急等级过滤（可选）
    /// - limit: 返回记录数限制（可选）
    ///
    /// # 返回
    /// - Ok(OrderFailureSetResponse): 订单失败集合响应
    /// - Err(ApiError): API错误
    pub fn get_unsatisfied_urgent_materials_full(
        &self,
        version_id: &str,
        fail_type_filter: Option<Vec<String>>,
        urgency_level_filter: Option<Vec<String>>,
        limit: Option<u32>,
    ) -> ApiResult<OrderFailureSetResponse> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        let request = ListOrderFailureSetRequest {
            version_id: version_id.to_string(),
            fail_type_filter,
            urgency_level_filter,
            machine_codes: None,
            due_date_from: None,
            due_date_to: None,
            completion_rate_threshold: None,
            offset: None,
            limit,
        };

        self.decision_api
            .list_order_failure_set(request)
            .map_err(|e| ApiError::DatabaseError(e))
    }

    /// D3: 哪些冷料压库（简化版本，向后兼容 Tauri 命令）
    ///
    /// 依据: Master Spec PART G - "哪些冷料压库"
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - threshold_days: 阈值天数（未使用，为向后兼容保留）
    ///
    /// # 返回
    /// - Ok(ColdStockProfileResponse): 冷料压库概况响应
    /// - Err(ApiError): API错误
    ///
    /// # 向后兼容说明
    /// 此方法为向后兼容 Tauri 命令层保留
    /// threshold_days 参数未使用，因为 DecisionApi 使用不同的查询逻辑
    pub fn get_cold_stock_materials(
        &self,
        version_id: &str,
        _threshold_days: i32,
    ) -> ApiResult<ColdStockProfileResponse> {
        self.get_cold_stock_materials_full(version_id, None, None, Some(100))
    }

    /// D3: 哪些冷料压库（完整参数版本）
    ///
    /// 依据: Master Spec PART G - "哪些冷料压库"
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - machine_codes: 机组代码过滤（可选）
    /// - pressure_level_filter: 压库等级过滤（可选）
    /// - limit: 返回记录数限制（可选）
    ///
    /// # 返回
    /// - Ok(ColdStockProfileResponse): 冷料压库概况响应
    /// - Err(ApiError): API错误
    pub fn get_cold_stock_materials_full(
        &self,
        version_id: &str,
        machine_codes: Option<Vec<String>>,
        pressure_level_filter: Option<Vec<String>>,
        limit: Option<u32>,
    ) -> ApiResult<ColdStockProfileResponse> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        let request = GetColdStockProfileRequest {
            version_id: version_id.to_string(),
            machine_codes,
            pressure_level_filter,
            age_bin_filter: None,
            limit,
        };

        self.decision_api
            .get_cold_stock_profile(request)
            .map_err(|e| ApiError::DatabaseError(e))
    }

    /// D4: 哪个机组最堵（简化版本，向后兼容 Tauri 命令）
    ///
    /// 依据: Master Spec PART G - "哪个机组最堵"
    ///
    /// # 参数
    /// - version_id: 版本ID
    ///
    /// # 返回
    /// - Ok(MachineBottleneckProfileResponse): 机组堵塞概况响应（未来 7 天）
    /// - Err(ApiError): API错误
    ///
    /// # 向后兼容说明
    /// 此方法为向后兼容 Tauri 命令层保留，查询未来 7 天的机组堵塞情况
    pub fn get_most_congested_machine(
        &self,
        version_id: &str,
    ) -> ApiResult<MachineBottleneckProfileResponse> {
        // 使用默认日期范围（今天到 7 天后）
        let today = chrono::Local::now().date_naive();
        let date_from = today.to_string();
        let date_to = (today + chrono::Duration::days(7)).to_string();

        self.get_most_congested_machine_full(
            version_id,
            Some(&date_from),
            Some(&date_to),
            None,
            None,
            Some(10),
        )
    }

    /// D4: 哪个机组最堵（完整参数版本）
    ///
    /// 依据: Master Spec PART G - "哪个机组最堵"
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - date_from: 开始日期（可选，默认为今天）
    /// - date_to: 结束日期（可选，默认为 7 天后）
    /// - machine_codes: 机组代码过滤（可选）
    /// - bottleneck_level_filter: 堵塞等级过滤（可选）
    /// - limit: 返回记录数限制（可选）
    ///
    /// # 返回
    /// - Ok(MachineBottleneckProfileResponse): 机组堵塞概况响应
    /// - Err(ApiError): API错误
    pub fn get_most_congested_machine_full(
        &self,
        version_id: &str,
        date_from: Option<&str>,
        date_to: Option<&str>,
        machine_codes: Option<Vec<String>>,
        bottleneck_level_filter: Option<Vec<String>>,
        limit: Option<u32>,
    ) -> ApiResult<MachineBottleneckProfileResponse> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        // 默认日期范围：今天到 7 天后
        let today = chrono::Local::now().date_naive();
        let default_date_from = today.to_string();
        let default_date_to = (today + chrono::Duration::days(7)).to_string();

        let request = GetMachineBottleneckProfileRequest {
            version_id: version_id.to_string(),
            date_from: date_from.unwrap_or(&default_date_from).to_string(),
            date_to: date_to.unwrap_or(&default_date_to).to_string(),
            machine_codes,
            bottleneck_level_filter,
            bottleneck_type_filter: None,
            limit,
        };

        self.decision_api
            .get_machine_bottleneck_profile(request)
            .map_err(|e| ApiError::DatabaseError(e))
    }

    // ==========================================
    // 操作日志查询接口
    // ==========================================

    /// 查询操作日志（按时间范围）
    ///
    /// # 参数
    /// - start_time: 开始时间
    /// - end_time: 结束时间
    ///
    /// # 返回
    /// - Ok(Vec<ActionLog>): 操作日志列表
    /// - Err(ApiError): API错误
    pub fn list_action_logs(
        &self,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
    ) -> ApiResult<Vec<ActionLog>> {
        // 参数验证
        if start_time > end_time {
            return Err(ApiError::InvalidInput(
                "开始时间不能晚于结束时间".to_string(),
            ));
        }

        self.action_log_repo
            .find_by_time_range(start_time, end_time)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 查询操作日志（按版本）
    ///
    /// # 参数
    /// - version_id: 版本ID
    ///
    /// # 返回
    /// - Ok(Vec<ActionLog>): 操作日志列表
    /// - Err(ApiError): API错误
    pub fn list_action_logs_by_version(&self, version_id: &str) -> ApiResult<Vec<ActionLog>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        self.action_log_repo
            .find_by_version_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 查询最近操作
    ///
    /// # 参数
    /// - limit: 返回记录数上限
    ///
    /// # 返回
    /// - Ok(Vec<ActionLog>): 操作日志列表
    /// - Err(ApiError): API错误
    pub fn get_recent_actions(&self, limit: i32) -> ApiResult<Vec<ActionLog>> {
        if limit <= 0 || limit > 1000 {
            return Err(ApiError::InvalidInput(
                "limit必须在1-1000之间".to_string(),
            ));
        }

        self.action_log_repo
            .find_recent(limit)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }
}

// ==========================================
// DTO 类型定义
// ==========================================
// 注意: 决策查询的响应类型使用 DecisionApi 的 DTO
// 此处仅定义 DashboardAPI 特有的 DTO（如操作日志相关）
// ==========================================

// 操作日志相关 DTO 由 ActionLog domain 对象提供，无需额外定义

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_api_structure() {
        // 这个测试只是验证结构是否正确定义
        // 实际的集成测试在 tests/ 目录
    }
}

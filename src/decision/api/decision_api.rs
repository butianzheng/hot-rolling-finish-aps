// ==========================================
// 热轧精整排产系统 - DecisionApi Trait 定义
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md
// 职责: 定义决策支持层的 6 个核心查询接口
// ==========================================

use super::dto::*;

/// DecisionApi trait
///
/// 提供 6 个核心决策查询功能:
/// - D1: 哪天最危险
/// - D2: 哪些紧急单无法完成
/// - D3: 哪些冷料压库
/// - D4: 哪个机组最堵
/// - D5: 换辊是否异常
/// - D6: 是否存在产能优化空间
pub trait DecisionApi: Send + Sync {
    /// D1: 查询日期风险摘要 - "哪天最危险"
    ///
    /// # 参数
    /// - `request`: 查询请求,包含版本 ID 和日期范围
    ///
    /// # 返回
    /// - 成功: 日期风险摘要列表,按风险分数降序排列
    /// - 失败: 错误消息
    ///
    /// # 示例
    /// ```ignore
    /// let request = GetDecisionDaySummaryRequest {
    ///     version_id: "V20260123-001".to_string(),
    ///     date_from: "2026-01-24".to_string(),
    ///     date_to: "2026-01-30".to_string(),
    ///     risk_level_filter: Some(vec!["HIGH".to_string(), "CRITICAL".to_string()]),
    ///     limit: Some(5),
    ///     sort_by: Some("risk_score".to_string()),
    /// };
    /// let response = api.get_decision_day_summary(request)?;
    /// ```
    fn get_decision_day_summary(
        &self,
        request: GetDecisionDaySummaryRequest,
    ) -> Result<DecisionDaySummaryResponse, String>;

    /// D4: 查询机组堵塞概况 - "哪个机组最堵"
    ///
    /// # 参数
    /// - `request`: 查询请求,包含版本 ID、日期范围和机组过滤
    ///
    /// # 返回
    /// - 成功: 机组堵塞点列表,按堵塞分数降序排列
    /// - 失败: 错误消息
    ///
    /// # 示例
    /// ```ignore
    /// let request = GetMachineBottleneckProfileRequest {
    ///     version_id: "V20260123-001".to_string(),
    ///     date_from: "2026-01-24".to_string(),
    ///     date_to: "2026-01-30".to_string(),
    ///     machine_codes: Some(vec!["H032".to_string(), "H033".to_string()]),
    ///     bottleneck_level_filter: Some(vec!["HIGH".to_string(), "CRITICAL".to_string()]),
    ///     limit: Some(10),
    /// };
    /// let response = api.get_machine_bottleneck_profile(request)?;
    /// ```
    fn get_machine_bottleneck_profile(
        &self,
        request: GetMachineBottleneckProfileRequest,
    ) -> Result<MachineBottleneckProfileResponse, String>;

    /// D2: 查询紧急订单失败集合 - "哪些紧急单无法完成"
    ///
    /// # 参数
    /// - `request`: 查询请求,包含版本 ID 和过滤条件
    ///
    /// # 返回
    /// - 成功: 失败订单列表和统计摘要
    /// - 失败: 错误消息
    ///
    /// # 状态
    /// - ⏸ P3 优先级,待实现
    fn list_order_failure_set(
        &self,
        request: ListOrderFailureSetRequest,
    ) -> Result<OrderFailureSetResponse, String>;

    /// D2M: 查询材料失败集合 - "哪些材料无法满足"
    ///
    /// # 参数
    /// - `request`: 查询请求，包含版本 ID 和过滤条件
    ///
    /// # 返回
    /// - 成功: 材料失败列表 + 聚合摘要 + 合同聚合
    /// - 失败: 错误消息
    fn list_material_failure_set(
        &self,
        request: ListMaterialFailureSetRequest,
    ) -> Result<MaterialFailureSetResponse, String>;

    /// D3: 查询冷料压库概况 - "哪些冷料压库"
    ///
    /// # 参数
    /// - `request`: 查询请求,包含版本 ID 和过滤条件
    ///
    /// # 返回
    /// - 成功: 冷料分桶列表和统计摘要
    /// - 失败: 错误消息
    ///
    /// # 状态
    /// - ⏸ P3 优先级,待实现
    fn get_cold_stock_profile(
        &self,
        request: GetColdStockProfileRequest,
    ) -> Result<ColdStockProfileResponse, String>;

    /// D5: 查询换辊预警列表 - "换辊是否异常"
    ///
    /// # 参数
    /// - `request`: 查询请求,包含版本 ID 和过滤条件
    ///
    /// # 返回
    /// - 成功: 换辊预警列表和统计摘要
    /// - 失败: 错误消息
    ///
    /// # 状态
    /// - ⏸ P3 优先级,待实现
    fn list_roll_campaign_alerts(
        &self,
        request: ListRollCampaignAlertsRequest,
    ) -> Result<RollCampaignAlertsResponse, String>;

    /// D6: 查询产能优化机会 - "是否存在产能优化空间"
    ///
    /// # 参数
    /// - `request`: 查询请求,包含版本 ID 和过滤条件
    ///
    /// # 返回
    /// - 成功: 产能优化机会列表和统计摘要
    /// - 失败: 错误消息
    ///
    /// # 状态
    /// - ⏸ P3 优先级,待实现
    fn get_capacity_opportunity(
        &self,
        request: GetCapacityOpportunityRequest,
    ) -> Result<CapacityOpportunityResponse, String>;
}

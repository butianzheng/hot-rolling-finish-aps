use chrono::NaiveDateTime;

/// 换辊批次时间线状态
///
/// # 职责
/// - 追踪当前仿真时间线的状态
/// - 记录换辊批次信息（批次号、起始时间、累计重量）
/// - 记录当前处理的计划项索引和剩余重量
///
/// # 说明
/// - 用于 `simulate_to_as_of` 和 `produce_weight_until` 函数
/// - 状态包含：时间进度、重量进度、换辊批次进度
#[derive(Debug, Clone)]
pub(super) struct CampaignStreamState {
    /// 当前处理的计划项索引
    pub item_index: usize,
    /// 当前计划项剩余重量（吨）
    pub remaining_weight_t: f64,
    /// 当前仿真时间
    pub current_time: NaiveDateTime,
    /// 当前换辊批次号（从 1 开始）
    pub campaign_no: i32,
    /// 当前换辊批次起始时间
    pub campaign_start_at: NaiveDateTime,
    /// 当前换辊批次累计重量（吨）
    pub cum_weight_t: f64,
}

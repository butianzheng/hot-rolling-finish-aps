use chrono::NaiveDateTime;

/// 告警级别
///
/// # 级别说明
/// - `NONE`: 正常，无需关注
/// - `WARNING`: 警告，需要关注（利用率 >= 85%）
/// - `CRITICAL`: 严重，尽快处理（利用率 >= 95% 或已达阈值）
/// - `EMERGENCY`: 紧急，立即处理（超过硬限制或硬停止风险）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AlertLevel {
    None,
    Warning,
    Critical,
    Emergency,
}

impl AlertLevel {
    /// 转换为字符串（用于数据库存储）
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertLevel::None => "NONE",
            AlertLevel::Warning => "WARNING",
            AlertLevel::Critical => "CRITICAL",
            AlertLevel::Emergency => "EMERGENCY",
        }
    }
}

/// 告警计算结果
#[derive(Debug, Clone)]
pub(super) struct AlertResult {
    /// 告警级别
    pub level: AlertLevel,
    /// 告警原因
    pub reason: String,
    /// 是否需要立即换辊
    pub needs_immediate_change: bool,
    /// 建议操作（JSON 数组字符串）
    pub suggested_actions: String,
}

/// 计算告警级别
///
/// # 参数
/// - `cum_weight_t`: 当前累计重量（吨）
/// - `suggest_threshold_t`: 建议换辊阈值（吨）
/// - `hard_limit_t`: 硬限制（吨）
/// - `soft_reach`: 预计软限制触达时间
/// - `hard_reach`: 预计硬限制触达时间
/// - `planned_change_at`: 计划换辊时间
///
/// # 返回
/// - 告警计算结果
pub(super) fn calculate_alert(
    cum_weight_t: f64,
    suggest_threshold_t: f64,
    hard_limit_t: f64,
    soft_reach: Option<NaiveDateTime>,
    hard_reach: Option<NaiveDateTime>,
    planned_change_at: Option<NaiveDateTime>,
) -> AlertResult {
    let will_exceed_soft_before_change = match (soft_reach, planned_change_at) {
        (Some(s), Some(p)) => s < p,
        _ => false,
    };

    let will_hard_stop_before_change = match (hard_reach, planned_change_at) {
        (Some(h), Some(p)) => h <= p,
        _ => false,
    };

    let utilization_rate = if suggest_threshold_t > 0.0 {
        cum_weight_t / suggest_threshold_t
    } else {
        0.0
    };

    let (level, reason) = if will_hard_stop_before_change {
        (
            AlertLevel::Emergency,
            "计划换辊时间晚于预计硬限制触达，存在硬停止风险".to_string(),
        )
    } else if cum_weight_t >= hard_limit_t {
        (
            AlertLevel::Emergency,
            format!("已超过硬限制 {:.1} 吨，必须立即换辊", hard_limit_t),
        )
    } else if will_exceed_soft_before_change || cum_weight_t >= suggest_threshold_t {
        (
            AlertLevel::Critical,
            format!("已达到/超过建议阈值 {:.1} 吨", suggest_threshold_t),
        )
    } else if utilization_rate >= 0.95 {
        (
            AlertLevel::Critical,
            format!(
                "接近建议阈值 ({:.1}%)，建议尽快安排换辊",
                utilization_rate * 100.0
            ),
        )
    } else if utilization_rate >= 0.85 {
        (
            AlertLevel::Warning,
            format!("接近建议阈值 ({:.1}%)，请关注", utilization_rate * 100.0),
        )
    } else {
        (AlertLevel::None, "换辊状态正常".to_string())
    };

    let needs_immediate_change = matches!(level, AlertLevel::Emergency)
        || utilization_rate >= 0.95
        || will_hard_stop_before_change;

    let suggested_actions = if will_hard_stop_before_change {
        r#"["调整计划换辊时间（避免硬停止）","考虑提前换辊或增加停机时长"]"#.to_string()
    } else if matches!(level, AlertLevel::Emergency) {
        r#"["立即换辊"]"#.to_string()
    } else if matches!(level, AlertLevel::Critical) {
        r#"["尽快安排换辊（优先在计划停机）"]"#.to_string()
    } else if matches!(level, AlertLevel::Warning) {
        r#"["关注换辊时间与阈值触达"]"#.to_string()
    } else {
        "[]".to_string()
    };

    AlertResult {
        level,
        reason,
        needs_immediate_change,
        suggested_actions,
    }
}

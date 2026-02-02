// ==========================================
// 热轧精整排产系统 - D5 用例：换辊是否异常
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 5.5 节
// 职责: 回答"换辊是否异常"，返回换辊预警列表
// ==========================================

use serde::{Deserialize, Serialize};

/// D5 用例：换辊是否异常
///
/// 输入: version_id, alert_level (可选)
/// 输出: Vec<RollAlert> 按 alert_level 降序
/// 刷新触发: roll_campaign_changed, plan_item_changed
pub trait RollCampaignAlertUseCase {
    /// 查询换辊预警列表
    fn list_roll_campaign_alerts(
        &self,
        version_id: &str,
        alert_level: Option<&str>,
    ) -> Result<Vec<RollAlert>, String>;

    /// 查询特定机组的换辊预警
    fn get_machine_roll_alerts(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> Result<Vec<RollAlert>, String>;

    /// 统计换辊预警
    fn get_roll_alert_summary(&self, version_id: &str) -> Result<RollAlertSummary, String>;
}

/// 换辊预警
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollAlert {
    /// 版本 ID
    pub version_id: String,

    /// 机组代码
    pub machine_code: String,

    /// 活动编号
    pub campaign_no: i32,

    /// 累计重量 (吨)
    pub cum_weight_t: f64,

    /// 建议阈值 (吨)
    pub suggest_threshold_t: f64,

    /// 硬限制阈值 (吨)
    pub hard_limit_t: f64,

    /// 预警等级 (NONE/WARNING/CRITICAL/EMERGENCY)
    pub alert_level: String,

    /// 预警原因
    pub reason: Option<String>,

    /// 距离建议阈值 (吨)
    pub distance_to_suggest: f64,

    /// 距离硬限制 (吨)
    pub distance_to_hard: f64,

    /// 利用率 (相对建议阈值, 0.0-1.0+)
    pub utilization_rate: f64,

    /// 预计换辊日期 (YYYY-MM-DD)
    pub estimated_change_date: Option<String>,

    /// 当前换辊周期起点（YYYY-MM-DD HH:MM:SS，估算/人工微调）
    pub campaign_start_at: Option<String>,

    /// 计划换辊时刻（YYYY-MM-DD HH:MM:SS，可人工微调）
    pub planned_change_at: Option<String>,

    /// 计划停机时长（分钟）
    pub planned_downtime_minutes: Option<i32>,

    /// 预计达到软限制的日期时间（YYYY-MM-DD HH:MM:SS）
    pub estimated_soft_reach_at: Option<String>,

    /// 预计达到硬限制的日期时间（YYYY-MM-DD HH:MM:SS）
    pub estimated_hard_reach_at: Option<String>,

    /// 是否需要立即换辊
    pub needs_immediate_change: bool,

    /// 建议措施
    pub suggested_actions: Vec<String>,
}

/// 换辊预警统计
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollAlertSummary {
    /// 版本 ID
    pub version_id: String,

    /// 总预警数
    pub total_alerts: i32,

    /// 紧急预警数 (EMERGENCY)
    pub emergency_count: i32,

    /// 严重预警数 (CRITICAL)
    pub critical_count: i32,

    /// 警告数 (WARNING)
    pub warning_count: i32,

    /// 需要立即换辊的机组数
    pub immediate_change_needed: i32,

    /// 按机组分组统计
    pub by_machine: Vec<MachineRollStat>,
}

/// 机组换辊统计
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineRollStat {
    /// 机组代码
    pub machine_code: String,

    /// 当前活动编号
    pub campaign_no: i32,

    /// 累计重量 (吨)
    pub cum_weight_t: f64,

    /// 利用率
    pub utilization_rate: f64,

    /// 预警等级
    pub alert_level: String,
}

impl RollAlert {
    /// 创建新的换辊预警
    pub fn new(
        version_id: String,
        machine_code: String,
        campaign_no: i32,
        cum_weight_t: f64,
        suggest_threshold_t: f64,
        hard_limit_t: f64,
    ) -> Self {
        let distance_to_suggest = suggest_threshold_t - cum_weight_t;
        let distance_to_hard = hard_limit_t - cum_weight_t;
        let utilization_rate = if suggest_threshold_t > 0.0 {
            cum_weight_t / suggest_threshold_t
        } else {
            0.0
        };

        let mut alert = Self {
            version_id,
            machine_code,
            campaign_no,
            cum_weight_t,
            suggest_threshold_t,
            hard_limit_t,
            alert_level: "NONE".to_string(),
            reason: None,
            distance_to_suggest,
            distance_to_hard,
            utilization_rate,
            estimated_change_date: None,
            campaign_start_at: None,
            planned_change_at: None,
            planned_downtime_minutes: None,
            estimated_soft_reach_at: None,
            estimated_hard_reach_at: None,
            needs_immediate_change: false,
            suggested_actions: Vec::new(),
        };

        alert.calculate_alert_level();
        alert
    }

    /// 计算预警等级
    fn calculate_alert_level(&mut self) {
        // 超过硬限制
        if self.cum_weight_t >= self.hard_limit_t {
            self.alert_level = "EMERGENCY".to_string();
            self.reason = Some(format!(
                "已超过硬限制 {:.1} 吨，必须立即换辊",
                self.hard_limit_t
            ));
            self.needs_immediate_change = true;
            return;
        }

        // 接近硬限制 (95%)
        if self.utilization_rate >= 0.95 {
            self.alert_level = "CRITICAL".to_string();
            self.reason = Some(format!(
                "接近硬限制 ({:.1}%)，建议尽快换辊",
                self.utilization_rate * 100.0
            ));
            self.needs_immediate_change = true;
            return;
        }

        // 超过建议阈值
        if self.cum_weight_t >= self.suggest_threshold_t {
            self.alert_level = "CRITICAL".to_string();
            self.reason = Some(format!(
                "已超过建议阈值 {:.1} 吨",
                self.suggest_threshold_t
            ));
            return;
        }

        // 接近建议阈值 (85%)
        if self.utilization_rate >= 0.85 {
            self.alert_level = "WARNING".to_string();
            self.reason = Some(format!(
                "接近建议阈值 ({:.1}%)，请关注",
                self.utilization_rate * 100.0
            ));
            return;
        }

        // 正常
        self.alert_level = "NONE".to_string();
        self.reason = Some("换辊状态正常".to_string());
    }

    /// 设置预计换辊日期
    pub fn set_estimated_change_date(&mut self, date: String) {
        self.estimated_change_date = Some(date);
    }

    /// 添加建议措施
    pub fn add_suggested_action(&mut self, action: String) {
        self.suggested_actions.push(action);
    }

    /// 判断是否为高优先级预警
    pub fn is_high_priority(&self) -> bool {
        matches!(self.alert_level.as_str(), "CRITICAL" | "EMERGENCY")
    }

    /// 判断是否需要关注
    pub fn needs_attention(&self) -> bool {
        self.alert_level != "NONE"
    }

    /// 获取剩余可用重量
    pub fn remaining_capacity(&self) -> f64 {
        (self.suggest_threshold_t - self.cum_weight_t).max(0.0)
    }

    /// 获取超载重量
    pub fn overload_weight(&self) -> f64 {
        (self.cum_weight_t - self.suggest_threshold_t).max(0.0)
    }
}

impl RollAlertSummary {
    /// 创建新的换辊预警统计
    pub fn new(version_id: String) -> Self {
        Self {
            version_id,
            total_alerts: 0,
            emergency_count: 0,
            critical_count: 0,
            warning_count: 0,
            immediate_change_needed: 0,
            by_machine: Vec::new(),
        }
    }

    /// 添加预警到统计
    pub fn add_alert(&mut self, alert: &RollAlert) {
        if alert.needs_attention() {
            self.total_alerts += 1;

            match alert.alert_level.as_str() {
                "EMERGENCY" => self.emergency_count += 1,
                "CRITICAL" => self.critical_count += 1,
                "WARNING" => self.warning_count += 1,
                _ => {}
            }

            if alert.needs_immediate_change {
                self.immediate_change_needed += 1;
            }
        }
    }

    /// 添加机组统计
    pub fn add_machine_stat(&mut self, stat: MachineRollStat) {
        self.by_machine.push(stat);
    }

    /// 判断是否存在紧急预警
    pub fn has_emergency(&self) -> bool {
        self.emergency_count > 0
    }

    /// 判断是否存在严重预警
    pub fn has_critical(&self) -> bool {
        self.critical_count > 0
    }

    /// 获取高优先级预警总数
    pub fn high_priority_count(&self) -> i32 {
        self.emergency_count + self.critical_count
    }
}

impl std::fmt::Display for RollAlert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@Campaign#{} (cum: {:.1}t, util: {:.1}%, level: {})",
            self.machine_code,
            self.campaign_no,
            self.cum_weight_t,
            self.utilization_rate * 100.0,
            self.alert_level
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roll_alert_normal() {
        let alert = RollAlert::new(
            "V001".to_string(),
            "H032".to_string(),
            1,
            5000.0,
            10000.0,
            12000.0,
        );

        assert_eq!(alert.alert_level, "NONE");
        assert!(!alert.needs_immediate_change);
        assert!(!alert.needs_attention());
    }

    #[test]
    fn test_roll_alert_warning() {
        let alert = RollAlert::new(
            "V001".to_string(),
            "H032".to_string(),
            1,
            8700.0,
            10000.0,
            12000.0,
        );

        assert_eq!(alert.alert_level, "WARNING");
        assert!(!alert.needs_immediate_change);
        assert!(alert.needs_attention());
    }

    #[test]
    fn test_roll_alert_critical() {
        let alert = RollAlert::new(
            "V001".to_string(),
            "H032".to_string(),
            1,
            10500.0,
            10000.0,
            12000.0,
        );

        assert_eq!(alert.alert_level, "CRITICAL");
        assert!(alert.is_high_priority());
        assert!(alert.overload_weight() > 0.0);
    }

    #[test]
    fn test_roll_alert_emergency() {
        let alert = RollAlert::new(
            "V001".to_string(),
            "H032".to_string(),
            1,
            12000.0,
            10000.0,
            12000.0,
        );

        assert_eq!(alert.alert_level, "EMERGENCY");
        assert!(alert.needs_immediate_change);
        assert!(alert.is_high_priority());
    }

    #[test]
    fn test_roll_alert_summary() {
        let mut summary = RollAlertSummary::new("V001".to_string());

        let alert1 = RollAlert::new(
            "V001".to_string(),
            "H032".to_string(),
            1,
            10500.0,
            10000.0,
            12000.0,
        );

        let alert2 = RollAlert::new(
            "V001".to_string(),
            "H033".to_string(),
            1,
            12000.0,
            10000.0,
            12000.0,
        );

        summary.add_alert(&alert1);
        summary.add_alert(&alert2);

        assert_eq!(summary.total_alerts, 2);
        assert_eq!(summary.critical_count, 1);
        assert_eq!(summary.emergency_count, 1);
        assert!(summary.has_emergency());
        assert_eq!(summary.high_priority_count(), 2);
    }

    #[test]
    fn test_remaining_capacity() {
        let alert = RollAlert::new(
            "V001".to_string(),
            "H032".to_string(),
            1,
            7000.0,
            10000.0,
            12000.0,
        );

        assert_eq!(alert.remaining_capacity(), 3000.0);
        assert_eq!(alert.overload_weight(), 0.0);
    }
}

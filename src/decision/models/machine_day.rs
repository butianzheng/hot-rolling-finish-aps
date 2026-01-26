// ==========================================
// 热轧精整排产系统 - 决策对象：机组-日
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 6 节
// 职责: 定义机组-日决策对象，作为产能占用、堵塞、换辊风险的主载体
// ==========================================

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// 机组-日 (MachineDay)
///
/// machine_code × plan_date，是产能占用、堵塞、换辊风险的主载体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineDay {
    /// 机组代码
    pub machine_code: String,

    /// 排产日期
    pub plan_date: NaiveDate,

    /// 所属版本 ID
    pub version_id: String,

    /// 目标产能 (吨)
    pub target_capacity_t: f64,

    /// 限制产能 (吨)
    pub limit_capacity_t: f64,

    /// 已使用产能 (吨)
    pub used_capacity_t: f64,

    /// 剩余产能 (吨)
    pub remaining_capacity_t: f64,

    /// 产能利用率 (0.0-1.0)
    pub capacity_utilization: f64,

    /// 是否在冻结区
    pub is_frozen: bool,

    /// 是否需要换辊
    pub needs_roll_change: bool,

    /// 当前换辊窗口累计吨位
    pub campaign_cum_weight_t: Option<f64>,
}

impl MachineDay {
    /// 创建新的机组-日
    pub fn new(
        machine_code: String,
        plan_date: NaiveDate,
        version_id: String,
        target_capacity_t: f64,
        limit_capacity_t: f64,
    ) -> Self {
        Self {
            machine_code,
            plan_date,
            version_id,
            target_capacity_t,
            limit_capacity_t,
            used_capacity_t: 0.0,
            remaining_capacity_t: target_capacity_t,
            capacity_utilization: 0.0,
            is_frozen: false,
            needs_roll_change: false,
            campaign_cum_weight_t: None,
        }
    }

    /// 从字符串创建机组-日
    pub fn from_str(
        machine_code: String,
        date_str: &str,
        version_id: String,
        target_capacity_t: f64,
        limit_capacity_t: f64,
    ) -> Result<Self, chrono::ParseError> {
        let plan_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
        Ok(Self::new(machine_code, plan_date, version_id, target_capacity_t, limit_capacity_t))
    }

    /// 获取日期字符串
    pub fn date_string(&self) -> String {
        self.plan_date.format("%Y-%m-%d").to_string()
    }

    /// 获取机组-日唯一标识
    pub fn key(&self) -> String {
        format!("{}_{}", self.machine_code, self.date_string())
    }

    /// 更新已使用产能
    pub fn update_used_capacity(&mut self, used_capacity_t: f64) {
        self.used_capacity_t = used_capacity_t;
        self.remaining_capacity_t = self.target_capacity_t - used_capacity_t;
        self.capacity_utilization = if self.target_capacity_t > 0.0 {
            used_capacity_t / self.target_capacity_t
        } else {
            0.0
        };
    }

    /// 判断是否超过目标产能
    pub fn is_over_target(&self) -> bool {
        self.used_capacity_t > self.target_capacity_t
    }

    /// 判断是否超过限制产能
    pub fn is_over_limit(&self) -> bool {
        self.used_capacity_t > self.limit_capacity_t
    }

    /// 判断是否接近目标产能 (阈值默认 0.9)
    pub fn is_near_target(&self, threshold: f64) -> bool {
        self.capacity_utilization >= threshold
    }

    /// 判断是否存在产能瓶颈
    pub fn is_bottleneck(&self, threshold: f64) -> bool {
        self.is_near_target(threshold) || self.is_over_target()
    }

    /// 计算产能松弛度 (slack)
    pub fn capacity_slack(&self) -> f64 {
        self.remaining_capacity_t.max(0.0)
    }

    /// 计算产能超载量
    pub fn capacity_overload(&self) -> f64 {
        (self.used_capacity_t - self.target_capacity_t).max(0.0)
    }

    /// 设置换辊窗口累计吨位
    pub fn set_campaign_weight(&mut self, cum_weight_t: f64) {
        self.campaign_cum_weight_t = Some(cum_weight_t);
    }

    /// 设置换辊需求标志
    pub fn set_needs_roll_change(&mut self, needs: bool) {
        self.needs_roll_change = needs;
    }
}

impl std::fmt::Display for MachineDay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{} (used: {:.1}t / target: {:.1}t, util: {:.1}%)",
            self.machine_code,
            self.date_string(),
            self.used_capacity_t,
            self.target_capacity_t,
            self.capacity_utilization * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_machine_day_creation() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 22).unwrap();
        let md = MachineDay::new(
            "H032".to_string(),
            date,
            "V001".to_string(),
            1000.0,
            1200.0,
        );

        assert_eq!(md.machine_code, "H032");
        assert_eq!(md.target_capacity_t, 1000.0);
        assert_eq!(md.limit_capacity_t, 1200.0);
        assert_eq!(md.used_capacity_t, 0.0);
        assert_eq!(md.remaining_capacity_t, 1000.0);
        assert_eq!(md.capacity_utilization, 0.0);
    }

    #[test]
    fn test_update_used_capacity() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 22).unwrap();
        let mut md = MachineDay::new(
            "H032".to_string(),
            date,
            "V001".to_string(),
            1000.0,
            1200.0,
        );

        md.update_used_capacity(800.0);
        assert_eq!(md.used_capacity_t, 800.0);
        assert_eq!(md.remaining_capacity_t, 200.0);
        assert_eq!(md.capacity_utilization, 0.8);
    }

    #[test]
    fn test_capacity_checks() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 22).unwrap();
        let mut md = MachineDay::new(
            "H032".to_string(),
            date,
            "V001".to_string(),
            1000.0,
            1200.0,
        );

        // 未超过目标
        md.update_used_capacity(800.0);
        assert!(!md.is_over_target());
        assert!(!md.is_over_limit());
        assert!(!md.is_near_target(0.9));

        // 接近目标
        md.update_used_capacity(950.0);
        assert!(md.is_near_target(0.9));
        assert!(!md.is_over_target());

        // 超过目标但未超过限制
        md.update_used_capacity(1100.0);
        assert!(md.is_over_target());
        assert!(!md.is_over_limit());

        // 超过限制
        md.update_used_capacity(1300.0);
        assert!(md.is_over_target());
        assert!(md.is_over_limit());
    }

    #[test]
    fn test_capacity_slack_and_overload() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 22).unwrap();
        let mut md = MachineDay::new(
            "H032".to_string(),
            date,
            "V001".to_string(),
            1000.0,
            1200.0,
        );

        // 有松弛度
        md.update_used_capacity(800.0);
        assert_eq!(md.capacity_slack(), 200.0);
        assert_eq!(md.capacity_overload(), 0.0);

        // 无松弛度，有超载
        md.update_used_capacity(1100.0);
        assert_eq!(md.capacity_slack(), 0.0);
        assert_eq!(md.capacity_overload(), 100.0);
    }

    #[test]
    fn test_key_generation() {
        let md = MachineDay::from_str(
            "H032".to_string(),
            "2026-01-22",
            "V001".to_string(),
            1000.0,
            1200.0,
        ).unwrap();

        assert_eq!(md.key(), "H032_2026-01-22");
    }
}

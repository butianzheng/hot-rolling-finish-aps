// ==========================================
// 热轧精整排产系统 - 决策对象：排产日
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 6 节
// 职责: 定义排产日决策对象，作为决策基本时间粒度
// ==========================================

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// 排产日 (PlanningDay)
///
/// 以 YYYY-MM-DD 表示的决策基本时间粒度。
/// 所有风险/产能/堵塞的归一粒度。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanningDay {
    /// 排产日期 (YYYY-MM-DD)
    pub date: NaiveDate,

    /// 所属版本 ID
    pub version_id: String,

    /// 是否在冻结区内
    pub is_frozen: bool,

    /// 是否在计算窗口内
    pub is_in_window: bool,

    /// 工作日标志 (true=工作日, false=休息日)
    pub is_working_day: bool,
}

impl PlanningDay {
    /// 创建新的排产日
    pub fn new(date: NaiveDate, version_id: String) -> Self {
        Self {
            date,
            version_id,
            is_frozen: false,
            is_in_window: true,
            is_working_day: true,
        }
    }

    /// 从字符串创建排产日
    pub fn from_str(date_str: &str, version_id: String) -> Result<Self, chrono::ParseError> {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
        Ok(Self::new(date, version_id))
    }

    /// 转换为字符串格式 (YYYY-MM-DD)
    pub fn to_string(&self) -> String {
        self.date.format("%Y-%m-%d").to_string()
    }

    /// 设置冻结标志
    pub fn set_frozen(&mut self, is_frozen: bool) {
        self.is_frozen = is_frozen;
    }

    /// 设置窗口标志
    pub fn set_in_window(&mut self, is_in_window: bool) {
        self.is_in_window = is_in_window;
    }

    /// 设置工作日标志
    pub fn set_working_day(&mut self, is_working_day: bool) {
        self.is_working_day = is_working_day;
    }

    /// 判断是否可以进行排产计算
    pub fn is_schedulable(&self) -> bool {
        self.is_in_window && self.is_working_day && !self.is_frozen
    }

    /// 获取下一个排产日
    pub fn next_day(&self) -> Self {
        let next_date = self.date + chrono::Duration::days(1);
        Self {
            date: next_date,
            version_id: self.version_id.clone(),
            is_frozen: false,
            is_in_window: self.is_in_window,
            is_working_day: true, // 需要根据实际日历判断
        }
    }

    /// 获取前一个排产日
    pub fn prev_day(&self) -> Self {
        let prev_date = self.date - chrono::Duration::days(1);
        Self {
            date: prev_date,
            version_id: self.version_id.clone(),
            is_frozen: false,
            is_in_window: self.is_in_window,
            is_working_day: true, // 需要根据实际日历判断
        }
    }

    /// 计算与另一个排产日的天数差
    pub fn days_between(&self, other: &PlanningDay) -> i64 {
        (self.date - other.date).num_days()
    }
}

impl std::fmt::Display for PlanningDay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planning_day_creation() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 22).unwrap();
        let day = PlanningDay::new(date, "V001".to_string());

        assert_eq!(day.date, date);
        assert_eq!(day.version_id, "V001");
        assert!(!day.is_frozen);
        assert!(day.is_in_window);
        assert!(day.is_working_day);
    }

    #[test]
    fn test_planning_day_from_str() {
        let day = PlanningDay::from_str("2026-01-22", "V001".to_string()).unwrap();
        assert_eq!(day.to_string(), "2026-01-22");
    }

    #[test]
    fn test_is_schedulable() {
        let mut day = PlanningDay::from_str("2026-01-22", "V001".to_string()).unwrap();

        // 默认可排产
        assert!(day.is_schedulable());

        // 冻结后不可排产
        day.set_frozen(true);
        assert!(!day.is_schedulable());

        // 非工作日不可排产
        day.set_frozen(false);
        day.set_working_day(false);
        assert!(!day.is_schedulable());

        // 不在窗口内不可排产
        day.set_working_day(true);
        day.set_in_window(false);
        assert!(!day.is_schedulable());
    }

    #[test]
    fn test_next_prev_day() {
        let day = PlanningDay::from_str("2026-01-22", "V001".to_string()).unwrap();

        let next = day.next_day();
        assert_eq!(next.to_string(), "2026-01-23");

        let prev = day.prev_day();
        assert_eq!(prev.to_string(), "2026-01-21");
    }

    #[test]
    fn test_days_between() {
        let day1 = PlanningDay::from_str("2026-01-22", "V001".to_string()).unwrap();
        let day2 = PlanningDay::from_str("2026-01-25", "V001".to_string()).unwrap();

        assert_eq!(day2.days_between(&day1), 3);
        assert_eq!(day1.days_between(&day2), -3);
    }
}

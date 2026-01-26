// ==========================================
// 热轧精整排产系统 - 决策对象：承诺单元
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 6 节
// 职责: 定义承诺单元决策对象，以合同影子字段聚合形成的承诺单元
// ==========================================

use serde::{Deserialize, Serialize};

/// 承诺单元 (CommitmentUnit)
///
/// 以 material_master 的合同影子字段聚合形成（合同不独立建表的前提下）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommitmentUnit {
    /// 合同号
    pub contract_no: String,

    /// 所属版本 ID
    pub version_id: String,

    /// 交货期
    pub due_date: String,

    /// 合同性质代码
    pub contract_nature: Option<String>,

    /// 按周交货标志
    pub weekly_delivery_flag: Option<String>,

    /// 出口标记
    pub export_flag: Option<String>,

    /// 催料标志
    pub rush_flag: Option<String>,

    /// 材料总数
    pub total_materials: i32,

    /// 总重量 (吨)
    pub total_weight_t: f64,

    /// 已排产数量
    pub scheduled_count: i32,

    /// 已排产重量 (吨)
    pub scheduled_weight_t: f64,

    /// 未排产数量
    pub unscheduled_count: i32,

    /// 未排产重量 (吨)
    pub unscheduled_weight_t: f64,

    /// 完成率 (0.0-1.0)
    pub completion_rate: f64,

    /// 是否临期
    pub is_near_due: bool,

    /// 是否超期
    pub is_overdue: bool,

    /// 距离交货期天数 (负数表示超期)
    pub days_to_due: i32,

    /// 风险等级
    pub risk_level: String,

    /// 风险原因列表
    pub risk_reasons: Vec<String>,
}

impl CommitmentUnit {
    /// 创建新的承诺单元
    pub fn new(
        contract_no: String,
        version_id: String,
        due_date: String,
    ) -> Self {
        Self {
            contract_no,
            version_id,
            due_date,
            contract_nature: None,
            weekly_delivery_flag: None,
            export_flag: None,
            rush_flag: None,
            total_materials: 0,
            total_weight_t: 0.0,
            scheduled_count: 0,
            scheduled_weight_t: 0.0,
            unscheduled_count: 0,
            unscheduled_weight_t: 0.0,
            completion_rate: 0.0,
            is_near_due: false,
            is_overdue: false,
            days_to_due: 0,
            risk_level: "LOW".to_string(),
            risk_reasons: Vec::new(),
        }
    }

    /// 添加材料到承诺单元
    pub fn add_material(&mut self, weight_t: f64, is_scheduled: bool) {
        self.total_materials += 1;
        self.total_weight_t += weight_t;

        if is_scheduled {
            self.scheduled_count += 1;
            self.scheduled_weight_t += weight_t;
        } else {
            self.unscheduled_count += 1;
            self.unscheduled_weight_t += weight_t;
        }

        self.recalculate_completion();
    }

    /// 重新计算完成率
    fn recalculate_completion(&mut self) {
        if self.total_weight_t > 0.0 {
            self.completion_rate = self.scheduled_weight_t / self.total_weight_t;
        } else {
            self.completion_rate = 0.0;
        }
    }

    /// 设置交货期信息
    pub fn set_due_date_info(&mut self, days_to_due: i32, near_due_threshold: i32) {
        self.days_to_due = days_to_due;
        self.is_overdue = days_to_due < 0;
        self.is_near_due = days_to_due >= 0 && days_to_due <= near_due_threshold;

        // 更新风险等级
        self.update_risk_level();
    }

    /// 更新风险等级
    fn update_risk_level(&mut self) {
        self.risk_reasons.clear();

        // 超期风险
        if self.is_overdue {
            self.risk_level = "CRITICAL".to_string();
            self.risk_reasons.push(format!("已超期 {} 天", -self.days_to_due));
            return;
        }

        // 临期风险
        if self.is_near_due {
            if self.completion_rate < 0.5 {
                self.risk_level = "HIGH".to_string();
                self.risk_reasons.push(format!(
                    "临期 {} 天，完成率仅 {:.1}%",
                    self.days_to_due,
                    self.completion_rate * 100.0
                ));
            } else if self.completion_rate < 0.8 {
                self.risk_level = "MEDIUM".to_string();
                self.risk_reasons.push(format!(
                    "临期 {} 天，完成率 {:.1}%",
                    self.days_to_due,
                    self.completion_rate * 100.0
                ));
            } else {
                self.risk_level = "LOW".to_string();
            }
            return;
        }

        // 正常情况
        if self.completion_rate < 0.3 {
            self.risk_level = "MEDIUM".to_string();
            self.risk_reasons.push(format!("完成率较低 ({:.1}%)", self.completion_rate * 100.0));
        } else {
            self.risk_level = "LOW".to_string();
        }
    }

    /// 添加风险原因
    pub fn add_risk_reason(&mut self, reason: String) {
        self.risk_reasons.push(reason);
    }

    /// 判断是否为高风险承诺
    pub fn is_high_risk(&self) -> bool {
        matches!(self.risk_level.as_str(), "HIGH" | "CRITICAL")
    }

    /// 判断是否为催料合同
    pub fn is_rush(&self) -> bool {
        self.rush_flag.as_deref() == Some("1") || self.rush_flag.as_deref() == Some("Y")
    }

    /// 判断是否为出口合同
    pub fn is_export(&self) -> bool {
        self.export_flag.as_deref() == Some("1")
    }

    /// 获取未完成材料数量
    pub fn pending_count(&self) -> i32 {
        self.unscheduled_count
    }

    /// 获取未完成重量
    pub fn pending_weight(&self) -> f64 {
        self.unscheduled_weight_t
    }
}

impl std::fmt::Display for CommitmentUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (due: {}, completion: {:.1}%, risk: {})",
            self.contract_no,
            self.due_date,
            self.completion_rate * 100.0,
            self.risk_level
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_unit_creation() {
        let unit = CommitmentUnit::new(
            "C001".to_string(),
            "V001".to_string(),
            "2026-02-01".to_string(),
        );

        assert_eq!(unit.contract_no, "C001");
        assert_eq!(unit.total_materials, 0);
        assert_eq!(unit.completion_rate, 0.0);
    }

    #[test]
    fn test_add_material() {
        let mut unit = CommitmentUnit::new(
            "C001".to_string(),
            "V001".to_string(),
            "2026-02-01".to_string(),
        );

        unit.add_material(100.0, true);
        unit.add_material(50.0, false);

        assert_eq!(unit.total_materials, 2);
        assert_eq!(unit.total_weight_t, 150.0);
        assert_eq!(unit.scheduled_count, 1);
        assert_eq!(unit.scheduled_weight_t, 100.0);
        assert_eq!(unit.unscheduled_count, 1);
        assert_eq!(unit.unscheduled_weight_t, 50.0);
        assert_eq!(unit.completion_rate, 100.0 / 150.0);
    }

    #[test]
    fn test_risk_level_overdue() {
        let mut unit = CommitmentUnit::new(
            "C001".to_string(),
            "V001".to_string(),
            "2026-02-01".to_string(),
        );

        unit.add_material(100.0, false);
        unit.set_due_date_info(-5, 7);

        assert!(unit.is_overdue);
        assert_eq!(unit.risk_level, "CRITICAL");
        assert!(!unit.risk_reasons.is_empty());
    }

    #[test]
    fn test_risk_level_near_due() {
        let mut unit = CommitmentUnit::new(
            "C001".to_string(),
            "V001".to_string(),
            "2026-02-01".to_string(),
        );

        unit.add_material(100.0, true);
        unit.add_material(100.0, false);
        unit.set_due_date_info(5, 7);

        assert!(unit.is_near_due);
        assert_eq!(unit.risk_level, "MEDIUM");
    }

    #[test]
    fn test_rush_and_export_flags() {
        let mut unit = CommitmentUnit::new(
            "C001".to_string(),
            "V001".to_string(),
            "2026-02-01".to_string(),
        );

        unit.rush_flag = Some("1".to_string());
        unit.export_flag = Some("1".to_string());

        assert!(unit.is_rush());
        assert!(unit.is_export());
    }

    #[test]
    fn test_pending_info() {
        let mut unit = CommitmentUnit::new(
            "C001".to_string(),
            "V001".to_string(),
            "2026-02-01".to_string(),
        );

        unit.add_material(100.0, true);
        unit.add_material(50.0, false);
        unit.add_material(30.0, false);

        assert_eq!(unit.pending_count(), 2);
        assert_eq!(unit.pending_weight(), 80.0);
    }
}

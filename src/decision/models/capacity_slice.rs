// ==========================================
// 热轧精整排产系统 - 决策对象：产能切片
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 6 节
// 职责: 定义产能切片决策对象，表示机组-日的可用产能、已占用、剩余、约束解释
// ==========================================

use serde::{Deserialize, Serialize};

/// 产能切片 (CapacitySlice)
///
/// 机组-日的可用产能、已占用、剩余、硬约束/软约束解释。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapacitySlice {
    /// 机组代码
    pub machine_code: String,

    /// 排产日期
    pub plan_date: String,

    /// 所属版本 ID
    pub version_id: String,

    /// 目标产能 (吨)
    pub target_capacity_t: f64,

    /// 限制产能 (吨)
    pub limit_capacity_t: f64,

    /// 已占用产能 (吨)
    pub used_capacity_t: f64,

    /// 剩余产能 (吨)
    pub remaining_capacity_t: f64,

    /// 产能利用率 (0.0-1.0)
    pub utilization_rate: f64,

    /// 是否在冻结区
    pub is_frozen: bool,

    /// 硬约束列表
    pub hard_constraints: Vec<CapacityConstraint>,

    /// 软约束列表
    pub soft_constraints: Vec<CapacityConstraint>,

    /// 约束违反列表
    pub violations: Vec<String>,
}

/// 产能约束
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapacityConstraint {
    /// 约束类型
    pub constraint_type: String,

    /// 约束值
    pub constraint_value: f64,

    /// 当前值
    pub current_value: f64,

    /// 是否违反
    pub is_violated: bool,

    /// 约束描述
    pub description: String,
}

impl CapacitySlice {
    /// 创建新的产能切片
    pub fn new(
        machine_code: String,
        plan_date: String,
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
            utilization_rate: 0.0,
            is_frozen: false,
            hard_constraints: Vec::new(),
            soft_constraints: Vec::new(),
            violations: Vec::new(),
        }
    }

    /// 更新已使用产能
    pub fn update_used_capacity(&mut self, used_capacity_t: f64) {
        self.used_capacity_t = used_capacity_t;
        self.remaining_capacity_t = self.target_capacity_t - used_capacity_t;
        self.utilization_rate = if self.target_capacity_t > 0.0 {
            used_capacity_t / self.target_capacity_t
        } else {
            0.0
        };

        // 检查约束违反
        self.check_constraints();
    }

    /// 添加硬约束
    pub fn add_hard_constraint(&mut self, constraint: CapacityConstraint) {
        self.hard_constraints.push(constraint);
    }

    /// 添加软约束
    pub fn add_soft_constraint(&mut self, constraint: CapacityConstraint) {
        self.soft_constraints.push(constraint);
    }

    /// 检查约束违反
    fn check_constraints(&mut self) {
        self.violations.clear();

        // 检查硬约束
        for constraint in &self.hard_constraints {
            if constraint.is_violated {
                self.violations.push(format!(
                    "硬约束违反: {} (当前: {:.1}, 限制: {:.1})",
                    constraint.description,
                    constraint.current_value,
                    constraint.constraint_value
                ));
            }
        }

        // 检查软约束
        for constraint in &self.soft_constraints {
            if constraint.is_violated {
                self.violations.push(format!(
                    "软约束违反: {} (当前: {:.1}, 目标: {:.1})",
                    constraint.description,
                    constraint.current_value,
                    constraint.constraint_value
                ));
            }
        }

        // 检查产能限制
        if self.used_capacity_t > self.limit_capacity_t {
            self.violations.push(format!(
                "超过限制产能: {:.1}t > {:.1}t",
                self.used_capacity_t, self.limit_capacity_t
            ));
        }
    }

    /// 判断是否有硬约束违反
    pub fn has_hard_violations(&self) -> bool {
        self.hard_constraints.iter().any(|c| c.is_violated)
    }

    /// 判断是否有软约束违反
    pub fn has_soft_violations(&self) -> bool {
        self.soft_constraints.iter().any(|c| c.is_violated)
    }

    /// 判断是否有任何约束违反
    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }

    /// 计算产能松弛度
    pub fn slack(&self) -> f64 {
        self.remaining_capacity_t.max(0.0)
    }

    /// 计算产能超载量
    pub fn overload(&self) -> f64 {
        (self.used_capacity_t - self.target_capacity_t).max(0.0)
    }

    /// 判断是否可以添加指定重量的材料
    pub fn can_add(&self, weight_t: f64) -> bool {
        self.used_capacity_t + weight_t <= self.limit_capacity_t
    }

    /// 获取可添加的最大重量
    pub fn max_addable_weight(&self) -> f64 {
        (self.limit_capacity_t - self.used_capacity_t).max(0.0)
    }
}

impl CapacityConstraint {
    /// 创建新的产能约束
    pub fn new(
        constraint_type: String,
        constraint_value: f64,
        current_value: f64,
        description: String,
    ) -> Self {
        let is_violated = current_value > constraint_value;
        Self {
            constraint_type,
            constraint_value,
            current_value,
            is_violated,
            description,
        }
    }

    /// 更新当前值并检查违反
    pub fn update_current_value(&mut self, current_value: f64) {
        self.current_value = current_value;
        self.is_violated = current_value > self.constraint_value;
    }
}

impl std::fmt::Display for CapacitySlice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{} (used: {:.1}t / target: {:.1}t / limit: {:.1}t, util: {:.1}%)",
            self.machine_code,
            self.plan_date,
            self.used_capacity_t,
            self.target_capacity_t,
            self.limit_capacity_t,
            self.utilization_rate * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capacity_slice_creation() {
        let slice = CapacitySlice::new(
            "H032".to_string(),
            "2026-01-22".to_string(),
            "V001".to_string(),
            1000.0,
            1200.0,
        );

        assert_eq!(slice.machine_code, "H032");
        assert_eq!(slice.target_capacity_t, 1000.0);
        assert_eq!(slice.limit_capacity_t, 1200.0);
        assert_eq!(slice.used_capacity_t, 0.0);
        assert_eq!(slice.remaining_capacity_t, 1000.0);
    }

    #[test]
    fn test_update_used_capacity() {
        let mut slice = CapacitySlice::new(
            "H032".to_string(),
            "2026-01-22".to_string(),
            "V001".to_string(),
            1000.0,
            1200.0,
        );

        slice.update_used_capacity(800.0);
        assert_eq!(slice.used_capacity_t, 800.0);
        assert_eq!(slice.remaining_capacity_t, 200.0);
        assert_eq!(slice.utilization_rate, 0.8);
    }

    #[test]
    fn test_constraints() {
        let mut slice = CapacitySlice::new(
            "H032".to_string(),
            "2026-01-22".to_string(),
            "V001".to_string(),
            1000.0,
            1200.0,
        );

        // 添加硬约束
        let hard_constraint = CapacityConstraint::new(
            "limit".to_string(),
            1200.0,
            1300.0,
            "产能限制".to_string(),
        );
        slice.add_hard_constraint(hard_constraint);

        assert!(slice.has_hard_violations());
    }

    #[test]
    fn test_can_add() {
        let mut slice = CapacitySlice::new(
            "H032".to_string(),
            "2026-01-22".to_string(),
            "V001".to_string(),
            1000.0,
            1200.0,
        );

        slice.update_used_capacity(1000.0);

        assert!(slice.can_add(100.0));
        assert!(slice.can_add(200.0));
        assert!(!slice.can_add(300.0));

        assert_eq!(slice.max_addable_weight(), 200.0);
    }

    #[test]
    fn test_slack_and_overload() {
        let mut slice = CapacitySlice::new(
            "H032".to_string(),
            "2026-01-22".to_string(),
            "V001".to_string(),
            1000.0,
            1200.0,
        );

        // 有松弛度
        slice.update_used_capacity(800.0);
        assert_eq!(slice.slack(), 200.0);
        assert_eq!(slice.overload(), 0.0);

        // 无松弛度，有超载
        slice.update_used_capacity(1100.0);
        assert_eq!(slice.slack(), 0.0);
        assert_eq!(slice.overload(), 100.0);
    }
}

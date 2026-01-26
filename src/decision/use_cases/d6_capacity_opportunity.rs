// ==========================================
// 热轧精整排产系统 - D6 用例：是否存在产能优化空间
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 5.6 节
// 职责: 回答"是否存在产能优化空间"，返回产能优化机会与敏感性分析
// ==========================================

use serde::{Deserialize, Serialize};

/// D6 用例：是否存在产能优化空间
///
/// 输入: version_id, machine_code (可选), date_range
/// 输出: Vec<CapacityOpportunity> 按 slack_t 降序
/// 刷新触发: plan_item_changed, capacity_pool_changed
pub trait CapacityOpportunityUseCase {
    /// 查询产能优化机会
    fn get_capacity_opportunity(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<CapacityOpportunity>, String>;

    /// 查询最大优化空间
    fn get_top_opportunities(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        top_n: usize,
    ) -> Result<Vec<CapacityOpportunity>, String>;

    /// 获取产能优化总结
    fn get_optimization_summary(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<OptimizationSummary, String>;
}

/// 产能优化机会
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapacityOpportunity {
    /// 版本 ID
    pub version_id: String,

    /// 机组代码
    pub machine_code: String,

    /// 排产日期 (YYYY-MM-DD)
    pub plan_date: String,

    /// 松弛产能 (吨)
    pub slack_t: f64,

    /// 软约束可调整空间 (吨)
    pub soft_adjust_space_t: Option<f64>,

    /// 产能利用率 (0.0-1.0)
    pub utilization_rate: f64,

    /// 绑定约束列表
    pub binding_constraints: Vec<BindingConstraint>,

    /// 优化潜力等级 (NONE/LOW/MEDIUM/HIGH)
    pub opportunity_level: String,

    /// 敏感性分析
    pub sensitivity: Option<SensitivityAnalysis>,

    /// 建议优化措施
    pub suggested_optimizations: Vec<String>,
}

/// 绑定约束
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BindingConstraint {
    /// 约束代码
    pub code: String,

    /// 约束描述
    pub description: String,

    /// 约束类型 (HARD/SOFT)
    pub constraint_type: String,

    /// 影响程度 (0.0-1.0)
    pub impact: f64,

    /// 是否可放松
    pub is_relaxable: bool,
}

/// 敏感性分析
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensitivityAnalysis {
    /// 如果放松软约束，可增加产能 (吨)
    pub soft_constraint_gain_t: f64,

    /// 如果调整目标产能，可增加产能 (吨)
    pub target_adjustment_gain_t: f64,

    /// 如果优化结构，可增加产能 (吨)
    pub structure_optimization_gain_t: f64,

    /// 总潜在增益 (吨)
    pub total_potential_gain_t: f64,

    /// 风险评估
    pub risk_assessment: String,
}

/// 优化总结
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptimizationSummary {
    /// 版本 ID
    pub version_id: String,

    /// 日期范围
    pub date_range: (String, String),

    /// 总松弛产能 (吨)
    pub total_slack_t: f64,

    /// 总软约束可调整空间 (吨)
    pub total_soft_adjust_space_t: f64,

    /// 平均产能利用率 (0.0-1.0)
    pub avg_utilization_rate: f64,

    /// 高优化潜力机组-日数量
    pub high_opportunity_count: i32,

    /// 按机组分组统计
    pub by_machine: Vec<MachineOpportunityStat>,

    /// 总潜在增益 (吨)
    pub total_potential_gain_t: f64,
}

/// 机组优化机会统计
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineOpportunityStat {
    /// 机组代码
    pub machine_code: String,

    /// 松弛产能 (吨)
    pub slack_t: f64,

    /// 平均利用率
    pub avg_utilization: f64,

    /// 优化机会数量
    pub opportunity_count: i32,
}

impl CapacityOpportunity {
    /// 创建新的产能优化机会
    pub fn new(
        version_id: String,
        machine_code: String,
        plan_date: String,
        slack_t: f64,
        utilization_rate: f64,
    ) -> Self {
        let mut opportunity = Self {
            version_id,
            machine_code,
            plan_date,
            slack_t,
            soft_adjust_space_t: None,
            utilization_rate,
            binding_constraints: Vec::new(),
            opportunity_level: "NONE".to_string(),
            sensitivity: None,
            suggested_optimizations: Vec::new(),
        };

        opportunity.calculate_opportunity_level();
        opportunity
    }

    /// 计算优化潜力等级
    fn calculate_opportunity_level(&mut self) {
        // 基于松弛产能和利用率判断
        if self.slack_t >= 500.0 && self.utilization_rate < 0.7 {
            self.opportunity_level = "HIGH".to_string();
        } else if self.slack_t >= 200.0 && self.utilization_rate < 0.85 {
            self.opportunity_level = "MEDIUM".to_string();
        } else if self.slack_t >= 50.0 {
            self.opportunity_level = "LOW".to_string();
        } else {
            self.opportunity_level = "NONE".to_string();
        }
    }

    /// 设置软约束可调整空间
    pub fn set_soft_adjust_space(&mut self, space_t: f64) {
        self.soft_adjust_space_t = Some(space_t);
        self.calculate_opportunity_level();
    }

    /// 添加绑定约束
    pub fn add_binding_constraint(
        &mut self,
        code: String,
        description: String,
        constraint_type: String,
        impact: f64,
        is_relaxable: bool,
    ) {
        self.binding_constraints.push(BindingConstraint {
            code,
            description,
            constraint_type,
            impact,
            is_relaxable,
        });
    }

    /// 设置敏感性分析
    pub fn set_sensitivity(&mut self, sensitivity: SensitivityAnalysis) {
        self.sensitivity = Some(sensitivity);
    }

    /// 添加建议优化措施
    pub fn add_suggested_optimization(&mut self, optimization: String) {
        self.suggested_optimizations.push(optimization);
    }

    /// 判断是否有优化空间
    pub fn has_opportunity(&self) -> bool {
        self.opportunity_level != "NONE"
    }

    /// 判断是否为高优化潜力
    pub fn is_high_opportunity(&self) -> bool {
        self.opportunity_level == "HIGH"
    }

    /// 获取可放松的约束数量
    pub fn relaxable_constraint_count(&self) -> usize {
        self.binding_constraints
            .iter()
            .filter(|c| c.is_relaxable)
            .count()
    }

    /// 获取总潜在增益
    pub fn total_potential_gain(&self) -> f64 {
        self.sensitivity
            .as_ref()
            .map(|s| s.total_potential_gain_t)
            .unwrap_or(0.0)
    }
}

impl SensitivityAnalysis {
    /// 创建新的敏感性分析
    pub fn new(
        soft_constraint_gain_t: f64,
        target_adjustment_gain_t: f64,
        structure_optimization_gain_t: f64,
    ) -> Self {
        let total_potential_gain_t =
            soft_constraint_gain_t + target_adjustment_gain_t + structure_optimization_gain_t;

        let risk_assessment = if total_potential_gain_t >= 1000.0 {
            "高增益，建议优先考虑".to_string()
        } else if total_potential_gain_t >= 500.0 {
            "中等增益，可考虑优化".to_string()
        } else if total_potential_gain_t >= 100.0 {
            "低增益，视情况优化".to_string()
        } else {
            "增益较小，不建议优化".to_string()
        };

        Self {
            soft_constraint_gain_t,
            target_adjustment_gain_t,
            structure_optimization_gain_t,
            total_potential_gain_t,
            risk_assessment,
        }
    }

    /// 判断是否值得优化
    pub fn is_worth_optimizing(&self) -> bool {
        self.total_potential_gain_t >= 100.0
    }
}

impl OptimizationSummary {
    /// 创建新的优化总结
    pub fn new(version_id: String, start_date: String, end_date: String) -> Self {
        Self {
            version_id,
            date_range: (start_date, end_date),
            total_slack_t: 0.0,
            total_soft_adjust_space_t: 0.0,
            avg_utilization_rate: 0.0,
            high_opportunity_count: 0,
            by_machine: Vec::new(),
            total_potential_gain_t: 0.0,
        }
    }

    /// 添加优化机会到总结
    pub fn add_opportunity(&mut self, opportunity: &CapacityOpportunity) {
        self.total_slack_t += opportunity.slack_t;
        if let Some(soft_space) = opportunity.soft_adjust_space_t {
            self.total_soft_adjust_space_t += soft_space;
        }
        if opportunity.is_high_opportunity() {
            self.high_opportunity_count += 1;
        }
        self.total_potential_gain_t += opportunity.total_potential_gain();
    }

    /// 添加机组统计
    pub fn add_machine_stat(&mut self, stat: MachineOpportunityStat) {
        self.by_machine.push(stat);
    }

    /// 计算平均利用率
    pub fn calculate_avg_utilization(&mut self, total_utilization: f64, count: i32) {
        if count > 0 {
            self.avg_utilization_rate = total_utilization / count as f64;
        }
    }

    /// 判断是否存在优化空间
    pub fn has_optimization_space(&self) -> bool {
        self.total_slack_t > 0.0 || self.total_soft_adjust_space_t > 0.0
    }
}

impl std::fmt::Display for CapacityOpportunity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{} (slack: {:.1}t, util: {:.1}%, level: {})",
            self.machine_code,
            self.plan_date,
            self.slack_t,
            self.utilization_rate * 100.0,
            self.opportunity_level
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capacity_opportunity_creation() {
        let opportunity = CapacityOpportunity::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-23".to_string(),
            600.0,
            0.65,
        );

        assert_eq!(opportunity.slack_t, 600.0);
        assert_eq!(opportunity.opportunity_level, "HIGH");
        assert!(opportunity.has_opportunity());
        assert!(opportunity.is_high_opportunity());
    }

    #[test]
    fn test_opportunity_levels() {
        let high = CapacityOpportunity::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-23".to_string(),
            600.0,
            0.65,
        );
        assert_eq!(high.opportunity_level, "HIGH");

        let medium = CapacityOpportunity::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-23".to_string(),
            300.0,
            0.80,
        );
        assert_eq!(medium.opportunity_level, "MEDIUM");

        let low = CapacityOpportunity::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-23".to_string(),
            100.0,
            0.90,
        );
        assert_eq!(low.opportunity_level, "LOW");

        let none = CapacityOpportunity::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-23".to_string(),
            30.0,
            0.97,
        );
        assert_eq!(none.opportunity_level, "NONE");
    }

    #[test]
    fn test_binding_constraints() {
        let mut opportunity = CapacityOpportunity::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-23".to_string(),
            600.0,
            0.65,
        );

        opportunity.add_binding_constraint(
            "STRUCT".to_string(),
            "结构约束".to_string(),
            "SOFT".to_string(),
            0.7,
            true,
        );

        opportunity.add_binding_constraint(
            "CAP".to_string(),
            "产能约束".to_string(),
            "HARD".to_string(),
            0.9,
            false,
        );

        assert_eq!(opportunity.binding_constraints.len(), 2);
        assert_eq!(opportunity.relaxable_constraint_count(), 1);
    }

    #[test]
    fn test_sensitivity_analysis() {
        let sensitivity = SensitivityAnalysis::new(300.0, 200.0, 500.0);

        assert_eq!(sensitivity.total_potential_gain_t, 1000.0);
        assert!(sensitivity.is_worth_optimizing());
        assert!(sensitivity.risk_assessment.contains("高增益"));
    }

    #[test]
    fn test_optimization_summary() {
        let mut summary = OptimizationSummary::new(
            "V001".to_string(),
            "2026-01-23".to_string(),
            "2026-01-25".to_string(),
        );

        let mut opp1 = CapacityOpportunity::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-23".to_string(),
            600.0,
            0.65,
        );
        opp1.set_sensitivity(SensitivityAnalysis::new(300.0, 200.0, 500.0));

        let opp2 = CapacityOpportunity::new(
            "V001".to_string(),
            "H033".to_string(),
            "2026-01-23".to_string(),
            300.0,
            0.80,
        );

        summary.add_opportunity(&opp1);
        summary.add_opportunity(&opp2);

        assert_eq!(summary.total_slack_t, 900.0);
        assert_eq!(summary.high_opportunity_count, 1);
        assert_eq!(summary.total_potential_gain_t, 1000.0);
        assert!(summary.has_optimization_space());
    }
}

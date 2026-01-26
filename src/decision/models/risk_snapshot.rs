// ==========================================
// 热轧精整排产系统 - 决策对象：风险快照视图
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 6 节
// 职责: 定义风险快照视图决策对象，对 risk_snapshot 表的标准化读模型对象
// ==========================================

use serde::{Deserialize, Serialize};

/// 风险快照视图 (RiskSnapshotView)
///
/// risk_snapshot 表的标准化读模型对象。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskSnapshotView {
    /// 所属版本 ID
    pub version_id: String,

    /// 机组代码
    pub machine_code: String,

    /// 排产日期
    pub plan_date: String,

    /// 风险等级 (LOW/MEDIUM/HIGH/CRITICAL)
    pub risk_level: String,

    /// 风险分数 (0-100)
    pub risk_score: f64,

    /// 已使用产能 (吨)
    pub used_capacity_t: f64,

    /// 目标产能 (吨)
    pub target_capacity_t: f64,

    /// 限制产能 (吨)
    pub limit_capacity_t: f64,

    /// 产能利用率 (0.0-1.0)
    pub capacity_utilization: f64,

    /// 风险因素列表
    pub risk_factors: Vec<RiskFactor>,

    /// 主要风险原因
    pub primary_reason: Option<String>,

    /// 建议措施
    pub suggested_actions: Vec<String>,
}

/// 风险因素
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskFactor {
    /// 因素代码
    pub code: String,
    /// 因素描述
    pub description: String,
    /// 严重程度 (0.0-1.0)
    pub severity: f64,
    /// 权重 (0.0-1.0)
    pub weight: f64,
}

impl RiskSnapshotView {
    /// 创建新的风险快照视图
    pub fn new(
        version_id: String,
        machine_code: String,
        plan_date: String,
    ) -> Self {
        Self {
            version_id,
            machine_code,
            plan_date,
            risk_level: "LOW".to_string(),
            risk_score: 0.0,
            used_capacity_t: 0.0,
            target_capacity_t: 0.0,
            limit_capacity_t: 0.0,
            capacity_utilization: 0.0,
            risk_factors: Vec::new(),
            primary_reason: None,
            suggested_actions: Vec::new(),
        }
    }

    /// 设置产能信息
    pub fn set_capacity_info(
        &mut self,
        used_t: f64,
        target_t: f64,
        limit_t: f64,
    ) {
        self.used_capacity_t = used_t;
        self.target_capacity_t = target_t;
        self.limit_capacity_t = limit_t;
        self.capacity_utilization = if target_t > 0.0 {
            used_t / target_t
        } else {
            0.0
        };
    }

    /// 添加风险因素
    pub fn add_risk_factor(
        &mut self,
        code: String,
        description: String,
        severity: f64,
        weight: f64,
    ) {
        self.risk_factors.push(RiskFactor {
            code,
            description,
            severity,
            weight,
        });
        self.recalculate_risk();
    }

    /// 重新计算风险分数和等级
    fn recalculate_risk(&mut self) {
        if self.risk_factors.is_empty() {
            self.risk_score = 0.0;
            self.risk_level = "LOW".to_string();
            self.primary_reason = None;
            return;
        }

        // 计算加权风险分数
        let weighted_sum: f64 = self.risk_factors
            .iter()
            .map(|f| f.severity * f.weight)
            .sum();

        let total_weight: f64 = self.risk_factors.iter().map(|f| f.weight).sum();

        self.risk_score = if total_weight > 0.0 {
            (weighted_sum / total_weight * 100.0).min(100.0)
        } else {
            0.0
        };

        // 确定风险等级
        self.risk_level = match self.risk_score {
            s if s >= 80.0 => "CRITICAL".to_string(),
            s if s >= 60.0 => "HIGH".to_string(),
            s if s >= 40.0 => "MEDIUM".to_string(),
            _ => "LOW".to_string(),
        };

        // 确定主要风险原因
        self.primary_reason = self.risk_factors
            .iter()
            .max_by(|a, b| {
                let score_a = a.severity * a.weight;
                let score_b = b.severity * b.weight;
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|f| f.description.clone());
    }

    /// 添加建议措施
    pub fn add_suggested_action(&mut self, action: String) {
        self.suggested_actions.push(action);
    }

    /// 判断是否为高风险
    pub fn is_high_risk(&self) -> bool {
        matches!(self.risk_level.as_str(), "HIGH" | "CRITICAL")
    }

    /// 判断是否超过目标产能
    pub fn is_over_target(&self) -> bool {
        self.used_capacity_t > self.target_capacity_t
    }

    /// 判断是否超过限制产能
    pub fn is_over_limit(&self) -> bool {
        self.used_capacity_t > self.limit_capacity_t
    }

    /// 获取产能松弛度
    pub fn capacity_slack(&self) -> f64 {
        (self.target_capacity_t - self.used_capacity_t).max(0.0)
    }

    /// 获取产能超载量
    pub fn capacity_overload(&self) -> f64 {
        (self.used_capacity_t - self.target_capacity_t).max(0.0)
    }

    /// 获取风险因素数量
    pub fn risk_factor_count(&self) -> usize {
        self.risk_factors.len()
    }

    /// 获取高严重度风险因素 (severity > 阈值)
    pub fn high_severity_factors(&self, threshold: f64) -> Vec<&RiskFactor> {
        self.risk_factors
            .iter()
            .filter(|f| f.severity > threshold)
            .collect()
    }
}

impl std::fmt::Display for RiskSnapshotView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{} (risk: {}, score: {:.1}, util: {:.1}%)",
            self.machine_code,
            self.plan_date,
            self.risk_level,
            self.risk_score,
            self.capacity_utilization * 100.0
        )
    }
}

impl RiskFactor {
    /// 创建新的风险因素
    pub fn new(code: String, description: String, severity: f64, weight: f64) -> Self {
        Self {
            code,
            description,
            severity,
            weight,
        }
    }

    /// 计算加权分数
    pub fn weighted_score(&self) -> f64 {
        self.severity * self.weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_snapshot_view_creation() {
        let view = RiskSnapshotView::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-22".to_string(),
        );

        assert_eq!(view.risk_level, "LOW");
        assert_eq!(view.risk_score, 0.0);
    }

    #[test]
    fn test_capacity_info() {
        let mut view = RiskSnapshotView::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-22".to_string(),
        );

        view.set_capacity_info(950.0, 1000.0, 1200.0);

        assert_eq!(view.used_capacity_t, 950.0);
        assert_eq!(view.target_capacity_t, 1000.0);
        assert_eq!(view.capacity_utilization, 0.95);
        assert!(!view.is_over_target());
        assert!(!view.is_over_limit());
    }

    #[test]
    fn test_risk_calculation() {
        let mut view = RiskSnapshotView::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-22".to_string(),
        );

        view.add_risk_factor(
            "CAP_HIGH".to_string(),
            "产能利用率高".to_string(),
            0.9,
            0.5,
        );
        view.add_risk_factor(
            "COLD_STOCK".to_string(),
            "冷料压库".to_string(),
            0.7,
            0.3,
        );

        assert!(view.risk_score > 0.0);
        assert!(view.is_high_risk());
        assert!(view.primary_reason.is_some());
    }

    #[test]
    fn test_risk_levels() {
        let mut view = RiskSnapshotView::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-22".to_string(),
        );

        // CRITICAL
        view.add_risk_factor("TEST".to_string(), "测试".to_string(), 0.9, 1.0);
        assert_eq!(view.risk_level, "CRITICAL");

        // HIGH
        view.risk_factors.clear();
        view.add_risk_factor("TEST".to_string(), "测试".to_string(), 0.7, 1.0);
        view.recalculate_risk();
        assert_eq!(view.risk_level, "HIGH");

        // MEDIUM
        view.risk_factors.clear();
        view.add_risk_factor("TEST".to_string(), "测试".to_string(), 0.5, 1.0);
        view.recalculate_risk();
        assert_eq!(view.risk_level, "MEDIUM");

        // LOW
        view.risk_factors.clear();
        view.add_risk_factor("TEST".to_string(), "测试".to_string(), 0.3, 1.0);
        view.recalculate_risk();
        assert_eq!(view.risk_level, "LOW");
    }

    #[test]
    fn test_high_severity_factors() {
        let mut view = RiskSnapshotView::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-22".to_string(),
        );

        view.add_risk_factor("F1".to_string(), "因素1".to_string(), 0.9, 0.5);
        view.add_risk_factor("F2".to_string(), "因素2".to_string(), 0.5, 0.3);
        view.add_risk_factor("F3".to_string(), "因素3".to_string(), 0.3, 0.2);

        let high_factors = view.high_severity_factors(0.7);
        assert_eq!(high_factors.len(), 1);
        assert_eq!(high_factors[0].code, "F1");
    }

    #[test]
    fn test_suggested_actions() {
        let mut view = RiskSnapshotView::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-22".to_string(),
        );

        view.add_suggested_action("调整产能参数".to_string());
        view.add_suggested_action("安排换辊".to_string());

        assert_eq!(view.suggested_actions.len(), 2);
    }
}

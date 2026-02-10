// ==========================================
// 热轧精整排产系统 - D1 用例：哪天最危险
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 5.1 节
// 职责: 回答"哪天最危险"，按日期返回风险排行与解释
// ==========================================

use serde::{Deserialize, Serialize};

/// D1 用例：哪天最危险
///
/// 输入: version_id, date_range
/// 输出: Vec<DaySummary> 按 risk_score 降序
/// 刷新触发: risk_snapshot_updated, plan_item_changed
pub trait MostRiskyDayUseCase {
    /// 查询日期风险摘要
    fn get_day_summary(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<DaySummary>, String>;

    /// 查询最危险的 N 天
    fn get_top_risky_days(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        top_n: usize,
    ) -> Result<Vec<DaySummary>, String>;
}

/// 日期风险摘要
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaySummary {
    /// 排产日期 (YYYY-MM-DD)
    pub plan_date: String,

    /// 风险分数 (0-100)
    pub risk_score: f64,

    /// 风险等级 (LOW/MEDIUM/HIGH/CRITICAL)
    pub risk_level: String,

    /// 产能利用率 (0.0-1.0)
    pub capacity_util_pct: f64,

    /// 主要风险原因列表
    pub top_reasons: Vec<ReasonItem>,

    /// 受影响机组数量
    pub affected_machines: i32,

    /// 堵塞机组数量
    pub bottleneck_machines: i32,

    /// 是否存在换辊风险
    pub has_roll_risk: bool,

    /// 建议措施
    pub suggested_actions: Vec<String>,
}

/// 风险原因项
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasonItem {
    /// 原因代码
    pub code: String,

    /// 原因描述
    pub msg: String,

    /// 权重 (0.0-1.0)
    pub weight: f64,

    /// 严重程度 (0.0-1.0)
    pub severity: f64,
}

impl DaySummary {
    /// 创建新的日期摘要
    pub fn new(plan_date: String) -> Self {
        Self {
            plan_date,
            risk_score: 0.0,
            risk_level: "LOW".to_string(),
            capacity_util_pct: 0.0,
            top_reasons: Vec::new(),
            affected_machines: 0,
            bottleneck_machines: 0,
            has_roll_risk: false,
            suggested_actions: Vec::new(),
        }
    }

    /// 添加风险原因
    pub fn add_reason(&mut self, code: String, msg: String, weight: f64, severity: f64) {
        self.top_reasons.push(ReasonItem {
            code,
            msg,
            weight,
            severity,
        });
        self.recalculate_risk();
    }

    /// 重新计算风险分数
    fn recalculate_risk(&mut self) {
        if self.top_reasons.is_empty() {
            self.risk_score = 0.0;
            self.risk_level = "LOW".to_string();
            return;
        }

        // 计算加权风险分数
        let weighted_sum: f64 = self.top_reasons.iter().map(|r| r.severity * r.weight).sum();

        let total_weight: f64 = self.top_reasons.iter().map(|r| r.weight).sum();

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
    }

    /// 设置产能信息
    pub fn set_capacity_info(&mut self, util_pct: f64, affected: i32, bottleneck: i32) {
        self.capacity_util_pct = util_pct;
        self.affected_machines = affected;
        self.bottleneck_machines = bottleneck;
    }

    /// 设置换辊风险
    pub fn set_roll_risk(&mut self, has_risk: bool) {
        self.has_roll_risk = has_risk;
    }

    /// 添加建议措施
    pub fn add_suggested_action(&mut self, action: String) {
        self.suggested_actions.push(action);
    }

    /// 判断是否为高风险日期
    pub fn is_high_risk(&self) -> bool {
        matches!(self.risk_level.as_str(), "HIGH" | "CRITICAL")
    }

    /// 判断是否存在堵塞
    pub fn has_bottleneck(&self) -> bool {
        self.bottleneck_machines > 0
    }

    /// 获取最严重的原因
    pub fn primary_reason(&self) -> Option<&ReasonItem> {
        self.top_reasons.iter().max_by(|a, b| {
            let score_a = a.severity * a.weight;
            let score_b = b.severity * b.weight;
            score_a
                .partial_cmp(&score_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

impl ReasonItem {
    /// 创建新的原因项
    pub fn new(code: String, msg: String, weight: f64, severity: f64) -> Self {
        Self {
            code,
            msg,
            weight,
            severity,
        }
    }

    /// 计算加权分数
    pub fn weighted_score(&self) -> f64 {
        self.severity * self.weight
    }
}

impl std::fmt::Display for DaySummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (risk: {}, score: {:.1}, util: {:.1}%)",
            self.plan_date,
            self.risk_level,
            self.risk_score,
            self.capacity_util_pct * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_summary_creation() {
        let summary = DaySummary::new("2026-01-23".to_string());
        assert_eq!(summary.plan_date, "2026-01-23");
        assert_eq!(summary.risk_level, "LOW");
        assert_eq!(summary.risk_score, 0.0);
    }

    #[test]
    fn test_add_reason() {
        let mut summary = DaySummary::new("2026-01-23".to_string());
        summary.add_reason("CAP_HIGH".to_string(), "产能利用率高".to_string(), 0.5, 0.9);
        summary.add_reason("BOTTLENECK".to_string(), "存在堵塞".to_string(), 0.3, 0.8);

        assert_eq!(summary.top_reasons.len(), 2);
        assert!(summary.risk_score > 0.0);
        assert!(summary.is_high_risk());
    }

    #[test]
    fn test_risk_levels() {
        let mut summary = DaySummary::new("2026-01-23".to_string());

        // CRITICAL
        summary.add_reason("TEST".to_string(), "测试".to_string(), 1.0, 0.9);
        assert_eq!(summary.risk_level, "CRITICAL");

        // HIGH
        summary.top_reasons.clear();
        summary.add_reason("TEST".to_string(), "测试".to_string(), 1.0, 0.7);
        summary.recalculate_risk();
        assert_eq!(summary.risk_level, "HIGH");

        // MEDIUM
        summary.top_reasons.clear();
        summary.add_reason("TEST".to_string(), "测试".to_string(), 1.0, 0.5);
        summary.recalculate_risk();
        assert_eq!(summary.risk_level, "MEDIUM");

        // LOW
        summary.top_reasons.clear();
        summary.add_reason("TEST".to_string(), "测试".to_string(), 1.0, 0.3);
        summary.recalculate_risk();
        assert_eq!(summary.risk_level, "LOW");
    }

    #[test]
    fn test_capacity_info() {
        let mut summary = DaySummary::new("2026-01-23".to_string());
        summary.set_capacity_info(0.95, 3, 1);

        assert_eq!(summary.capacity_util_pct, 0.95);
        assert_eq!(summary.affected_machines, 3);
        assert_eq!(summary.bottleneck_machines, 1);
        assert!(summary.has_bottleneck());
    }

    #[test]
    fn test_primary_reason() {
        let mut summary = DaySummary::new("2026-01-23".to_string());
        summary.add_reason("R1".to_string(), "原因1".to_string(), 0.5, 0.9);
        summary.add_reason("R2".to_string(), "原因2".to_string(), 0.3, 0.7);

        let primary = summary.primary_reason();
        assert!(primary.is_some());
        assert_eq!(primary.unwrap().code, "R1");
    }

    #[test]
    fn test_suggested_actions() {
        let mut summary = DaySummary::new("2026-01-23".to_string());
        summary.add_suggested_action("调整产能参数".to_string());
        summary.add_suggested_action("安排换辊".to_string());

        assert_eq!(summary.suggested_actions.len(), 2);
    }
}

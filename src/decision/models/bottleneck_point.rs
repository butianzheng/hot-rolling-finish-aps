// ==========================================
// 热轧精整排产系统 - 决策对象：瓶颈点
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 6 节
// 职责: 定义瓶颈点决策对象，表示机组-日在产能、结构、换辊等维度的综合堵塞信号
// ==========================================

use serde::{Deserialize, Serialize};

/// 瓶颈点 (BottleneckPoint)
///
/// 机组-日在产能、结构、换辊等维度的综合堵塞信号。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BottleneckPoint {
    /// 所属版本 ID
    pub version_id: String,

    /// 机组代码
    pub machine_code: String,

    /// 排产日期
    pub plan_date: String,

    /// 瓶颈分数 (0-100)
    pub bottleneck_score: f64,

    /// 瓶颈类型
    pub bottleneck_type: BottleneckType,

    /// 瓶颈原因列表
    pub reasons: Vec<BottleneckReason>,

    /// 剩余产能 (吨)
    pub remaining_capacity_t: f64,

    /// 产能利用率 (0.0-1.0)
    pub capacity_utilization: f64,

    /// 是否需要换辊
    pub needs_roll_change: bool,

    /// 结构违规数量
    pub structure_violations: i32,

    /// 建议措施
    pub suggested_actions: Vec<String>,
}

/// 瓶颈类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottleneckType {
    /// 无瓶颈
    None,
    /// 产能瓶颈
    Capacity,
    /// 结构瓶颈
    Structure,
    /// 换辊瓶颈
    RollChange,
    /// 综合瓶颈
    Combined,
}

/// 瓶颈原因
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BottleneckReason {
    /// 原因代码
    pub code: String,
    /// 原因描述
    pub message: String,
    /// 严重程度 (0.0-1.0)
    pub severity: f64,
}

impl BottleneckPoint {
    /// 创建新的瓶颈点
    pub fn new(version_id: String, machine_code: String, plan_date: String) -> Self {
        Self {
            version_id,
            machine_code,
            plan_date,
            bottleneck_score: 0.0,
            bottleneck_type: BottleneckType::None,
            reasons: Vec::new(),
            remaining_capacity_t: 0.0,
            capacity_utilization: 0.0,
            needs_roll_change: false,
            structure_violations: 0,
            suggested_actions: Vec::new(),
        }
    }

    /// 添加瓶颈原因
    pub fn add_reason(&mut self, code: String, message: String, severity: f64) {
        self.reasons.push(BottleneckReason {
            code,
            message,
            severity,
        });
        self.recalculate_score();
    }

    /// 设置产能信息
    pub fn set_capacity_info(&mut self, remaining_t: f64, utilization: f64) {
        self.remaining_capacity_t = remaining_t;
        self.capacity_utilization = utilization;

        // 判断是否为产能瓶颈
        if utilization >= 0.95 {
            self.add_reason(
                "CAP_CRITICAL".to_string(),
                format!("产能利用率达到 {:.1}%", utilization * 100.0),
                0.9,
            );
        } else if utilization >= 0.85 {
            self.add_reason(
                "CAP_HIGH".to_string(),
                format!("产能利用率达到 {:.1}%", utilization * 100.0),
                0.7,
            );
        }
    }

    /// 设置换辊信息
    pub fn set_roll_change_info(&mut self, needs_change: bool) {
        self.needs_roll_change = needs_change;
        if needs_change {
            self.add_reason("ROLL_CHANGE".to_string(), "需要换辊".to_string(), 0.8);
        }
    }

    /// 设置结构违规信息
    pub fn set_structure_violations(&mut self, violations: i32) {
        self.structure_violations = violations;
        if violations > 0 {
            self.add_reason(
                "STRUCTURE_VIOLATION".to_string(),
                format!("存在 {} 个结构违规", violations),
                0.6,
            );
        }
    }

    /// 重新计算瓶颈分数
    fn recalculate_score(&mut self) {
        if self.reasons.is_empty() {
            self.bottleneck_score = 0.0;
            self.bottleneck_type = BottleneckType::None;
            return;
        }

        // 计算加权平均严重程度
        let total_severity: f64 = self.reasons.iter().map(|r| r.severity).sum();
        let avg_severity = total_severity / self.reasons.len() as f64;

        // 瓶颈分数 = 平均严重程度 * 100
        self.bottleneck_score = (avg_severity * 100.0).min(100.0);

        // 判断瓶颈类型
        self.bottleneck_type = self.determine_bottleneck_type();
    }

    /// 判断瓶颈类型
    fn determine_bottleneck_type(&self) -> BottleneckType {
        let has_capacity = self.reasons.iter().any(|r| r.code.starts_with("CAP_"));
        let has_structure = self.structure_violations > 0;
        let has_roll = self.needs_roll_change;

        match (has_capacity, has_structure, has_roll) {
            (true, true, _) | (true, _, true) | (_, true, true) => BottleneckType::Combined,
            (true, false, false) => BottleneckType::Capacity,
            (false, true, false) => BottleneckType::Structure,
            (false, false, true) => BottleneckType::RollChange,
            (false, false, false) => BottleneckType::None,
        }
    }

    /// 添加建议措施
    pub fn add_suggested_action(&mut self, action: String) {
        self.suggested_actions.push(action);
    }

    /// 判断是否为严重瓶颈 (分数 > 阈值)
    pub fn is_severe(&self, threshold: f64) -> bool {
        self.bottleneck_score > threshold
    }

    /// 获取瓶颈等级描述
    pub fn severity_level(&self) -> &str {
        match self.bottleneck_score {
            s if s >= 80.0 => "严重",
            s if s >= 60.0 => "高",
            s if s >= 40.0 => "中",
            s if s >= 20.0 => "低",
            _ => "正常",
        }
    }

    /// 获取主要瓶颈原因
    pub fn primary_reason(&self) -> Option<&BottleneckReason> {
        self.reasons.iter().max_by(|a, b| {
            a.severity
                .partial_cmp(&b.severity)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

impl std::fmt::Display for BottleneckPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{} (score: {:.1}, type: {:?}, level: {})",
            self.machine_code,
            self.plan_date,
            self.bottleneck_score,
            self.bottleneck_type,
            self.severity_level()
        )
    }
}

impl std::fmt::Display for BottleneckType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BottleneckType::None => write!(f, "无瓶颈"),
            BottleneckType::Capacity => write!(f, "产能瓶颈"),
            BottleneckType::Structure => write!(f, "结构瓶颈"),
            BottleneckType::RollChange => write!(f, "换辊瓶颈"),
            BottleneckType::Combined => write!(f, "综合瓶颈"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bottleneck_point_creation() {
        let point = BottleneckPoint::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-22".to_string(),
        );

        assert_eq!(point.bottleneck_score, 0.0);
        assert_eq!(point.bottleneck_type, BottleneckType::None);
    }

    #[test]
    fn test_capacity_bottleneck() {
        let mut point = BottleneckPoint::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-22".to_string(),
        );

        point.set_capacity_info(50.0, 0.96);

        assert!(point.bottleneck_score > 0.0);
        assert_eq!(point.bottleneck_type, BottleneckType::Capacity);
        assert_eq!(point.severity_level(), "严重");
    }

    #[test]
    fn test_combined_bottleneck() {
        let mut point = BottleneckPoint::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-22".to_string(),
        );

        point.set_capacity_info(50.0, 0.96);
        point.set_roll_change_info(true);
        point.set_structure_violations(3);

        assert_eq!(point.bottleneck_type, BottleneckType::Combined);
        assert!(point.is_severe(70.0));
    }

    #[test]
    fn test_primary_reason() {
        let mut point = BottleneckPoint::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-22".to_string(),
        );

        point.add_reason("REASON1".to_string(), "原因1".to_string(), 0.5);
        point.add_reason("REASON2".to_string(), "原因2".to_string(), 0.9);
        point.add_reason("REASON3".to_string(), "原因3".to_string(), 0.3);

        let primary = point.primary_reason().unwrap();
        assert_eq!(primary.code, "REASON2");
        assert_eq!(primary.severity, 0.9);
    }

    #[test]
    fn test_suggested_actions() {
        let mut point = BottleneckPoint::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-22".to_string(),
        );

        point.add_suggested_action("调整产能参数".to_string());
        point.add_suggested_action("安排换辊".to_string());

        assert_eq!(point.suggested_actions.len(), 2);
    }
}

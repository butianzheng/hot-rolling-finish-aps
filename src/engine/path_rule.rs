// ==========================================
// 热轧精整排产系统 - 宽厚路径规则引擎
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 14-18 (v0.6)
// 规则: 由宽到窄、由厚到薄（允许容差）
// ==========================================

use crate::domain::types::{PathRuleStatus, PathViolationType, UrgentLevel};

/// 路径规则检查结果
#[derive(Debug, Clone)]
pub struct PathRuleResult {
    pub status: PathRuleStatus,
    pub violation_type: Option<PathViolationType>,
    pub width_delta_mm: f64,
    pub thickness_delta_mm: f64,
}

/// 路径规则配置
#[derive(Debug, Clone)]
pub struct PathRuleConfig {
    pub enabled: bool,
    pub width_tolerance_mm: f64,
    pub thickness_tolerance_mm: f64,
    pub override_allowed_urgency_levels: Vec<UrgentLevel>,
}

impl Default for PathRuleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            width_tolerance_mm: 50.0,
            thickness_tolerance_mm: 1.0,
            override_allowed_urgency_levels: vec![UrgentLevel::L2, UrgentLevel::L3],
        }
    }
}

/// 锚点状态（上一块入池材料的宽厚）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    pub width_mm: f64,
    pub thickness_mm: f64,
}

/// PathRuleEngine - 宽厚路径规则引擎
pub struct PathRuleEngine {
    config: PathRuleConfig,
}

impl PathRuleEngine {
    pub fn new(config: PathRuleConfig) -> Self {
        Self { config }
    }

    /// 检查材料是否满足路径约束
    ///
    /// # 参数
    /// - `candidate_width_mm`: 候选材料宽度
    /// - `candidate_thickness_mm`: 候选材料厚度
    /// - `candidate_urgent_level`: 候选材料紧急等级
    /// - `anchor`: 当前锚点（None 表示无锚点，跳过检查）
    /// - `user_confirmed`: 是否已人工确认（突破）
    pub fn check(
        &self,
        candidate_width_mm: f64,
        candidate_thickness_mm: f64,
        candidate_urgent_level: UrgentLevel,
        anchor: Option<&Anchor>,
        user_confirmed: bool,
    ) -> PathRuleResult {
        // 规则禁用：直接放行
        if !self.config.enabled {
            return PathRuleResult {
                status: PathRuleStatus::Ok,
                violation_type: None,
                width_delta_mm: 0.0,
                thickness_delta_mm: 0.0,
            };
        }

        // 无锚点：首块材料不做路径门控
        let anchor = match anchor {
            Some(a) => a,
            None => {
                return PathRuleResult {
                    status: PathRuleStatus::Ok,
                    violation_type: None,
                    width_delta_mm: 0.0,
                    thickness_delta_mm: 0.0,
                };
            }
        };

        let raw_width_delta = candidate_width_mm - anchor.width_mm - self.config.width_tolerance_mm;
        let raw_thickness_delta =
            candidate_thickness_mm - anchor.thickness_mm - self.config.thickness_tolerance_mm;

        let width_exceeded = raw_width_delta > 0.0;
        let thickness_exceeded = raw_thickness_delta > 0.0;

        // 无违规
        if !width_exceeded && !thickness_exceeded {
            return PathRuleResult {
                status: PathRuleStatus::Ok,
                violation_type: None,
                width_delta_mm: 0.0,
                thickness_delta_mm: 0.0,
            };
        }

        let violation_type = if width_exceeded && thickness_exceeded {
            PathViolationType::BothExceeded
        } else if width_exceeded {
            PathViolationType::WidthExceeded
        } else {
            PathViolationType::ThicknessExceeded
        };

        let width_delta_mm = raw_width_delta.max(0.0);
        let thickness_delta_mm = raw_thickness_delta.max(0.0);

        // 已人工确认：放行，但保留违规信息供可解释性/审计
        if user_confirmed {
            return PathRuleResult {
                status: PathRuleStatus::Ok,
                violation_type: Some(violation_type),
                width_delta_mm,
                thickness_delta_mm,
            };
        }

        let override_allowed = self
            .config
            .override_allowed_urgency_levels
            .contains(&candidate_urgent_level);

        let status = if override_allowed {
            PathRuleStatus::OverrideRequired
        } else {
            PathRuleStatus::HardViolation
        };

        PathRuleResult {
            status,
            violation_type: Some(violation_type),
            width_delta_mm,
            thickness_delta_mm,
        }
    }
}

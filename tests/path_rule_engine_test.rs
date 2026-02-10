// ==========================================
// PathRuleEngine 单元测试
// ==========================================

use hot_rolling_aps::domain::types::{PathRuleStatus, PathViolationType, UrgentLevel};
use hot_rolling_aps::engine::{Anchor, PathRuleConfig, PathRuleEngine};

#[test]
fn test_no_anchor_returns_ok() {
    let engine = PathRuleEngine::new(PathRuleConfig::default());

    let r = engine.check(2000.0, 30.0, UrgentLevel::L0, None, false);
    assert_eq!(r.status, PathRuleStatus::Ok);
    assert!(r.violation_type.is_none());
}

#[test]
fn test_within_tolerance_returns_ok() {
    let engine = PathRuleEngine::new(PathRuleConfig::default());
    let anchor = Anchor {
        width_mm: 1200.0,
        thickness_mm: 10.0,
    };

    // width: +40 <= 50 tolerance, thickness: +0.5 <= 1.0 tolerance
    let r = engine.check(1240.0, 10.5, UrgentLevel::L0, Some(&anchor), false);
    assert_eq!(r.status, PathRuleStatus::Ok);
    assert!(r.violation_type.is_none());
    assert_eq!(r.width_delta_mm, 0.0);
    assert_eq!(r.thickness_delta_mm, 0.0);
}

#[test]
fn test_width_exceeded_returns_correct_violation() {
    let engine = PathRuleEngine::new(PathRuleConfig::default());
    let anchor = Anchor {
        width_mm: 1200.0,
        thickness_mm: 10.0,
    };

    // width: +60 exceeds 50 tolerance => delta 10
    let r = engine.check(1260.0, 10.0, UrgentLevel::L0, Some(&anchor), false);
    assert_eq!(r.status, PathRuleStatus::HardViolation);
    assert_eq!(r.violation_type, Some(PathViolationType::WidthExceeded));
    assert_eq!(r.width_delta_mm, 10.0);
    assert_eq!(r.thickness_delta_mm, 0.0);
}

#[test]
fn test_thickness_exceeded_returns_correct_violation() {
    let engine = PathRuleEngine::new(PathRuleConfig::default());
    let anchor = Anchor {
        width_mm: 1200.0,
        thickness_mm: 10.0,
    };

    // thickness: +1.2 exceeds 1.0 tolerance => delta 0.2
    let r = engine.check(1200.0, 11.2, UrgentLevel::L0, Some(&anchor), false);
    assert_eq!(r.status, PathRuleStatus::HardViolation);
    assert_eq!(r.violation_type, Some(PathViolationType::ThicknessExceeded));
    assert_eq!(r.width_delta_mm, 0.0);
    assert!((r.thickness_delta_mm - 0.2).abs() < 1e-9);
}

#[test]
fn test_both_exceeded_returns_correct_violation() {
    let engine = PathRuleEngine::new(PathRuleConfig::default());
    let anchor = Anchor {
        width_mm: 1200.0,
        thickness_mm: 10.0,
    };

    let r = engine.check(1300.0, 12.5, UrgentLevel::L0, Some(&anchor), false);
    assert_eq!(r.status, PathRuleStatus::HardViolation);
    assert_eq!(r.violation_type, Some(PathViolationType::BothExceeded));
    assert!(r.width_delta_mm > 0.0);
    assert!(r.thickness_delta_mm > 0.0);
}

#[test]
fn test_l2_l3_exceeded_returns_override_required() {
    let engine = PathRuleEngine::new(PathRuleConfig::default());
    let anchor = Anchor {
        width_mm: 1200.0,
        thickness_mm: 10.0,
    };

    let r = engine.check(1300.0, 10.0, UrgentLevel::L2, Some(&anchor), false);
    assert_eq!(r.status, PathRuleStatus::OverrideRequired);
    assert_eq!(r.violation_type, Some(PathViolationType::WidthExceeded));
}

#[test]
fn test_user_confirmed_allows_ok_with_violation_flag() {
    let engine = PathRuleEngine::new(PathRuleConfig::default());
    let anchor = Anchor {
        width_mm: 1200.0,
        thickness_mm: 10.0,
    };

    let r = engine.check(1300.0, 10.0, UrgentLevel::L2, Some(&anchor), true);
    assert_eq!(r.status, PathRuleStatus::Ok);
    assert_eq!(r.violation_type, Some(PathViolationType::WidthExceeded));
    assert!(r.width_delta_mm > 0.0);
}

#[test]
fn test_disabled_rule_always_ok() {
    let mut cfg = PathRuleConfig::default();
    cfg.enabled = false;
    let engine = PathRuleEngine::new(cfg);
    let anchor = Anchor {
        width_mm: 1200.0,
        thickness_mm: 10.0,
    };

    let r = engine.check(99999.0, 99999.0, UrgentLevel::L0, Some(&anchor), false);
    assert_eq!(r.status, PathRuleStatus::Ok);
    assert!(r.violation_type.is_none());
}

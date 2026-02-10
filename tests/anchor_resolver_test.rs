// ==========================================
// AnchorResolver 单元测试
// ==========================================

use hot_rolling_aps::domain::types::AnchorSource;
use hot_rolling_aps::engine::{AnchorResolver, MaterialSummary, SeedS2Config};

fn ms(
    material_id: &str,
    width_mm: f64,
    thickness_mm: f64,
    seq_no: i32,
    user_confirmed_at: Option<&str>,
) -> MaterialSummary {
    MaterialSummary {
        material_id: material_id.to_string(),
        width_mm,
        thickness_mm,
        seq_no,
        user_confirmed_at: user_confirmed_at.map(|s| s.to_string()),
    }
}

#[test]
fn test_frozen_has_highest_priority() {
    let resolver = AnchorResolver::new(SeedS2Config::default());

    let frozen = vec![
        ms("F1", 1200.0, 10.0, 1, None),
        ms("F2", 1100.0, 9.0, 3, None),
    ];
    let locked = vec![ms("L1", 1300.0, 11.0, 10, None)];
    let confirmed = vec![ms("C1", 1400.0, 12.0, 0, Some("2026-01-02T00:00:00Z"))];
    let candidates = vec![ms("X1", 1000.0, 8.0, 0, None)];

    let r = resolver.resolve(&frozen, &locked, &confirmed, &candidates);
    assert_eq!(r.source, AnchorSource::FrozenLast);
    assert_eq!(r.material_id.as_deref(), Some("F2"));
    let anchor = r.anchor.unwrap();
    assert_eq!(anchor.width_mm, 1100.0);
    assert_eq!(anchor.thickness_mm, 9.0);
}

#[test]
fn test_locked_is_second_priority() {
    let resolver = AnchorResolver::new(SeedS2Config::default());

    let frozen: Vec<MaterialSummary> = vec![];
    let locked = vec![
        ms("L1", 1300.0, 11.0, 1, None),
        ms("L2", 1250.0, 10.5, 2, None),
    ];
    let confirmed: Vec<MaterialSummary> = vec![];
    let candidates: Vec<MaterialSummary> = vec![];

    let r = resolver.resolve(&frozen, &locked, &confirmed, &candidates);
    assert_eq!(r.source, AnchorSource::LockedLast);
    assert_eq!(r.material_id.as_deref(), Some("L2"));
}

#[test]
fn test_user_confirmed_is_third_priority() {
    let resolver = AnchorResolver::new(SeedS2Config::default());

    let frozen: Vec<MaterialSummary> = vec![];
    let locked: Vec<MaterialSummary> = vec![];
    let confirmed = vec![
        ms("C1", 1200.0, 10.0, 0, Some("2026-01-01T00:00:00Z")),
        ms("C2", 1100.0, 9.0, 0, Some("2026-01-03T00:00:00Z")),
    ];
    let candidates: Vec<MaterialSummary> = vec![];

    let r = resolver.resolve(&frozen, &locked, &confirmed, &candidates);
    assert_eq!(r.source, AnchorSource::UserConfirmedLast);
    assert_eq!(r.material_id.as_deref(), Some("C2"));
}

#[test]
fn test_seed_s2_large_sample_uses_percentile() {
    let resolver = AnchorResolver::new(SeedS2Config {
        percentile: 0.95,
        small_sample_threshold: 10,
    });

    let frozen: Vec<MaterialSummary> = vec![];
    let locked: Vec<MaterialSummary> = vec![];
    let confirmed: Vec<MaterialSummary> = vec![];

    // widths: 1..=100 => idx=(100*0.95)=95 => sorted[95]=96
    let candidates: Vec<MaterialSummary> = (1..=100)
        .map(|i| ms(&format!("M{:03}", i), i as f64, i as f64, i, None))
        .collect();

    let r = resolver.resolve(&frozen, &locked, &confirmed, &candidates);
    assert_eq!(r.source, AnchorSource::SeedS2);
    let anchor = r.anchor.unwrap();
    assert_eq!(anchor.width_mm, 96.0);
    assert_eq!(anchor.thickness_mm, 96.0);
}

#[test]
fn test_seed_s2_small_sample_uses_max() {
    let resolver = AnchorResolver::new(SeedS2Config {
        percentile: 0.95,
        small_sample_threshold: 10,
    });

    let frozen: Vec<MaterialSummary> = vec![];
    let locked: Vec<MaterialSummary> = vec![];
    let confirmed: Vec<MaterialSummary> = vec![];

    let candidates = vec![
        ms("A", 100.0, 1.0, 1, None),
        ms("B", 200.0, 2.0, 2, None),
        ms("C", 150.0, 3.0, 3, None),
        ms("D", 180.0, 4.0, 4, None),
        ms("E", 210.0, 5.0, 5, None),
        ms("F", 190.0, 6.0, 6, None),
        ms("G", 205.0, 7.0, 7, None),
        ms("H", 199.0, 8.0, 8, None),
        ms("I", 201.0, 9.0, 9, None),
    ];

    let r = resolver.resolve(&frozen, &locked, &confirmed, &candidates);
    assert_eq!(r.source, AnchorSource::SeedS2);
    let anchor = r.anchor.unwrap();
    assert_eq!(anchor.width_mm, 210.0);
    assert_eq!(anchor.thickness_mm, 9.0);
}

#[test]
fn test_no_anchor_returns_none() {
    let resolver = AnchorResolver::new(SeedS2Config::default());

    let frozen: Vec<MaterialSummary> = vec![];
    let locked: Vec<MaterialSummary> = vec![];
    let confirmed: Vec<MaterialSummary> = vec![];
    let candidates: Vec<MaterialSummary> = vec![];

    let r = resolver.resolve(&frozen, &locked, &confirmed, &candidates);
    assert_eq!(r.source, AnchorSource::None);
    assert!(r.anchor.is_none());
}

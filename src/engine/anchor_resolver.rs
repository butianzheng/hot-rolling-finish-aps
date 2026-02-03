// ==========================================
// 热轧精整排产系统 - 路径锚点解析器
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 14-18 (v0.6)
// 优先级: FROZEN_LAST -> LOCKED_LAST -> USER_CONFIRMED_LAST -> SEED_S2 -> NONE
// ==========================================

use crate::domain::types::AnchorSource;
use crate::engine::path_rule::Anchor;

/// 锚点解析结果
#[derive(Debug, Clone)]
pub struct ResolvedAnchor {
    pub source: AnchorSource,
    pub material_id: Option<String>,
    pub anchor: Option<Anchor>,
}

/// 候选材料摘要（用于锚点解析）
#[derive(Debug, Clone)]
pub struct MaterialSummary {
    pub material_id: String,
    pub width_mm: f64,
    pub thickness_mm: f64,
    pub seq_no: i32,
    pub user_confirmed_at: Option<String>,
}

/// S2 种子策略配置
#[derive(Debug, Clone)]
pub struct SeedS2Config {
    pub percentile: f64,
    pub small_sample_threshold: i32,
}

impl Default for SeedS2Config {
    fn default() -> Self {
        Self {
            percentile: 0.95,
            small_sample_threshold: 10,
        }
    }
}

/// AnchorResolver - 锚点解析器
pub struct AnchorResolver {
    seed_config: SeedS2Config,
}

impl AnchorResolver {
    pub fn new(seed_config: SeedS2Config) -> Self {
        Self { seed_config }
    }

    /// 按优先级解析锚点
    pub fn resolve(
        &self,
        frozen_items: &[MaterialSummary],
        locked_items: &[MaterialSummary],
        user_confirmed_items: &[MaterialSummary],
        candidates: &[MaterialSummary],
    ) -> ResolvedAnchor {
        // 1) 冻结区最后一块
        if let Some(last) = frozen_items.iter().max_by_key(|m| m.seq_no) {
            return ResolvedAnchor {
                source: AnchorSource::FrozenLast,
                material_id: Some(last.material_id.clone()),
                anchor: Some(Anchor {
                    width_mm: last.width_mm,
                    thickness_mm: last.thickness_mm,
                }),
            };
        }

        // 2) 锁定区最后一块
        if let Some(last) = locked_items.iter().max_by_key(|m| m.seq_no) {
            return ResolvedAnchor {
                source: AnchorSource::LockedLast,
                material_id: Some(last.material_id.clone()),
                anchor: Some(Anchor {
                    width_mm: last.width_mm,
                    thickness_mm: last.thickness_mm,
                }),
            };
        }

        // 3) 人工确认队列最后一块（按 user_confirmed_at）
        if let Some(last) = user_confirmed_items
            .iter()
            .filter(|m| m.user_confirmed_at.is_some())
            .max_by(|a, b| a.user_confirmed_at.cmp(&b.user_confirmed_at))
        {
            return ResolvedAnchor {
                source: AnchorSource::UserConfirmedLast,
                material_id: Some(last.material_id.clone()),
                anchor: Some(Anchor {
                    width_mm: last.width_mm,
                    thickness_mm: last.thickness_mm,
                }),
            };
        }

        // 4) S2 种子策略兜底
        if let Some(anchor) = self.compute_seed_s2(candidates) {
            return ResolvedAnchor {
                source: AnchorSource::SeedS2,
                material_id: None,
                anchor: Some(anchor),
            };
        }

        // 5) 无锚点
        ResolvedAnchor {
            source: AnchorSource::None,
            material_id: None,
            anchor: None,
        }
    }

    fn compute_seed_s2(&self, candidates: &[MaterialSummary]) -> Option<Anchor> {
        if candidates.is_empty() {
            return None;
        }

        let widths: Vec<f64> = candidates
            .iter()
            .map(|m| m.width_mm)
            .filter(|w| w.is_finite() && *w > 0.0)
            .collect();
        let thicknesses: Vec<f64> = candidates
            .iter()
            .map(|m| m.thickness_mm)
            .filter(|t| t.is_finite() && *t > 0.0)
            .collect();

        if widths.is_empty() || thicknesses.is_empty() {
            return None;
        }

        let anchor_width = self.compute_upper_bound(&widths);
        let anchor_thickness = self.compute_upper_bound(&thicknesses);

        Some(Anchor {
            width_mm: anchor_width,
            thickness_mm: anchor_thickness,
        })
    }

    /// 计算上界（分位点或 max）
    fn compute_upper_bound(&self, values: &[f64]) -> f64 {
        let mut sorted: Vec<f64> = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        if sorted.is_empty() {
            return 0.0;
        }

        let threshold = self.seed_config.small_sample_threshold.max(1) as usize;
        if sorted.len() >= threshold {
            let p = self.seed_config.percentile.clamp(0.0, 1.0);
            let idx = ((sorted.len() as f64) * p) as usize;
            let idx = idx.min(sorted.len() - 1);
            sorted[idx]
        } else {
            *sorted.last().unwrap_or(&0.0)
        }
    }
}


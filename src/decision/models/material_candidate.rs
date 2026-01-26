// ==========================================
// 热轧精整排产系统 - 决策对象：可排候选材料
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 6 节
// 职责: 定义可排候选材料决策对象，满足适温/冻结/锁定等约束后的候选集
// ==========================================

use serde::{Deserialize, Serialize};

/// 可排候选材料 (MaterialCandidate)
///
/// 满足适温/冻结/锁定等约束后的候选集（含解释 reason）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialCandidate {
    /// 材料 ID
    pub material_id: String,

    /// 所属版本 ID
    pub version_id: String,

    /// 机组代码
    pub machine_code: String,

    /// 紧急等级 (L0/L1/L2/L3)
    pub urgent_level: String,

    /// 优先级分数
    pub priority_score: f64,

    /// 重量 (吨)
    pub weight_t: f64,

    /// 是否适温
    pub is_mature: bool,

    /// 距离适温还需天数
    pub ready_in_days: i32,

    /// 最早可排日期
    pub earliest_sched_date: String,

    /// 是否被锁定
    pub is_locked: bool,

    /// 是否强制放行
    pub is_force_released: bool,

    /// 是否在冻结区
    pub is_in_frozen_zone: bool,

    /// 合同号
    pub contract_no: Option<String>,

    /// 交货期
    pub due_date: Option<String>,

    /// 库存天数
    pub stock_age_days: i32,

    /// 可排原因列表
    pub eligible_reasons: Vec<String>,

    /// 不可排原因列表
    pub ineligible_reasons: Vec<String>,
}

impl MaterialCandidate {
    /// 创建新的候选材料
    pub fn new(
        material_id: String,
        version_id: String,
        machine_code: String,
        weight_t: f64,
    ) -> Self {
        Self {
            material_id,
            version_id,
            machine_code,
            urgent_level: "L0".to_string(),
            priority_score: 0.0,
            weight_t,
            is_mature: false,
            ready_in_days: 0,
            earliest_sched_date: String::new(),
            is_locked: false,
            is_force_released: false,
            is_in_frozen_zone: false,
            contract_no: None,
            due_date: None,
            stock_age_days: 0,
            eligible_reasons: Vec::new(),
            ineligible_reasons: Vec::new(),
        }
    }

    /// 判断是否可排产
    pub fn is_eligible(&self) -> bool {
        // 冻结区材料不可排（最高优先级约束，即使强制放行也不可排）
        if self.is_in_frozen_zone {
            return false;
        }

        // 强制放行可绕过锁定和适温约束
        if self.is_force_released {
            return true;
        }

        // 锁定材料不可排
        if self.is_locked {
            return false;
        }

        // 必须适温
        self.is_mature
    }

    /// 添加可排原因
    pub fn add_eligible_reason(&mut self, reason: String) {
        self.eligible_reasons.push(reason);
    }

    /// 添加不可排原因
    pub fn add_ineligible_reason(&mut self, reason: String) {
        self.ineligible_reasons.push(reason);
    }

    /// 获取主要不可排原因
    pub fn primary_ineligible_reason(&self) -> Option<String> {
        self.ineligible_reasons.first().cloned()
    }

    /// 判断是否为紧急材料 (L2 及以上)
    pub fn is_urgent(&self) -> bool {
        matches!(self.urgent_level.as_str(), "L2" | "L3")
    }

    /// 判断是否为冷料 (库存天数 > 阈值)
    pub fn is_cold_stock(&self, threshold_days: i32) -> bool {
        self.stock_age_days > threshold_days
    }

    /// 判断是否临期 (距离交货期 < 阈值)
    pub fn is_near_due(&self, threshold_days: i32) -> bool {
        if let Some(due_date_str) = &self.due_date {
            // 简化实现，实际需要解析日期并计算天数差
            // 这里假设 due_date 格式为 YYYY-MM-DD
            return true; // TODO: 实现日期比较逻辑
        }
        false
    }

    /// 设置适温状态
    pub fn set_maturity(&mut self, is_mature: bool, ready_in_days: i32, earliest_date: String) {
        self.is_mature = is_mature;
        self.ready_in_days = ready_in_days;
        self.earliest_sched_date = earliest_date;

        if is_mature {
            self.add_eligible_reason("已适温".to_string());
        } else {
            self.add_ineligible_reason(format!("未适温，还需 {} 天", ready_in_days));
        }
    }

    /// 设置锁定状态
    pub fn set_locked(&mut self, is_locked: bool) {
        self.is_locked = is_locked;
        if is_locked {
            self.add_ineligible_reason("材料已锁定".to_string());
        }
    }

    /// 设置强制放行状态
    pub fn set_force_released(&mut self, is_force_released: bool) {
        self.is_force_released = is_force_released;
        if is_force_released {
            self.add_eligible_reason("强制放行".to_string());
        }
    }

    /// 设置冻结区状态
    pub fn set_frozen_zone(&mut self, is_in_frozen_zone: bool) {
        self.is_in_frozen_zone = is_in_frozen_zone;
        if is_in_frozen_zone {
            self.add_ineligible_reason("在冻结区内".to_string());
        }
    }
}

impl std::fmt::Display for MaterialCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}, {:.1}t, {}, eligible: {})",
            self.material_id,
            self.urgent_level,
            self.weight_t,
            self.machine_code,
            self.is_eligible()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_candidate_creation() {
        let candidate = MaterialCandidate::new(
            "M001".to_string(),
            "V001".to_string(),
            "H032".to_string(),
            100.5,
        );

        assert_eq!(candidate.material_id, "M001");
        assert_eq!(candidate.weight_t, 100.5);
        assert!(!candidate.is_eligible());
    }

    #[test]
    fn test_eligibility_checks() {
        let mut candidate = MaterialCandidate::new(
            "M001".to_string(),
            "V001".to_string(),
            "H032".to_string(),
            100.5,
        );

        // 默认不可排（未适温）
        assert!(!candidate.is_eligible());

        // 适温后可排
        candidate.set_maturity(true, 0, "2026-01-22".to_string());
        assert!(candidate.is_eligible());

        // 锁定后不可排
        candidate.set_locked(true);
        assert!(!candidate.is_eligible());

        // 强制放行后可排（即使锁定）
        candidate.set_force_released(true);
        assert!(candidate.is_eligible());

        // 冻结区不可排（即使强制放行）
        candidate.set_frozen_zone(true);
        assert!(!candidate.is_eligible());
    }

    #[test]
    fn test_reasons_tracking() {
        let mut candidate = MaterialCandidate::new(
            "M001".to_string(),
            "V001".to_string(),
            "H032".to_string(),
            100.5,
        );

        candidate.set_maturity(false, 3, "2026-01-25".to_string());
        candidate.set_locked(true);

        assert_eq!(candidate.ineligible_reasons.len(), 2);
        assert_eq!(candidate.primary_ineligible_reason(), Some("未适温，还需 3 天".to_string()));
    }

    #[test]
    fn test_urgent_and_cold_stock() {
        let mut candidate = MaterialCandidate::new(
            "M001".to_string(),
            "V001".to_string(),
            "H032".to_string(),
            100.5,
        );

        // 默认不紧急
        assert!(!candidate.is_urgent());

        // 设置为 L2 紧急
        candidate.urgent_level = "L2".to_string();
        assert!(candidate.is_urgent());

        // 冷料判断
        candidate.stock_age_days = 15;
        assert!(candidate.is_cold_stock(10));
        assert!(!candidate.is_cold_stock(20));
    }
}

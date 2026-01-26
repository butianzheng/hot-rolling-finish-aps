// ==========================================
// 热轧精整排产系统 - 风险快照领域模型
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART G 成功判定
// 依据: Engine_Specs_v0.3_Integrated.md - 8. Risk Engine
// ==========================================

use crate::domain::types::RiskLevel;
use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

// ==========================================
// RiskSnapshot - 风险快照
// ==========================================
// 用途: 驾驶舱指标,只读数据源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskSnapshot {
    pub snapshot_id: String,       // 快照ID
    pub version_id: String,        // 关联排产版本
    pub machine_code: String,      // 机组代码
    pub snapshot_date: NaiveDate,  // 快照日期

    // ===== 产能指标 =====
    pub used_capacity_t: f64,      // 已用产能
    pub target_capacity_t: f64,    // 目标产能
    pub limit_capacity_t: f64,     // 上限产能
    pub overflow_t: f64,           // 超限吨位

    // ===== 紧急材料统计 =====
    pub urgent_total_t: f64,       // 紧急材料总吨位 (L2+L3)
    pub l3_count: i32,             // L3 红线材料数量
    pub l2_count: i32,             // L2 紧急材料数量

    // ===== 冷料压力 =====
    pub mature_backlog_t: f64,     // 适温待排积压吨位
    pub immature_backlog_t: f64,   // 未成熟材料吨位

    // ===== 风险等级 =====
    pub risk_level: RiskLevel,     // 风险等级
    pub risk_reason: String,       // 风险原因 (可解释性)

    // ===== 换辊风险 =====
    pub roll_status: Option<String>, // 换辊状态
    pub roll_risk: Option<String>,   // 换辊风险提示

    // ===== 元数据 =====
    pub created_at: NaiveDateTime, // 创建时间
}

// ==========================================
// Trait: RiskAssessment
// ==========================================
// 用途: Risk Engine 评估逻辑接口
pub trait RiskAssessment {
    /// 计算风险等级 (返回风险等级和原因)
    fn assess_risk_level(&self) -> (RiskLevel, String);

    /// 判断是否最危险日期
    fn is_most_risky(&self, other_snapshots: &[RiskSnapshot]) -> bool;

    /// 计算产能利用率
    fn capacity_utilization_ratio(&self) -> f64;

    /// 判断是否存在产能优化空间
    fn has_optimization_opportunity(&self) -> bool;
}

// ==========================================
// RiskAssessment Trait 实现
// ==========================================
impl RiskAssessment for RiskSnapshot {
    /// 计算风险等级 (已在创建时计算，直接返回)
    fn assess_risk_level(&self) -> (RiskLevel, String) {
        (self.risk_level, self.risk_reason.clone())
    }

    /// 判断是否最危险日期
    ///
    /// # 规则
    /// - 风险等级最高 (RED > ORANGE > YELLOW > GREEN)
    /// - 相同等级比较超限吨位
    /// - 相同超限比较L3材料数量
    fn is_most_risky(&self, other_snapshots: &[RiskSnapshot]) -> bool {
        for other in other_snapshots {
            // 比较风险等级
            if other.risk_level > self.risk_level {
                return false;
            }

            // 相同等级比较超限吨位
            if other.risk_level == self.risk_level && other.overflow_t > self.overflow_t {
                return false;
            }

            // 相同超限比较L3数量
            if other.risk_level == self.risk_level
                && other.overflow_t == self.overflow_t
                && other.l3_count > self.l3_count
            {
                return false;
            }
        }

        true
    }

    /// 计算产能利用率
    ///
    /// # 返回
    /// 利用率 (0.0 - 1.0+)
    fn capacity_utilization_ratio(&self) -> f64 {
        if self.target_capacity_t <= 0.0 {
            return 0.0;
        }

        self.used_capacity_t / self.target_capacity_t
    }

    /// 判断是否存在产能优化空间
    ///
    /// # 规则
    /// - 利用率 < 80%: 有空间
    /// - 冷料压库大于目标产能: 有空间
    /// - L2+L3 材料多: 需要优化
    fn has_optimization_opportunity(&self) -> bool {
        let utilization = self.capacity_utilization_ratio();

        // 利用率不足80%
        if utilization < 0.8 {
            return true;
        }

        // 冷料压库严重
        if self.mature_backlog_t > self.target_capacity_t {
            return true;
        }

        // 紧急材料较多
        if self.l2_count >= 5 || self.l3_count >= 3 {
            return true;
        }

        false
    }
}

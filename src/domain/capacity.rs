// ==========================================
// 热轧精整排产系统 - 产能池领域模型
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART B3 产能与换辊
// 依据: Engine_Specs_v0.3_Integrated.md - capacity_pool
// ==========================================

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

// ==========================================
// CapacityPool - 产能池
// ==========================================
// 红线: 产能约束优先于材料优先级
// 用途: 吨位池管理,换辊触发
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityPool {
    // ===== 主键 (版本化后) =====
    pub version_id: String,        // 所属版本ID (P1-1: 版本化改造)
    pub machine_code: String,      // 机组代码
    pub plan_date: NaiveDate,      // 排产日期

    // ===== 产能参数 =====
    pub target_capacity_t: f64,    // 目标产能 (吨)
    pub limit_capacity_t: f64,     // 上限产能 (吨)

    // ===== 实际使用 =====
    pub used_capacity_t: f64,      // 已使用产能 (吨)
    pub overflow_t: f64,           // 超限吨位 (> limit)

    // ===== 冻结区 =====
    pub frozen_capacity_t: f64,    // 冻结区吨位

    // ===== 换辊相关 =====
    pub accumulated_tonnage_t: f64, // 累计吨位 (用于换辊判断)
    pub roll_campaign_id: Option<String>, // 关联换辊窗口

    // TODO: 添加结构目标字段 (品种配比)
    // TODO: 添加锁定标志
}

// ==========================================
// Trait: CapacityConstraint
// ==========================================
// 用途: Capacity Filler 约束检查接口
pub trait CapacityConstraint {
    /// 检查是否可添加材料
    fn can_add_material(&self, weight_t: f64) -> bool;

    /// 检查是否超限
    fn is_overflow(&self) -> bool;

    /// 计算剩余产能
    fn remaining_capacity_t(&self) -> f64;

    /// 计算超限比例
    fn overflow_ratio(&self) -> f64;
}

// ==========================================
// CapacityConstraint trait 实现
// ==========================================
impl CapacityConstraint for CapacityPool {
    /// 检查是否可添加材料
    ///
    /// # 参数
    /// - `weight_t`: 材料重量（吨）
    ///
    /// # 返回
    /// - `true`: 可以添加（不超过 limit_capacity_t）
    /// - `false`: 不可添加（会超过 limit_capacity_t）
    fn can_add_material(&self, weight_t: f64) -> bool {
        self.used_capacity_t + weight_t <= self.limit_capacity_t
    }

    /// 检查是否超限
    ///
    /// # 返回
    /// - `true`: 已超过 limit_capacity_t
    /// - `false`: 未超过 limit_capacity_t
    fn is_overflow(&self) -> bool {
        self.used_capacity_t > self.limit_capacity_t
    }

    /// 计算剩余产能
    ///
    /// # 返回
    /// 剩余产能（吨），相对于 limit_capacity_t
    fn remaining_capacity_t(&self) -> f64 {
        (self.limit_capacity_t - self.used_capacity_t).max(0.0)
    }

    /// 计算超限比例
    ///
    /// # 返回
    /// 超限比例（0.0 - 1.0），相对于 limit_capacity_t
    fn overflow_ratio(&self) -> f64 {
        if self.limit_capacity_t <= 0.0 {
            return 0.0;
        }
        ((self.used_capacity_t - self.limit_capacity_t) / self.limit_capacity_t).max(0.0)
    }
}

// TODO: 实现数据库映射 (sqlx derive)

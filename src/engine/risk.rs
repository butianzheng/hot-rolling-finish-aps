// ==========================================
// 热轧精整排产系统 - 风险快照引擎
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 8. Risk Engine
// 依据: Claude_Dev_Master_Spec.md - PART G 成功判定
// ==========================================
// 职责: 驾驶舱指标生成
// 输入: 产能池 + 排产明细 + 材料状态
// 输出: risk_snapshot (风险快照)
// ==========================================

use crate::domain::capacity::CapacityPool;
use crate::domain::material::MaterialState;
use crate::domain::plan::PlanItem;
use crate::domain::risk::RiskSnapshot;
use crate::domain::types::{RiskLevel, SchedState, UrgentLevel};
use chrono::{NaiveDate, Utc};
use serde_json::json;
use uuid::Uuid;

// ==========================================
// RiskEngine - 风险快照引擎
// ==========================================
pub struct RiskEngine {
    // 无状态引擎,不需要注入依赖
    // Repository 操作由调用方处理
}

impl RiskEngine {
    /// 构造函数
    ///
    /// # 返回
    /// 新的 RiskEngine 实例
    pub fn new() -> Self {
        Self {}
    }

    // ==========================================
    // 核心方法
    // ==========================================

    /// 生成风险快照 (单日单机组)
    ///
    /// # 参数
    /// - `version_id`: 排产版本ID
    /// - `machine_code`: 机组代码
    /// - `snapshot_date`: 快照日期
    /// - `pool`: 产能池
    /// - `scheduled_items`: 已排产材料列表
    /// - `all_materials`: 所有材料状态列表
    /// - `material_weights`: 材料ID到重量的映射 (material_id -> weight_t)
    /// - `roll_status`: 换辊状态
    ///
    /// # 返回
    /// RiskSnapshot 风险快照
    pub fn generate_snapshot(
        &self,
        version_id: &str,
        machine_code: &str,
        snapshot_date: NaiveDate,
        pool: &CapacityPool,
        scheduled_items: &[PlanItem],
        all_materials: &[MaterialState],
        material_weights: &std::collections::HashMap<String, f64>,
        roll_status: Option<&str>,
    ) -> RiskSnapshot {
        // 1. 计算产能指标
        let (used_capacity_t, overflow_t) = self.calculate_capacity_metrics(pool, scheduled_items);

        // 2. 计算紧急材料统计
        let (urgent_total_t, l3_count, l2_count) =
            self.calculate_urgent_metrics(all_materials, machine_code, material_weights);

        // 3. 计算冷料压力
        let (mature_backlog_t, immature_backlog_t) =
            self.calculate_backlog_metrics(all_materials, machine_code, material_weights);

        // 4. 评估风险等级
        let (risk_level, risk_reason) = self.assess_risk_level(
            pool,
            used_capacity_t,
            overflow_t,
            urgent_total_t,
            l3_count,
            l2_count,
            mature_backlog_t,
            immature_backlog_t,
            roll_status,
        );

        // 5. 生成换辊风险提示
        let roll_risk = self.generate_roll_risk(roll_status);

        // 6. 构造快照
        RiskSnapshot {
            snapshot_id: Uuid::new_v4().to_string(),
            version_id: version_id.to_string(),
            machine_code: machine_code.to_string(),
            snapshot_date,
            used_capacity_t,
            target_capacity_t: pool.target_capacity_t,
            limit_capacity_t: pool.limit_capacity_t,
            overflow_t,
            urgent_total_t,
            l3_count,
            l2_count,
            mature_backlog_t,
            immature_backlog_t,
            risk_level,
            risk_reason,
            roll_status: roll_status.map(|s| s.to_string()),
            roll_risk,
            created_at: Utc::now().naive_utc(),
        }
    }

    // ==========================================
    // 指标计算 (依据 Engine_Specs 8)
    // ==========================================

    /// 计算产能指标
    ///
    /// # 参数
    /// - `pool`: 产能池
    /// - `scheduled_items`: 已排产材料列表
    ///
    /// # 返回
    /// (used_capacity_t, overflow_t)
    fn calculate_capacity_metrics(
        &self,
        pool: &CapacityPool,
        scheduled_items: &[PlanItem],
    ) -> (f64, f64) {
        // 计算已用产能
        let used_capacity_t: f64 = scheduled_items
            .iter()
            .map(|item| item.weight_t)
            .sum();

        // 计算超限吨位
        let overflow_t = if used_capacity_t > pool.limit_capacity_t {
            used_capacity_t - pool.limit_capacity_t
        } else {
            0.0
        };

        (used_capacity_t, overflow_t)
    }

    /// 计算紧急材料统计
    ///
    /// # 参数
    /// - `materials`: 材料状态列表
    /// - `machine_code`: 机组代码
    /// - `material_weights`: 材料ID到重量的映射
    ///
    /// # 返回
    /// (urgent_total_t, l3_count, l2_count)
    fn calculate_urgent_metrics(
        &self,
        materials: &[MaterialState],
        machine_code: &str,
        material_weights: &std::collections::HashMap<String, f64>,
    ) -> (f64, i32, i32) {
        let mut urgent_total_t = 0.0;
        let mut l3_count = 0;
        let mut l2_count = 0;

        for material in materials {
            // 只统计当前机组的材料
            if material.scheduled_machine_code.as_deref() != Some(machine_code) {
                continue;
            }

            // 获取材料重量
            let weight_t = material_weights
                .get(&material.material_id)
                .copied()
                .unwrap_or(0.0);

            match material.urgent_level {
                UrgentLevel::L3 => {
                    l3_count += 1;
                    urgent_total_t += weight_t;
                }
                UrgentLevel::L2 => {
                    l2_count += 1;
                    urgent_total_t += weight_t;
                }
                _ => {}
            }
        }

        (urgent_total_t, l3_count, l2_count)
    }

    /// 计算冷料压力
    ///
    /// # 参数
    /// - `materials`: 材料状态列表
    /// - `machine_code`: 机组代码
    /// - `material_weights`: 材料ID到重量的映射
    ///
    /// # 返回
    /// (mature_backlog_t, immature_backlog_t)
    fn calculate_backlog_metrics(
        &self,
        materials: &[MaterialState],
        machine_code: &str,
        material_weights: &std::collections::HashMap<String, f64>,
    ) -> (f64, f64) {
        let mut mature_backlog_t = 0.0;
        let mut immature_backlog_t = 0.0;

        for material in materials {
            // 只统计当前机组的材料
            if material.scheduled_machine_code.as_deref() != Some(machine_code) {
                continue;
            }

            // 获取材料重量
            let weight_t = material_weights
                .get(&material.material_id)
                .copied()
                .unwrap_or(0.0);

            // 未排产的材料
            if material.sched_state != SchedState::Scheduled {
                match material.sched_state {
                    SchedState::Ready => {
                        // 适温待排
                        mature_backlog_t += weight_t;
                    }
                    SchedState::PendingMature => {
                        // 未成熟
                        immature_backlog_t += weight_t;
                    }
                    _ => {}
                }
            }
        }

        (mature_backlog_t, immature_backlog_t)
    }

    /// 评估风险等级
    ///
    /// 规则 (可解释):
    /// - RED: 超限严重 OR L3材料多 OR 换辊硬停止
    /// - ORANGE: 超限轻微 OR L2材料多 OR 冷料压库
    /// - YELLOW: 接近目标 OR 临期材料多
    /// - GREEN: 正常
    ///
    /// # 参数
    /// - `pool`: 产能池
    /// - `used_capacity_t`: 已用产能
    /// - `overflow_t`: 超限吨位
    /// - `urgent_total_t`: 紧急材料总吨位
    /// - `l3_count`: L3材料数量
    /// - `l2_count`: L2材料数量
    /// - `mature_backlog_t`: 适温待排积压吨位
    /// - `immature_backlog_t`: 未成熟材料吨位
    /// - `roll_status`: 换辊状态
    ///
    /// # 返回
    /// (RiskLevel, reason)
    #[allow(clippy::too_many_arguments)]
    fn assess_risk_level(
        &self,
        pool: &CapacityPool,
        used_capacity_t: f64,
        overflow_t: f64,
        _urgent_total_t: f64,
        l3_count: i32,
        l2_count: i32,
        mature_backlog_t: f64,
        _immature_backlog_t: f64,
        roll_status: Option<&str>,
    ) -> (RiskLevel, String) {
        let mut reasons = Vec::new();

        // RED 级别判定
        if overflow_t > pool.limit_capacity_t * 0.1 {
            reasons.push("超限严重(>10%)");
            return (
                RiskLevel::Red,
                json!({
                    "level": "RED",
                    "reasons": reasons,
                    "overflow_t": overflow_t,
                    "overflow_pct": (overflow_t / pool.limit_capacity_t * 100.0)
                })
                .to_string(),
            );
        }

        if l3_count >= 5 {
            reasons.push("L3红线材料过多(>=5)");
            return (
                RiskLevel::Red,
                json!({
                    "level": "RED",
                    "reasons": reasons,
                    "l3_count": l3_count
                })
                .to_string(),
            );
        }

        if roll_status == Some("HARD_STOP") {
            reasons.push("换辊硬停止");
            return (
                RiskLevel::Red,
                json!({
                    "level": "RED",
                    "reasons": reasons,
                    "roll_status": roll_status
                })
                .to_string(),
            );
        }

        // ORANGE 级别判定
        if overflow_t > 0.0 {
            reasons.push("超限轻微");
        }

        if l2_count >= 10 {
            reasons.push("L2紧急材料过多(>=10)");
        }

        if mature_backlog_t > pool.target_capacity_t * 2.0 {
            reasons.push("冷料压库严重(>2倍目标产能)");
        }

        if !reasons.is_empty() {
            return (
                RiskLevel::Orange,
                json!({
                    "level": "ORANGE",
                    "reasons": reasons,
                    "overflow_t": overflow_t,
                    "l2_count": l2_count,
                    "mature_backlog_t": mature_backlog_t
                })
                .to_string(),
            );
        }

        // YELLOW 级别判定
        let utilization = used_capacity_t / pool.target_capacity_t;
        if utilization > 0.9 {
            reasons.push("接近目标产能(>90%)");
        }

        if l2_count >= 5 {
            reasons.push("L2紧急材料较多(>=5)");
        }

        if mature_backlog_t > pool.target_capacity_t {
            reasons.push("冷料压库(>目标产能)");
        }

        if !reasons.is_empty() {
            return (
                RiskLevel::Yellow,
                json!({
                    "level": "YELLOW",
                    "reasons": reasons,
                    "utilization": utilization,
                    "l2_count": l2_count,
                    "mature_backlog_t": mature_backlog_t
                })
                .to_string(),
            );
        }

        // GREEN 正常
        (
            RiskLevel::Green,
            json!({
                "level": "GREEN",
                "reasons": ["正常"],
                "utilization": utilization
            })
            .to_string(),
        )
    }

    /// 生成换辊风险提示
    ///
    /// # 参数
    /// - `roll_status`: 换辊状态
    ///
    /// # 返回
    /// 换辊风险提示
    fn generate_roll_risk(&self, roll_status: Option<&str>) -> Option<String> {
        match roll_status {
            Some("HARD_STOP") => Some("换辊硬停止,必须立即换辊".to_string()),
            Some("SUGGEST") => Some("建议换辊".to_string()),
            Some("NORMAL") => None,
            _ => None,
        }
    }
}

// ==========================================
// Default trait 实现
// ==========================================
impl Default for RiskEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ==========================================
// 单元测试
// ==========================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{RiskLevel, SchedState, UrgentLevel};
    use chrono::{NaiveDate, Utc};
    use std::collections::HashMap;

    /// 创建测试用的产能池
    fn create_test_pool() -> CapacityPool {
        CapacityPool {
            machine_code: "H032".to_string(),
            plan_date: NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(),
            target_capacity_t: 1000.0,
            limit_capacity_t: 1200.0,
            used_capacity_t: 0.0,
            overflow_t: 0.0,
            frozen_capacity_t: 0.0,
            accumulated_tonnage_t: 0.0,
            roll_campaign_id: None,
        }
    }

    /// 创建测试用的排产明细
    fn create_test_plan_items() -> Vec<PlanItem> {
        vec![
            PlanItem {
                version_id: "v1".to_string(),
                material_id: "M001".to_string(),
                machine_code: "H032".to_string(),
                plan_date: NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(),
                seq_no: 1,
                weight_t: 300.0,
                source_type: "CALC".to_string(),
                locked_in_plan: false,
                force_release_in_plan: false,
                violation_flags: None,
                urgent_level: Some("L0".to_string()),
                sched_state: Some("SCHEDULED".to_string()),
                assign_reason: Some("Test".to_string()),
            },
            PlanItem {
                version_id: "v1".to_string(),
                material_id: "M002".to_string(),
                machine_code: "H032".to_string(),
                plan_date: NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(),
                seq_no: 2,
                weight_t: 450.0,
                source_type: "CALC".to_string(),
                locked_in_plan: false,
                force_release_in_plan: false,
                violation_flags: None,
                urgent_level: Some("L1".to_string()),
                sched_state: Some("SCHEDULED".to_string()),
                assign_reason: Some("Test".to_string()),
            },
        ]
    }

    /// 创建测试用的材料状态
    fn create_test_materials() -> Vec<MaterialState> {
        vec![
            MaterialState {
                material_id: "M001".to_string(),
                sched_state: SchedState::Scheduled,
                lock_flag: false,
                force_release_flag: false,
                urgent_level: UrgentLevel::L2,
                urgent_reason: None,
                rush_level: crate::domain::types::RushLevel::L0,
                rolling_output_age_days: 5,
                ready_in_days: 0,
                earliest_sched_date: None,
                stock_age_days: 10,
                scheduled_date: Some(NaiveDate::from_ymd_opt(2025, 1, 20).unwrap()),
                scheduled_machine_code: Some("H032".to_string()),
                seq_no: Some(1),
                manual_urgent_flag: false,
                in_frozen_zone: false,
                last_calc_version_id: None,
                updated_at: Utc::now(),
                updated_by: None,
            },
            MaterialState {
                material_id: "M002".to_string(),
                sched_state: SchedState::Ready,
                lock_flag: false,
                force_release_flag: false,
                urgent_level: UrgentLevel::L3,
                urgent_reason: None,
                rush_level: crate::domain::types::RushLevel::L1,
                rolling_output_age_days: 8,
                ready_in_days: 0,
                earliest_sched_date: None,
                stock_age_days: 15,
                scheduled_date: None,
                scheduled_machine_code: Some("H032".to_string()),
                seq_no: None,
                manual_urgent_flag: false,
                in_frozen_zone: false,
                last_calc_version_id: None,
                updated_at: Utc::now(),
                updated_by: None,
            },
            MaterialState {
                material_id: "M003".to_string(),
                sched_state: SchedState::PendingMature,
                lock_flag: false,
                force_release_flag: false,
                urgent_level: UrgentLevel::L0,
                urgent_reason: None,
                rush_level: crate::domain::types::RushLevel::L0,
                rolling_output_age_days: 2,
                ready_in_days: 1,
                earliest_sched_date: Some(NaiveDate::from_ymd_opt(2025, 1, 21).unwrap()),
                stock_age_days: 3,
                scheduled_date: None,
                scheduled_machine_code: Some("H032".to_string()),
                seq_no: None,
                manual_urgent_flag: false,
                in_frozen_zone: false,
                last_calc_version_id: None,
                updated_at: Utc::now(),
                updated_by: None,
            },
        ]
    }

    /// 创建测试用的材料重量映射
    fn create_test_material_weights() -> HashMap<String, f64> {
        let mut weights = HashMap::new();
        weights.insert("M001".to_string(), 300.0);
        weights.insert("M002".to_string(), 450.0);
        weights.insert("M003".to_string(), 200.0);
        weights
    }

    #[test]
    fn test_calculate_capacity_metrics() {
        let engine = RiskEngine::new();
        let pool = create_test_pool();
        let items = create_test_plan_items();

        let (used, overflow) = engine.calculate_capacity_metrics(&pool, &items);

        assert_eq!(used, 750.0); // 300 + 450
        assert_eq!(overflow, 0.0); // 未超限
    }

    #[test]
    fn test_calculate_capacity_metrics_with_overflow() {
        let engine = RiskEngine::new();
        let mut pool = create_test_pool();
        pool.limit_capacity_t = 600.0; // 降低上限

        let items = create_test_plan_items();

        let (used, overflow) = engine.calculate_capacity_metrics(&pool, &items);

        assert_eq!(used, 750.0);
        assert_eq!(overflow, 150.0); // 750 - 600
    }

    #[test]
    fn test_calculate_urgent_metrics() {
        let engine = RiskEngine::new();
        let materials = create_test_materials();
        let weights = create_test_material_weights();

        let (urgent_total, l3_count, l2_count) = engine.calculate_urgent_metrics(
            &materials,
            "H032",
            &weights,
        );

        assert_eq!(l3_count, 1); // M002
        assert_eq!(l2_count, 1); // M001
        assert_eq!(urgent_total, 750.0); // 300 + 450
    }

    #[test]
    fn test_calculate_backlog_metrics() {
        let engine = RiskEngine::new();
        let materials = create_test_materials();
        let weights = create_test_material_weights();

        let (mature_backlog, immature_backlog) = engine.calculate_backlog_metrics(
            &materials,
            "H032",
            &weights,
        );

        assert_eq!(mature_backlog, 450.0); // M002 (Ready)
        assert_eq!(immature_backlog, 200.0); // M003 (PendingMature)
    }

    #[test]
    fn test_assess_risk_level_green() {
        let engine = RiskEngine::new();
        let pool = create_test_pool();

        let (risk_level, _reason) = engine.assess_risk_level(
            &pool,
            500.0,  // used_capacity_t
            0.0,    // overflow_t
            0.0,    // urgent_total_t
            0,      // l3_count
            0,      // l2_count
            100.0,  // mature_backlog_t
            50.0,   // immature_backlog_t
            None,   // roll_status
        );

        assert_eq!(risk_level, RiskLevel::Green);
    }

    #[test]
    fn test_assess_risk_level_yellow() {
        let engine = RiskEngine::new();
        let pool = create_test_pool();

        let (risk_level, _reason) = engine.assess_risk_level(
            &pool,
            950.0,  // used_capacity_t (95% utilization)
            0.0,    // overflow_t
            0.0,    // urgent_total_t
            0,      // l3_count
            0,      // l2_count
            100.0,  // mature_backlog_t
            50.0,   // immature_backlog_t
            None,   // roll_status
        );

        assert_eq!(risk_level, RiskLevel::Yellow);
    }

    #[test]
    fn test_assess_risk_level_orange() {
        let engine = RiskEngine::new();
        let pool = create_test_pool();

        let (risk_level, _reason) = engine.assess_risk_level(
            &pool,
            1250.0, // used_capacity_t
            50.0,   // overflow_t (轻微超限)
            0.0,    // urgent_total_t
            0,      // l3_count
            0,      // l2_count
            100.0,  // mature_backlog_t
            50.0,   // immature_backlog_t
            None,   // roll_status
        );

        assert_eq!(risk_level, RiskLevel::Orange);
    }

    #[test]
    fn test_assess_risk_level_red_overflow() {
        let engine = RiskEngine::new();
        let pool = create_test_pool();

        let (risk_level, _reason) = engine.assess_risk_level(
            &pool,
            1400.0, // used_capacity_t
            200.0,  // overflow_t (>10% 严重超限)
            0.0,    // urgent_total_t
            0,      // l3_count
            0,      // l2_count
            100.0,  // mature_backlog_t
            50.0,   // immature_backlog_t
            None,   // roll_status
        );

        assert_eq!(risk_level, RiskLevel::Red);
    }

    #[test]
    fn test_assess_risk_level_red_l3_materials() {
        let engine = RiskEngine::new();
        let pool = create_test_pool();

        let (risk_level, _reason) = engine.assess_risk_level(
            &pool,
            500.0,  // used_capacity_t
            0.0,    // overflow_t
            0.0,    // urgent_total_t
            5,      // l3_count (>=5)
            0,      // l2_count
            100.0,  // mature_backlog_t
            50.0,   // immature_backlog_t
            None,   // roll_status
        );

        assert_eq!(risk_level, RiskLevel::Red);
    }

    #[test]
    fn test_assess_risk_level_red_hard_stop() {
        let engine = RiskEngine::new();
        let pool = create_test_pool();

        let (risk_level, _reason) = engine.assess_risk_level(
            &pool,
            500.0,  // used_capacity_t
            0.0,    // overflow_t
            0.0,    // urgent_total_t
            0,      // l3_count
            0,      // l2_count
            100.0,  // mature_backlog_t
            50.0,   // immature_backlog_t
            Some("HARD_STOP"), // roll_status
        );

        assert_eq!(risk_level, RiskLevel::Red);
    }

    #[test]
    fn test_generate_roll_risk() {
        let engine = RiskEngine::new();

        assert_eq!(
            engine.generate_roll_risk(Some("HARD_STOP")),
            Some("换辊硬停止,必须立即换辊".to_string())
        );
        assert_eq!(
            engine.generate_roll_risk(Some("SUGGEST")),
            Some("建议换辊".to_string())
        );
        assert_eq!(engine.generate_roll_risk(Some("NORMAL")), None);
        assert_eq!(engine.generate_roll_risk(None), None);
    }

    #[test]
    fn test_generate_snapshot() {
        let engine = RiskEngine::new();
        let pool = create_test_pool();
        let items = create_test_plan_items();
        let materials = create_test_materials();
        let weights = create_test_material_weights();

        let snapshot = engine.generate_snapshot(
            "v1",
            "H032",
            NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(),
            &pool,
            &items,
            &materials,
            &weights,
            None,
        );

        assert_eq!(snapshot.version_id, "v1");
        assert_eq!(snapshot.machine_code, "H032");
        assert_eq!(snapshot.used_capacity_t, 750.0);
        assert_eq!(snapshot.target_capacity_t, 1000.0);
        assert_eq!(snapshot.limit_capacity_t, 1200.0);
        assert_eq!(snapshot.overflow_t, 0.0);
        assert_eq!(snapshot.l3_count, 1);
        assert_eq!(snapshot.l2_count, 1);
        assert_eq!(snapshot.urgent_total_t, 750.0);
        assert_eq!(snapshot.mature_backlog_t, 450.0);
        assert_eq!(snapshot.immature_backlog_t, 200.0);
    }
}

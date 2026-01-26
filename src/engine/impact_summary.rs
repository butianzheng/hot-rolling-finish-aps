// ==========================================
// 热轧精整排产系统 - 影响摘要引擎
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 9. Impact Summary Engine
// 依据: Claude_Dev_Master_Spec.md - PART A3 审计增强
// ==========================================
// 职责: 调整影响摘要生成
// 输入: 操作前后快照
// 输出: action_log.impact_summary_json
// ==========================================

use crate::domain::action_log::{
    CapacityChange, ImpactSummary, MaterialChange, RiskChange,
};
use crate::domain::capacity::CapacityPool;
use crate::domain::material::MaterialState;
use crate::domain::plan::PlanItem;
use crate::domain::risk::RiskSnapshot;
use chrono::NaiveDate;
use std::collections::HashMap;

// ==========================================
// ImpactSummaryEngine - 影响摘要引擎
// ==========================================
// 红线: 无状态引擎,所有方法都是纯函数
pub struct ImpactSummaryEngine;

impl ImpactSummaryEngine {
    /// 创建新的影响摘要引擎
    pub fn new() -> Self {
        Self
    }

    // ==========================================
    // 核心方法
    // ==========================================

    /// 生成影响摘要
    ///
    /// # 参数
    /// - `before_items`: 操作前计划项
    /// - `after_items`: 操作后计划项
    /// - `before_pools`: 操作前产能池
    /// - `after_pools`: 操作后产能池
    /// - `before_risks`: 操作前风险快照
    /// - `after_risks`: 操作后风险快照
    /// - `all_materials`: 所有材料状态 (用于获取urgency_level)
    /// - `material_weights`: 材料重量映射
    ///
    /// # 返回
    /// 影响摘要
    pub fn generate_impact(
        &self,
        before_items: &[PlanItem],
        after_items: &[PlanItem],
        before_pools: &[CapacityPool],
        after_pools: &[CapacityPool],
        before_risks: &[RiskSnapshot],
        after_risks: &[RiskSnapshot],
        all_materials: &[MaterialState],
        material_weights: &HashMap<String, f64>,
    ) -> ImpactSummary {
        // 1. 材料影响分析
        let material_changes = self.analyze_material_changes(before_items, after_items);
        let moved_count = material_changes
            .iter()
            .filter(|c| c.change_type == "moved")
            .count() as i32;
        let squeezed_out_count = material_changes
            .iter()
            .filter(|c| c.change_type == "squeezed_out")
            .count() as i32;
        let added_count = material_changes
            .iter()
            .filter(|c| c.change_type == "added")
            .count() as i32;

        // 2. 产能影响分析
        let capacity_changes = self.analyze_capacity_changes(before_pools, after_pools);
        let capacity_delta_t = capacity_changes.iter().map(|c| c.delta_t).sum();
        let overflow_delta_t = self.calculate_overflow_delta(before_pools, after_pools);

        // 3. 风险影响分析
        let risk_changes = self.analyze_risk_changes(before_risks, after_risks);
        let (risk_level_before, risk_level_after) = self.get_overall_risk_levels(before_risks, after_risks);

        // 4. 换辊影响分析
        let (roll_campaign_affected, roll_tonnage_delta_t) =
            self.analyze_roll_campaign_impact(before_items, after_items, material_weights);

        // 5. 紧急单影响分析
        let (urgent_material_affected, l3_critical_count) =
            self.analyze_urgent_material_impact(&material_changes, all_materials);

        // 6. 冲突检测
        let locked_conflicts = self.detect_locked_conflicts(&material_changes, all_materials);
        let frozen_conflicts = self.detect_frozen_conflicts(&material_changes, all_materials);
        let structure_suggestions =
            self.generate_structure_suggestions(&material_changes, all_materials);

        ImpactSummary {
            moved_count,
            squeezed_out_count,
            added_count,
            material_changes,
            capacity_delta_t,
            overflow_delta_t,
            capacity_changes,
            risk_level_before,
            risk_level_after,
            risk_changes,
            roll_campaign_affected,
            roll_tonnage_delta_t,
            urgent_material_affected,
            l3_critical_count,
            locked_conflicts,
            frozen_conflicts,
            structure_suggestions,
        }
    }

    // ==========================================
    // 材料影响分析
    // ==========================================

    /// 分析材料变更
    fn analyze_material_changes(
        &self,
        before_items: &[PlanItem],
        after_items: &[PlanItem],
    ) -> Vec<MaterialChange> {
        let mut changes = Vec::new();

        // 建立before和after的映射
        let before_map: HashMap<String, &PlanItem> = before_items
            .iter()
            .map(|item| (item.material_id.clone(), item))
            .collect();

        let after_map: HashMap<String, &PlanItem> = after_items
            .iter()
            .map(|item| (item.material_id.clone(), item))
            .collect();

        // 检测移动和挤出
        for (material_id, before_item) in before_map.iter() {
            if let Some(after_item) = after_map.get(material_id) {
                // 材料存在,检查日期是否变化
                if before_item.plan_date != after_item.plan_date {
                    changes.push(MaterialChange {
                        material_no: material_id.clone(), // 使用material_no作为展示字段
                        change_type: "moved".to_string(),
                        from_date: Some(before_item.plan_date),
                        to_date: Some(after_item.plan_date),
                        reason: format!(
                            "从{}移动到{}",
                            before_item.plan_date, after_item.plan_date
                        ),
                    });
                }
            } else {
                // 材料被挤出
                changes.push(MaterialChange {
                    material_no: material_id.clone(), // 使用material_no作为展示字段
                    change_type: "squeezed_out".to_string(),
                    from_date: Some(before_item.plan_date),
                    to_date: None,
                    reason: "被挤出排程".to_string(),
                });
            }
        }

        // 检测新增材料
        for (material_id, after_item) in after_map.iter() {
            if !before_map.contains_key(material_id) {
                changes.push(MaterialChange {
                    material_no: material_id.clone(), // 使用material_no作为展示字段
                    change_type: "added".to_string(),
                    from_date: None,
                    to_date: Some(after_item.plan_date),
                    reason: format!("新增到{}", after_item.plan_date),
                });
            }
        }

        changes
    }

    // ==========================================
    // 产能影响分析
    // ==========================================

    /// 分析产能变化
    fn analyze_capacity_changes(
        &self,
        before_pools: &[CapacityPool],
        after_pools: &[CapacityPool],
    ) -> Vec<CapacityChange> {
        let mut changes = Vec::new();

        // 建立before映射
        let before_map: HashMap<(NaiveDate, String), &CapacityPool> = before_pools
            .iter()
            .map(|pool| ((pool.plan_date, pool.machine_code.clone()), pool))
            .collect();

        // 比较产能变化
        for after_pool in after_pools {
            let key = (after_pool.plan_date, after_pool.machine_code.clone());
            if let Some(before_pool) = before_map.get(&key) {
                let delta = after_pool.used_capacity_t - before_pool.used_capacity_t;
                if delta.abs() > 0.01 {
                    changes.push(CapacityChange {
                        date: after_pool.plan_date,
                        machine_code: after_pool.machine_code.clone(),
                        used_capacity_before_t: before_pool.used_capacity_t,
                        used_capacity_after_t: after_pool.used_capacity_t,
                        delta_t: delta,
                    });
                }
            }
        }

        changes
    }

    /// 计算超限变化
    fn calculate_overflow_delta(
        &self,
        before_pools: &[CapacityPool],
        after_pools: &[CapacityPool],
    ) -> f64 {
        let before_overflow: f64 = before_pools
            .iter()
            .map(|pool| (pool.used_capacity_t - pool.limit_capacity_t).max(0.0))
            .sum();

        let after_overflow: f64 = after_pools
            .iter()
            .map(|pool| (pool.used_capacity_t - pool.limit_capacity_t).max(0.0))
            .sum();

        after_overflow - before_overflow
    }

    // ==========================================
    // 风险影响分析
    // ==========================================

    /// 分析风险变化
    fn analyze_risk_changes(
        &self,
        before_risks: &[RiskSnapshot],
        after_risks: &[RiskSnapshot],
    ) -> Vec<RiskChange> {
        let mut changes = Vec::new();

        // 建立before映射
        let before_map: HashMap<(NaiveDate, String), &RiskSnapshot> = before_risks
            .iter()
            .map(|risk| ((risk.snapshot_date, risk.machine_code.clone()), risk))
            .collect();

        // 比较风险变化
        for after_risk in after_risks {
            let key = (after_risk.snapshot_date, after_risk.machine_code.clone());
            if let Some(before_risk) = before_map.get(&key) {
                if before_risk.risk_level != after_risk.risk_level {
                    changes.push(RiskChange {
                        date: after_risk.snapshot_date,
                        machine_code: after_risk.machine_code.clone(),
                        risk_before: before_risk.risk_level.to_string(),
                        risk_after: after_risk.risk_level.to_string(),
                        reason: after_risk.risk_reason.clone(),
                    });
                }
            }
        }

        changes
    }

    /// 获取总体风险等级
    fn get_overall_risk_levels(
        &self,
        before_risks: &[RiskSnapshot],
        after_risks: &[RiskSnapshot],
    ) -> (String, String) {
        let before_max = before_risks
            .iter()
            .max_by_key(|r| r.risk_level)
            .map(|r| r.risk_level.to_string())
            .unwrap_or_else(|| "UNKNOWN".to_string());

        let after_max = after_risks
            .iter()
            .max_by_key(|r| r.risk_level)
            .map(|r| r.risk_level.to_string())
            .unwrap_or_else(|| "UNKNOWN".to_string());

        (before_max, after_max)
    }

    // ==========================================
    // 换辊影响分析
    // ==========================================

    /// 分析换辊影响
    fn analyze_roll_campaign_impact(
        &self,
        before_items: &[PlanItem],
        after_items: &[PlanItem],
        material_weights: &HashMap<String, f64>,
    ) -> (bool, Option<f64>) {
        // 计算前后总吨位
        let before_tonnage: f64 = before_items
            .iter()
            .filter_map(|item| material_weights.get(&item.material_id))
            .sum();

        let after_tonnage: f64 = after_items
            .iter()
            .filter_map(|item| material_weights.get(&item.material_id))
            .sum();

        let delta = after_tonnage - before_tonnage;

        // 如果吨位变化超过1吨,认为影响了换辊窗口
        if delta.abs() > 1.0 {
            (true, Some(delta))
        } else {
            (false, None)
        }
    }

    // ==========================================
    // 紧急单影响分析
    // ==========================================

    /// 分析紧急材料影响
    fn analyze_urgent_material_impact(
        &self,
        material_changes: &[MaterialChange],
        all_materials: &[MaterialState],
    ) -> (i32, i32) {
        // 建立材料映射
        let material_map: HashMap<String, &MaterialState> = all_materials
            .iter()
            .map(|m| (m.material_id.clone(), m))
            .collect();

        let mut urgent_count = 0;
        let mut l3_count = 0;

        for change in material_changes {
            if let Some(material) = material_map.get(&change.material_no) {
                match material.urgent_level {
                    crate::domain::types::UrgentLevel::L3 => {
                        urgent_count += 1;
                        l3_count += 1;
                    }
                    crate::domain::types::UrgentLevel::L2 => {
                        urgent_count += 1;
                    }
                    _ => {}
                }
            }
        }

        (urgent_count, l3_count)
    }

    // ==========================================
    // 冲突检测
    // ==========================================

    /// 检测锁定冲突
    fn detect_locked_conflicts(
        &self,
        material_changes: &[MaterialChange],
        all_materials: &[MaterialState],
    ) -> Vec<String> {
        let material_map: HashMap<String, &MaterialState> = all_materials
            .iter()
            .map(|m| (m.material_id.clone(), m))
            .collect();

        material_changes
            .iter()
            .filter(|change| {
                // 只检测移动和挤出
                change.change_type == "moved" || change.change_type == "squeezed_out"
            })
            .filter_map(|change| {
                material_map
                    .get(&change.material_no)
                    .filter(|m| m.lock_flag)
                    .map(|_| change.material_no.clone())
            })
            .collect()
    }

    /// 检测冻结冲突
    fn detect_frozen_conflicts(
        &self,
        material_changes: &[MaterialChange],
        all_materials: &[MaterialState],
    ) -> Vec<String> {
        let material_map: HashMap<String, &MaterialState> = all_materials
            .iter()
            .map(|m| (m.material_id.clone(), m))
            .collect();

        material_changes
            .iter()
            .filter(|change| {
                // 只检测移动和挤出
                change.change_type == "moved" || change.change_type == "squeezed_out"
            })
            .filter_map(|change| {
                material_map
                    .get(&change.material_no)
                    .filter(|m| m.sched_state == crate::domain::types::SchedState::Locked)
                    .map(|_| change.material_no.clone())
            })
            .collect()
    }

    /// 生成结构补偿建议
    fn generate_structure_suggestions(
        &self,
        material_changes: &[MaterialChange],
        all_materials: &[MaterialState],
    ) -> Vec<String> {
        let material_map: HashMap<String, &MaterialState> = all_materials
            .iter()
            .map(|m| (m.material_id.clone(), m))
            .collect();

        let suggestions = Vec::new();

        // 统计挤出材料中的结构类型
        let _squeezed_out: Vec<_> = material_changes
            .iter()
            .filter(|c| c.change_type == "squeezed_out")
            .filter_map(|c| material_map.get(&c.material_no))
            .collect();

        // MaterialState暂时没有structure_key字段，未来添加后可以生成建议
        // if !squeezed_out.is_empty() {
        //     let mut structure_count: HashMap<String, i32> = HashMap::new();
        //     for material in squeezed_out.iter() {
        //         *structure_count
        //             .entry(material.structure_key.clone())
        //             .or_insert(0) += 1;
        //     }
        //     for (structure, count) in structure_count.iter() {
        //         if *count > 0 {
        //             suggestions.push(format!(
        //                 "结构{}被挤出{}个材料,建议补充相同结构材料",
        //                 structure, count
        //             ));
        //         }
        //     }
        // }

        suggestions
    }

    // ==========================================
    // 辅助方法
    // ==========================================

    /// 计算影响日期范围
    pub fn calculate_impacted_date_range(
        &self,
        material_changes: &[MaterialChange],
    ) -> Option<(NaiveDate, NaiveDate)> {
        let dates: Vec<NaiveDate> = material_changes
            .iter()
            .flat_map(|change| {
                let mut dates = vec![];
                if let Some(d) = change.from_date {
                    dates.push(d);
                }
                if let Some(d) = change.to_date {
                    dates.push(d);
                }
                dates
            })
            .collect();

        if dates.is_empty() {
            return None;
        }

        let min_date = *dates.iter().min()?;
        let max_date = *dates.iter().max()?;

        Some((min_date, max_date))
    }

    /// 生成可读描述
    pub fn generate_readable_description(
        &self,
        impact: &ImpactSummary,
        action_type: &str,
    ) -> String {
        let mut parts = vec![format!("操作类型: {}", action_type)];

        if impact.moved_count > 0 {
            parts.push(format!("移动{}个材料", impact.moved_count));
        }
        if impact.squeezed_out_count > 0 {
            parts.push(format!("挤出{}个材料", impact.squeezed_out_count));
        }
        if impact.added_count > 0 {
            parts.push(format!("新增{}个材料", impact.added_count));
        }
        if impact.capacity_delta_t.abs() > 0.01 {
            parts.push(format!("产能变化{:.1}吨", impact.capacity_delta_t));
        }
        if impact.risk_level_before != impact.risk_level_after {
            parts.push(format!(
                "风险{}→{}",
                impact.risk_level_before, impact.risk_level_after
            ));
        }
        if !impact.locked_conflicts.is_empty() {
            parts.push(format!("锁定冲突{}个", impact.locked_conflicts.len()));
        }
        if !impact.frozen_conflicts.is_empty() {
            parts.push(format!("冻结冲突{}个", impact.frozen_conflicts.len()));
        }

        parts.join("; ")
    }
}

// ==========================================
// 单元测试
// ==========================================
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn make_plan_item(material_id: &str, date: NaiveDate) -> PlanItem {
        PlanItem {
            version_id: "v1".to_string(),
            material_id: material_id.to_string(),
            machine_code: "M01".to_string(),
            plan_date: date,
            seq_no: 1,
            weight_t: 100.0,
            source_type: "CALC".to_string(),
            locked_in_plan: false,
            force_release_in_plan: false,
            violation_flags: None,
            urgent_level: Some("L1".to_string()),
            sched_state: Some("READY".to_string()),
            assign_reason: Some("test".to_string()),
        }
    }

    fn make_material_state(material_id: &str, urgency: crate::domain::types::UrgentLevel, is_locked: bool) -> MaterialState {
        MaterialState {
            material_id: material_id.to_string(),
            sched_state: if is_locked {
                crate::domain::types::SchedState::Locked
            } else {
                crate::domain::types::SchedState::Ready
            },
            lock_flag: is_locked,
            force_release_flag: false,
            urgent_level: urgency,
            urgent_reason: None,
            rush_level: crate::domain::types::RushLevel::L0,
            rolling_output_age_days: 3,
            ready_in_days: 0,
            earliest_sched_date: Some(make_date(2025, 1, 15)),
            stock_age_days: 5,
            scheduled_date: None,
            scheduled_machine_code: None,
            seq_no: None,
            manual_urgent_flag: false,
            in_frozen_zone: false,
            last_calc_version_id: None,
            updated_at: chrono::Utc::now(),
            updated_by: None,
        }
    }

    #[test]
    fn test_analyze_material_changes_moved() {
        let engine = ImpactSummaryEngine::new();

        let before = vec![
            make_plan_item("M001", make_date(2025, 1, 15)),
            make_plan_item("M002", make_date(2025, 1, 16)),
        ];

        let after = vec![
            make_plan_item("M001", make_date(2025, 1, 16)), // 移动
            make_plan_item("M002", make_date(2025, 1, 16)), // 不变
        ];

        let changes = engine.analyze_material_changes(&before, &after);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].material_no, "M001");
        assert_eq!(changes[0].change_type, "moved");
    }

    #[test]
    fn test_analyze_material_changes_squeezed_out() {
        let engine = ImpactSummaryEngine::new();

        let before = vec![
            make_plan_item("M001", make_date(2025, 1, 15)),
            make_plan_item("M002", make_date(2025, 1, 16)),
        ];

        let after = vec![make_plan_item("M001", make_date(2025, 1, 15))];

        let changes = engine.analyze_material_changes(&before, &after);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].material_no, "M002");
        assert_eq!(changes[0].change_type, "squeezed_out");
    }

    #[test]
    fn test_analyze_material_changes_added() {
        let engine = ImpactSummaryEngine::new();

        let before = vec![make_plan_item("M001", make_date(2025, 1, 15))];

        let after = vec![
            make_plan_item("M001", make_date(2025, 1, 15)),
            make_plan_item("M002", make_date(2025, 1, 16)),
        ];

        let changes = engine.analyze_material_changes(&before, &after);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].material_no, "M002");
        assert_eq!(changes[0].change_type, "added");
    }

    #[test]
    fn test_detect_locked_conflicts() {
        let engine = ImpactSummaryEngine::new();

        let materials = vec![
            make_material_state("M001", crate::domain::types::UrgentLevel::L2, true),  // 锁定
            make_material_state("M002", crate::domain::types::UrgentLevel::L1, false), // 未锁定
        ];

        let changes = vec![
            MaterialChange {
                material_no: "M001".to_string(),
                change_type: "moved".to_string(),
                from_date: Some(make_date(2025, 1, 15)),
                to_date: Some(make_date(2025, 1, 16)),
                reason: "test".to_string(),
            },
            MaterialChange {
                material_no: "M002".to_string(),
                change_type: "moved".to_string(),
                from_date: Some(make_date(2025, 1, 15)),
                to_date: Some(make_date(2025, 1, 16)),
                reason: "test".to_string(),
            },
        ];

        let conflicts = engine.detect_locked_conflicts(&changes, &materials);

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0], "M001");
    }

    #[test]
    fn test_analyze_urgent_material_impact() {
        let engine = ImpactSummaryEngine::new();

        let materials = vec![
            make_material_state("M001", crate::domain::types::UrgentLevel::L3, false),
            make_material_state("M002", crate::domain::types::UrgentLevel::L2, false),
            make_material_state("M003", crate::domain::types::UrgentLevel::L1, false),
        ];

        let changes = vec![
            MaterialChange {
                material_no: "M001".to_string(),
                change_type: "moved".to_string(),
                from_date: Some(make_date(2025, 1, 15)),
                to_date: Some(make_date(2025, 1, 16)),
                reason: "test".to_string(),
            },
            MaterialChange {
                material_no: "M002".to_string(),
                change_type: "moved".to_string(),
                from_date: Some(make_date(2025, 1, 15)),
                to_date: Some(make_date(2025, 1, 16)),
                reason: "test".to_string(),
            },
            MaterialChange {
                material_no: "M003".to_string(),
                change_type: "moved".to_string(),
                from_date: Some(make_date(2025, 1, 15)),
                to_date: Some(make_date(2025, 1, 16)),
                reason: "test".to_string(),
            },
        ];

        let (urgent_count, l3_count) = engine.analyze_urgent_material_impact(&changes, &materials);

        assert_eq!(urgent_count, 2); // L3 + L2
        assert_eq!(l3_count, 1); // 只有L3
    }

    #[test]
    fn test_calculate_impacted_date_range() {
        let engine = ImpactSummaryEngine::new();

        let changes = vec![
            MaterialChange {
                material_no: "M001".to_string(),
                change_type: "moved".to_string(),
                from_date: Some(make_date(2025, 1, 15)),
                to_date: Some(make_date(2025, 1, 20)),
                reason: "test".to_string(),
            },
            MaterialChange {
                material_no: "M002".to_string(),
                change_type: "moved".to_string(),
                from_date: Some(make_date(2025, 1, 10)),
                to_date: Some(make_date(2025, 1, 18)),
                reason: "test".to_string(),
            },
        ];

        let range = engine.calculate_impacted_date_range(&changes);

        assert!(range.is_some());
        let (start, end) = range.unwrap();
        assert_eq!(start, make_date(2025, 1, 10));
        assert_eq!(end, make_date(2025, 1, 20));
    }

    #[test]
    fn test_generate_readable_description() {
        let engine = ImpactSummaryEngine::new();

        let impact = ImpactSummary {
            moved_count: 2,
            squeezed_out_count: 1,
            added_count: 0,
            material_changes: vec![],
            capacity_delta_t: 50.5,
            overflow_delta_t: 0.0,
            capacity_changes: vec![],
            risk_level_before: "YELLOW".to_string(),
            risk_level_after: "RED".to_string(),
            risk_changes: vec![],
            roll_campaign_affected: false,
            roll_tonnage_delta_t: None,
            urgent_material_affected: 0,
            l3_critical_count: 0,
            locked_conflicts: vec!["M001".to_string()],
            frozen_conflicts: vec![],
            structure_suggestions: vec![],
        };

        let desc = engine.generate_readable_description(&impact, "LocalAdjust");

        assert!(desc.contains("操作类型: LocalAdjust"));
        assert!(desc.contains("移动2个材料"));
        assert!(desc.contains("挤出1个材料"));
        assert!(desc.contains("产能变化50.5吨"));
        assert!(desc.contains("风险YELLOW→RED"));
        assert!(desc.contains("锁定冲突1个"));
    }
}

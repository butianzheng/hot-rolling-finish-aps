// ==========================================
// 热轧精整排产系统 - 结构控制引擎
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - Structure Corrector
// 依据: Claude_Dev_Master_Spec.md - PART B3 产能与换辊
// 红线: 结构目标为软约束
// 红线: 锁定材料不可跳过（即使违反结构目标）
// ==========================================
// 职责: 结构软控制与违规标记
// 输入: 产能池 + 排产明细 + 材料主数据
// 输出: 结构违规标记 + 调整建议
// ==========================================
// 注: MVP 以提示为主,不强制调整
// ==========================================

use crate::domain::capacity::{CapacityConstraint, CapacityPool};
use crate::domain::material::{MaterialMaster, MaterialState};
use crate::domain::plan::PlanItem;
use crate::domain::types::SchedState;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

// ==========================================
// StructureCorrector - 结构控制引擎
// ==========================================
// MVP: 无状态引擎,结构目标通过参数传入
pub struct StructureCorrector {
    // 无状态,所有配置通过参数传入
}

impl StructureCorrector {
    /// 创建新的结构控制引擎
    pub fn new() -> Self {
        Self {}
    }

    // ==========================================
    // 配置验证
    // ==========================================

    /// 验证目标配比配置的有效性
    ///
    /// # 参数
    /// - `target_ratio`: 目标配比 (steel_mark -> 目标比例)
    ///
    /// # 返回
    /// - `Ok(())`: 配置有效
    /// - `Err(String)`: 配置无效,返回错误描述
    ///
    /// # 验证规则
    /// 1. 目标配比不能为空
    /// 2. 所有配比值必须在 [0.0, 1.0] 范围内
    /// 3. 所有配比值之和应接近 1.0 (允许 1% 误差)
    /// 4. 钢种名称不能为空
    pub fn validate_target_ratio(
        &self,
        target_ratio: &HashMap<String, f64>,
    ) -> Result<(), String> {
        // 1. 检查是否为空
        if target_ratio.is_empty() {
            warn!("目标配比为空");
            return Err("目标配比不能为空".to_string());
        }

        // 2. 检查每个配比值的有效性
        for (steel_mark, ratio) in target_ratio {
            // 检查钢种名称
            if steel_mark.trim().is_empty() {
                warn!("目标配比包含空钢种名称");
                return Err("钢种名称不能为空".to_string());
            }

            // 检查配比值范围
            if *ratio < 0.0 || *ratio > 1.0 {
                warn!(
                    steel_mark = %steel_mark,
                    ratio = ratio,
                    "目标配比值超出有效范围 [0.0, 1.0]"
                );
                return Err(format!(
                    "钢种 {} 的目标配比 {} 超出有效范围 [0.0, 1.0]",
                    steel_mark, ratio
                ));
            }

            // 检查是否为 NaN 或无穷大
            if ratio.is_nan() {
                warn!(steel_mark = %steel_mark, "目标配比值为 NaN");
                return Err(format!("钢种 {} 的目标配比为 NaN", steel_mark));
            }

            if ratio.is_infinite() {
                warn!(steel_mark = %steel_mark, "目标配比值为无穷大");
                return Err(format!("钢种 {} 的目标配比为无穷大", steel_mark));
            }
        }

        // 3. 检查配比之和是否接近 1.0
        let sum: f64 = target_ratio.values().sum();
        let tolerance = 0.01; // 1% 误差容忍度

        if (sum - 1.0).abs() > tolerance {
            warn!(
                sum = sum,
                tolerance = tolerance,
                "目标配比之和不等于 1.0"
            );
            return Err(format!(
                "目标配比之和 {:.4} 不等于 1.0 (允许误差 {})",
                sum, tolerance
            ));
        }

        info!(
            steel_grades = target_ratio.len(),
            sum = sum,
            "目标配比验证通过"
        );

        Ok(())
    }

    /// 验证偏差阈值的有效性
    ///
    /// # 参数
    /// - `deviation_threshold`: 偏差阈值
    ///
    /// # 返回
    /// - `Ok(())`: 配置有效
    /// - `Err(String)`: 配置无效,返回错误描述
    ///
    /// # 验证规则
    /// 1. 阈值必须在 [0.0, 1.0] 范围内
    /// 2. 阈值不能为 NaN 或无穷大
    pub fn validate_deviation_threshold(&self, deviation_threshold: f64) -> Result<(), String> {
        // 检查范围
        if deviation_threshold < 0.0 || deviation_threshold > 1.0 {
            warn!(
                deviation_threshold = deviation_threshold,
                "偏差阈值超出有效范围 [0.0, 1.0]"
            );
            return Err(format!(
                "偏差阈值 {} 超出有效范围 [0.0, 1.0]",
                deviation_threshold
            ));
        }

        // 检查 NaN
        if deviation_threshold.is_nan() {
            warn!("偏差阈值为 NaN");
            return Err("偏差阈值为 NaN".to_string());
        }

        // 检查无穷大
        if deviation_threshold.is_infinite() {
            warn!("偏差阈值为无穷大");
            return Err("偏差阈值为无穷大".to_string());
        }

        debug!(
            deviation_threshold = deviation_threshold,
            "偏差阈值验证通过"
        );

        Ok(())
    }

    // ==========================================
    // 核心方法
    // ==========================================

    /// 检查单日结构违规
    ///
    /// # 参数
    /// - `pool`: 产能池
    /// - `items`: 排产明细列表
    /// - `materials`: 材料主数据映射 (material_id -> MaterialMaster)
    /// - `material_states`: 材料状态映射 (material_id -> MaterialState)
    /// - `target_ratio`: 目标配比 (steel_mark -> 目标比例)
    /// - `deviation_threshold`: 偏差阈值 (0.0-1.0, 例如 0.15 表示 15%)
    ///
    /// # 返回
    /// 结构违规报告
    pub fn check_structure_violation(
        &self,
        pool: &CapacityPool,
        items: &[PlanItem],
        materials: &HashMap<String, MaterialMaster>,
        material_states: &HashMap<String, MaterialState>,
        target_ratio: &HashMap<String, f64>,
        deviation_threshold: f64,
    ) -> StructureViolationReport {
        debug!(
            machine_code = %pool.machine_code,
            plan_date = %pool.plan_date,
            items_count = items.len(),
            target_grades = target_ratio.len(),
            deviation_threshold = deviation_threshold,
            "开始检查结构违规"
        );

        // 0. 空列表时不报违规
        if items.is_empty() {
            info!(
                machine_code = %pool.machine_code,
                plan_date = %pool.plan_date,
                "排产明细为空,不报违规"
            );
            return StructureViolationReport {
                machine_code: pool.machine_code.clone(),
                plan_date: pool.plan_date,
                is_violated: false,
                violation_desc: None,
                suggestions: Vec::new(),
                deviation_ratio: 0.0,
                actual_ratio: HashMap::new(),
                target_ratio: target_ratio.clone(),
            };
        }

        // 1. 计算实际钢种配比
        let actual_ratio = self.calculate_steel_grade_ratio(items, materials);

        // 2. 计算偏差
        let deviation = self.calculate_deviation(&actual_ratio, target_ratio);

        // 3. 检查是否违规
        let is_violated = deviation > deviation_threshold;

        if is_violated {
            warn!(
                machine_code = %pool.machine_code,
                plan_date = %pool.plan_date,
                deviation_ratio = deviation,
                deviation_threshold = deviation_threshold,
                "检测到结构违规"
            );
        } else {
            info!(
                machine_code = %pool.machine_code,
                plan_date = %pool.plan_date,
                deviation_ratio = deviation,
                deviation_threshold = deviation_threshold,
                "结构配比符合目标"
            );
        }

        // 4. 生成违规说明
        let violation_desc = if is_violated {
            Some(self.generate_violation_description(&actual_ratio, target_ratio, deviation))
        } else {
            None
        };

        // 5. 生成调整建议（如果违规）
        let suggestions = if is_violated {
            self.generate_suggestions(
                pool,
                items,
                materials,
                material_states,
                &actual_ratio,
                target_ratio,
                deviation,
            )
        } else {
            Vec::new()
        };

        // 6. 构建报告
        StructureViolationReport {
            machine_code: pool.machine_code.clone(),
            plan_date: pool.plan_date,
            is_violated,
            violation_desc,
            suggestions,
            deviation_ratio: deviation,
            actual_ratio,
            target_ratio: target_ratio.clone(),
        }
    }

    /// 批量检查结构违规
    ///
    /// # 参数
    /// - `pools`: 产能池列表
    /// - `items_by_date`: 按日期分组的排产明细
    /// - `materials`: 材料主数据映射
    /// - `material_states`: 材料状态映射
    /// - `target_ratios`: 按机组和日期的目标配比
    /// - `deviation_threshold`: 偏差阈值
    ///
    /// # 返回
    /// 结构违规报告列表
    pub fn check_batch(
        &self,
        pools: Vec<CapacityPool>,
        items_by_date: HashMap<(String, NaiveDate), Vec<PlanItem>>,
        materials: &HashMap<String, MaterialMaster>,
        material_states: &HashMap<String, MaterialState>,
        target_ratios: &HashMap<(String, NaiveDate), HashMap<String, f64>>,
        deviation_threshold: f64,
    ) -> Vec<StructureViolationReport> {
        pools
            .iter()
            .map(|pool| {
                let key = (pool.machine_code.clone(), pool.plan_date);
                let items = items_by_date.get(&key).cloned().unwrap_or_default();
                let target_ratio = target_ratios.get(&key).cloned().unwrap_or_default();

                self.check_structure_violation(
                    pool,
                    &items,
                    materials,
                    material_states,
                    &target_ratio,
                    deviation_threshold,
                )
            })
            .collect()
    }

    // ==========================================
    // 结构分析
    // ==========================================

    /// 计算钢种配比
    ///
    /// # 参数
    /// - `items`: 排产明细列表
    /// - `materials`: 材料主数据映射
    ///
    /// # 返回
    /// 钢种配比 (steel_mark -> 重量占比)
    ///
    /// # 注意
    /// 只计算有钢种信息的材料,缺失钢种的材料不计入总重量
    pub fn calculate_steel_grade_ratio(
        &self,
        items: &[PlanItem],
        materials: &HashMap<String, MaterialMaster>,
    ) -> HashMap<String, f64> {
        // 1. 按钢种分组统计重量(只统计有钢种信息的材料)
        let mut steel_grade_weights: HashMap<String, f64> = HashMap::new();
        let mut missing_steel_mark_count = 0;
        let mut missing_material_count = 0;

        for item in items {
            if let Some(material) = materials.get(&item.material_id) {
                if let Some(steel_mark) = &material.steel_mark {
                    *steel_grade_weights.entry(steel_mark.clone()).or_insert(0.0) +=
                        item.weight_t;
                } else {
                    // 材料缺失钢种信息
                    missing_steel_mark_count += 1;
                    warn!(
                        material_id = %item.material_id,
                        weight_t = item.weight_t,
                        "材料缺失钢种信息 (steel_mark),该材料不计入结构配比计算"
                    );
                }
            } else {
                // 材料主数据缺失
                missing_material_count += 1;
                warn!(
                    material_id = %item.material_id,
                    weight_t = item.weight_t,
                    "材料主数据缺失,该材料不计入结构配比计算"
                );
            }
        }

        // 记录统计信息
        if missing_steel_mark_count > 0 || missing_material_count > 0 {
            info!(
                total_items = items.len(),
                missing_steel_mark = missing_steel_mark_count,
                missing_material = missing_material_count,
                valid_items = items.len() - missing_steel_mark_count - missing_material_count,
                "结构配比计算完成,部分材料因缺失信息被排除"
            );
        }

        // 2. 计算总重量(只计算有钢种信息的材料)
        let total_weight: f64 = steel_grade_weights.values().sum();

        if total_weight <= 0.0 {
            warn!("所有材料的总重量为0或所有材料缺失钢种信息,无法计算配比");
            return HashMap::new();
        }

        // 3. 计算配比（重量占比）
        let ratio: HashMap<String, f64> = steel_grade_weights
            .into_iter()
            .map(|(steel_mark, weight)| (steel_mark, weight / total_weight))
            .collect();

        debug!(
            total_weight_t = total_weight,
            steel_grades = ratio.len(),
            "钢种配比计算完成"
        );

        ratio
    }

    /// 计算配比偏差
    ///
    /// # 参数
    /// - `actual_ratio`: 实际配比
    /// - `target_ratio`: 目标配比
    ///
    /// # 返回
    /// 最大偏差比例 (0.0-1.0+)
    ///
    /// # 算法
    /// 计算每个钢种的绝对偏差,返回最大值
    pub fn calculate_deviation(
        &self,
        actual_ratio: &HashMap<String, f64>,
        target_ratio: &HashMap<String, f64>,
    ) -> f64 {
        if target_ratio.is_empty() {
            return 0.0;
        }

        let mut max_deviation: f64 = 0.0;

        // 检查所有目标钢种的偏差
        for (steel_mark, target_pct) in target_ratio {
            let actual_pct = actual_ratio.get(steel_mark).copied().unwrap_or(0.0);
            let deviation = (actual_pct - target_pct).abs();
            max_deviation = max_deviation.max(deviation);
        }

        // 检查实际中存在但目标中没有的钢种
        for (steel_mark, actual_pct) in actual_ratio {
            if !target_ratio.contains_key(steel_mark) {
                max_deviation = max_deviation.max(*actual_pct);
            }
        }

        max_deviation
    }

    /// 生成违规描述
    fn generate_violation_description(
        &self,
        actual_ratio: &HashMap<String, f64>,
        target_ratio: &HashMap<String, f64>,
        deviation: f64,
    ) -> String {
        let mut violations = Vec::new();

        // 检查每个目标钢种
        for (steel_mark, target_pct) in target_ratio {
            let actual_pct = actual_ratio.get(steel_mark).copied().unwrap_or(0.0);
            let diff = actual_pct - target_pct;
            if diff.abs() > 0.01 {
                // 忽略 1% 以内的微小差异
                let sign = if diff > 0.0 { "超出" } else { "不足" };
                violations.push(format!(
                    "{}：目标 {:.1}%，实际 {:.1}%（{} {:.1}%）",
                    steel_mark,
                    target_pct * 100.0,
                    actual_pct * 100.0,
                    sign,
                    diff.abs() * 100.0
                ));
            }
        }

        // 检查非目标钢种
        for (steel_mark, actual_pct) in actual_ratio {
            if !target_ratio.contains_key(steel_mark) && *actual_pct > 0.01 {
                violations.push(format!(
                    "{}：非目标钢种，实际占比 {:.1}%",
                    steel_mark,
                    actual_pct * 100.0
                ));
            }
        }

        format!(
            "结构偏差 {:.1}%。{}",
            deviation * 100.0,
            violations.join("；")
        )
    }

    /// 生成调整建议
    ///
    /// # MVP 版本
    /// - 提示超出目标的钢种可以延后
    /// - 提示不足目标的钢种需要补充
    /// - 检查是否有锁定材料冲突
    fn generate_suggestions(
        &self,
        pool: &CapacityPool,
        items: &[PlanItem],
        materials: &HashMap<String, MaterialMaster>,
        material_states: &HashMap<String, MaterialState>,
        actual_ratio: &HashMap<String, f64>,
        target_ratio: &HashMap<String, f64>,
        _deviation: f64,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        // 1. 分析超出目标的钢种
        let mut over_target_grades = Vec::new();
        for (steel_mark, target_pct) in target_ratio {
            let actual_pct = actual_ratio.get(steel_mark).copied().unwrap_or(0.0);
            if actual_pct > target_pct + 0.05 {
                // 超出 5%
                over_target_grades.push((steel_mark.clone(), actual_pct - target_pct));
            }
        }

        // 2. 分析不足目标的钢种
        let mut under_target_grades = Vec::new();
        for (steel_mark, target_pct) in target_ratio {
            let actual_pct = actual_ratio.get(steel_mark).copied().unwrap_or(0.0);
            if actual_pct < target_pct - 0.05 {
                // 不足 5%
                under_target_grades.push((steel_mark.clone(), target_pct - actual_pct));
            }
        }

        // 3. 检查锁定材料冲突
        let locked_conflict_grades = self.find_locked_material_conflicts(
            items,
            materials,
            material_states,
            &over_target_grades
                .iter()
                .map(|(g, _)| g.clone())
                .collect::<Vec<_>>(),
        );

        // 4. 生成建议
        if !over_target_grades.is_empty() {
            for (steel_mark, excess_pct) in &over_target_grades {
                if locked_conflict_grades.contains(steel_mark) {
                    suggestions.push(format!(
                        "【锁定冲突】钢种 {} 超出目标 {:.1}%，但包含锁定材料，无法调整",
                        steel_mark,
                        excess_pct * 100.0
                    ));
                } else {
                    suggestions.push(format!(
                        "建议延后部分钢种 {} 材料（超出目标 {:.1}%）",
                        steel_mark,
                        excess_pct * 100.0
                    ));
                }
            }
        }

        if !under_target_grades.is_empty() {
            for (steel_mark, shortage_pct) in &under_target_grades {
                suggestions.push(format!(
                    "建议补充钢种 {} 材料（不足目标 {:.1}%）",
                    steel_mark,
                    shortage_pct * 100.0
                ));
            }
        }

        // 5. 产能池状态提示
        let remaining = pool.remaining_capacity_t();
        if remaining > 0.0 {
            suggestions.push(format!("剩余产能 {:.2} 吨，可用于结构调整", remaining));
        } else {
            suggestions.push("产能已满，调整需要替换现有材料".to_string());
        }

        suggestions
    }

    /// 查找锁定材料冲突的钢种
    ///
    /// # 参数
    /// - `items`: 排产明细
    /// - `materials`: 材料主数据
    /// - `material_states`: 材料状态
    /// - `steel_marks`: 待检查的钢种列表
    ///
    /// # 返回
    /// 包含锁定材料的钢种列表
    fn find_locked_material_conflicts(
        &self,
        items: &[PlanItem],
        materials: &HashMap<String, MaterialMaster>,
        material_states: &HashMap<String, MaterialState>,
        steel_marks: &[String],
    ) -> Vec<String> {
        let mut locked_grades = Vec::new();

        for steel_mark in steel_marks {
            // 检查该钢种是否有锁定材料
            let mut locked_materials = Vec::new();

            let has_locked = items.iter().any(|item| {
                if let Some(material) = materials.get(&item.material_id) {
                    if let Some(ref item_steel_mark) = material.steel_mark {
                        if item_steel_mark == steel_mark {
                            // 检查是否锁定
                            if let Some(state) = material_states.get(&item.material_id) {
                                let is_locked = state.sched_state == SchedState::Locked
                                    || state.lock_flag;
                                if is_locked {
                                    locked_materials.push(item.material_id.clone());
                                }
                                return is_locked;
                            }
                        }
                    }
                }
                false
            });

            if has_locked {
                locked_grades.push(steel_mark.clone());
                warn!(
                    steel_mark = %steel_mark,
                    locked_materials_count = locked_materials.len(),
                    locked_materials = ?locked_materials,
                    "检测到锁定材料冲突,该钢种包含锁定材料,无法调整"
                );
            }
        }

        if !locked_grades.is_empty() {
            info!(
                locked_grades_count = locked_grades.len(),
                locked_grades = ?locked_grades,
                "锁定材料冲突检测完成"
            );
        }

        locked_grades
    }
}

impl Default for StructureCorrector {
    fn default() -> Self {
        Self::new()
    }
}

// ==========================================
// StructureViolationReport - 结构违规报告
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureViolationReport {
    /// 机组代码
    pub machine_code: String,

    /// 排产日期
    pub plan_date: NaiveDate,

    /// 是否违规
    pub is_violated: bool,

    /// 违规描述
    pub violation_desc: Option<String>,

    /// 调整建议列表
    pub suggestions: Vec<String>,

    /// 偏差比例（最大偏差）
    pub deviation_ratio: f64,

    /// 实际配比 (steel_mark -> 占比)
    pub actual_ratio: HashMap<String, f64>,

    /// 目标配比 (steel_mark -> 占比)
    pub target_ratio: HashMap<String, f64>,
}

// ==========================================
// 测试模块
// ==========================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::capacity::CapacityPool;
    use crate::domain::material::{MaterialMaster, MaterialState};
    use crate::domain::plan::PlanItem;
    use crate::domain::types::{RushLevel, SchedState, UrgentLevel};
    use chrono::{NaiveDate, Utc};
    use std::collections::HashMap;

    // ==========================================
    // 测试辅助函数
    // ==========================================

    /// 创建测试用的产能池
    fn create_test_capacity_pool(
        machine_code: &str,
        plan_date: NaiveDate,
        target_capacity_t: f64,
        limit_capacity_t: f64,
        used_capacity_t: f64,
    ) -> CapacityPool {
        CapacityPool {
            version_id: "VTEST".to_string(),
            machine_code: machine_code.to_string(),
            plan_date,
            target_capacity_t,
            limit_capacity_t,
            used_capacity_t,
            overflow_t: 0.0,
            frozen_capacity_t: 0.0,
            accumulated_tonnage_t: 0.0,
            roll_campaign_id: None,
        }
    }

    /// 创建测试用的排产明细
    fn create_test_plan_item(
        item_id: &str,
        version_id: &str,
        material_id: &str,
        machine_code: &str,
        plan_date: NaiveDate,
        weight_t: f64,
        is_frozen: bool,
    ) -> PlanItem {
        PlanItem {
            version_id: version_id.to_string(),
            material_id: material_id.to_string(),
            machine_code: machine_code.to_string(),
            plan_date,
            seq_no: 1,
            weight_t,
            source_type: if is_frozen { "FROZEN".to_string() } else { "CALC".to_string() },
            locked_in_plan: is_frozen,
            force_release_in_plan: false,
            violation_flags: None,
            urgent_level: Some("L0".to_string()),
            sched_state: Some("READY".to_string()),
            assign_reason: Some("TEST".to_string()),
            steel_grade: None,
        }
    }

    /// 创建测试用的材料主数据
    fn create_test_material_master(material_id: &str, steel_mark: Option<&str>) -> MaterialMaster {
        MaterialMaster {
            material_id: material_id.to_string(),
            manufacturing_order_id: None,
            material_status_code_src: None,
            steel_mark: steel_mark.map(|s| s.to_string()),
            slab_id: None,
            next_machine_code: None,
            rework_machine_code: None,
            current_machine_code: Some("H032".to_string()),
            width_mm: Some(1500.0),
            thickness_mm: Some(10.0),
            length_m: Some(10.0),
            weight_t: Some(1.0),
            available_width_mm: Some(1500.0),
            due_date: None,
            stock_age_days: Some(10),
            output_age_days_raw: Some(5),
            status_updated_at: None,
            contract_no: None,
            contract_nature: None,
            weekly_delivery_flag: None,
            export_flag: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 创建测试用的材料状态
    fn create_test_material_state(
        material_id: &str,
        sched_state: SchedState,
        lock_flag: bool,
    ) -> MaterialState {
        MaterialState {
            material_id: material_id.to_string(),
            sched_state,
            lock_flag,
            force_release_flag: false,
            urgent_level: UrgentLevel::L0,
            urgent_reason: None,
            rush_level: RushLevel::L0,
            rolling_output_age_days: 5,
            ready_in_days: 0,
            earliest_sched_date: None,
            stock_age_days: 10,
            scheduled_date: None,
            scheduled_machine_code: None,
            seq_no: None,
            manual_urgent_flag: false,
            in_frozen_zone: false,
            last_calc_version_id: None,
            updated_at: Utc::now(),
            updated_by: None,
        }
    }

    // ==========================================
    // 正常案例测试
    // ==========================================

    #[test]
    fn test_scenario_1_perfect_match() {
        // 场景1: 完全达标
        let corrector = StructureCorrector::new();

        let pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            800.0,
            900.0,
            600.0,
        );

        let items = vec![
            create_test_plan_item(
                "item1",
                "v1",
                "M001",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                360.0,
                false,
            ),
            create_test_plan_item(
                "item2",
                "v1",
                "M002",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                240.0,
                false,
            ),
        ];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );
        materials.insert(
            "M002".to_string(),
            create_test_material_master("M002", Some("Q235")),
        );

        let material_states = HashMap::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.6);
        target_ratio.insert("Q235".to_string(), 0.4);

        let report = corrector.check_structure_violation(
            &pool,
            &items,
            &materials,
            &material_states,
            &target_ratio,
            0.15,
        );

        assert!(!report.is_violated);
        assert!((report.deviation_ratio - 0.0).abs() < 0.001);
        assert!(report.violation_desc.is_none());
        assert_eq!(report.suggestions.len(), 0);
    }

    #[test]
    fn test_scenario_2_minor_deviation() {
        // 场景2: 轻度违规（在阈值内）
        let corrector = StructureCorrector::new();

        let pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            800.0,
            900.0,
            600.0,
        );

        let items = vec![
            create_test_plan_item(
                "item1",
                "v1",
                "M001",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                400.0,
                false,
            ),
            create_test_plan_item(
                "item2",
                "v1",
                "M002",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                200.0,
                false,
            ),
        ];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );
        materials.insert(
            "M002".to_string(),
            create_test_material_master("M002", Some("Q235")),
        );

        let material_states = HashMap::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.6);
        target_ratio.insert("Q235".to_string(), 0.4);

        let report = corrector.check_structure_violation(
            &pool,
            &items,
            &materials,
            &material_states,
            &target_ratio,
            0.15,
        );

        assert!(!report.is_violated);
        assert!((report.deviation_ratio - 0.067).abs() < 0.01);
        assert_eq!(report.suggestions.len(), 0);
    }

    #[test]
    fn test_scenario_3_severe_violation() {
        // 场景3: 严重违规（超过阈值）
        let corrector = StructureCorrector::new();

        let pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            800.0,
            900.0,
            700.0,
        );

        let items = vec![
            create_test_plan_item(
                "item1",
                "v1",
                "M001",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                560.0,
                false,
            ),
            create_test_plan_item(
                "item2",
                "v1",
                "M002",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                140.0,
                false,
            ),
        ];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );
        materials.insert(
            "M002".to_string(),
            create_test_material_master("M002", Some("Q235")),
        );

        let material_states = HashMap::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.6);
        target_ratio.insert("Q235".to_string(), 0.4);

        let report = corrector.check_structure_violation(
            &pool,
            &items,
            &materials,
            &material_states,
            &target_ratio,
            0.15,
        );

        assert!(report.is_violated);
        assert!((report.deviation_ratio - 0.2).abs() < 0.001);
        assert!(report.violation_desc.is_some());
        assert!(report
            .violation_desc
            .as_ref()
            .unwrap()
            .contains("结构偏差 20.0%"));
        assert!(report.suggestions.len() >= 3);
        assert!(report
            .suggestions
            .iter()
            .any(|s| s.contains("延后") && s.contains("Q345")));
        assert!(report
            .suggestions
            .iter()
            .any(|s| s.contains("补充") && s.contains("Q235")));
        assert!(report.suggestions.iter().any(|s| s.contains("剩余产能")));
    }

    #[test]
    fn test_scenario_5_ratio_calculation() {
        // 场景5: 钢种配比计算准确性
        let corrector = StructureCorrector::new();

        let items = vec![
            create_test_plan_item(
                "item1",
                "v1",
                "M001",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                300.0,
                false,
            ),
            create_test_plan_item(
                "item2",
                "v1",
                "M002",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                200.0,
                false,
            ),
            create_test_plan_item(
                "item3",
                "v1",
                "M003",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                400.0,
                false,
            ),
            create_test_plan_item(
                "item4",
                "v1",
                "M004",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                100.0,
                false,
            ),
        ];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );
        materials.insert(
            "M002".to_string(),
            create_test_material_master("M002", Some("Q345")),
        );
        materials.insert(
            "M003".to_string(),
            create_test_material_master("M003", Some("Q235")),
        );
        materials.insert(
            "M004".to_string(),
            create_test_material_master("M004", Some("Q390")),
        );

        let actual_ratio = corrector.calculate_steel_grade_ratio(&items, &materials);

        assert!((actual_ratio.get("Q345").unwrap() - 0.5).abs() < 0.001);
        assert!((actual_ratio.get("Q235").unwrap() - 0.4).abs() < 0.001);
        assert!((actual_ratio.get("Q390").unwrap() - 0.1).abs() < 0.001);

        let sum: f64 = actual_ratio.values().sum();
        assert!((sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_scenario_6_deviation_calculation() {
        // 场景6: 偏差计算准确性
        let corrector = StructureCorrector::new();

        let mut actual_ratio = HashMap::new();
        actual_ratio.insert("Q345".to_string(), 0.7);
        actual_ratio.insert("Q235".to_string(), 0.2);
        actual_ratio.insert("Q390".to_string(), 0.1);

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.5);
        target_ratio.insert("Q235".to_string(), 0.3);
        target_ratio.insert("Q390".to_string(), 0.2);

        let deviation = corrector.calculate_deviation(&actual_ratio, &target_ratio);

        assert!((deviation - 0.2).abs() < 0.001);
    }

    // ==========================================
    // 边界案例测试
    // ==========================================

    #[test]
    fn test_scenario_7_empty_items() {
        // 场景7: 空排产明细
        let corrector = StructureCorrector::new();

        let pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            800.0,
            900.0,
            0.0,
        );

        let items = vec![];
        let materials = HashMap::new();
        let material_states = HashMap::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.6);
        target_ratio.insert("Q235".to_string(), 0.4);

        let report = corrector.check_structure_violation(
            &pool,
            &items,
            &materials,
            &material_states,
            &target_ratio,
            0.15,
        );

        assert!(!report.is_violated);
        assert!((report.deviation_ratio - 0.0).abs() < 0.001);
        assert!(report.actual_ratio.is_empty());
        assert_eq!(report.suggestions.len(), 0);
    }

    #[test]
    fn test_scenario_8_no_target_ratio() {
        // 场景8: 无目标配比
        let corrector = StructureCorrector::new();

        let pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            800.0,
            900.0,
            600.0,
        );

        let items = vec![
            create_test_plan_item(
                "item1",
                "v1",
                "M001",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                400.0,
                false,
            ),
            create_test_plan_item(
                "item2",
                "v1",
                "M002",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                200.0,
                false,
            ),
        ];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );
        materials.insert(
            "M002".to_string(),
            create_test_material_master("M002", Some("Q235")),
        );

        let material_states = HashMap::new();
        let target_ratio = HashMap::new(); // 空目标配比

        let report = corrector.check_structure_violation(
            &pool,
            &items,
            &materials,
            &material_states,
            &target_ratio,
            0.15,
        );

        assert!(!report.is_violated);
        assert!((report.deviation_ratio - 0.0).abs() < 0.001);
        assert!(report.violation_desc.is_none());
    }

    #[test]
    fn test_scenario_9_single_steel_grade() {
        // 场景9: 单个钢种
        let corrector = StructureCorrector::new();

        let pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            800.0,
            900.0,
            500.0,
        );

        let items = vec![create_test_plan_item(
            "item1",
            "v1",
            "M001",
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            500.0,
            false,
        )];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );

        let material_states = HashMap::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 1.0);

        let report = corrector.check_structure_violation(
            &pool,
            &items,
            &materials,
            &material_states,
            &target_ratio,
            0.15,
        );

        assert!(!report.is_violated);
        assert!((report.actual_ratio.get("Q345").unwrap() - 1.0).abs() < 0.001);
        assert!((report.deviation_ratio - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_scenario_10_missing_steel_mark() {
        // 场景10: 缺失钢种信息
        let corrector = StructureCorrector::new();

        let items = vec![
            create_test_plan_item(
                "item1",
                "v1",
                "M001",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                400.0,
                false,
            ),
            create_test_plan_item(
                "item2",
                "v1",
                "M002",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                200.0,
                false,
            ),
        ];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );
        materials.insert(
            "M002".to_string(),
            create_test_material_master("M002", None), // 缺失钢种
        );

        let actual_ratio = corrector.calculate_steel_grade_ratio(&items, &materials);

        // 缺失钢种的材料不计入总重量,所以 Q345 占比应该是 400/(400+0) = 1.0
        // 但实际上总重量是 400+200=600,所以 Q345 占比是 400/600 = 0.667
        assert_eq!(actual_ratio.len(), 1);
        assert!((actual_ratio.get("Q345").unwrap() - 400.0 / 400.0).abs() < 0.001);
    }

    // ==========================================
    // 工业边缘案例测试
    // ==========================================

    #[test]
    fn test_scenario_12_locked_material_conflict() {
        // 场景12: 锁定材料冲突
        let corrector = StructureCorrector::new();

        let pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            800.0,
            900.0,
            700.0,
        );

        let items = vec![
            create_test_plan_item(
                "item1",
                "v1",
                "M001",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                560.0,
                false,
            ),
            create_test_plan_item(
                "item2",
                "v1",
                "M002",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                140.0,
                false,
            ),
        ];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );
        materials.insert(
            "M002".to_string(),
            create_test_material_master("M002", Some("Q235")),
        );

        let mut material_states = HashMap::new();
        material_states.insert(
            "M001".to_string(),
            create_test_material_state("M001", SchedState::Locked, true),
        );
        material_states.insert(
            "M002".to_string(),
            create_test_material_state("M002", SchedState::Ready, false),
        );

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.6);
        target_ratio.insert("Q235".to_string(), 0.4);

        let report = corrector.check_structure_violation(
            &pool,
            &items,
            &materials,
            &material_states,
            &target_ratio,
            0.15,
        );

        assert!(report.is_violated);
        assert!(report
            .suggestions
            .iter()
            .any(|s| s.contains("【锁定冲突】") && s.contains("Q345")));
    }

    #[test]
    fn test_scenario_4_single_steel_grade_excess() {
        // 场景4: 单钢种超标
        let corrector = StructureCorrector::new();

        let pool = create_test_capacity_pool(
            "H033",
            NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
            1000.0,
            1100.0,
            800.0,
        );

        let items = vec![
            create_test_plan_item(
                "item1",
                "v1",
                "M001",
                "H033",
                NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
                500.0,
                false,
            ),
            create_test_plan_item(
                "item2",
                "v1",
                "M002",
                "H033",
                NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
                200.0,
                false,
            ),
            create_test_plan_item(
                "item3",
                "v1",
                "M003",
                "H033",
                NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
                100.0,
                false,
            ),
        ];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );
        materials.insert(
            "M002".to_string(),
            create_test_material_master("M002", Some("Q235")),
        );
        materials.insert(
            "M003".to_string(),
            create_test_material_master("M003", Some("Q390")),
        );

        let material_states = HashMap::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.5);
        target_ratio.insert("Q235".to_string(), 0.3);
        target_ratio.insert("Q390".to_string(), 0.2);

        let report = corrector.check_structure_violation(
            &pool,
            &items,
            &materials,
            &material_states,
            &target_ratio,
            0.10,
        );

        // 实际配比: Q345=0.625, Q235=0.25, Q390=0.125
        // 最大偏差: Q345 超标 0.125 (12.5%)
        assert!(report.is_violated);
        assert!((report.deviation_ratio - 0.125).abs() < 0.001);
        assert!(report.suggestions.iter().any(|s| s.contains("Q345")));
    }

    #[test]
    fn test_scenario_11_zero_threshold() {
        // 场景11: 偏差阈值为0
        let corrector = StructureCorrector::new();

        let pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            800.0,
            900.0,
            600.0,
        );

        let items = vec![
            create_test_plan_item(
                "item1",
                "v1",
                "M001",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                361.0,
                false,
            ),
            create_test_plan_item(
                "item2",
                "v1",
                "M002",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                239.0,
                false,
            ),
        ];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );
        materials.insert(
            "M002".to_string(),
            create_test_material_master("M002", Some("Q235")),
        );

        let material_states = HashMap::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.6);
        target_ratio.insert("Q235".to_string(), 0.4);

        let report = corrector.check_structure_violation(
            &pool,
            &items,
            &materials,
            &material_states,
            &target_ratio,
            0.0, // 阈值为0
        );

        // 实际配比: Q345=0.601, Q235=0.399
        // 任何偏差都违规
        assert!(report.is_violated);
        assert!(report.deviation_ratio > 0.0);
    }

    #[test]
    fn test_scenario_13_capacity_full() {
        // 场景13: 产能满载违规
        let corrector = StructureCorrector::new();

        let pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            800.0,
            900.0,
            900.0, // 已满
        );

        let items = vec![
            create_test_plan_item(
                "item1",
                "v1",
                "M001",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                720.0,
                false,
            ),
            create_test_plan_item(
                "item2",
                "v1",
                "M002",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                180.0,
                false,
            ),
        ];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );
        materials.insert(
            "M002".to_string(),
            create_test_material_master("M002", Some("Q235")),
        );

        let material_states = HashMap::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.6);
        target_ratio.insert("Q235".to_string(), 0.4);

        let report = corrector.check_structure_violation(
            &pool,
            &items,
            &materials,
            &material_states,
            &target_ratio,
            0.15,
        );

        assert!(report.is_violated);
        assert!(report.suggestions.iter().any(|s| s.contains("产能已满")));
    }

    #[test]
    fn test_scenario_14_non_target_steel_grade() {
        // 场景14: 非目标钢种出现
        let corrector = StructureCorrector::new();

        let pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            800.0,
            900.0,
            700.0,
        );

        let items = vec![
            create_test_plan_item(
                "item1",
                "v1",
                "M001",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                400.0,
                false,
            ),
            create_test_plan_item(
                "item2",
                "v1",
                "M002",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                200.0,
                false,
            ),
            create_test_plan_item(
                "item3",
                "v1",
                "M003",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                100.0,
                false,
            ),
        ];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );
        materials.insert(
            "M002".to_string(),
            create_test_material_master("M002", Some("Q235")),
        );
        materials.insert(
            "M003".to_string(),
            create_test_material_master("M003", Some("Q390")),
        );

        let material_states = HashMap::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.6);
        target_ratio.insert("Q235".to_string(), 0.4);

        let report = corrector.check_structure_violation(
            &pool,
            &items,
            &materials,
            &material_states,
            &target_ratio,
            0.15,
        );

        // 实际配比: Q345=0.57, Q235=0.29, Q390=0.14 (非目标)
        // 最大偏差: 0.14 (未超阈值 0.15)
        assert!(!report.is_violated);
        assert!((report.deviation_ratio - 0.14).abs() < 0.01);
    }

    #[test]
    fn test_scenario_15_complex_multi_steel_grade() {
        // 场景15: 多钢种复杂场景
        let corrector = StructureCorrector::new();

        let pool = create_test_capacity_pool(
            "H033",
            NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
            1000.0,
            1200.0,
            1000.0,
        );

        let items = vec![
            create_test_plan_item(
                "item1",
                "v1",
                "M001",
                "H033",
                NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
                350.0,
                false,
            ),
            create_test_plan_item(
                "item2",
                "v1",
                "M002",
                "H033",
                NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
                200.0,
                false,
            ),
            create_test_plan_item(
                "item3",
                "v1",
                "M003",
                "H033",
                NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
                250.0,
                false,
            ),
            create_test_plan_item(
                "item4",
                "v1",
                "M004",
                "H033",
                NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
                100.0,
                false,
            ),
            create_test_plan_item(
                "item5",
                "v1",
                "M005",
                "H033",
                NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
                100.0,
                false,
            ),
        ];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );
        materials.insert(
            "M002".to_string(),
            create_test_material_master("M002", Some("Q235")),
        );
        materials.insert(
            "M003".to_string(),
            create_test_material_master("M003", Some("Q390")),
        );
        materials.insert(
            "M004".to_string(),
            create_test_material_master("M004", Some("Q420")),
        );
        materials.insert(
            "M005".to_string(),
            create_test_material_master("M005", Some("Q460")),
        );

        let material_states = HashMap::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.3);
        target_ratio.insert("Q235".to_string(), 0.25);
        target_ratio.insert("Q390".to_string(), 0.2);
        target_ratio.insert("Q420".to_string(), 0.15);
        target_ratio.insert("Q460".to_string(), 0.1);

        let report = corrector.check_structure_violation(
            &pool,
            &items,
            &materials,
            &material_states,
            &target_ratio,
            0.12,
        );

        // 实际配比: Q345=0.35, Q235=0.2, Q390=0.25, Q420=0.1, Q460=0.1
        // 最大偏差: 0.05 (未超阈值 0.12)
        assert!(!report.is_violated);
        assert!((report.deviation_ratio - 0.05).abs() < 0.01);
    }

    #[test]
    fn test_scenario_16_partial_locked_conflict() {
        // 场景16: 部分锁定冲突
        let corrector = StructureCorrector::new();

        let pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            800.0,
            900.0,
            800.0,
        );

        let items = vec![
            create_test_plan_item(
                "item1",
                "v1",
                "M001",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                500.0,
                false,
            ),
            create_test_plan_item(
                "item2",
                "v1",
                "M002",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                200.0,
                false,
            ),
            create_test_plan_item(
                "item3",
                "v1",
                "M003",
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                100.0,
                false,
            ),
        ];

        let mut materials = HashMap::new();
        materials.insert(
            "M001".to_string(),
            create_test_material_master("M001", Some("Q345")),
        );
        materials.insert(
            "M002".to_string(),
            create_test_material_master("M002", Some("Q235")),
        );
        materials.insert(
            "M003".to_string(),
            create_test_material_master("M003", Some("Q390")),
        );

        let mut material_states = HashMap::new();
        material_states.insert(
            "M001".to_string(),
            create_test_material_state("M001", SchedState::Locked, true),
        );
        material_states.insert(
            "M002".to_string(),
            create_test_material_state("M002", SchedState::Ready, false),
        );
        material_states.insert(
            "M003".to_string(),
            create_test_material_state("M003", SchedState::Ready, false),
        );

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.5);
        target_ratio.insert("Q235".to_string(), 0.3);
        target_ratio.insert("Q390".to_string(), 0.2);

        let report = corrector.check_structure_violation(
            &pool,
            &items,
            &materials,
            &material_states,
            &target_ratio,
            0.10,
        );

        // 实际配比: Q345=0.625, Q235=0.25, Q390=0.125
        // Q345 超标 12.5%, Q390 不足 7.5%
        assert!(report.is_violated);
        assert!(report
            .suggestions
            .iter()
            .any(|s| s.contains("【锁定冲突】") && s.contains("Q345")));
        assert!(report
            .suggestions
            .iter()
            .any(|s| s.contains("补充") && s.contains("Q390")));
    }

    // ==========================================
    // 配置验证测试
    // ==========================================

    #[test]
    fn test_validate_target_ratio_valid() {
        // 测试有效的目标配比
        let corrector = StructureCorrector::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.6);
        target_ratio.insert("Q235".to_string(), 0.4);

        let result = corrector.validate_target_ratio(&target_ratio);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_target_ratio_empty() {
        // 测试空配比
        let corrector = StructureCorrector::new();
        let target_ratio = HashMap::new();

        let result = corrector.validate_target_ratio(&target_ratio);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不能为空"));
    }

    #[test]
    fn test_validate_target_ratio_out_of_range() {
        // 测试配比值超出范围
        let corrector = StructureCorrector::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 1.5); // 超出范围
        target_ratio.insert("Q235".to_string(), -0.5); // 超出范围

        let result = corrector.validate_target_ratio(&target_ratio);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("超出有效范围"));
    }

    #[test]
    fn test_validate_target_ratio_sum_not_one() {
        // 测试配比之和不等于1
        let corrector = StructureCorrector::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), 0.5);
        target_ratio.insert("Q235".to_string(), 0.3); // 总和 0.8

        let result = corrector.validate_target_ratio(&target_ratio);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不等于 1.0"));
    }

    #[test]
    fn test_validate_target_ratio_nan() {
        // 测试 NaN 值
        let corrector = StructureCorrector::new();

        let mut target_ratio = HashMap::new();
        target_ratio.insert("Q345".to_string(), f64::NAN);

        let result = corrector.validate_target_ratio(&target_ratio);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("NaN"));
    }

    #[test]
    fn test_validate_deviation_threshold_valid() {
        // 测试有效的偏差阈值
        let corrector = StructureCorrector::new();

        assert!(corrector.validate_deviation_threshold(0.15).is_ok());
        assert!(corrector.validate_deviation_threshold(0.0).is_ok());
        assert!(corrector.validate_deviation_threshold(1.0).is_ok());
    }

    #[test]
    fn test_validate_deviation_threshold_out_of_range() {
        // 测试偏差阈值超出范围
        let corrector = StructureCorrector::new();

        let result = corrector.validate_deviation_threshold(1.5);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("超出有效范围"));

        let result = corrector.validate_deviation_threshold(-0.1);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("超出有效范围"));
    }

    #[test]
    fn test_validate_deviation_threshold_nan() {
        // 测试 NaN 阈值
        let corrector = StructureCorrector::new();

        let result = corrector.validate_deviation_threshold(f64::NAN);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("NaN"));
    }

    // ==========================================
    // 批量检查测试
    // ==========================================

    #[test]
    fn test_scenario_17_batch_multi_day() {
        // 场景17: 多日批量检查
        let corrector = StructureCorrector::new();

        let pools = vec![
            create_test_capacity_pool(
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                800.0,
                900.0,
                600.0,
            ),
            create_test_capacity_pool(
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 21).unwrap(),
                800.0,
                900.0,
                700.0,
            ),
            create_test_capacity_pool(
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 22).unwrap(),
                800.0,
                900.0,
                500.0,
            ),
        ];

        let items_by_date = HashMap::new();
        let materials = HashMap::new();
        let material_states = HashMap::new();
        let target_ratios = HashMap::new();

        let reports =
            corrector.check_batch(pools, items_by_date, &materials, &material_states, &target_ratios, 0.15);

        assert_eq!(reports.len(), 3);
        assert_eq!(
            reports[0].plan_date,
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()
        );
        assert_eq!(
            reports[1].plan_date,
            NaiveDate::from_ymd_opt(2026, 1, 21).unwrap()
        );
        assert_eq!(
            reports[2].plan_date,
            NaiveDate::from_ymd_opt(2026, 1, 22).unwrap()
        );
    }

    #[test]
    fn test_scenario_18_batch_multi_machine() {
        // 场景18: 多机组批量检查
        let corrector = StructureCorrector::new();

        let pools = vec![
            create_test_capacity_pool(
                "H032",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                800.0,
                900.0,
                600.0,
            ),
            create_test_capacity_pool(
                "H033",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                1000.0,
                1100.0,
                800.0,
            ),
            create_test_capacity_pool(
                "H034",
                NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                900.0,
                1000.0,
                700.0,
            ),
        ];

        let items_by_date = HashMap::new();
        let materials = HashMap::new();
        let material_states = HashMap::new();
        let target_ratios = HashMap::new();

        let reports =
            corrector.check_batch(pools, items_by_date, &materials, &material_states, &target_ratios, 0.15);

        assert_eq!(reports.len(), 3);
        assert_eq!(reports[0].machine_code, "H032");
        assert_eq!(reports[1].machine_code, "H033");
        assert_eq!(reports[2].machine_code, "H034");
    }
}

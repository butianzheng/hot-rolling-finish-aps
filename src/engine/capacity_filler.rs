// ==========================================
// 热轧精整排产系统 - 产能池填充引擎
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 5. Capacity Filler
// 红线: 产能约束优先于材料优先级
// ==========================================
// 职责: 吨位驱动的产能池填充
// 输入: 排序后材料列表 + 产能池 + 冻结区材料
// 输出: plan_item + 更新 material_state.sched_state=SCHEDULED
// ==========================================

use crate::domain::capacity::{CapacityConstraint, CapacityPool};
use crate::domain::material::{MaterialMaster, MaterialState};
use crate::domain::plan::PlanItem;
use crate::domain::types::{PathRuleStatus, SchedState};
use crate::engine::path_rule::{Anchor, PathRuleEngine};
use chrono::NaiveDate;
use tracing::instrument;

// ==========================================
// CapacityFiller - 产能池填充引擎
// ==========================================
pub struct CapacityFiller {
    // 无状态引擎，不需要注入依赖
}

/// 单日填充结果（用于承载路径规则锚点状态）
#[derive(Debug, Clone)]
pub struct FillSingleDayResult {
    pub plan_items: Vec<PlanItem>,
    pub skipped_materials: Vec<(MaterialMaster, MaterialState, String)>,
    pub path_override_pending: Vec<PathOverridePendingItem>,
    pub final_anchor: Option<Anchor>,
    pub final_anchor_material_id: Option<String>,
}

/// 路径规则：待人工确认记录（由 CapacityFiller 产生，供上层落库/汇总）
#[derive(Debug, Clone)]
pub struct PathOverridePendingItem {
    pub material_id: String,
    pub urgent_level: String,
    pub violation_type: String,
    pub width_mm: f64,
    pub thickness_mm: f64,
    pub anchor_width_mm: f64,
    pub anchor_thickness_mm: f64,
    pub width_delta_mm: f64,
    pub thickness_delta_mm: f64,
}

impl CapacityFiller {
    /// 构造函数
    ///
    /// # 返回
    /// 新的 CapacityFiller 实例
    pub fn new() -> Self {
        Self {}
    }

    // ==========================================
    // 核心方法
    // ==========================================

    /// 填充产能池（单日单机组）
    ///
    /// 规则（依据 Engine_Specs 5.3）：
    /// 1) 冻结区材料优先且不改变
    /// 2) 计算区填充至 target_capacity_t
    /// 3) 允许填充到 limit_capacity_t（需风险标记）
    /// 4) 允许结构跳过，但锁定材料不可跳过
    ///
    /// # 参数
    /// - `capacity_pool`: 产能池（会被修改）
    /// - `candidates`: 候选材料列表（已排序）
    /// - `frozen_items`: 冻结区材料（不可改变）
    /// - `version_id`: 方案版本ID
    ///
    /// # 返回
    /// (填充的 plan_item 列表, 被跳过的材料列表)
    #[instrument(skip(self, candidates, frozen_items), fields(
        machine_code = %capacity_pool.machine_code,
        plan_date = %capacity_pool.plan_date,
        candidates_count = candidates.len(),
        frozen_count = frozen_items.len()
    ))]
    pub fn fill_single_day(
        &self,
        capacity_pool: &mut CapacityPool,
        candidates: Vec<(MaterialMaster, MaterialState)>,
        frozen_items: Vec<PlanItem>,
        version_id: &str,
    ) -> (Vec<PlanItem>, Vec<(MaterialMaster, MaterialState, String)>) {
        let result = self.fill_single_day_with_path_rule(
            capacity_pool,
            &candidates,
            frozen_items,
            version_id,
            None,
            None,
            None,
        );
        (result.plan_items, result.skipped_materials)
    }

    /// 填充产能池（单日单机组）- 支持宽厚路径规则门控
    #[allow(clippy::too_many_arguments)]
    pub fn fill_single_day_with_path_rule(
        &self,
        capacity_pool: &mut CapacityPool,
        candidates: &[(MaterialMaster, MaterialState)],
        frozen_items: Vec<PlanItem>,
        version_id: &str,
        path_rule_engine: Option<&PathRuleEngine>,
        initial_anchor: Option<Anchor>,
        initial_anchor_material_id: Option<String>,
    ) -> FillSingleDayResult {
        let mut plan_items = Vec::new();
        let mut skipped_materials = Vec::new();
        let mut path_override_pending = Vec::new();

        // 当前锚点（跨材料更新）
        let mut current_anchor = initial_anchor;
        let mut current_anchor_material_id = initial_anchor_material_id;

        // 1. 先添加冻结区材料（冻结区吨位必须计入 used_capacity_t，否则会导致“计算区”在冻结项之上继续填充，出现单日超限）
        //
        // 说明：
        // - capacity_pool.used_capacity_t 可能被调用方用于表达“预占用吨位”（例如外部约束/历史残留）。
        // - frozen_capacity_t 专门承载冻结区吨位，便于前端解释与风险快照聚合。
        // - seq_no：冻结项保持不变；新排产从当日冻结项的最大 seq_no+1 开始，避免重复 seq_no。
        capacity_pool.frozen_capacity_t = 0.0;
        let mut max_frozen_seq_no: i32 = 0;
        for frozen_item in frozen_items {
            capacity_pool.used_capacity_t += frozen_item.weight_t;
            capacity_pool.frozen_capacity_t += frozen_item.weight_t;
            max_frozen_seq_no = max_frozen_seq_no.max(frozen_item.seq_no);
            plan_items.push(frozen_item);
        }
        let mut sequence_no = max_frozen_seq_no.saturating_add(1).max(1);

        // 2. 填充计算区材料
        let plan_date = capacity_pool.plan_date;
        for (master, state) in candidates.iter() {
            let weight = master.weight_t.unwrap_or(0.0);

            let candidate_width_mm = master.width_mm.unwrap_or(0.0);
            let candidate_thickness_mm = master.thickness_mm.unwrap_or(0.0);
            let dims_valid = candidate_width_mm.is_finite()
                && candidate_thickness_mm.is_finite()
                && candidate_width_mm > 0.0
                && candidate_thickness_mm > 0.0;

            // 0) 路径门控（锁定材料不可跳过）
            if state.sched_state != SchedState::Locked {
                if let (Some(engine), true) = (path_rule_engine, dims_valid) {
                    let check = engine.check(
                        candidate_width_mm,
                        candidate_thickness_mm,
                        state.urgent_level,
                        current_anchor.as_ref(),
                        state.user_confirmed,
                    );

                    match check.status {
                        PathRuleStatus::HardViolation => {
                            skipped_materials.push((
                                master.clone(),
                                state.clone(),
                                format!(
                                    "PATH_HARD_VIOLATION: violation={}, width_delta_mm={:.3}, thickness_delta_mm={:.3}",
                                    check
                                        .violation_type
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| "UNKNOWN".to_string()),
                                    check.width_delta_mm,
                                    check.thickness_delta_mm
                                ),
                            ));
                            continue;
                        }
                        PathRuleStatus::OverrideRequired => {
                            let (anchor_width_mm, anchor_thickness_mm) =
                                current_anchor.as_ref().map(|a| (a.width_mm, a.thickness_mm)).unwrap_or((0.0, 0.0));
                            path_override_pending.push(PathOverridePendingItem {
                                material_id: master.material_id.clone(),
                                urgent_level: state.urgent_level.to_string(),
                                violation_type: check
                                    .violation_type
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "UNKNOWN".to_string()),
                                width_mm: candidate_width_mm,
                                thickness_mm: candidate_thickness_mm,
                                anchor_width_mm,
                                anchor_thickness_mm,
                                width_delta_mm: check.width_delta_mm,
                                thickness_delta_mm: check.thickness_delta_mm,
                            });
                            skipped_materials.push((
                                master.clone(),
                                state.clone(),
                                format!(
                                    "PATH_OVERRIDE_REQUIRED: violation={}, width_delta_mm={:.3}, thickness_delta_mm={:.3}",
                                    check
                                        .violation_type
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| "UNKNOWN".to_string()),
                                    check.width_delta_mm,
                                    check.thickness_delta_mm
                                ),
                            ));
                            continue;
                        }
                        PathRuleStatus::Ok => {
                            // 继续执行产能门控
                        }
                    }
                }
            }

            // 检查锁定材料不可跳过（锁定材料优先处理）
            if state.sched_state == SchedState::Locked {
                // 锁定材料必须添加，即使超过 limit
                let plan_item = self.create_plan_item(
                    master,
                    state,
                    version_id,
                    plan_date,
                    sequence_no,
                    false,
                    "LOCKED_MATERIAL",
                );
                plan_items.push(plan_item);
                capacity_pool.used_capacity_t += weight;
                sequence_no += 1;

                // 入池后更新锚点（宽厚有效时）
                if dims_valid {
                    current_anchor = Some(Anchor {
                        width_mm: candidate_width_mm,
                        thickness_mm: candidate_thickness_mm,
                    });
                    current_anchor_material_id = Some(master.material_id.clone());
                }
                continue;
            }

            // 检查是否可以添加（普通材料）
            if !capacity_pool.can_add_material(weight) {
                // 超过 limit_capacity_t，跳过
                skipped_materials.push((
                    master.clone(),
                    state.clone(),
                    format!(
                        "CAPACITY_LIMIT_EXCEEDED: would exceed limit_capacity_t ({} + {} > {})",
                        capacity_pool.used_capacity_t, weight, capacity_pool.limit_capacity_t
                    ),
                ));
                continue;
            }

            // 普通材料：填充至 target，允许填充到 limit
            let assign_reason = if capacity_pool.used_capacity_t < capacity_pool.target_capacity_t {
                "FILL_TO_TARGET"
            } else if capacity_pool.used_capacity_t < capacity_pool.limit_capacity_t {
                "FILL_TO_LIMIT"
            } else {
                // 已达到 limit，跳过
                skipped_materials.push((
                    master.clone(),
                    state.clone(),
                    format!(
                        "TARGET_REACHED: capacity pool is full ({} >= {})",
                        capacity_pool.used_capacity_t, capacity_pool.limit_capacity_t
                    ),
                ));
                continue;
            };

            // 添加材料
            let plan_item = self.create_plan_item(
                master,
                state,
                version_id,
                plan_date,
                sequence_no,
                false,
                assign_reason,
            );
            plan_items.push(plan_item);
            capacity_pool.used_capacity_t += weight;
            sequence_no += 1;

            // 入池后更新锚点（宽厚有效时）
            if dims_valid {
                current_anchor = Some(Anchor {
                    width_mm: candidate_width_mm,
                    thickness_mm: candidate_thickness_mm,
                });
                current_anchor_material_id = Some(master.material_id.clone());
            }
        }

        // 3. 更新产能池的 overflow_t
        if capacity_pool.used_capacity_t > capacity_pool.limit_capacity_t {
            capacity_pool.overflow_t = capacity_pool.used_capacity_t - capacity_pool.limit_capacity_t;
        } else {
            capacity_pool.overflow_t = 0.0;
        }

        FillSingleDayResult {
            plan_items,
            skipped_materials,
            path_override_pending,
            final_anchor: current_anchor,
            final_anchor_material_id: current_anchor_material_id,
        }
    }

    // ==========================================
    // 辅助方法
    // ==========================================

    /// 创建 PlanItem
    fn create_plan_item(
        &self,
        master: &MaterialMaster,
        state: &MaterialState,
        version_id: &str,
        plan_date: NaiveDate,
        sequence_no: i32,
        is_frozen: bool,
        assign_reason: &str,
    ) -> PlanItem {
        PlanItem {
            version_id: version_id.to_string(),
            material_id: master.material_id.clone(),
            machine_code: master
                .current_machine_code
                .clone()
                .unwrap_or_else(|| "UNKNOWN".to_string()),
            // 排程落位的日期应由当前产能池/排程窗口决定，而不是依赖 material_state.scheduled_date（可能为空/历史值）。
            plan_date,
            seq_no: sequence_no,
            weight_t: master.weight_t.unwrap_or(0.0),
            source_type: if is_frozen { "FROZEN".to_string() } else { "CALC".to_string() },
            locked_in_plan: is_frozen,
            force_release_in_plan: state.force_release_flag,
            violation_flags: None,
            urgent_level: Some(state.urgent_level.to_string()),
            sched_state: Some(state.sched_state.to_string()),
            assign_reason: Some(assign_reason.to_string()),
            steel_grade: None,
            width_mm: master.width_mm,
            thickness_mm: master.thickness_mm,
        }
    }

    /// 生成填充原因的 JSON 字符串
    ///
    /// # 参数
    /// - `reason_type`: 原因类型
    /// - `capacity_pool`: 产能池
    /// - `weight_t`: 材料重量
    ///
    /// # 返回
    /// JSON 格式的填充原因字符串
    pub fn generate_fill_reason(
        &self,
        reason_type: &str,
        capacity_pool: &CapacityPool,
        weight_t: f64,
    ) -> String {
        format!(
            r#"{{"reason":"{}","used_capacity_t":{},"target_capacity_t":{},"limit_capacity_t":{},"material_weight_t":{},"remaining_capacity_t":{}}}"#,
            reason_type,
            capacity_pool.used_capacity_t,
            capacity_pool.target_capacity_t,
            capacity_pool.limit_capacity_t,
            weight_t,
            capacity_pool.remaining_capacity_t()
        )
    }
}

// ==========================================
// Default trait 实现
// ==========================================
impl Default for CapacityFiller {
    fn default() -> Self {
        Self::new()
    }
}

// ==========================================
// 测试模块
// ==========================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{RushLevel, UrgentLevel};
    use chrono::Utc;
    use crate::engine::path_rule::{PathRuleConfig, PathRuleEngine};

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

    /// 创建测试用的材料数据
    fn create_test_material(
        material_id: &str,
        machine_code: &str,
        sched_state: SchedState,
        weight_t: f64,
    ) -> (MaterialMaster, MaterialState) {
        let master = MaterialMaster {
            material_id: material_id.to_string(),
            manufacturing_order_id: None,
            material_status_code_src: None,
            steel_mark: None,
            slab_id: None,
            next_machine_code: None,
            rework_machine_code: None,
            current_machine_code: Some(machine_code.to_string()),
            width_mm: None,
            thickness_mm: None,
            length_m: None,
            weight_t: Some(weight_t),
            available_width_mm: None,
            due_date: None,
            stock_age_days: Some(10),
            output_age_days_raw: None,
            status_updated_at: None,
            contract_no: None,
            contract_nature: None,
            weekly_delivery_flag: None,
            export_flag: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let state = MaterialState {
            material_id: material_id.to_string(),
            sched_state,
            lock_flag: sched_state == SchedState::Locked,
            force_release_flag: sched_state == SchedState::ForceRelease,
            urgent_level: UrgentLevel::L0,
            urgent_reason: None,
            rush_level: RushLevel::L0,
            rolling_output_age_days: 5,
            ready_in_days: 0,
            earliest_sched_date: None,
            stock_age_days: 10,
            scheduled_date: Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()),
            scheduled_machine_code: Some(machine_code.to_string()),
            seq_no: None,
            manual_urgent_flag: false,
            user_confirmed: false,
            user_confirmed_at: None,
            user_confirmed_by: None,
            user_confirmed_reason: None,
            in_frozen_zone: false,
            last_calc_version_id: None,
            updated_at: Utc::now(),
            updated_by: None,
        };

        (master, state)
    }

    fn create_test_material_with_dims(
        material_id: &str,
        machine_code: &str,
        sched_state: SchedState,
        urgent_level: UrgentLevel,
        width_mm: f64,
        thickness_mm: f64,
        weight_t: f64,
        user_confirmed: bool,
    ) -> (MaterialMaster, MaterialState) {
        let (mut master, mut state) =
            create_test_material(material_id, machine_code, sched_state, weight_t);
        master.width_mm = Some(width_mm);
        master.thickness_mm = Some(thickness_mm);
        state.urgent_level = urgent_level;
        state.user_confirmed = user_confirmed;
        (master, state)
    }

    // ==========================================
    // 基础功能测试
    // ==========================================

    #[test]
    fn test_fill_empty_pool_to_target() {
        // 测试：空产能池填充到目标产能
        let filler = CapacityFiller::new();
        let mut pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0, // target
            1200.0, // limit
            0.0,    // used
        );

        let materials = vec![
            create_test_material("M001", "H032", SchedState::Ready, 300.0),
            create_test_material("M002", "H032", SchedState::Ready, 400.0),
            create_test_material("M003", "H032", SchedState::Ready, 200.0),
        ];

        let (plan_items, skipped) = filler.fill_single_day(
            &mut pool,
            materials,
            vec![],
            "version-001",
        );

        // 断言
        assert_eq!(plan_items.len(), 3); // 全部添加
        assert_eq!(skipped.len(), 0); // 无跳过
        assert_eq!(pool.used_capacity_t, 900.0); // 300 + 400 + 200
        assert_eq!(pool.overflow_t, 0.0); // 未超限
    }

    #[test]
    fn test_fill_to_limit() {
        // 测试：填充到上限产能
        let filler = CapacityFiller::new();
        let mut pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0, // target
            1200.0, // limit
            950.0,  // used (接近 target)
        );

        let materials = vec![
            create_test_material("M001", "H032", SchedState::Ready, 100.0), // 填充到 target
            create_test_material("M002", "H032", SchedState::Ready, 150.0), // 填充到 limit
        ];

        let (plan_items, skipped) = filler.fill_single_day(
            &mut pool,
            materials,
            vec![],
            "version-001",
        );

        // 断言
        assert_eq!(plan_items.len(), 2);
        assert_eq!(skipped.len(), 0);
        assert_eq!(pool.used_capacity_t, 1200.0); // 950 + 100 + 150
        assert_eq!(plan_items[0].assign_reason, Some("FILL_TO_TARGET".to_string()));
        assert_eq!(plan_items[1].assign_reason, Some("FILL_TO_LIMIT".to_string()));
    }

    #[test]
    fn test_skip_when_exceed_limit() {
        // 测试：超过上限时跳过材料
        let filler = CapacityFiller::new();
        let mut pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0, // target
            1200.0, // limit
            1150.0, // used (接近 limit)
        );

        let materials = vec![
            create_test_material("M001", "H032", SchedState::Ready, 30.0),  // 可添加
            create_test_material("M002", "H032", SchedState::Ready, 100.0), // 会超限，跳过
        ];

        let (plan_items, skipped) = filler.fill_single_day(
            &mut pool,
            materials,
            vec![],
            "version-001",
        );

        // 断言
        assert_eq!(plan_items.len(), 1); // 只添加 M001
        assert_eq!(skipped.len(), 1); // M002 被跳过
        assert_eq!(pool.used_capacity_t, 1180.0); // 1150 + 30
        assert_eq!(skipped[0].0.material_id, "M002");
        assert!(skipped[0].2.contains("CAPACITY_LIMIT_EXCEEDED"));
    }

    #[test]
    fn test_locked_material_must_add() {
        // 测试：锁定材料必须添加
        let filler = CapacityFiller::new();
        let mut pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0, // target
            1200.0, // limit
            1100.0, // used
        );

        let materials = vec![
            create_test_material("M001", "H032", SchedState::Locked, 50.0), // 锁定材料
            create_test_material("M002", "H032", SchedState::Ready, 100.0), // 普通材料，会超限
        ];

        let (plan_items, skipped) = filler.fill_single_day(
            &mut pool,
            materials,
            vec![],
            "version-001",
        );

        // 断言
        assert_eq!(plan_items.len(), 1); // 只添加锁定材料
        assert_eq!(skipped.len(), 1); // 普通材料被跳过
        assert_eq!(pool.used_capacity_t, 1150.0); // 1100 + 50
        assert_eq!(plan_items[0].material_id, "M001");
        assert_eq!(plan_items[0].assign_reason, Some("LOCKED_MATERIAL".to_string()));
    }

    #[test]
    fn test_frozen_items_preserved() {
        // 测试：冻结区材料保持不变
        let filler = CapacityFiller::new();
        let mut pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0, // target
            1200.0, // limit
            0.0,    // used（由 CapacityFiller 根据冻结区材料自动计入）
        );

        // 创建冻结区材料
        let frozen_items = vec![
            PlanItem {
                version_id: "version-001".to_string(),
                material_id: "F001".to_string(),
                machine_code: "H032".to_string(),
                plan_date: NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                seq_no: 1,
                weight_t: 500.0,
                source_type: "FROZEN".to_string(),
                locked_in_plan: true,
                force_release_in_plan: false,
                violation_flags: None,
                urgent_level: Some("L0".to_string()),
                sched_state: Some("Ready".to_string()),
                assign_reason: Some("FROZEN".to_string()),
                steel_grade: None,
                width_mm: None,
                thickness_mm: None,
            },
        ];

        let materials = vec![
            create_test_material("M001", "H032", SchedState::Ready, 300.0),
        ];

        let (plan_items, _skipped) = filler.fill_single_day(
            &mut pool,
            materials,
            frozen_items,
            "version-001",
        );

        // 断言
        assert_eq!(plan_items.len(), 2); // 1个冻结 + 1个新增
        assert_eq!(plan_items[0].material_id, "F001"); // 冻结区在前
        assert_eq!(plan_items[0].is_frozen(), true);
        assert_eq!(plan_items[1].material_id, "M001"); // 新增材料在后
        assert_eq!(pool.used_capacity_t, 800.0); // 500 + 300
        assert_eq!(pool.frozen_capacity_t, 500.0);
    }

    #[test]
    fn test_frozen_items_sequence_no_gap_should_start_after_max() {
        // 测试：冻结区 seq_no 存在空洞时，新排产 seq_no 从最大值+1 开始，避免重复
        let filler = CapacityFiller::new();
        let plan_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let mut pool = create_test_capacity_pool(
            "H032",
            plan_date,
            1000.0, // target
            1200.0, // limit
            0.0,    // used
        );

        let frozen_items = vec![
            PlanItem {
                version_id: "version-001".to_string(),
                material_id: "F005".to_string(),
                machine_code: "H032".to_string(),
                plan_date,
                seq_no: 5,
                weight_t: 100.0,
                source_type: "FROZEN".to_string(),
                locked_in_plan: true,
                force_release_in_plan: false,
                violation_flags: None,
                urgent_level: Some("L0".to_string()),
                sched_state: Some("Ready".to_string()),
                assign_reason: Some("FROZEN".to_string()),
                steel_grade: None,
                width_mm: None,
                thickness_mm: None,
            },
            PlanItem {
                version_id: "version-001".to_string(),
                material_id: "F007".to_string(),
                machine_code: "H032".to_string(),
                plan_date,
                seq_no: 7,
                weight_t: 100.0,
                source_type: "FROZEN".to_string(),
                locked_in_plan: true,
                force_release_in_plan: false,
                violation_flags: None,
                urgent_level: Some("L0".to_string()),
                sched_state: Some("Ready".to_string()),
                assign_reason: Some("FROZEN".to_string()),
                steel_grade: None,
                width_mm: None,
                thickness_mm: None,
            },
        ];

        let materials = vec![create_test_material("M001", "H032", SchedState::Ready, 50.0)];

        let (plan_items, _skipped) =
            filler.fill_single_day(&mut pool, materials, frozen_items, "version-001");

        assert_eq!(plan_items.len(), 3);
        assert_eq!(plan_items[0].material_id, "F005");
        assert_eq!(plan_items[0].seq_no, 5);
        assert_eq!(plan_items[1].material_id, "F007");
        assert_eq!(plan_items[1].seq_no, 7);
        assert_eq!(plan_items[2].material_id, "M001");
        assert_eq!(plan_items[2].seq_no, 8);
        assert_eq!(pool.used_capacity_t, 250.0);
        assert_eq!(pool.frozen_capacity_t, 200.0);
    }

    #[test]
    fn test_overflow_calculation() {
        // 测试：超限计算
        let filler = CapacityFiller::new();
        let mut pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0, // target
            1200.0, // limit
            1100.0, // used
        );

        let materials = vec![
            create_test_material("M001", "H032", SchedState::Locked, 150.0), // 锁定材料，会超限
        ];

        let (plan_items, _skipped) = filler.fill_single_day(
            &mut pool,
            materials,
            vec![],
            "version-001",
        );

        // 断言
        assert_eq!(plan_items.len(), 1);
        assert_eq!(pool.used_capacity_t, 1250.0); // 1100 + 150
        assert_eq!(pool.overflow_t, 50.0); // 1250 - 1200
    }

    // ==========================================
    // CapacityConstraint trait 测试
    // ==========================================

    #[test]
    fn test_can_add_material() {
        let pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0,
            1200.0,
            1100.0,
        );

        assert!(pool.can_add_material(50.0)); // 1100 + 50 <= 1200
        assert!(pool.can_add_material(100.0)); // 1100 + 100 = 1200
        assert!(!pool.can_add_material(150.0)); // 1100 + 150 > 1200
    }

    #[test]
    fn test_is_overflow() {
        let mut pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0,
            1200.0,
            1100.0,
        );

        assert!(!pool.is_overflow()); // 1100 <= 1200

        pool.used_capacity_t = 1250.0;
        assert!(pool.is_overflow()); // 1250 > 1200
    }

    #[test]
    fn test_remaining_capacity() {
        let pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0,
            1200.0,
            1100.0,
        );

        assert_eq!(pool.remaining_capacity_t(), 100.0); // 1200 - 1100
    }

    #[test]
    fn test_overflow_ratio() {
        let mut pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0,
            1200.0,
            1250.0,
        );

        let ratio = pool.overflow_ratio();
        assert!((ratio - 0.0417).abs() < 0.001); // (1250 - 1200) / 1200 ≈ 0.0417

        pool.used_capacity_t = 1100.0;
        assert_eq!(pool.overflow_ratio(), 0.0); // 未超限
    }

    #[test]
    fn test_path_rule_hard_violation_should_skip() {
        let filler = CapacityFiller::new();
        let mut pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0,
            1200.0,
            0.0,
        );

        let engine = PathRuleEngine::new(PathRuleConfig {
            enabled: true,
            width_tolerance_mm: 0.0,
            thickness_tolerance_mm: 0.0,
            override_allowed_urgency_levels: vec![UrgentLevel::L2, UrgentLevel::L3],
        });

        let candidates = vec![create_test_material_with_dims(
            "M001",
            "H032",
            SchedState::Ready,
            UrgentLevel::L1,
            1100.0, // anchor=1000 => violation
            9.0,
            100.0,
            false,
        )];

        let result = filler.fill_single_day_with_path_rule(
            &mut pool,
            &candidates,
            vec![],
            "version-001",
            Some(&engine),
            Some(Anchor {
                width_mm: 1000.0,
                thickness_mm: 10.0,
            }),
            Some("ANCHOR_M".to_string()),
        );

        assert_eq!(result.plan_items.len(), 0);
        assert_eq!(result.skipped_materials.len(), 1);
        assert!(result.skipped_materials[0].2.contains("PATH_HARD_VIOLATION"));
    }

    #[test]
    fn test_path_rule_override_required_should_skip() {
        let filler = CapacityFiller::new();
        let mut pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0,
            1200.0,
            0.0,
        );

        let engine = PathRuleEngine::new(PathRuleConfig {
            enabled: true,
            width_tolerance_mm: 0.0,
            thickness_tolerance_mm: 0.0,
            override_allowed_urgency_levels: vec![UrgentLevel::L2, UrgentLevel::L3],
        });

        let candidates = vec![create_test_material_with_dims(
            "M001",
            "H032",
            SchedState::Ready,
            UrgentLevel::L2,
            1100.0, // anchor=1000 => violation but override allowed
            9.0,
            100.0,
            false,
        )];

        let result = filler.fill_single_day_with_path_rule(
            &mut pool,
            &candidates,
            vec![],
            "version-001",
            Some(&engine),
            Some(Anchor {
                width_mm: 1000.0,
                thickness_mm: 10.0,
            }),
            Some("ANCHOR_M".to_string()),
        );

        assert_eq!(result.plan_items.len(), 0);
        assert_eq!(result.skipped_materials.len(), 1);
        assert!(result.skipped_materials[0].2.contains("PATH_OVERRIDE_REQUIRED"));
    }

    #[test]
    fn test_path_rule_user_confirmed_should_pass_and_update_anchor() {
        let filler = CapacityFiller::new();
        let mut pool = create_test_capacity_pool(
            "H032",
            NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
            1000.0,
            1200.0,
            0.0,
        );

        let engine = PathRuleEngine::new(PathRuleConfig {
            enabled: true,
            width_tolerance_mm: 0.0,
            thickness_tolerance_mm: 0.0,
            override_allowed_urgency_levels: vec![UrgentLevel::L2, UrgentLevel::L3],
        });

        let candidates = vec![create_test_material_with_dims(
            "M001",
            "H032",
            SchedState::Ready,
            UrgentLevel::L2,
            1100.0, // violates but user_confirmed=true => pass
            9.0,
            100.0,
            true,
        )];

        let result = filler.fill_single_day_with_path_rule(
            &mut pool,
            &candidates,
            vec![],
            "version-001",
            Some(&engine),
            Some(Anchor {
                width_mm: 1000.0,
                thickness_mm: 10.0,
            }),
            Some("ANCHOR_M".to_string()),
        );

        assert_eq!(result.plan_items.len(), 1);
        assert_eq!(result.skipped_materials.len(), 0);
        assert_eq!(
            result.final_anchor,
            Some(Anchor {
                width_mm: 1100.0,
                thickness_mm: 9.0,
            })
        );
        assert_eq!(result.final_anchor_material_id.as_deref(), Some("M001"));
    }
}

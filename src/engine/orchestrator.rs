// ==========================================
// 热轧精整排产系统 - 引擎编排器
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 1.1 计算主流程
// 用途: 协调五大核心引擎的执行顺序
// ==========================================

use crate::config::ImportConfigReader;
use crate::domain::capacity::CapacityPool;
use crate::domain::material::{MaterialMaster, MaterialState};
use crate::domain::plan::PlanItem;
use crate::domain::types::{SchedState, UrgentLevel};
use crate::engine::{
    Anchor, PathRuleEngine,
    CapacityFiller, EligibilityEngine, PrioritySorter, StructureCorrector,
    StructureViolationReport, UrgencyEngine,
};
use crate::engine::capacity_filler::PathOverridePendingItem;
use crate::config::strategy_profile::CustomStrategyParameters;
use crate::engine::strategy::ScheduleStrategy;
use chrono::NaiveDate;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use tracing::{debug, info};

// ==========================================
// ScheduleResult - 排产结果
// ==========================================

#[derive(Debug, Clone)]
pub struct ScheduleResult {
    // Eligibility 输出
    pub eligible_materials: Vec<(MaterialMaster, MaterialState)>,
    pub blocked_materials: Vec<(MaterialMaster, MaterialState, String)>,

    // Urgency 输出
    pub urgent_levels: HashMap<String, (UrgentLevel, String)>,

    // Priority 输出
    pub sorted_materials: Vec<(MaterialMaster, MaterialState)>,

    // Capacity Filler 输出
    pub plan_items: Vec<PlanItem>,
    pub skipped_materials: Vec<(MaterialMaster, MaterialState, String)>,
    pub path_override_pending: Vec<PathOverridePendingItem>,
    pub updated_capacity_pool: CapacityPool,

    // Path Rule / RollCycle 输出（锚点状态）
    pub roll_cycle_anchor: Option<Anchor>,
    pub roll_cycle_anchor_material_id: Option<String>,

    // Structure 输出
    pub structure_report: StructureViolationReport,
}

// ==========================================
// ScheduleOrchestrator - 引擎编排器
// ==========================================

pub struct ScheduleOrchestrator<C>
where
    C: ImportConfigReader,
{
    config: Arc<C>,
    eligibility: EligibilityEngine<C>,
    urgency: UrgencyEngine,
    sorter: PrioritySorter,
    filler: CapacityFiller,
    structure: StructureCorrector,
    strategy: ScheduleStrategy,
    strategy_params: Option<CustomStrategyParameters>,
}

impl<C> ScheduleOrchestrator<C>
where
    C: ImportConfigReader,
{
    /// 创建新的编排器实例
    ///
    /// # 参数
    /// - config: 配置读取器
    pub fn new(config: Arc<C>) -> Self {
        Self::new_with_strategy(config, ScheduleStrategy::Balanced)
    }

    pub fn new_with_strategy(config: Arc<C>, strategy: ScheduleStrategy) -> Self {
        Self {
            eligibility: EligibilityEngine::new(config.clone()),
            urgency: UrgencyEngine::new(),
            sorter: PrioritySorter::new(),
            filler: CapacityFiller::new(),
            structure: StructureCorrector::new(),
            config,
            strategy,
            strategy_params: None,
        }
    }

    pub fn new_with_strategy_parameters(
        config: Arc<C>,
        strategy: ScheduleStrategy,
        params: CustomStrategyParameters,
    ) -> Self {
        Self {
            eligibility: EligibilityEngine::new(config.clone()),
            urgency: UrgencyEngine::new(),
            sorter: PrioritySorter::new(),
            filler: CapacityFiller::new(),
            structure: StructureCorrector::new(),
            config,
            strategy,
            strategy_params: Some(params),
        }
    }

    /// 执行完整排产流程（单日单机组）
    ///
    /// # 参数
    /// - materials: 材料主数据列表
    /// - states: 材料状态列表
    /// - capacity_pool: 产能池（会被修改）
    /// - frozen_items: 冻结区材料
    /// - target_ratio: 目标钢种配比
    /// - deviation_threshold: 偏差阈值
    /// - today: 当前日期
    /// - version_id: 方案版本ID
    ///
    /// # 返回
    /// 排产结果
    pub async fn execute_single_day_schedule(
        &self,
        materials: Vec<MaterialMaster>,
        states: Vec<MaterialState>,
        capacity_pool: &mut CapacityPool,
        frozen_items: Vec<PlanItem>,
        target_ratio: &HashMap<String, f64>,
        deviation_threshold: f64,
        today: NaiveDate,
        version_id: &str,
    ) -> Result<ScheduleResult, Box<dyn Error>> {
        self.execute_single_day_schedule_with_path_rule(
            materials,
            states,
            capacity_pool,
            frozen_items,
            target_ratio,
            deviation_threshold,
            today,
            version_id,
            None,
            None,
            None,
        )
        .await
    }

    /// 执行完整排产流程（单日单机组）- 支持宽厚路径规则门控
    #[allow(clippy::too_many_arguments)]
    pub async fn execute_single_day_schedule_with_path_rule<'a>(
        &self,
        materials: Vec<MaterialMaster>,
        states: Vec<MaterialState>,
        capacity_pool: &mut CapacityPool,
        frozen_items: Vec<PlanItem>,
        target_ratio: &HashMap<String, f64>,
        deviation_threshold: f64,
        today: NaiveDate,
        version_id: &str,
        path_rule_engine: Option<&'a PathRuleEngine>,
        initial_anchor: Option<Anchor>,
        initial_anchor_material_id: Option<String>,
    ) -> Result<ScheduleResult, Box<dyn Error>> {
        info!(
            machine_code = %capacity_pool.machine_code,
            plan_date = %capacity_pool.plan_date,
            materials_count = materials.len(),
            frozen_items_count = frozen_items.len(),
            "开始执行排产流程"
        );

        // ==========================================
        // 步骤1: Eligibility Engine - 适温准入判定
        // ==========================================
        debug!("步骤1: 执行适温准入判定");

        let mut eligible_materials = Vec::new();
        let mut blocked_materials = Vec::new();

        for (material, state) in materials.into_iter().zip(states.into_iter()) {
            let (updated_state, reasons) = self
                .eligibility
                .evaluate_single(&material, &state, today)
                .await?;

            // 只有 READY, LOCKED, FORCE_RELEASE 状态的材料才能进入排产
            if updated_state.sched_state == SchedState::Ready
                || updated_state.sched_state == SchedState::Locked
                || updated_state.sched_state == SchedState::ForceRelease
            {
                eligible_materials.push((material, updated_state));
            } else {
                blocked_materials.push((material, updated_state, reasons.join("; ")));
            }
        }

        info!(
            eligible_count = eligible_materials.len(),
            blocked_count = blocked_materials.len(),
            "适温准入判定完成"
        );

        // ==========================================
        // 步骤2: Urgency Engine - 紧急等级判定
        // ==========================================
        debug!("步骤2: 执行紧急等级判定");

        let mut urgent_levels = HashMap::new();

        // 获取 N1/N2 阈值（从配置中读取）
        let n1_days = <C as ImportConfigReader>::get_n1_threshold_days(&*self.config).await?;
        let n2_days = <C as ImportConfigReader>::get_n2_threshold_days(&*self.config).await?;

        for (material, state) in &mut eligible_materials {
            // 计算催料等级
            let (rush_level, _rush_reason) = self.urgency.calculate_rush_level(
                material.contract_nature.as_deref(),
                material.weekly_delivery_flag.as_deref(),
                material.export_flag.as_deref(),
            );

            // 判定紧急等级
            let (level, reason) = self.urgency.determine_urgent_level(
                state,
                material,
                rush_level,
                today,
                n1_days,
                n2_days,
            );

            // 更新状态
            state.urgent_level = level;
            state.urgent_reason = Some(reason.clone());
            state.rush_level = rush_level;

            // 记录等级
            urgent_levels.insert(material.material_id.clone(), (level, reason));
        }

        info!(
            urgent_levels_count = urgent_levels.len(),
            "紧急等级判定完成"
        );

        // ==========================================
        // 步骤3: Priority Sorter - 等级内排序
        // ==========================================
        debug!("步骤3: 执行等级内排序");

        let sorted_materials = match self.strategy_params.as_ref() {
            Some(params) => self.sorter.sort_with_parameters(
                eligible_materials.clone(),
                self.strategy,
                params,
                today,
            ),
            None => self
                .sorter
                .sort_with_strategy(eligible_materials.clone(), self.strategy),
        };

        info!(
            sorted_count = sorted_materials.len(),
            "等级内排序完成"
        );

        // ==========================================
        // 步骤4: Capacity Filler - 产能池填充
        // ==========================================
        debug!("步骤4: 执行产能池填充");

        let fill_result = self.filler.fill_single_day_with_path_rule(
            capacity_pool,
            &sorted_materials,
            frozen_items,
            version_id,
            path_rule_engine,
            initial_anchor,
            initial_anchor_material_id,
        );
        let crate::engine::capacity_filler::FillSingleDayResult {
            plan_items,
            skipped_materials,
            path_override_pending,
            final_anchor,
            final_anchor_material_id,
        } = fill_result;

        info!(
            plan_items_count = plan_items.len(),
            skipped_count = skipped_materials.len(),
            used_capacity = capacity_pool.used_capacity_t,
            "产能池填充完成"
        );

        // ==========================================
        // 步骤5: Structure Corrector - 结构违规检查
        // ==========================================
        debug!("步骤5: 执行结构违规检查");

        // 构建材料和状态的 HashMap（仅包含 plan_items 中引用的 material_id，避免对全量候选做无效 clone）
        let mut materials_map: HashMap<String, MaterialMaster> = HashMap::new();
        let mut states_map: HashMap<String, MaterialState> = HashMap::new();
        if !plan_items.is_empty() {
            let ids: HashSet<&str> = plan_items.iter().map(|i| i.material_id.as_str()).collect();
            for (m, s) in &sorted_materials {
                if ids.contains(m.material_id.as_str()) {
                    materials_map.insert(m.material_id.clone(), m.clone());
                    states_map.insert(m.material_id.clone(), s.clone());
                }
            }
        }

        let structure_report = self.structure.check_structure_violation(
            capacity_pool,
            &plan_items,
            &materials_map,
            &states_map,
            target_ratio,
            deviation_threshold,
        );

        info!(
            is_violated = structure_report.is_violated,
            deviation_ratio = structure_report.deviation_ratio,
            "结构违规检查完成"
        );

        // ==========================================
        // 返回结果
        // ==========================================

        Ok(ScheduleResult {
            eligible_materials,
            blocked_materials,
            urgent_levels,
            sorted_materials,
            plan_items,
            skipped_materials,
            path_override_pending,
            updated_capacity_pool: capacity_pool.clone(),
            roll_cycle_anchor: final_anchor,
            roll_cycle_anchor_material_id: final_anchor_material_id,
            structure_report,
        })
    }
}

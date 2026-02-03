use super::scoring::compute_param_score;
use crate::config::strategy_profile::CustomStrategyParameters;
use crate::domain::material::{MaterialMaster, MaterialState};
use crate::domain::types::{SchedState, UrgentLevel};
use crate::engine::strategy::ScheduleStrategy;
use chrono::NaiveDate;
use std::cmp::Ordering;
use std::collections::HashMap;

// ==========================================
// PrioritySorter - 等级内排序引擎
// ==========================================
pub struct PrioritySorter {
    // 无状态引擎,不需要注入依赖
}

impl PrioritySorter {
    /// 构造函数
    ///
    /// # 返回
    /// 新的 PrioritySorter 实例
    pub fn new() -> Self {
        Self {}
    }

    // ==========================================
    // 核心方法
    // ==========================================

    /// 排序材料列表
    ///
    /// 排序键 (依据 Engine_Specs 4):
    /// 1) FORCE_RELEASE 优先
    /// 2) LOCKED 优先
    /// 3) stock_age_days 降序 (冷料优先)
    /// 4) rolling_output_age_days 降序
    /// 5) due_date 升序 (早交期优先)
    ///
    /// # 参数
    /// - `materials`: 待排序的材料列表
    ///
    /// # 返回
    /// 排序后的材料列表（按优先级从高到低）
    pub fn sort(
        &self,
        mut materials: Vec<(MaterialMaster, MaterialState)>,
    ) -> Vec<(MaterialMaster, MaterialState)> {
        materials.sort_by(|a, b| self.compare(a, b));
        materials
    }

    /// 按指定策略排序（用于策略草案/多策略对比）
    pub fn sort_with_strategy(
        &self,
        mut materials: Vec<(MaterialMaster, MaterialState)>,
        strategy: ScheduleStrategy,
    ) -> Vec<(MaterialMaster, MaterialState)> {
        if strategy == ScheduleStrategy::Balanced {
            // 保持与现有行为一致，避免无意改变默认排程结果
            materials.sort_by(|a, b| self.compare(a, b));
            return materials;
        }

        materials.sort_by(|a, b| self.compare_with_strategy(a, b, strategy));
        materials
    }

    /// 按“参数化策略”排序（用于自定义策略）
    ///
    /// 说明：
    /// - FORCE_RELEASE / LOCKED 仍保持最高优先级（红线：锁定/强制放行不可被“策略”覆盖）。
    /// - 若参数均为空（未设置），则退化为 `sort_with_strategy`（避免行为漂移）。
    pub fn sort_with_parameters(
        &self,
        mut materials: Vec<(MaterialMaster, MaterialState)>,
        base_strategy: ScheduleStrategy,
        params: &CustomStrategyParameters,
        today: NaiveDate,
    ) -> Vec<(MaterialMaster, MaterialState)> {
        let has_any = params.urgent_weight.is_some()
            || params.capacity_weight.is_some()
            || params.cold_stock_weight.is_some()
            || params.due_date_weight.is_some()
            || params.rolling_output_age_weight.is_some()
            || params.cold_stock_age_threshold_days.is_some();

        if !has_any {
            return self.sort_with_strategy(materials, base_strategy);
        }

        // 预计算 score，避免 sort_by 中重复计算。
        let mut score_by_id: HashMap<String, f64> = HashMap::with_capacity(materials.len());
        for (master, state) in &materials {
            let score = compute_param_score(master, state, params, today);
            score_by_id.insert(master.material_id.clone(), score);
        }

        materials.sort_by(|a, b| {
            let (master_a, state_a) = a;
            let (master_b, state_b) = b;

            if let Some(ord) = self.compare_sched_state(state_a.sched_state, state_b.sched_state)
            {
                return ord;
            }

            let sa = score_by_id.get(&master_a.material_id).copied().unwrap_or(0.0);
            let sb = score_by_id.get(&master_b.material_id).copied().unwrap_or(0.0);

            // 分数高者优先
            match sb.total_cmp(&sa) {
                Ordering::Equal => {
                    // tie-break：回落到基于预设策略的稳定排序（可解释性更强，且避免不稳定）。
                    self.compare_with_strategy(a, b, base_strategy)
                }
                other => other,
            }
        });

        materials
    }

    /// 按机组分组排序
    ///
    /// # 参数
    /// - `materials`: 待排序的材料列表
    ///
    /// # 返回
    /// HashMap<机组代码, 排序后的材料列表>
    pub fn sort_by_machine(
        &self,
        materials: Vec<(MaterialMaster, MaterialState)>,
    ) -> HashMap<String, Vec<(MaterialMaster, MaterialState)>> {
        let mut grouped: HashMap<String, Vec<(MaterialMaster, MaterialState)>> = HashMap::new();

        // 按机组分组
        for material in materials {
            let machine_code = material
                .0
                .current_machine_code
                .clone()
                .unwrap_or_else(|| "UNKNOWN".to_string());
            grouped
                .entry(machine_code)
                .or_insert_with(Vec::new)
                .push(material);
        }

        // 对每组排序
        for materials in grouped.values_mut() {
            materials.sort_by(|a, b| self.compare(a, b));
        }

        grouped
    }

    /// 按机组+紧急等级分组排序
    ///
    /// # 参数
    /// - `materials`: 待排序的材料列表
    ///
    /// # 返回
    /// HashMap<(机组代码, 紧急等级), 排序后的材料列表>
    pub fn sort_by_machine_and_urgent_level(
        &self,
        materials: Vec<(MaterialMaster, MaterialState)>,
    ) -> HashMap<(String, UrgentLevel), Vec<(MaterialMaster, MaterialState)>> {
        let mut grouped: HashMap<(String, UrgentLevel), Vec<(MaterialMaster, MaterialState)>> =
            HashMap::new();

        // 按机组+紧急等级分组
        for material in materials {
            let machine_code = material
                .0
                .current_machine_code
                .clone()
                .unwrap_or_else(|| "UNKNOWN".to_string());
            let urgent_level = material.1.urgent_level;
            let key = (machine_code, urgent_level);
            grouped.entry(key).or_insert_with(Vec::new).push(material);
        }

        // 对每组排序
        for materials in grouped.values_mut() {
            materials.sort_by(|a, b| self.compare(a, b));
        }

        grouped
    }

    // ==========================================
    // 比较方法
    // ==========================================

    /// 比较两个材料的优先级
    ///
    /// 按5键排序规则依次比较：
    /// 1. FORCE_RELEASE 优先
    /// 2. LOCKED 优先
    /// 3. stock_age_days 降序
    /// 4. rolling_output_age_days 降序
    /// 5. due_date 升序
    ///
    /// # 参数
    /// - `a`: 材料A
    /// - `b`: 材料B
    ///
    /// # 返回
    /// Ordering::Less 表示 a 优先于 b
    fn compare(
        &self,
        a: &(MaterialMaster, MaterialState),
        b: &(MaterialMaster, MaterialState),
    ) -> Ordering {
        let (master_a, state_a) = a;
        let (master_b, state_b) = b;

        // 1. 比较状态优先级 (FORCE_RELEASE > LOCKED)
        if let Some(ord) = self.compare_sched_state(state_a.sched_state, state_b.sched_state) {
            return ord;
        }

        // 2. 比较 stock_age_days (降序，越大越优先)
        match state_b.stock_age_days.cmp(&state_a.stock_age_days) {
            Ordering::Equal => {}
            other => return other,
        }

        // 3. 比较 rolling_output_age_days (降序，越大越优先)
        match state_b
            .rolling_output_age_days
            .cmp(&state_a.rolling_output_age_days)
        {
            Ordering::Equal => {}
            other => return other,
        }

        // 4. 比较 due_date (升序，越早越优先)
        let due_a = master_a.due_date.unwrap_or(NaiveDate::MAX);
        let due_b = master_b.due_date.unwrap_or(NaiveDate::MAX);
        due_a.cmp(&due_b)
    }

    fn compare_with_strategy(
        &self,
        a: &(MaterialMaster, MaterialState),
        b: &(MaterialMaster, MaterialState),
        strategy: ScheduleStrategy,
    ) -> Ordering {
        let (master_a, state_a) = a;
        let (master_b, state_b) = b;

        // 0. 共享最高优先级：FORCE_RELEASE / LOCKED
        if let Some(ord) = self.compare_sched_state(state_a.sched_state, state_b.sched_state) {
            return ord;
        }

        match strategy {
            ScheduleStrategy::Balanced => self.compare(a, b),
            ScheduleStrategy::UrgentFirst => {
                // 1) urgent_level 降序（L3 > L2 > L1 > L0）
                match state_b.urgent_level.cmp(&state_a.urgent_level) {
                    Ordering::Equal => {}
                    other => return other,
                }

                // 2) due_date 升序（早交期优先）
                let due_a = master_a.due_date.unwrap_or(NaiveDate::MAX);
                let due_b = master_b.due_date.unwrap_or(NaiveDate::MAX);
                match due_a.cmp(&due_b) {
                    Ordering::Equal => {}
                    other => return other,
                }

                // 3) stock_age_days 降序（冷料优先）
                match state_b.stock_age_days.cmp(&state_a.stock_age_days) {
                    Ordering::Equal => {}
                    other => return other,
                }

                // 4) rolling_output_age_days 降序
                match state_b.rolling_output_age_days.cmp(&state_a.rolling_output_age_days) {
                    Ordering::Equal => {}
                    other => return other,
                }

                Ordering::Equal
            }
            ScheduleStrategy::CapacityFirst => {
                // 1) weight_t 降序（尽快填满产能）
                let wa = master_a.weight_t.unwrap_or(0.0);
                let wb = master_b.weight_t.unwrap_or(0.0);
                let wa = if wa.is_finite() { wa } else { 0.0 };
                let wb = if wb.is_finite() { wb } else { 0.0 };
                match wb.total_cmp(&wa) {
                    Ordering::Equal => {}
                    other => return other,
                }

                // 2) due_date 升序（兜底）
                let due_a = master_a.due_date.unwrap_or(NaiveDate::MAX);
                let due_b = master_b.due_date.unwrap_or(NaiveDate::MAX);
                match due_a.cmp(&due_b) {
                    Ordering::Equal => {}
                    other => return other,
                }

                // 3) stock_age_days 降序
                match state_b.stock_age_days.cmp(&state_a.stock_age_days) {
                    Ordering::Equal => {}
                    other => return other,
                }

                Ordering::Equal
            }
            ScheduleStrategy::ColdStockFirst => {
                // 1) stock_age_days 降序（冷坨优先）
                match state_b.stock_age_days.cmp(&state_a.stock_age_days) {
                    Ordering::Equal => {}
                    other => return other,
                }

                // 2) rolling_output_age_days 降序（更“老”的优先）
                match state_b.rolling_output_age_days.cmp(&state_a.rolling_output_age_days) {
                    Ordering::Equal => {}
                    other => return other,
                }

                // 3) due_date 升序（兜底）
                let due_a = master_a.due_date.unwrap_or(NaiveDate::MAX);
                let due_b = master_b.due_date.unwrap_or(NaiveDate::MAX);
                due_a.cmp(&due_b)
            }
        }
    }

    /// 比较状态优先级
    ///
    /// # 参数
    /// - `a`: 状态A
    /// - `b`: 状态B
    ///
    /// # 返回
    /// - `Some(Ordering::Less)`: a 优先于 b
    /// - `Some(Ordering::Greater)`: b 优先于 a
    /// - `None`: 状态相同或不影响排序，继续比较下一个键
    fn compare_sched_state(&self, a: SchedState, b: SchedState) -> Option<Ordering> {
        match (a, b) {
            (SchedState::ForceRelease, SchedState::ForceRelease) => None,
            (SchedState::ForceRelease, _) => Some(Ordering::Less),
            (_, SchedState::ForceRelease) => Some(Ordering::Greater),

            (SchedState::Locked, SchedState::Locked) => None,
            (SchedState::Locked, _) => Some(Ordering::Less),
            (_, SchedState::Locked) => Some(Ordering::Greater),

            // 其他状态不影响排序
            _ => None,
        }
    }

    /// 生成排序原因 (可解释性)
    ///
    /// # 参数
    /// - `state`: 材料状态
    ///
    /// # 返回
    /// JSON 格式的排序原因字符串
    pub fn generate_sort_reason(&self, state: &MaterialState, master: &MaterialMaster) -> String {
        let primary_factor = match state.sched_state {
            SchedState::ForceRelease => "FORCE_RELEASE",
            SchedState::Locked => "LOCKED",
            _ => {
                if state.stock_age_days > 0 {
                    "STOCK_AGE"
                } else if state.rolling_output_age_days > 0 {
                    "ROLLING_OUTPUT_AGE"
                } else {
                    "DUE_DATE"
                }
            }
        };

        format!(
            r#"{{"sort_keys":{{"sched_state":"{}","stock_age_days":{},"rolling_output_age_days":{},"due_date":"{}"}},"primary_factor":"{}"}}"#,
            state.sched_state.to_string(),
            state.stock_age_days,
            state.rolling_output_age_days,
            master
                .due_date
                .map(|d| d.to_string())
                .unwrap_or_else(|| "null".to_string()),
            primary_factor
        )
    }
}

// ==========================================
// Default trait 实现
// ==========================================
impl Default for PrioritySorter {
    fn default() -> Self {
        Self::new()
    }
}


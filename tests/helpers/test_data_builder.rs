// ==========================================
// 测试数据构建器 - 用于集成测试
// ==========================================

use chrono::{NaiveDate, Utc};
use hot_rolling_aps::domain::capacity::CapacityPool;
use hot_rolling_aps::domain::material::{MaterialMaster, MaterialState};
use hot_rolling_aps::domain::plan::PlanItem;
use hot_rolling_aps::domain::types::{RushLevel, SchedState, UrgentLevel};

// ==========================================
// MaterialMaster 构建器
// ==========================================

pub struct MaterialBuilder {
    material_id: String,
    steel_mark: Option<String>,
    weight_t: Option<f64>,
    output_age_days_raw: Option<i32>,
    current_machine_code: Option<String>,
    due_date: Option<NaiveDate>,
    contract_nature: Option<String>,
    weekly_delivery_flag: Option<String>,
    export_flag: Option<String>,
}

impl MaterialBuilder {
    pub fn new(material_id: &str) -> Self {
        Self {
            material_id: material_id.to_string(),
            steel_mark: None,
            weight_t: None,
            output_age_days_raw: None,
            current_machine_code: None,
            due_date: None,
            contract_nature: None,
            weekly_delivery_flag: None,
            export_flag: None,
        }
    }

    pub fn steel_mark(mut self, mark: &str) -> Self {
        self.steel_mark = Some(mark.to_string());
        self
    }

    pub fn weight(mut self, weight: f64) -> Self {
        self.weight_t = Some(weight);
        self
    }

    pub fn output_age_days(mut self, days: i32) -> Self {
        self.output_age_days_raw = Some(days);
        self
    }

    pub fn machine(mut self, machine: &str) -> Self {
        self.current_machine_code = Some(machine.to_string());
        self
    }

    pub fn due_date(mut self, date: NaiveDate) -> Self {
        self.due_date = Some(date);
        self
    }

    pub fn contract_nature(mut self, nature: &str) -> Self {
        self.contract_nature = Some(nature.to_string());
        self
    }

    pub fn weekly_delivery_flag(mut self, flag: &str) -> Self {
        self.weekly_delivery_flag = Some(flag.to_string());
        self
    }

    pub fn export_flag(mut self, flag: &str) -> Self {
        self.export_flag = Some(flag.to_string());
        self
    }

    pub fn build(self) -> MaterialMaster {
        MaterialMaster {
            material_id: self.material_id,
            steel_mark: self.steel_mark,
            weight_t: self.weight_t,
            output_age_days_raw: self.output_age_days_raw,
            current_machine_code: self.current_machine_code,
            due_date: self.due_date,
            contract_nature: self.contract_nature,
            weekly_delivery_flag: self.weekly_delivery_flag,
            export_flag: self.export_flag,
            manufacturing_order_id: None,
            material_status_code_src: None,
            slab_id: None,
            next_machine_code: None,
            rework_machine_code: None,
            width_mm: None,
            thickness_mm: None,
            length_m: None,
            available_width_mm: None,
            stock_age_days: None,
            status_updated_at: None,
            contract_no: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

// ==========================================
// MaterialState 构建器
// ==========================================

pub struct MaterialStateBuilder {
    material_id: String,
    sched_state: SchedState,
    lock_flag: bool,
    force_release_flag: bool,
    stock_age_days: i32,
    urgent_level: UrgentLevel,
    urgent_reason: Option<String>,
    rush_level: RushLevel,
    rolling_output_age_days: i32,
    ready_in_days: i32,
    earliest_sched_date: Option<NaiveDate>,
    scheduled_date: Option<NaiveDate>,
    scheduled_machine_code: Option<String>,
    seq_no: Option<i32>,
    manual_urgent_flag: bool,
    in_frozen_zone: bool,
    last_calc_version_id: Option<String>,
    updated_by: Option<String>,
}

impl MaterialStateBuilder {
    pub fn new(material_id: &str) -> Self {
        Self {
            material_id: material_id.to_string(),
            sched_state: SchedState::Ready,
            lock_flag: false,
            force_release_flag: false,
            stock_age_days: 0,
            urgent_level: UrgentLevel::L0,
            urgent_reason: None,
            rush_level: RushLevel::L0,
            rolling_output_age_days: 0,
            ready_in_days: 0,
            earliest_sched_date: None,
            scheduled_date: None,
            scheduled_machine_code: None,
            seq_no: None,
            manual_urgent_flag: false,
            in_frozen_zone: false,
            last_calc_version_id: None,
            updated_by: None,
        }
    }

    pub fn sched_state(mut self, state: SchedState) -> Self {
        self.sched_state = state;
        self
    }

    pub fn locked(mut self) -> Self {
        self.lock_flag = true;
        self.sched_state = SchedState::Locked;
        self
    }

    pub fn force_release(mut self) -> Self {
        self.force_release_flag = true;
        self.sched_state = SchedState::ForceRelease;
        self
    }

    pub fn stock_age_days(mut self, days: i32) -> Self {
        self.stock_age_days = days;
        self
    }

    pub fn urgent_level(mut self, level: UrgentLevel) -> Self {
        self.urgent_level = level;
        self
    }

    pub fn rush_level(mut self, level: RushLevel) -> Self {
        self.rush_level = level;
        self
    }

    pub fn build(self) -> MaterialState {
        MaterialState {
            material_id: self.material_id,
            sched_state: self.sched_state,
            lock_flag: self.lock_flag,
            force_release_flag: self.force_release_flag,
            stock_age_days: self.stock_age_days,
            urgent_level: self.urgent_level,
            urgent_reason: self.urgent_reason,
            rush_level: self.rush_level,
            rolling_output_age_days: self.rolling_output_age_days,
            ready_in_days: self.ready_in_days,
            earliest_sched_date: self.earliest_sched_date,
            scheduled_date: self.scheduled_date,
            scheduled_machine_code: self.scheduled_machine_code,
            seq_no: self.seq_no,
            manual_urgent_flag: self.manual_urgent_flag,
            in_frozen_zone: self.in_frozen_zone,
            last_calc_version_id: self.last_calc_version_id,
            updated_at: Utc::now(),
            updated_by: self.updated_by,
        }
    }
}

// ==========================================
// CapacityPool 构建器
// ==========================================

pub struct CapacityPoolBuilder {
    version_id: String,
    machine_code: String,
    plan_date: NaiveDate,
    target_capacity_t: f64,
    limit_capacity_t: f64,
    used_capacity_t: f64,
    overflow_t: f64,
    frozen_capacity_t: f64,
    accumulated_tonnage_t: f64,
    roll_campaign_id: Option<String>,
}

impl CapacityPoolBuilder {
    pub fn new(machine_code: &str, plan_date: NaiveDate) -> Self {
        Self {
            version_id: "v1".to_string(),
            machine_code: machine_code.to_string(),
            plan_date,
            target_capacity_t: 800.0,
            limit_capacity_t: 900.0,
            used_capacity_t: 0.0,
            overflow_t: 0.0,
            frozen_capacity_t: 0.0,
            accumulated_tonnage_t: 0.0,
            roll_campaign_id: None,
        }
    }

    pub fn version_id(mut self, version_id: &str) -> Self {
        self.version_id = version_id.to_string();
        self
    }

    pub fn target(mut self, target: f64) -> Self {
        self.target_capacity_t = target;
        self
    }

    pub fn limit(mut self, limit: f64) -> Self {
        self.limit_capacity_t = limit;
        self
    }

    pub fn used(mut self, used: f64) -> Self {
        self.used_capacity_t = used;
        self
    }

    pub fn frozen(mut self, frozen: f64) -> Self {
        self.frozen_capacity_t = frozen;
        self
    }

    pub fn build(self) -> CapacityPool {
        CapacityPool {
            version_id: self.version_id,
            machine_code: self.machine_code,
            plan_date: self.plan_date,
            target_capacity_t: self.target_capacity_t,
            limit_capacity_t: self.limit_capacity_t,
            used_capacity_t: self.used_capacity_t,
            overflow_t: self.overflow_t,
            frozen_capacity_t: self.frozen_capacity_t,
            accumulated_tonnage_t: self.accumulated_tonnage_t,
            roll_campaign_id: self.roll_campaign_id,
        }
    }
}

// ==========================================
// PlanItem 构建器
// ==========================================

pub struct PlanItemBuilder {
    version_id: String,
    material_id: String,
    machine_code: String,
    plan_date: NaiveDate,
    seq_no: i32,
    weight_t: f64,
    source_type: String,
    locked_in_plan: bool,
    force_release_in_plan: bool,
    violation_flags: Option<String>,
    urgent_level: Option<String>,
    sched_state: Option<String>,
    assign_reason: Option<String>,
    steel_grade: Option<String>,
}

impl PlanItemBuilder {
    pub fn new(
        version_id: &str,
        material_id: &str,
        machine_code: &str,
        plan_date: NaiveDate,
    ) -> Self {
        Self {
            version_id: version_id.to_string(),
            material_id: material_id.to_string(),
            machine_code: machine_code.to_string(),
            plan_date,
            seq_no: 1,
            weight_t: 0.0,
            source_type: "CALC".to_string(),
            locked_in_plan: false,
            force_release_in_plan: false,
            violation_flags: None,
            urgent_level: Some("L0".to_string()),
            sched_state: Some("READY".to_string()),
            assign_reason: Some("TEST".to_string()),
            steel_grade: None,
        }
    }

    pub fn seq_no(mut self, seq: i32) -> Self {
        self.seq_no = seq;
        self
    }

    pub fn weight(mut self, weight: f64) -> Self {
        self.weight_t = weight;
        self
    }

    pub fn frozen(mut self) -> Self {
        self.locked_in_plan = true;
        self.source_type = "FROZEN".to_string();
        self
    }

    pub fn source_type(mut self, source_type: &str) -> Self {
        self.source_type = source_type.to_string();
        self
    }

    pub fn urgent_level(mut self, level: &str) -> Self {
        self.urgent_level = Some(level.to_string());
        self
    }

    pub fn build(self) -> PlanItem {
        PlanItem {
            version_id: self.version_id,
            material_id: self.material_id,
            machine_code: self.machine_code,
            plan_date: self.plan_date,
            seq_no: self.seq_no,
            weight_t: self.weight_t,
            source_type: self.source_type,
            locked_in_plan: self.locked_in_plan,
            force_release_in_plan: self.force_release_in_plan,
            violation_flags: self.violation_flags,
            urgent_level: self.urgent_level,
            sched_state: self.sched_state,
            assign_reason: self.assign_reason,
            steel_grade: None,
        }
    }
}

// ==========================================
// 便捷函数
// ==========================================

/// 创建测试用的产能池
pub fn create_capacity_pool(
    machine_code: &str,
    plan_date: NaiveDate,
    target: f64,
    limit: f64,
    used: f64,
) -> CapacityPool {
    CapacityPoolBuilder::new(machine_code, plan_date)
        .target(target)
        .limit(limit)
        .used(used)
        .build()
}

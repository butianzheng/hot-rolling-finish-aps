use crate::config::strategy_profile::CustomStrategyParameters;
use crate::domain::material::{MaterialMaster, MaterialState};
use crate::domain::types::UrgentLevel;
use chrono::NaiveDate;

pub(super) fn compute_param_score(
    master: &MaterialMaster,
    state: &MaterialState,
    params: &CustomStrategyParameters,
    today: NaiveDate,
) -> f64 {
    let urgent_w = params.urgent_weight.unwrap_or(0.0);
    let capacity_w = params.capacity_weight.unwrap_or(0.0);
    let cold_w = params.cold_stock_weight.unwrap_or(0.0);
    let due_w = params.due_date_weight.unwrap_or(0.0);
    let roll_age_w = params.rolling_output_age_weight.unwrap_or(0.0);

    let urgent_rank = match state.urgent_level {
        UrgentLevel::L0 => 0.0,
        UrgentLevel::L1 => 1.0,
        UrgentLevel::L2 => 2.0,
        UrgentLevel::L3 => 3.0,
    };

    let weight_t = master.weight_t.unwrap_or(0.0);
    let weight_t = if weight_t.is_finite() { weight_t } else { 0.0 };

    let mut cold_age = state.stock_age_days.max(0) as f64;
    if let Some(threshold) = params.cold_stock_age_threshold_days {
        if state.stock_age_days < threshold {
            cold_age = 0.0;
        }
    }

    let roll_age = state.rolling_output_age_days.max(0) as f64;

    // due_urgency 越大越紧急：
    // - overdue: days_to_due < 0 => due_urgency > 0
    // - far future: days_to_due > 0 => due_urgency < 0
    let due_date = master.due_date.unwrap_or(NaiveDate::MAX);
    let mut days_to_due = (due_date - today).num_days();
    // clamp，避免 due_date 缺失被 NaiveDate::MAX 放大到极端
    if days_to_due > 3650 {
        days_to_due = 3650;
    } else if days_to_due < -3650 {
        days_to_due = -3650;
    }
    let due_urgency = -(days_to_due as f64);

    urgent_w * urgent_rank
        + capacity_w * weight_t
        + cold_w * cold_age
        + due_w * due_urgency
        + roll_age_w * roll_age
}

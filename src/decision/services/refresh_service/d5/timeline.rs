use super::campaign_state::CampaignStreamState;
use chrono::NaiveDateTime;

/// 计划项精简结构（用于时间线仿真）
///
/// # 职责
/// - 存储时间线仿真所需的最小计划项信息
/// - 包含：最早开始时间、重量
#[derive(Debug, Clone)]
pub(super) struct PlanItemLite {
    /// 最早开始时间
    pub earliest_start_at: NaiveDateTime,
    /// 重量（吨）
    pub weight_t: f64,
}

/// 按指定重量推进时间线
///
/// # 参数
/// - `items`: 计划项列表
/// - `state`: 当前状态
/// - `rate_t_per_sec`: 生产速率（吨/秒）
/// - `additional_weight_t`: 需要额外生产的重量（吨）
///
/// # 返回
/// - `Some(state)`: 成功推进到目标重量
/// - `None`: 无法推进（速率为0或计划项已耗尽）
///
/// # 说明
/// - 此函数用于估算到达特定重量阈值的时间
/// - 不会触发换辊重置（用于单向预测）
pub(super) fn produce_weight_until(
    items: &[PlanItemLite],
    mut state: CampaignStreamState,
    rate_t_per_sec: f64,
    additional_weight_t: f64,
) -> Option<CampaignStreamState> {
    if additional_weight_t <= 0.0 {
        return Some(state);
    }
    if rate_t_per_sec <= 0.0 {
        return None;
    }

    let mut need = additional_weight_t;
    while need > 1e-9 {
        if state.item_index >= items.len() {
            return None;
        }
        let item = &items[state.item_index];
        if state.current_time < item.earliest_start_at {
            state.current_time = item.earliest_start_at;
        }

        let take = state.remaining_weight_t.min(need);
        let seconds_f = take / rate_t_per_sec;
        let seconds = seconds_f.round().max(0.0) as i64;
        state.current_time += chrono::Duration::seconds(seconds);
        state.remaining_weight_t -= take;
        need -= take;

        if state.remaining_weight_t <= 1e-9 {
            state.item_index += 1;
            if state.item_index < items.len() {
                state.remaining_weight_t = items[state.item_index].weight_t;
            } else {
                state.remaining_weight_t = 0.0;
            }
        }
    }
    Some(state)
}

/// 仿真时间线到指定时刻
///
/// # 参数
/// - `items`: 计划项列表
/// - `rate_t_per_sec`: 生产速率（吨/秒）
/// - `initial_start_at`: 换辊批次初始起始时间
/// - `suggest_threshold_t`: 建议换辊阈值（吨）
/// - `downtime_minutes`: 换辊停机时长（分钟）
/// - `as_of`: 目标仿真时刻
///
/// # 返回
/// - 仿真到 `as_of` 时刻的状态
///
/// # 说明
/// - 此函数模拟从第一个计划项到 `as_of` 的完整时间线
/// - 到达软限制阈值时自动触发换辊（重置累计重量，增加批次号）
/// - 换辊会产生停机时间（downtime_minutes）
/// - 返回的状态包含：当前批次信息、累计重量、仿真时间
pub(super) fn simulate_to_as_of(
    items: &[PlanItemLite],
    rate_t_per_sec: f64,
    initial_start_at: NaiveDateTime,
    suggest_threshold_t: f64,
    downtime_minutes: i64,
    as_of: NaiveDateTime,
) -> CampaignStreamState {
    let mut state = CampaignStreamState {
        item_index: 0,
        remaining_weight_t: items.first().map(|i| i.weight_t).unwrap_or(0.0),
        current_time: items.first().map(|i| i.earliest_start_at).unwrap_or(as_of),
        campaign_no: 1,
        campaign_start_at: initial_start_at,
        cum_weight_t: 0.0,
    };

    // If as_of is before the schedule starts, clamp current_time to as_of and exit early.
    if state.current_time > as_of {
        state.current_time = as_of;
        return state;
    }

    let mut campaign_active = state.current_time >= initial_start_at;
    if !campaign_active && state.current_time < initial_start_at && as_of >= initial_start_at {
        // Campaign becomes active sometime before/as_of (possibly during idle); we will handle more precisely below.
    }

    while state.current_time < as_of {
        if state.item_index >= items.len() {
            // No more production; idle until as_of.
            state.current_time = as_of;
            break;
        }

        let item = &items[state.item_index];
        let item_start = if state.current_time < item.earliest_start_at {
            item.earliest_start_at
        } else {
            state.current_time
        };

        // Idle gap before next item.
        if state.current_time < item_start {
            if !campaign_active && initial_start_at <= item_start && initial_start_at <= as_of {
                campaign_active = true;
                state.campaign_start_at = initial_start_at;
            }

            if as_of < item_start {
                state.current_time = as_of;
                break;
            }
            state.current_time = item_start;
        }

        // If campaign starts during this item's processing, split at initial_start_at.
        if !campaign_active && state.current_time < initial_start_at && initial_start_at <= as_of {
            // Produce until initial_start_at (does not count into cum_weight_t)
            let seconds_until = (initial_start_at - state.current_time).num_seconds();
            if seconds_until > 0 && rate_t_per_sec > 0.0 {
                let producible = (seconds_until as f64) * rate_t_per_sec;
                let produced = state.remaining_weight_t.min(producible);
                let actual_seconds = (produced / rate_t_per_sec).round().max(0.0) as i64;
                state.current_time += chrono::Duration::seconds(actual_seconds);
                state.remaining_weight_t -= produced;
                if state.remaining_weight_t <= 1e-9 {
                    state.item_index += 1;
                    if state.item_index < items.len() {
                        state.remaining_weight_t = items[state.item_index].weight_t;
                    } else {
                        state.remaining_weight_t = 0.0;
                    }
                }
            }

            if state.current_time >= initial_start_at {
                campaign_active = true;
                state.campaign_start_at = initial_start_at;
            }

            continue;
        }

        // No production capacity
        if rate_t_per_sec <= 0.0 {
            state.current_time = as_of;
            break;
        }

        // Process the current item in small segments: (as_of boundary) and (soft-threshold boundary).
        let mut seg_start = state.current_time;
        while seg_start < as_of && state.remaining_weight_t > 1e-9 {
            let seconds_to_finish_item =
                (state.remaining_weight_t / rate_t_per_sec).round().max(0.0) as i64;
            let finish_time = seg_start + chrono::Duration::seconds(seconds_to_finish_item);
            let mut next_event_time = finish_time;

            // Stop at as_of
            if as_of < next_event_time {
                next_event_time = as_of;
            }

            // Soft limit reach -> triggers roll change (auto), but only when campaign is active.
            if campaign_active && suggest_threshold_t > 0.0 {
                let remaining_to_soft = suggest_threshold_t - state.cum_weight_t;
                if remaining_to_soft >= 0.0 && state.remaining_weight_t >= remaining_to_soft {
                    let sec_to_soft =
                        (remaining_to_soft / rate_t_per_sec).round().max(0.0) as i64;
                    let soft_time = seg_start + chrono::Duration::seconds(sec_to_soft);
                    if soft_time < next_event_time {
                        next_event_time = soft_time;
                    }
                }
            }

            let delta_seconds = (next_event_time - seg_start).num_seconds().max(0);
            let produced = (delta_seconds as f64) * rate_t_per_sec;
            let produced = produced.min(state.remaining_weight_t).max(0.0);

            if campaign_active {
                state.cum_weight_t += produced;
            }

            state.remaining_weight_t -= produced;
            state.current_time = next_event_time;
            seg_start = next_event_time;

            // Reached as_of
            if state.current_time >= as_of {
                break;
            }

            // Finished item
            if (finish_time - state.current_time).num_seconds().abs() <= 1 {
                state.item_index += 1;
                if state.item_index < items.len() {
                    state.remaining_weight_t = items[state.item_index].weight_t;
                } else {
                    state.remaining_weight_t = 0.0;
                }
                break;
            }

            // Soft threshold reached -> downtime + reset (if downtime fits before as_of)
            if campaign_active && suggest_threshold_t > 0.0 {
                let reached_soft = (state.cum_weight_t - suggest_threshold_t).abs() <= 1e-6
                    || state.cum_weight_t >= suggest_threshold_t;
                if reached_soft {
                    let downtime_end =
                        state.current_time + chrono::Duration::minutes(downtime_minutes);
                    if downtime_end > as_of {
                        // as_of within downtime: stop here, keep current campaign as-is.
                        state.current_time = as_of;
                        return state;
                    }

                    // Apply downtime and start next campaign
                    state.current_time = downtime_end;
                    state.campaign_no += 1;
                    state.cum_weight_t = 0.0;
                    state.campaign_start_at = state.current_time;
                    // Continue processing remaining weight (same item) after downtime
                    seg_start = state.current_time;
                    continue;
                }
            }
        }
    }

    state.current_time = as_of;
    state
}

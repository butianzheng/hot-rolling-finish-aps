use crate::decision::api::dto::*;

// ==========================================
// 辅助函数：领域对象 -> DTO 转换
// ==========================================

/// 转换 DaySummary -> DaySummaryDto
pub(super) fn convert_day_summary_to_dto(
    summary: &crate::decision::use_cases::d1_most_risky_day::DaySummary,
) -> DaySummaryDto {
    DaySummaryDto {
        plan_date: summary.plan_date.clone(),
        risk_score: summary.risk_score,
        risk_level: summary.risk_level.clone(),
        capacity_util_pct: summary.capacity_util_pct * 100.0, // 转换为百分比
        overload_weight_t: 0.0,                               // TODO: 从用例层获取
        urgent_failure_count: 0,                              // TODO: 从用例层获取
        top_reasons: summary
            .top_reasons
            .iter()
            .map(|r| ReasonItemDto {
                code: r.code.clone(),
                msg: r.msg.clone(),
                weight: r.weight,
                affected_count: None, // TODO: 从用例层获取
            })
            .collect(),
        involved_machines: vec![], // TODO: 从用例层获取
    }
}

/// 转换 MachineBottleneckProfile -> BottleneckPointDto
pub(super) fn convert_bottleneck_profile_to_dto(
    profile: &crate::decision::use_cases::d4_machine_bottleneck::MachineBottleneckProfile,
) -> BottleneckPointDto {
    BottleneckPointDto {
        machine_code: profile.machine_code.clone(),
        plan_date: profile.plan_date.clone(),
        bottleneck_score: profile.bottleneck_score,
        bottleneck_level: profile.bottleneck_level.clone(),
        bottleneck_types: profile
            .bottleneck_types
            .iter()
            .map(|bt| bottleneck_type_to_string(bt))
            .collect(),
        capacity_util_pct: profile.capacity_utilization * 100.0, // 转换为百分比
        pending_material_count: profile.pending_materials as u32,
        pending_weight_t: profile.pending_weight_t, // 从 material_state 查询的真实待排材料重量
        scheduled_material_count: profile.scheduled_materials as u32,
        scheduled_weight_t: profile.scheduled_weight_t,
        reasons: profile
            .reasons
            .iter()
            .map(|r| ReasonItemDto {
                code: r.code.clone(),
                msg: r.description.clone(),
                // 权重口径要求 0-1（前端契约校验也按 0-1）。
                // 读模型 reasons.severity 可能直接写入了利用率比值（>1），这里做兜底钳制避免前端校验失败。
                weight: r.severity.max(0.0).min(1.0),
                affected_count: Some(r.affected_materials as u32),
            })
            .collect(),
        recommended_actions: if profile.suggested_actions.is_empty() {
            None
        } else {
            Some(profile.suggested_actions.clone())
        },
    }
}

/// 转换 BottleneckType -> String
pub(super) fn bottleneck_type_to_string(
    bt: &crate::decision::use_cases::d4_machine_bottleneck::BottleneckType,
) -> String {
    use crate::decision::use_cases::d4_machine_bottleneck::BottleneckType;
    match bt {
        BottleneckType::Capacity => "Capacity".to_string(),
        BottleneckType::Structure => "Structure".to_string(),
        BottleneckType::RollChange => "RollChange".to_string(),
        BottleneckType::ColdStock => "ColdStock".to_string(),
        BottleneckType::Mixed => "Mixed".to_string(),
    }
}

/// 转换 OrderFailure -> OrderFailureDto
pub(super) fn convert_order_failure_to_dto(
    failure: &crate::decision::use_cases::d2_order_failure::OrderFailure,
    machine_code: &str,
    material_id: Option<&str>,
) -> OrderFailureDto {
    let scheduled_weight_t = (failure.total_materials - failure.unscheduled_count) as f64
        * (failure.unscheduled_weight_t / failure.unscheduled_count.max(1) as f64);
    let total_weight_t = scheduled_weight_t + failure.unscheduled_weight_t;

    OrderFailureDto {
        contract_no: failure.contract_no.clone(),
        material_id: material_id.map(|v| v.to_string()),
        due_date: failure.due_date.clone(),
        days_to_due: failure.days_to_due,
        urgency_level: failure.urgency_level.clone(),
        fail_type: match failure.fail_type {
            crate::decision::use_cases::d2_order_failure::FailureType::Overdue => {
                "Overdue".to_string()
            }
            crate::decision::use_cases::d2_order_failure::FailureType::NearDueImpossible => {
                "NearDueImpossible".to_string()
            }
            crate::decision::use_cases::d2_order_failure::FailureType::CapacityShortage => {
                "CapacityShortage".to_string()
            }
            crate::decision::use_cases::d2_order_failure::FailureType::StructureConflict => {
                "StructureConflict".to_string()
            }
            crate::decision::use_cases::d2_order_failure::FailureType::ColdStockNotReady => {
                "ColdStockNotReady".to_string()
            }
            crate::decision::use_cases::d2_order_failure::FailureType::Other => "Other".to_string(),
        },
        completion_rate: failure.completion_rate * 100.0, // 转换为百分比
        total_weight_t,
        scheduled_weight_t,
        unscheduled_weight_t: failure.unscheduled_weight_t,
        machine_code: machine_code.to_string(),
        blocking_factors: failure
            .blocking_factors
            .iter()
            .map(|bf| BlockingFactorDto {
                factor_type: bf.code.clone(),
                description: bf.description.clone(),
                impact: bf.affected_weight_t / total_weight_t.max(1.0),
                affected_material_count: bf.affected_count as u32,
            })
            .collect(),
        failure_reasons: failure.failure_reasons.clone(),
        recommended_actions: Some(failure.suggested_actions.clone()),
    }
}

/// 转换 ColdStockProfile -> ColdStockBucketDto
pub(super) fn convert_cold_stock_to_dto(
    profile: &crate::decision::use_cases::d3_cold_stock::ColdStockProfile,
) -> ColdStockBucketDto {
    ColdStockBucketDto {
        machine_code: profile.machine_code.clone(),
        age_bin: profile.age_bin.clone(),
        count: profile.count as u32,
        weight_t: profile.weight_t,
        pressure_score: profile.pressure_score,
        pressure_level: profile.pressure_level.clone(),
        avg_age_days: profile.avg_age(),
        max_age_days: profile.age_max_days.unwrap_or(999),
        structure_gap: profile
            .structure_gap
            .clone()
            .unwrap_or_else(|| "无".to_string()),
        reasons: profile
            .reasons
            .iter()
            .map(|r| ReasonItemDto {
                code: "AGE_HIGH".to_string(),
                msg: r.clone(),
                weight: 0.8,
                affected_count: Some(profile.count as u32),
            })
            .collect(),
        trend: None, // 领域对象中没有趋势信息
    }
}

/// Normalize date-like strings into `YYYY-MM-DD` (front-end schema requirement).
///
/// The decision read models may store dates as:
/// - `YYYY-MM-DD`
/// - `YYYY-MM-DD HH:MM:SS` (SQLite datetime)
/// - RFC3339 timestamps
pub(super) fn normalize_date_ymd(date_str: &str) -> Option<String> {
    if let Ok(d) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return Some(d.to_string());
    }

    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.date().to_string());
    }

    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.date_naive().to_string());
    }

    None
}

/// 转换 RollAlert -> RollAlertDto
pub(super) fn convert_roll_alert_to_dto(
    alert: &crate::decision::use_cases::d5_roll_campaign_alert::RollAlert,
) -> RollAlertDto {
    let remaining_t = alert.hard_limit_t - alert.cum_weight_t;
    let alert_type = if alert.cum_weight_t >= alert.hard_limit_t {
        "HARD_LIMIT_EXCEEDED"
    } else if alert.cum_weight_t >= alert.suggest_threshold_t {
        "SOFT_LIMIT_EXCEEDED"
    } else {
        "NORMAL"
    };

    // 前端/Schema 约束: campaign_start_date 必须是 YYYY-MM-DD。
    // 新口径: 优先使用读模型提供的 campaign_start_at；若缺失则兜底为当前日期。
    let campaign_start_date = alert
        .campaign_start_at
        .as_deref()
        .and_then(normalize_date_ymd)
        .unwrap_or_else(|| chrono::Utc::now().date_naive().to_string());
    let estimated_hard_stop_date = alert
        .estimated_change_date
        .as_deref()
        .and_then(normalize_date_ymd);

    RollAlertDto {
        machine_code: alert.machine_code.clone(),
        campaign_id: format!("C{:03}", alert.campaign_no),
        campaign_start_date,
        current_tonnage_t: alert.cum_weight_t,
        soft_limit_t: alert.suggest_threshold_t,
        hard_limit_t: alert.hard_limit_t,
        remaining_tonnage_t: remaining_t.max(0.0),
        alert_level: alert.alert_level.clone(),
        alert_type: alert_type.to_string(),
        estimated_hard_stop_date,
        campaign_start_at: alert.campaign_start_at.clone(),
        planned_change_at: alert.planned_change_at.clone(),
        planned_downtime_minutes: alert.planned_downtime_minutes,
        estimated_soft_reach_at: alert.estimated_soft_reach_at.clone(),
        estimated_hard_reach_at: alert.estimated_hard_reach_at.clone(),
        alert_message: alert
            .reason
            .clone()
            .unwrap_or_else(|| "无预警信息".to_string()),
        impact_description: format!(
            "已使用 {:.1}%, 剩余 {:.1} 吨",
            alert.utilization_rate * 100.0,
            remaining_t.max(0.0)
        ),
        recommended_actions: alert.suggested_actions.clone(),
    }
}

/// 转换 CapacityOpportunity -> CapacityOpportunityDto
pub(super) fn convert_capacity_opportunity_to_dto(
    opportunity: &crate::decision::use_cases::d6_capacity_opportunity::CapacityOpportunity,
) -> CapacityOpportunityDto {
    // 从 utilization_rate 反推产能数据
    let used_capacity_t = 1000.0 * opportunity.utilization_rate; // 假设标准产能 1000吨
    let target_capacity_t = 1000.0;
    let opportunity_space_t = opportunity.slack_t;
    let optimized_util_pct =
        ((used_capacity_t + opportunity_space_t * 0.5) / target_capacity_t * 100.0).min(100.0);

    let opportunity_type = match opportunity.opportunity_level.as_str() {
        "HIGH" => "CAPACITY_UNDERUTILIZATION",
        "MEDIUM" => "MODERATE_SLACK",
        "LOW" => "MINOR_OPTIMIZATION",
        _ => "NO_OPPORTUNITY",
    };

    let sensitivity_dto = opportunity
        .sensitivity
        .as_ref()
        .map(|s| SensitivityAnalysisDto {
            scenarios: vec![
                ScenarioDto {
                    name: "保守方案".to_string(),
                    adjustment: format!("增加 {:.1} 吨", s.soft_constraint_gain_t),
                    util_pct: (used_capacity_t + s.soft_constraint_gain_t) / target_capacity_t
                        * 100.0,
                    risk_score: 20.0,
                    affected_material_count: 0,
                },
                ScenarioDto {
                    name: "激进方案".to_string(),
                    adjustment: format!("增加 {:.1} 吨", s.total_potential_gain_t),
                    util_pct: (used_capacity_t + s.total_potential_gain_t) / target_capacity_t
                        * 100.0,
                    risk_score: 70.0,
                    affected_material_count: 0,
                },
            ],
            best_scenario_index: 0,
        });

    CapacityOpportunityDto {
        machine_code: opportunity.machine_code.clone(),
        plan_date: opportunity.plan_date.clone(),
        opportunity_type: opportunity_type.to_string(),
        current_util_pct: opportunity.utilization_rate * 100.0,
        target_capacity_t,
        used_capacity_t,
        opportunity_space_t,
        optimized_util_pct,
        sensitivity: sensitivity_dto,
        description: format!(
            "机组 {} 在 {} 有 {:.1} 吨优化空间 ({})",
            opportunity.machine_code,
            opportunity.plan_date,
            opportunity_space_t,
            opportunity.opportunity_level
        ),
        recommended_actions: opportunity.suggested_optimizations.clone(),
        potential_benefits: vec![
            format!("可增加产能 {:.1} 吨", opportunity_space_t),
            format!("利用率可提升至 {:.1}%", optimized_util_pct),
        ],
    }
}

/// 生成热力图统计
pub(super) fn generate_heatmap_stats(bottleneck_points: &[BottleneckPointDto]) -> HeatmapStatsDto {
    use std::collections::HashMap;

    let total_score: f64 = bottleneck_points.iter().map(|p| p.bottleneck_score).sum();
    let avg_score = total_score / (bottleneck_points.len() as f64);
    let max_score = bottleneck_points
        .iter()
        .map(|p| p.bottleneck_score)
        .fold(0.0, f64::max);
    let bottleneck_days_count = bottleneck_points
        .iter()
        .filter(|p| p.bottleneck_score > 50.0)
        .count() as u32;

    // 按机组聚合统计
    let mut machine_stats_map: HashMap<String, Vec<f64>> = HashMap::new();
    for point in bottleneck_points {
        machine_stats_map
            .entry(point.machine_code.clone())
            .or_insert_with(Vec::new)
            .push(point.bottleneck_score);
    }

    let by_machine: Vec<MachineStatsDto> = machine_stats_map
        .into_iter()
        .map(|(machine_code, scores)| {
            let avg_score = scores.iter().sum::<f64>() / (scores.len() as f64);
            let max_score = scores.iter().fold(0.0_f64, |a, &b| a.max(b));
            let bottleneck_days = scores.iter().filter(|&&s| s > 50.0).count() as u32;
            MachineStatsDto {
                machine_code,
                avg_score,
                max_score,
                bottleneck_days,
            }
        })
        .collect();

    HeatmapStatsDto {
        avg_score,
        max_score,
        bottleneck_days_count,
        by_machine,
    }
}

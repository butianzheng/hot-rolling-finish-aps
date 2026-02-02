// ==========================================
// 热轧精整排产系统 - DecisionApi 实现
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md
// 职责: DecisionApi trait 的具体实现
// ==========================================

use super::decision_api::DecisionApi;
use super::dto::*;
use crate::decision::use_cases::{
    d1_most_risky_day::MostRiskyDayUseCase,
    d2_order_failure::OrderFailureUseCase,
    d3_cold_stock::ColdStockUseCase,
    d4_machine_bottleneck::MachineBottleneckUseCase,
    d5_roll_campaign_alert::RollCampaignAlertUseCase,
    d6_capacity_opportunity::CapacityOpportunityUseCase,
    impls::*,
};
use std::sync::Arc;

/// DecisionApi 实现 (P2 版本: 支持 D1-D6)
pub struct DecisionApiImpl {
    /// D1 用例实现
    d1_use_case: Arc<MostRiskyDayUseCaseImpl>,
    /// D2 用例实现
    d2_use_case: Option<Arc<OrderFailureUseCaseImpl>>,
    /// D3 用例实现
    d3_use_case: Option<Arc<ColdStockUseCaseImpl>>,
    /// D4 用例实现
    d4_use_case: Arc<MachineBottleneckUseCaseImpl>,
    /// D5 用例实现
    d5_use_case: Option<Arc<RollCampaignAlertUseCaseImpl>>,
    /// D6 用例实现
    d6_use_case: Option<Arc<CapacityOpportunityUseCaseImpl>>,
}

impl DecisionApiImpl {
    /// 创建新的 DecisionApiImpl 实例(P1 版本：仅 D1 + D4，保持向后兼容)
    pub fn new(
        d1_use_case: Arc<MostRiskyDayUseCaseImpl>,
        d4_use_case: Arc<MachineBottleneckUseCaseImpl>,
    ) -> Self {
        Self {
            d1_use_case,
            d2_use_case: None,
            d3_use_case: None,
            d4_use_case,
            d5_use_case: None,
            d6_use_case: None,
        }
    }

    /// 创建完整的 DecisionApiImpl 实例(P2 版本：支持 D1-D6)
    pub fn new_full(
        d1_use_case: Arc<MostRiskyDayUseCaseImpl>,
        d2_use_case: Arc<OrderFailureUseCaseImpl>,
        d3_use_case: Arc<ColdStockUseCaseImpl>,
        d4_use_case: Arc<MachineBottleneckUseCaseImpl>,
        d5_use_case: Arc<RollCampaignAlertUseCaseImpl>,
        d6_use_case: Arc<CapacityOpportunityUseCaseImpl>,
    ) -> Self {
        Self {
            d1_use_case,
            d2_use_case: Some(d2_use_case),
            d3_use_case: Some(d3_use_case),
            d4_use_case,
            d5_use_case: Some(d5_use_case),
            d6_use_case: Some(d6_use_case),
        }
    }
}

impl DecisionApi for DecisionApiImpl {
    fn get_decision_day_summary(
        &self,
        request: GetDecisionDaySummaryRequest,
    ) -> Result<DecisionDaySummaryResponse, String> {
        // 调用用例层
        let summaries = self.d1_use_case.get_day_summary(
            &request.version_id,
            &request.date_from,
            &request.date_to,
        )?;

        // 应用过滤器
        let mut filtered_summaries = summaries;

        // 风险等级过滤
        if let Some(ref risk_levels) = request.risk_level_filter {
            filtered_summaries.retain(|s| risk_levels.contains(&s.risk_level));
        }

        // 排序（默认已按 risk_score 降序）
        if let Some(ref sort_by) = request.sort_by {
            match sort_by.as_str() {
                "plan_date" => {
                    filtered_summaries.sort_by(|a, b| a.plan_date.cmp(&b.plan_date));
                }
                "capacity_util_pct" => {
                    filtered_summaries.sort_by(|a, b| {
                        b.capacity_util_pct
                            .partial_cmp(&a.capacity_util_pct)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                _ => {
                    // 默认或 "risk_score"，已经是按风险分数降序
                }
            }
        }

        // 应用限制
        let total_count = filtered_summaries.len() as u32;
        let limit = request.limit.unwrap_or(10) as usize;
        let items: Vec<_> = filtered_summaries.into_iter().take(limit).collect();

        // 转换为 DTO
        let dto_items: Vec<DaySummaryDto> = items
            .into_iter()
            .map(|s| convert_day_summary_to_dto(&s))
            .collect();

        Ok(DecisionDaySummaryResponse {
            version_id: request.version_id,
            as_of: chrono::Utc::now().to_rfc3339(),
            items: dto_items,
            total_count,
        })
    }

    fn get_machine_bottleneck_profile(
        &self,
        request: GetMachineBottleneckProfileRequest,
    ) -> Result<MachineBottleneckProfileResponse, String> {
        // 构建机组代码过滤（用例层只支持单个机组或全部）
        let machine_code_filter = if let Some(ref codes) = request.machine_codes {
            if codes.is_empty() {
                None
            } else if codes.len() == 1 {
                Some(codes[0].as_str())
            } else {
                // 多个机组：查询所有，然后过滤
                None
            }
        } else {
            None
        };

        // 调用用例层
        let profiles = self.d4_use_case.get_machine_bottleneck_profile(
            &request.version_id,
            machine_code_filter,
            &request.date_from,
            &request.date_to,
        )?;

        // 应用多机组过滤
        let mut filtered_profiles = profiles;
        if let Some(ref codes) = request.machine_codes {
            if codes.len() > 1 {
                filtered_profiles.retain(|p| codes.contains(&p.machine_code));
            }
        }

        // 应用堵塞等级过滤
        if let Some(ref levels) = request.bottleneck_level_filter {
            filtered_profiles.retain(|p| levels.contains(&p.bottleneck_level));
        }

        // 应用堵塞类型过滤
        if let Some(ref types) = request.bottleneck_type_filter {
            filtered_profiles.retain(|p| {
                p.bottleneck_types
                    .iter()
                    .any(|bt| types.contains(&bottleneck_type_to_string(bt)))
            });
        }

        // 应用限制
        let total_count = filtered_profiles.len() as u32;
        let limit = request.limit.unwrap_or(50) as usize;
        let items: Vec<_> = filtered_profiles.into_iter().take(limit).collect();

        // 转换为 DTO
        let dto_items: Vec<BottleneckPointDto> = items
            .into_iter()
            .map(|p| convert_bottleneck_profile_to_dto(&p))
            .collect();

        // 生成热力图统计（可选）
        let heatmap_stats = if dto_items.len() > 0 {
            Some(generate_heatmap_stats(&dto_items))
        } else {
            None
        };

        Ok(MachineBottleneckProfileResponse {
            version_id: request.version_id,
            as_of: chrono::Utc::now().to_rfc3339(),
            items: dto_items,
            total_count,
            heatmap_stats,
        })
    }

    fn list_order_failure_set(
        &self,
        request: ListOrderFailureSetRequest,
    ) -> Result<OrderFailureSetResponse, String> {
        // 检查是否已配置 D2 用例
        let d2_use_case = self.d2_use_case.as_ref()
            .ok_or("D2 用例未配置,请使用 new_full() 创建 DecisionApiImpl 实例".to_string())?;

        // 调用用例层查询失败订单
        let fail_type_param = request.fail_type_filter.as_ref()
            .and_then(|v| v.first())
            .map(|s| s.as_str());

        let failures = d2_use_case.list_order_failures(
            &request.version_id,
            fail_type_param,
        )?;

        // 获取统计信息
        let stats = d2_use_case.count_failures(&request.version_id)?;

        // 应用过滤器
        let mut filtered_failures = failures;

        // 紧急等级过滤
        if let Some(ref levels) = request.urgency_level_filter {
            filtered_failures.retain(|f| levels.contains(&f.urgency_level));
        }

        // 完成率阈值过滤
        if let Some(threshold) = request.completion_rate_threshold {
            filtered_failures.retain(|f| f.completion_rate <= threshold);
        }

        // 分页
        let total_count = filtered_failures.len() as u32;
        let offset = request.offset.unwrap_or(0) as usize;
        let limit = request.limit.unwrap_or(50) as usize;
        let items: Vec<_> = filtered_failures
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        // 批量补齐 machine_code（读模型未存储，按合同从 material_state 推断主机组）
        let contract_nos: Vec<String> = items.iter().map(|f| f.contract_no.clone()).collect();
        let machine_code_map = d2_use_case
            .get_primary_machine_codes(&contract_nos)
            .unwrap_or_default();

        // 转换为 DTO
        let dto_items: Vec<OrderFailureDto> = items
            .into_iter()
            .map(|f| {
                let machine_code = machine_code_map
                    .get(&f.contract_no)
                    .map(|s| s.as_str())
                    .unwrap_or("UNKNOWN");
                convert_order_failure_to_dto(&f, machine_code)
            })
            .collect();

        let summary_dto = OrderFailureSummaryDto {
            total_failures: stats.total_failures as u32,
            by_fail_type: vec![
                TypeCountDto {
                    type_name: "Overdue".to_string(),
                    count: stats.overdue_count as u32,
                    weight_t: 0.0, // 领域对象中没有按类型的重量统计
                },
                TypeCountDto {
                    type_name: "NearDueImpossible".to_string(),
                    count: stats.near_due_impossible_count as u32,
                    weight_t: 0.0,
                },
                TypeCountDto {
                    type_name: "CapacityShortage".to_string(),
                    count: stats.capacity_shortage_count as u32,
                    weight_t: 0.0,
                },
                TypeCountDto {
                    type_name: "StructureConflict".to_string(),
                    count: stats.structure_conflict_count as u32,
                    weight_t: 0.0,
                },
            ],
            by_urgency: vec![], // 领域对象中没有按紧急度的统计
            total_unscheduled_weight_t: stats.total_affected_weight_t,
        };

        Ok(OrderFailureSetResponse {
            version_id: request.version_id,
            as_of: chrono::Utc::now().to_rfc3339(),
            items: dto_items,
            total_count,
            summary: summary_dto,
        })
    }

    fn get_cold_stock_profile(
        &self,
        request: GetColdStockProfileRequest,
    ) -> Result<ColdStockProfileResponse, String> {
        let d3_use_case = self.d3_use_case.as_ref()
            .ok_or("D3 用例未配置,请使用 new_full() 创建 DecisionApiImpl 实例".to_string())?;

        // 调用用例层
        let machine_code_filter = if let Some(ref codes) = request.machine_codes {
            if codes.len() == 1 {
                Some(codes[0].as_str())
            } else {
                None
            }
        } else {
            None
        };

        let profiles = d3_use_case.get_cold_stock_profile(&request.version_id, machine_code_filter)?;
        let summary = d3_use_case.get_cold_stock_summary(&request.version_id)?;

        // 应用过滤器
        let mut filtered_profiles = profiles;

        if let Some(ref codes) = request.machine_codes {
            if codes.len() > 1 {
                filtered_profiles.retain(|p| codes.contains(&p.machine_code));
            }
        }

        if let Some(ref levels) = request.pressure_level_filter {
            filtered_profiles.retain(|p| levels.contains(&p.pressure_level));
        }

        if let Some(ref bins) = request.age_bin_filter {
            filtered_profiles.retain(|p| bins.contains(&p.age_bin));
        }

        // 应用限制
        let total_count = filtered_profiles.len() as u32;
        let limit = request.limit.unwrap_or(50) as usize;
        let items: Vec<_> = filtered_profiles.into_iter().take(limit).collect();

        Ok(ColdStockProfileResponse {
            version_id: request.version_id,
            as_of: chrono::Utc::now().to_rfc3339(),
            items: items.into_iter().map(|p| convert_cold_stock_to_dto(&p)).collect(),
            total_count,
            summary: ColdStockSummaryDto {
                total_cold_stock_count: summary.total_count as u32,
                total_cold_stock_weight_t: summary.total_weight_t,
                avg_age_days: summary.avg_age_days,
                high_pressure_count: summary.high_pressure_machines as u32,
                by_machine: summary.by_machine.into_iter().map(|m| MachineStockStatsDto {
                    machine_code: m.machine_code,
                    count: m.count as u32,
                    weight_t: m.weight_t,
                    avg_pressure_score: m.pressure_score,
                }).collect(),
                by_age_bin: summary.by_age.into_iter().map(|a| AgeBinStatsDto {
                    age_bin: a.age_bin,
                    count: a.count as u32,
                    weight_t: a.weight_t,
                }).collect(),
            },
        })
    }

    fn list_roll_campaign_alerts(
        &self,
        request: ListRollCampaignAlertsRequest,
    ) -> Result<RollCampaignAlertsResponse, String> {
        let d5_use_case = self.d5_use_case.as_ref()
            .ok_or("D5 用例未配置,请使用 new_full() 创建 DecisionApiImpl 实例".to_string())?;

        // 调用用例层
        let alert_level_param = request.alert_level_filter.as_ref()
            .and_then(|v| v.first())
            .map(|s| s.as_str());

        let alerts = d5_use_case.list_roll_campaign_alerts(&request.version_id, alert_level_param)?;
        let summary = d5_use_case.get_roll_alert_summary(&request.version_id)?;

        // 应用过滤器
        let mut filtered_alerts = alerts;

        if let Some(ref codes) = request.machine_codes {
            if codes.len() > 1 {
                filtered_alerts.retain(|a| codes.contains(&a.machine_code));
            }
        }

        if let Some(ref levels) = request.alert_level_filter {
            filtered_alerts.retain(|a| levels.contains(&a.alert_level));
        }

        // 应用限制
        let total_count = filtered_alerts.len() as u32;
        let limit = request.limit.unwrap_or(50) as usize;
        let items: Vec<_> = filtered_alerts.into_iter().take(limit).collect();

        Ok(RollCampaignAlertsResponse {
            version_id: request.version_id,
            as_of: chrono::Utc::now().to_rfc3339(),
            items: items.into_iter().map(|a| convert_roll_alert_to_dto(&a)).collect(),
            total_count,
            summary: RollAlertSummaryDto {
                total_alerts: summary.total_alerts as u32,
                by_level: vec![
                    TypeCountDto {
                        type_name: "EMERGENCY".to_string(),
                        count: summary.emergency_count as u32,
                        weight_t: 0.0,
                    },
                    TypeCountDto {
                        type_name: "CRITICAL".to_string(),
                        count: summary.critical_count as u32,
                        weight_t: 0.0,
                    },
                    TypeCountDto {
                        type_name: "WARNING".to_string(),
                        count: summary.warning_count as u32,
                        weight_t: 0.0,
                    },
                ],
                by_type: vec![], // 领域对象中没有按类型的统计
                near_hard_stop_count: summary.immediate_change_needed as u32,
            },
        })
    }

    fn get_capacity_opportunity(
        &self,
        request: GetCapacityOpportunityRequest,
    ) -> Result<CapacityOpportunityResponse, String> {
        let d6_use_case = self.d6_use_case.as_ref()
            .ok_or("D6 用例未配置,请使用 new_full() 创建 DecisionApiImpl 实例".to_string())?;

        // 调用用例层
        let machine_code_param = request.machine_codes.as_ref()
            .and_then(|v| v.first())
            .map(|s| s.as_str());

        let start_date = request.date_from.as_deref().unwrap_or("2026-01-01");
        let end_date = request.date_to.as_deref().unwrap_or("2026-12-31");

        let opportunities = d6_use_case.get_capacity_opportunity(
            &request.version_id,
            machine_code_param,
            start_date,
            end_date,
        )?;

        let summary = d6_use_case.get_optimization_summary(
            &request.version_id,
            start_date,
            end_date,
        )?;

        // 应用过滤器
        let mut filtered_opportunities = opportunities;

        if let Some(ref codes) = request.machine_codes {
            if codes.len() > 1 {
                filtered_opportunities.retain(|o| codes.contains(&o.machine_code));
            }
        }

        if let Some(min_t) = request.min_opportunity_t {
            filtered_opportunities.retain(|o| o.slack_t >= min_t);
        }

        // 应用限制
        let total_count = filtered_opportunities.len() as u32;
        let limit = request.limit.unwrap_or(50) as usize;
        let items: Vec<_> = filtered_opportunities.into_iter().take(limit).collect();

        Ok(CapacityOpportunityResponse {
            version_id: request.version_id,
            as_of: chrono::Utc::now().to_rfc3339(),
            items: items.into_iter().map(|o| convert_capacity_opportunity_to_dto(&o)).collect(),
            total_count,
            summary: CapacityOpportunitySummaryDto {
                total_opportunities: total_count,
                total_opportunity_space_t: summary.total_slack_t,
                by_type: vec![], // 领域对象中没有按类型的统计
                avg_current_util_pct: summary.avg_utilization_rate * 100.0,
                avg_optimized_util_pct: (summary.avg_utilization_rate * 100.0 + 10.0).min(100.0), // 假设优化可提升 10%
            },
        })
    }
}

// ==========================================
// 辅助函数：领域对象 -> DTO 转换
// ==========================================

/// 转换 DaySummary -> DaySummaryDto
fn convert_day_summary_to_dto(
    summary: &crate::decision::use_cases::d1_most_risky_day::DaySummary,
) -> DaySummaryDto {
    DaySummaryDto {
        plan_date: summary.plan_date.clone(),
        risk_score: summary.risk_score,
        risk_level: summary.risk_level.clone(),
        capacity_util_pct: summary.capacity_util_pct * 100.0, // 转换为百分比
        overload_weight_t: 0.0, // TODO: 从用例层获取
        urgent_failure_count: 0, // TODO: 从用例层获取
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
fn convert_bottleneck_profile_to_dto(
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
fn bottleneck_type_to_string(
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
fn convert_order_failure_to_dto(
    failure: &crate::decision::use_cases::d2_order_failure::OrderFailure,
    machine_code: &str,
) -> OrderFailureDto {
    let scheduled_weight_t = (failure.total_materials - failure.unscheduled_count) as f64 *
        (failure.unscheduled_weight_t / failure.unscheduled_count.max(1) as f64);
    let total_weight_t = scheduled_weight_t + failure.unscheduled_weight_t;

    OrderFailureDto {
        contract_no: failure.contract_no.clone(),
        due_date: failure.due_date.clone(),
        days_to_due: failure.days_to_due,
        urgency_level: failure.urgency_level.clone(),
        fail_type: match failure.fail_type {
            crate::decision::use_cases::d2_order_failure::FailureType::Overdue => "Overdue".to_string(),
            crate::decision::use_cases::d2_order_failure::FailureType::NearDueImpossible => "NearDueImpossible".to_string(),
            crate::decision::use_cases::d2_order_failure::FailureType::CapacityShortage => "CapacityShortage".to_string(),
            crate::decision::use_cases::d2_order_failure::FailureType::StructureConflict => "StructureConflict".to_string(),
            crate::decision::use_cases::d2_order_failure::FailureType::ColdStockNotReady => "ColdStockNotReady".to_string(),
            crate::decision::use_cases::d2_order_failure::FailureType::Other => "Other".to_string(),
        },
        completion_rate: failure.completion_rate * 100.0, // 转换为百分比
        total_weight_t,
        scheduled_weight_t,
        unscheduled_weight_t: failure.unscheduled_weight_t,
        machine_code: machine_code.to_string(),
        blocking_factors: failure.blocking_factors.iter().map(|bf| BlockingFactorDto {
            factor_type: bf.code.clone(),
            description: bf.description.clone(),
            impact: bf.affected_weight_t / total_weight_t.max(1.0),
            affected_material_count: bf.affected_count as u32,
        }).collect(),
        failure_reasons: failure.failure_reasons.clone(),
        recommended_actions: Some(failure.suggested_actions.clone()),
    }
}

/// 转换 ColdStockProfile -> ColdStockBucketDto
fn convert_cold_stock_to_dto(
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
        structure_gap: profile.structure_gap.clone().unwrap_or_else(|| "无".to_string()),
        reasons: profile.reasons.iter().map(|r| ReasonItemDto {
            code: "AGE_HIGH".to_string(),
            msg: r.clone(),
            weight: 0.8,
            affected_count: Some(profile.count as u32),
        }).collect(),
        trend: None, // 领域对象中没有趋势信息
    }
}

/// Normalize date-like strings into `YYYY-MM-DD` (front-end schema requirement).
///
/// The decision read models may store dates as:
/// - `YYYY-MM-DD`
/// - `YYYY-MM-DD HH:MM:SS` (SQLite datetime)
/// - RFC3339 timestamps
fn normalize_date_ymd(date_str: &str) -> Option<String> {
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
fn convert_roll_alert_to_dto(
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
        alert_message: alert.reason.clone().unwrap_or_else(|| "无预警信息".to_string()),
        impact_description: format!(
            "已使用 {:.1}%, 剩余 {:.1} 吨",
            alert.utilization_rate * 100.0,
            remaining_t.max(0.0)
        ),
        recommended_actions: alert.suggested_actions.clone(),
    }
}

/// 转换 CapacityOpportunity -> CapacityOpportunityDto
fn convert_capacity_opportunity_to_dto(
    opportunity: &crate::decision::use_cases::d6_capacity_opportunity::CapacityOpportunity,
) -> CapacityOpportunityDto {
    // 从 utilization_rate 反推产能数据
    let used_capacity_t = 1000.0 * opportunity.utilization_rate; // 假设标准产能 1000吨
    let target_capacity_t = 1000.0;
    let opportunity_space_t = opportunity.slack_t;
    let optimized_util_pct = ((used_capacity_t + opportunity_space_t * 0.5) / target_capacity_t * 100.0).min(100.0);

    let opportunity_type = match opportunity.opportunity_level.as_str() {
        "HIGH" => "CAPACITY_UNDERUTILIZATION",
        "MEDIUM" => "MODERATE_SLACK",
        "LOW" => "MINOR_OPTIMIZATION",
        _ => "NO_OPPORTUNITY",
    };

    let sensitivity_dto = opportunity.sensitivity.as_ref().map(|s| SensitivityAnalysisDto {
        scenarios: vec![
            ScenarioDto {
                name: "保守方案".to_string(),
                adjustment: format!("增加 {:.1} 吨", s.soft_constraint_gain_t),
                util_pct: (used_capacity_t + s.soft_constraint_gain_t) / target_capacity_t * 100.0,
                risk_score: 20.0,
                affected_material_count: 0,
            },
            ScenarioDto {
                name: "激进方案".to_string(),
                adjustment: format!("增加 {:.1} 吨", s.total_potential_gain_t),
                util_pct: (used_capacity_t + s.total_potential_gain_t) / target_capacity_t * 100.0,
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
fn generate_heatmap_stats(bottleneck_points: &[BottleneckPointDto]) -> HeatmapStatsDto {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::repository::{
        bottleneck_repo::BottleneckRepository, day_summary_repo::DaySummaryRepository,
    };
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn setup_test_api() -> DecisionApiImpl {
        // 创建内存数据库
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));

        // 创建 risk_snapshot 表
        {
            let c = conn.lock().unwrap();
            c.execute(
                r#"
                CREATE TABLE IF NOT EXISTS risk_snapshot (
                    version_id TEXT NOT NULL,
                    machine_code TEXT NOT NULL,
                    snapshot_date TEXT NOT NULL,
                    risk_level TEXT NOT NULL,
                    risk_reasons TEXT,
                    target_capacity_t REAL NOT NULL,
                    used_capacity_t REAL NOT NULL,
                    limit_capacity_t REAL NOT NULL,
                    overflow_t REAL NOT NULL,
                    urgent_total_t REAL NOT NULL,
                    mature_backlog_t REAL,
                    immature_backlog_t REAL,
                    campaign_status TEXT,
                    created_at TEXT NOT NULL,
                    PRIMARY KEY (version_id, machine_code, snapshot_date)
                )
                "#,
                [],
            )
            .unwrap();

            // 插入测试数据
            c.execute(
                r#"
                INSERT INTO risk_snapshot VALUES (
                    'V001', 'H032', '2026-01-24', 'HIGH', '产能紧张',
                    1500.0, 1450.0, 2000.0, 0.0, 800.0, 500.0, 200.0, 'OK',
                    datetime('now')
                )
                "#,
                [],
            )
            .unwrap();
        }

        // 创建 D1 仓储和用例
        let d1_repo = Arc::new(DaySummaryRepository::new(conn.clone()));
        let d1_use_case = Arc::new(MostRiskyDayUseCaseImpl::new(d1_repo));

        // 创建 capacity_pool 和 plan_item 表
        {
            let c = conn.lock().unwrap();
            c.execute(
                r#"
                CREATE TABLE IF NOT EXISTS capacity_pool (
                    version_id TEXT NOT NULL,
                    machine_code TEXT NOT NULL,
                    plan_date TEXT NOT NULL,
                    target_capacity_t REAL NOT NULL,
                    limit_capacity_t REAL NOT NULL,
                    used_capacity_t REAL NOT NULL DEFAULT 0.0,
                    overflow_t REAL NOT NULL DEFAULT 0.0,
                    frozen_capacity_t REAL NOT NULL DEFAULT 0.0,
                    accumulated_tonnage_t REAL NOT NULL DEFAULT 0.0,
                    roll_campaign_id TEXT,
                    PRIMARY KEY (version_id, machine_code, plan_date)
                )
                "#,
                [],
            )
            .unwrap();

            c.execute(
                r#"
                CREATE TABLE IF NOT EXISTS plan_item (
                    version_id TEXT NOT NULL,
                    material_id TEXT NOT NULL,
                    machine_code TEXT NOT NULL,
                    plan_date TEXT NOT NULL,
                    seq_no INTEGER NOT NULL,
                    weight_t REAL NOT NULL,
                    source_type TEXT NOT NULL,
                    locked_in_plan INTEGER NOT NULL DEFAULT 0,
                    force_release_in_plan INTEGER NOT NULL DEFAULT 0,
                    violation_flags TEXT,
                    PRIMARY KEY (version_id, material_id)
                )
                "#,
                [],
            )
            .unwrap();

            c.execute(
                r#"
                CREATE TABLE IF NOT EXISTS material_master (
                    material_id TEXT PRIMARY KEY,
                    current_machine_code TEXT,
                    next_machine_code TEXT,
                    weight_t REAL,
                    created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                    updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
                )
                "#,
                [],
            )
            .unwrap();

            c.execute(
                r#"
                CREATE TABLE IF NOT EXISTS material_state (
                    material_id TEXT PRIMARY KEY,
                    sched_state TEXT NOT NULL DEFAULT 'READY',
                    updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
                )
                "#,
                [],
            )
            .unwrap();

            // 插入测试数据
            c.execute(
                r#"
                INSERT INTO capacity_pool VALUES (
                    'V001', 'H032', '2026-01-24', 1500.0, 2000.0, 1950.0, 0.0, 100.0, 15000.0, 'RC001'
                )
                "#,
                [],
            )
            .unwrap();

            c.execute(
                r#"
                INSERT INTO plan_item VALUES (
                    'V001', 'MAT001', 'H032', '2026-01-24', 1, 150.0, 'AUTO', 0, 0, ''
                )
                "#,
                [],
            )
            .unwrap();
        }

        // 创建 D4 仓储和用例
        let d4_repo = Arc::new(BottleneckRepository::new(conn));
        let d4_use_case = Arc::new(MachineBottleneckUseCaseImpl::new(d4_repo));

        DecisionApiImpl::new(d1_use_case, d4_use_case)
    }

    #[test]
    fn test_get_decision_day_summary() {
        let api = setup_test_api();

        let request = GetDecisionDaySummaryRequest {
            version_id: "V001".to_string(),
            date_from: "2026-01-24".to_string(),
            date_to: "2026-01-24".to_string(),
            risk_level_filter: None,
            limit: Some(10),
            sort_by: None,
        };

        let response = api.get_decision_day_summary(request).unwrap();

        assert_eq!(response.version_id, "V001");
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.total_count, 1);

        let day_summary = &response.items[0];
        assert_eq!(day_summary.plan_date, "2026-01-24");
        assert!(day_summary.risk_score > 0.0);
    }

    #[test]
    fn test_get_machine_bottleneck_profile() {
        let api = setup_test_api();

        let request = GetMachineBottleneckProfileRequest {
            version_id: "V001".to_string(),
            date_from: "2026-01-24".to_string(),
            date_to: "2026-01-24".to_string(),
            machine_codes: None,
            bottleneck_level_filter: None,
            bottleneck_type_filter: None,
            limit: Some(50),
        };

        let response = api.get_machine_bottleneck_profile(request).unwrap();

        assert_eq!(response.version_id, "V001");
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.total_count, 1);

        let bottleneck = &response.items[0];
        assert_eq!(bottleneck.machine_code, "H032");
        assert_eq!(bottleneck.plan_date, "2026-01-24");
        assert!(bottleneck.bottleneck_score >= 0.0);
    }

    #[test]
    fn test_unimplemented_apis() {
        let api = setup_test_api();

        // D2
        let d2_request = ListOrderFailureSetRequest {
            version_id: "V001".to_string(),
            fail_type_filter: None,
            urgency_level_filter: None,
            machine_codes: None,
            due_date_from: None,
            due_date_to: None,
            completion_rate_threshold: None,
            limit: None,
            offset: None,
        };
        assert!(api.list_order_failure_set(d2_request).is_err());

        // D3
        let d3_request = GetColdStockProfileRequest {
            version_id: "V001".to_string(),
            machine_codes: None,
            pressure_level_filter: None,
            age_bin_filter: None,
            limit: None,
        };
        assert!(api.get_cold_stock_profile(d3_request).is_err());

        // D5
        let d5_request = ListRollCampaignAlertsRequest {
            version_id: "V001".to_string(),
            machine_codes: None,
            alert_level_filter: None,
            alert_type_filter: None,
            date_from: None,
            date_to: None,
            limit: None,
        };
        assert!(api.list_roll_campaign_alerts(d5_request).is_err());

        // D6
        let d6_request = GetCapacityOpportunityRequest {
            version_id: "V001".to_string(),
            machine_codes: None,
            date_from: None,
            date_to: None,
            opportunity_type_filter: None,
            min_opportunity_t: None,
            limit: None,
        };
        assert!(api.get_capacity_opportunity(d6_request).is_err());
    }
}

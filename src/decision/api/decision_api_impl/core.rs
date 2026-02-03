use crate::decision::api::decision_api::DecisionApi;
use crate::decision::api::dto::*;
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

use super::conversions::{
    bottleneck_type_to_string,
    convert_bottleneck_profile_to_dto,
    convert_capacity_opportunity_to_dto,
    convert_cold_stock_to_dto,
    convert_day_summary_to_dto,
    convert_order_failure_to_dto,
    convert_roll_alert_to_dto,
    generate_heatmap_stats,
};

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

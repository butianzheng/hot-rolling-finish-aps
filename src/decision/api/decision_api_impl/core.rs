use crate::decision::api::decision_api::DecisionApi;
use crate::decision::api::dto::*;
use crate::decision::use_cases::{
    d1_most_risky_day::MostRiskyDayUseCase,
    d2_order_failure::{FailureType, OrderFailureUseCase},
    d3_cold_stock::ColdStockUseCase,
    d4_machine_bottleneck::{BottleneckReason, MachineBottleneckProfile},
    d4_machine_bottleneck::MachineBottleneckUseCase,
    d5_roll_campaign_alert::RollCampaignAlertUseCase,
    d6_capacity_opportunity::CapacityOpportunityUseCase,
    impls::*,
};
use chrono::NaiveDate;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::conversions::{
    bottleneck_type_to_string, convert_bottleneck_profile_to_dto,
    convert_capacity_opportunity_to_dto, convert_cold_stock_to_dto, convert_day_summary_to_dto,
    convert_order_failure_to_dto, convert_roll_alert_to_dto, generate_heatmap_stats,
};

fn failure_type_code(ft: &FailureType) -> &'static str {
    match ft {
        FailureType::Overdue => "Overdue",
        FailureType::NearDueImpossible => "NearDueImpossible",
        FailureType::CapacityShortage => "CapacityShortage",
        FailureType::StructureConflict => "StructureConflict",
        FailureType::ColdStockNotReady => "ColdStockNotReady",
        FailureType::Other => "Other",
    }
}

fn urgency_rank(level: &str) -> i32 {
    match level {
        "L3" => 3,
        "L2" => 2,
        "L1" => 1,
        _ => 0,
    }
}

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

    fn list_target_machine_codes(
        &self,
        request: &GetMachineBottleneckProfileRequest,
    ) -> Vec<String> {
        if let Some(ref codes) = request.machine_codes {
            if !codes.is_empty() {
                return codes.clone();
            }
        }

        self.d4_use_case
            .list_active_machine_codes()
            .unwrap_or_else(|e| {
                tracing::warn!("D4 查询活跃机组失败，跳过缺失机组补齐: {}", e);
                Vec::new()
            })
    }

    fn build_date_axis(date_from: &str, date_to: &str) -> Vec<String> {
        let Ok(mut date_cursor) = NaiveDate::parse_from_str(date_from, "%Y-%m-%d") else {
            return vec![date_from.to_string()];
        };
        let Ok(date_end) = NaiveDate::parse_from_str(date_to, "%Y-%m-%d") else {
            return vec![date_from.to_string()];
        };
        if date_cursor > date_end {
            return vec![date_from.to_string()];
        }

        let mut dates = Vec::new();
        while date_cursor <= date_end {
            dates.push(date_cursor.to_string());
            if dates.len() > 366 {
                break;
            }
            let Some(next_date) = date_cursor.succ_opt() else {
                break;
            };
            date_cursor = next_date;
        }
        if dates.is_empty() {
            vec![date_from.to_string()]
        } else {
            dates
        }
    }

    fn fill_missing_machine_profiles(
        &self,
        request: &GetMachineBottleneckProfileRequest,
        profiles: &mut Vec<MachineBottleneckProfile>,
    ) {
        let target_machine_codes = self.list_target_machine_codes(request);
        if target_machine_codes.is_empty() {
            return;
        }

        let existing_machines: HashSet<&str> =
            profiles.iter().map(|p| p.machine_code.as_str()).collect();
        let missing_machines: Vec<String> = target_machine_codes
            .into_iter()
            .filter(|m| !existing_machines.contains(m.as_str()))
            .collect();

        if missing_machines.is_empty() {
            return;
        }

        let dates = Self::build_date_axis(&request.date_from, &request.date_to);
        for machine_code in missing_machines {
            for plan_date in &dates {
                let mut profile = MachineBottleneckProfile::new(
                    request.version_id.clone(),
                    machine_code.clone(),
                    plan_date.clone(),
                );
                profile.reasons.push(BottleneckReason {
                    code: "NO_CAPACITY_POOL_CONFIG".to_string(),
                    description: "该机组无产能池配置，当前仅作为缺失行展示".to_string(),
                    severity: 0.0,
                    affected_materials: 0,
                });
                profile
                    .suggested_actions
                    .push("补齐该机组 capacity_pool 配置".to_string());
                profiles.push(profile);
            }
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

        // 补齐“无 capacity_pool 记录”的活跃机组，避免热力图缺行。
        self.fill_missing_machine_profiles(&request, &mut filtered_profiles);

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
        let d2_use_case = self
            .d2_use_case
            .as_ref()
            .ok_or("D2 用例未配置,请使用 new_full() 创建 DecisionApiImpl 实例".to_string())?;

        // 调用用例层查询失败订单
        let fail_type_param = request
            .fail_type_filter
            .as_ref()
            .and_then(|v| v.first())
            .map(|s| s.as_str());

        let failures = d2_use_case.list_order_failures(&request.version_id, fail_type_param)?;

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
        let material_id_map = d2_use_case
            .get_primary_material_ids(&request.version_id, &contract_nos)
            .unwrap_or_default();

        // 转换为 DTO
        let dto_items: Vec<OrderFailureDto> = items
            .into_iter()
            .map(|f| {
                let machine_code = machine_code_map
                    .get(&f.contract_no)
                    .map(|s| s.as_str())
                    .unwrap_or("UNKNOWN");
                let material_id = material_id_map.get(&f.contract_no).map(|s| s.as_str());
                convert_order_failure_to_dto(&f, machine_code, material_id)
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

    fn list_material_failure_set(
        &self,
        request: ListMaterialFailureSetRequest,
    ) -> Result<MaterialFailureSetResponse, String> {
        let d2_use_case = self
            .d2_use_case
            .as_ref()
            .ok_or("D2 用例未配置,请使用 new_full() 创建 DecisionApiImpl 实例".to_string())?;

        let zero_summary = MaterialFailureSummaryDto {
            total_failed_materials: 0,
            total_failed_contracts: 0,
            overdue_materials: 0,
            unscheduled_materials: 0,
            total_unscheduled_weight_t: 0.0,
            by_fail_type: vec![],
            by_urgency: vec![],
        };

        let raw_problem_scope = request
            .problem_scope
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("UNSCHEDULED_ONLY")
            .to_ascii_uppercase();
        let problem_scope = if raw_problem_scope == "DUE_WINDOW_CRITICAL" {
            "DUE_WINDOW_CRITICAL"
        } else {
            "UNSCHEDULED_ONLY"
        };
        let only_unscheduled = match request.only_unscheduled {
            Some(v) => Some(v),
            None if problem_scope == "UNSCHEDULED_ONLY" => Some(true),
            None => None,
        };
        // DUE_WINDOW_CRITICAL：聚焦临期窗口（含逾期）
        const DUE_WINDOW_CRITICAL_DAYS: i32 = 3;

        // 先按合同维度取失败集合，再下钻到材料维度，保证 D2 口径一致。
        let mut contract_failures = d2_use_case.list_order_failures(&request.version_id, None)?;

        if let Some(ref fail_types) = request.fail_type_filter {
            let allowed: HashSet<&str> = fail_types.iter().map(|s| s.as_str()).collect();
            if !allowed.is_empty() {
                contract_failures.retain(|f| allowed.contains(failure_type_code(&f.fail_type)));
            }
        }

        if let Some(ref levels) = request.urgency_level_filter {
            if !levels.is_empty() {
                contract_failures.retain(|f| levels.contains(&f.urgency_level));
            }
        }

        if let Some(threshold) = request.completion_rate_threshold {
            // 与 list_order_failure_set 保持一致：沿用现有阈值口径（领域对象 completion_rate）。
            contract_failures.retain(|f| f.completion_rate <= threshold);
        }

        if contract_failures.is_empty() {
            return Ok(MaterialFailureSetResponse {
                version_id: request.version_id,
                as_of: chrono::Utc::now().to_rfc3339(),
                items: vec![],
                total_count: 0,
                summary: zero_summary,
                contract_aggregates: vec![],
            });
        }

        let mut failure_by_contract: HashMap<
            String,
            crate::decision::use_cases::d2_order_failure::OrderFailure,
        > = HashMap::with_capacity(contract_failures.len());
        for f in contract_failures {
            failure_by_contract.insert(f.contract_no.clone(), f);
        }

        let contract_nos: Vec<String> = failure_by_contract.keys().cloned().collect();
        let mut material_rows = d2_use_case.list_material_failures(
            &request.version_id,
            &contract_nos,
            request.urgency_level_filter.as_deref(),
            request.machine_codes.as_deref(),
            request.due_date_from.as_deref(),
            request.due_date_to.as_deref(),
            only_unscheduled,
        )?;
        if problem_scope == "DUE_WINDOW_CRITICAL" {
            material_rows.retain(|row| row.days_to_due <= DUE_WINDOW_CRITICAL_DAYS);
        }

        let mut all_items: Vec<MaterialFailureDto> = material_rows
            .into_iter()
            .filter_map(|row| {
                let failure = failure_by_contract.get(&row.contract_no)?;
                let unscheduled_weight_t = if row.is_scheduled { 0.0 } else { row.weight_t };
                let denom = failure.unscheduled_weight_t.max(1.0);
                let blocking_factors = failure
                    .blocking_factors
                    .iter()
                    .map(|bf| BlockingFactorDto {
                        factor_type: bf.code.clone(),
                        description: bf.description.clone(),
                        impact: (bf.affected_weight_t / denom).clamp(0.0, 1.0),
                        affected_material_count: bf.affected_count.max(0) as u32,
                    })
                    .collect::<Vec<_>>();
                let recommended_actions = if failure.suggested_actions.is_empty() {
                    None
                } else {
                    Some(failure.suggested_actions.clone())
                };

                Some(MaterialFailureDto {
                    material_id: row.material_id,
                    contract_no: row.contract_no,
                    due_date: row.due_date,
                    days_to_due: row.days_to_due,
                    urgency_level: row.urgency_level,
                    fail_type: failure_type_code(&failure.fail_type).to_string(),
                    completion_rate: failure.completion_rate * 100.0,
                    weight_t: row.weight_t,
                    unscheduled_weight_t,
                    machine_code: row.machine_code,
                    is_scheduled: row.is_scheduled,
                    blocking_factors,
                    failure_reasons: failure.failure_reasons.clone(),
                    recommended_actions,
                })
            })
            .collect();

        all_items.sort_by(|a, b| {
            let u = urgency_rank(&b.urgency_level).cmp(&urgency_rank(&a.urgency_level));
            if u != std::cmp::Ordering::Equal {
                return u;
            }
            let unscheduled =
                (if !b.is_scheduled { 1 } else { 0 }).cmp(&(if !a.is_scheduled { 1 } else { 0 }));
            if unscheduled != std::cmp::Ordering::Equal {
                return unscheduled;
            }
            let due = a.due_date.cmp(&b.due_date);
            if due != std::cmp::Ordering::Equal {
                return due;
            }
            let contract = a.contract_no.cmp(&b.contract_no);
            if contract != std::cmp::Ordering::Equal {
                return contract;
            }
            a.material_id.cmp(&b.material_id)
        });

        let mut by_fail_type_map: HashMap<String, (u32, f64)> = HashMap::new();
        let mut by_urgency_map: HashMap<String, (u32, f64)> = HashMap::new();
        let mut contract_set: HashSet<String> = HashSet::new();
        let mut overdue_materials: u32 = 0;
        let mut unscheduled_materials: u32 = 0;
        let mut total_unscheduled_weight_t: f64 = 0.0;

        #[derive(Clone)]
        struct ContractAggTmp {
            contract_no: String,
            material_count: u32,
            unscheduled_count: u32,
            overdue_count: u32,
            earliest_due_date: String,
            max_urgency_level: String,
            representative_material_id: String,
            rep_unscheduled: i32,
            rep_urgency: i32,
            rep_due_date: String,
        }

        let mut contract_aggs: HashMap<String, ContractAggTmp> = HashMap::new();

        for item in &all_items {
            contract_set.insert(item.contract_no.clone());
            if item.days_to_due < 0 {
                overdue_materials += 1;
            }
            if !item.is_scheduled {
                unscheduled_materials += 1;
            }
            total_unscheduled_weight_t += item.unscheduled_weight_t;

            let fail_entry = by_fail_type_map
                .entry(item.fail_type.clone())
                .or_insert((0, 0.0));
            fail_entry.0 += 1;
            fail_entry.1 += item.unscheduled_weight_t;

            let urgency_entry = by_urgency_map
                .entry(item.urgency_level.clone())
                .or_insert((0, 0.0));
            urgency_entry.0 += 1;
            urgency_entry.1 += item.unscheduled_weight_t;

            let candidate_unscheduled = if !item.is_scheduled { 1 } else { 0 };
            let candidate_urgency = urgency_rank(&item.urgency_level);
            let candidate_due = item.due_date.clone();
            let candidate_material_id = item.material_id.clone();

            let entry = contract_aggs
                .entry(item.contract_no.clone())
                .or_insert_with(|| ContractAggTmp {
                    contract_no: item.contract_no.clone(),
                    material_count: 0,
                    unscheduled_count: 0,
                    overdue_count: 0,
                    earliest_due_date: item.due_date.clone(),
                    max_urgency_level: item.urgency_level.clone(),
                    representative_material_id: item.material_id.clone(),
                    rep_unscheduled: candidate_unscheduled,
                    rep_urgency: candidate_urgency,
                    rep_due_date: candidate_due.clone(),
                });

            entry.material_count += 1;
            if !item.is_scheduled {
                entry.unscheduled_count += 1;
            }
            if item.days_to_due < 0 {
                entry.overdue_count += 1;
            }
            if item.due_date < entry.earliest_due_date {
                entry.earliest_due_date = item.due_date.clone();
            }
            if urgency_rank(&item.urgency_level) > urgency_rank(&entry.max_urgency_level) {
                entry.max_urgency_level = item.urgency_level.clone();
            }

            let should_replace_rep = if candidate_unscheduled != entry.rep_unscheduled {
                candidate_unscheduled > entry.rep_unscheduled
            } else if candidate_urgency != entry.rep_urgency {
                candidate_urgency > entry.rep_urgency
            } else if candidate_due != entry.rep_due_date {
                candidate_due < entry.rep_due_date
            } else {
                candidate_material_id < entry.representative_material_id
            };

            if should_replace_rep {
                entry.representative_material_id = candidate_material_id;
                entry.rep_unscheduled = candidate_unscheduled;
                entry.rep_urgency = candidate_urgency;
                entry.rep_due_date = candidate_due;
            }
        }

        let mut by_fail_type: Vec<TypeCountDto> = by_fail_type_map
            .into_iter()
            .map(|(type_name, (count, weight_t))| TypeCountDto {
                type_name,
                count,
                weight_t,
            })
            .collect();
        by_fail_type.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.type_name.cmp(&b.type_name))
        });

        let mut by_urgency: Vec<TypeCountDto> = by_urgency_map
            .into_iter()
            .map(|(type_name, (count, weight_t))| TypeCountDto {
                type_name,
                count,
                weight_t,
            })
            .collect();
        by_urgency.sort_by(|a, b| {
            urgency_rank(&b.type_name)
                .cmp(&urgency_rank(&a.type_name))
                .then_with(|| a.type_name.cmp(&b.type_name))
        });

        let mut contract_aggregates: Vec<MaterialFailureContractAggregateDto> = contract_aggs
            .into_values()
            .map(|it| MaterialFailureContractAggregateDto {
                contract_no: it.contract_no,
                material_count: it.material_count,
                unscheduled_count: it.unscheduled_count,
                overdue_count: it.overdue_count,
                earliest_due_date: it.earliest_due_date,
                max_urgency_level: it.max_urgency_level,
                representative_material_id: it.representative_material_id,
            })
            .collect();
        contract_aggregates.sort_by(|a, b| {
            b.unscheduled_count
                .cmp(&a.unscheduled_count)
                .then_with(|| b.overdue_count.cmp(&a.overdue_count))
                .then_with(|| a.earliest_due_date.cmp(&b.earliest_due_date))
                .then_with(|| a.contract_no.cmp(&b.contract_no))
        });

        let total_count = all_items.len() as u32;
        let offset = request.offset.unwrap_or(0) as usize;
        let limit = request.limit.unwrap_or(50) as usize;
        let items = all_items.into_iter().skip(offset).take(limit).collect();

        let summary = MaterialFailureSummaryDto {
            total_failed_materials: total_count,
            total_failed_contracts: contract_set.len() as u32,
            overdue_materials,
            unscheduled_materials,
            total_unscheduled_weight_t,
            by_fail_type,
            by_urgency,
        };

        Ok(MaterialFailureSetResponse {
            version_id: request.version_id,
            as_of: chrono::Utc::now().to_rfc3339(),
            items,
            total_count,
            summary,
            contract_aggregates,
        })
    }

    fn get_cold_stock_profile(
        &self,
        request: GetColdStockProfileRequest,
    ) -> Result<ColdStockProfileResponse, String> {
        let d3_use_case = self
            .d3_use_case
            .as_ref()
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

        let profiles =
            d3_use_case.get_cold_stock_profile(&request.version_id, machine_code_filter)?;
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
            items: items
                .into_iter()
                .map(|p| convert_cold_stock_to_dto(&p))
                .collect(),
            total_count,
            summary: ColdStockSummaryDto {
                total_cold_stock_count: summary.total_count as u32,
                total_cold_stock_weight_t: summary.total_weight_t,
                avg_age_days: summary.avg_age_days,
                high_pressure_count: summary.high_pressure_machines as u32,
                by_machine: summary
                    .by_machine
                    .into_iter()
                    .map(|m| MachineStockStatsDto {
                        machine_code: m.machine_code,
                        count: m.count as u32,
                        weight_t: m.weight_t,
                        avg_pressure_score: m.pressure_score,
                    })
                    .collect(),
                by_age_bin: summary
                    .by_age
                    .into_iter()
                    .map(|a| AgeBinStatsDto {
                        age_bin: a.age_bin,
                        count: a.count as u32,
                        weight_t: a.weight_t,
                    })
                    .collect(),
            },
        })
    }

    fn list_roll_campaign_alerts(
        &self,
        request: ListRollCampaignAlertsRequest,
    ) -> Result<RollCampaignAlertsResponse, String> {
        let d5_use_case = self
            .d5_use_case
            .as_ref()
            .ok_or("D5 用例未配置,请使用 new_full() 创建 DecisionApiImpl 实例".to_string())?;

        // 调用用例层
        let alert_level_param = request
            .alert_level_filter
            .as_ref()
            .and_then(|v| v.first())
            .map(|s| s.as_str());

        let alerts =
            d5_use_case.list_roll_campaign_alerts(&request.version_id, alert_level_param)?;
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
            items: items
                .into_iter()
                .map(|a| convert_roll_alert_to_dto(&a))
                .collect(),
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
        let d6_use_case = self
            .d6_use_case
            .as_ref()
            .ok_or("D6 用例未配置,请使用 new_full() 创建 DecisionApiImpl 实例".to_string())?;

        // 调用用例层
        let machine_code_param = request
            .machine_codes
            .as_ref()
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

        let summary =
            d6_use_case.get_optimization_summary(&request.version_id, start_date, end_date)?;

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
            items: items
                .into_iter()
                .map(|o| convert_capacity_opportunity_to_dto(&o))
                .collect(),
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

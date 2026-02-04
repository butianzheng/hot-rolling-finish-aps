use super::{RecalcEngine, RescheduleResult};
use crate::config::config_keys;
use crate::config::strategy_profile::CustomStrategyParameters;
use crate::domain::material::MaterialState;
use crate::domain::plan::PlanItem;
use crate::domain::roller::RollerCampaign;
use crate::domain::types::{AnchorSource, SchedState, UrgentLevel};
use crate::engine::orchestrator::ScheduleOrchestrator;
use crate::engine::strategy::ScheduleStrategy;
use crate::engine::{
    Anchor, AnchorResolver, MaterialSummary, PathRuleConfig, PathRuleEngine, SeedS2Config,
};
use crate::repository::PathOverridePendingRecord;
use chrono::NaiveDate;
use std::collections::HashMap;
use std::error::Error;

impl RecalcEngine {
    /// 执行重排 (调用完整引擎链)
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `date_range`: 日期范围 (start, end)
    /// - `machine_codes`: 机组列表
    ///
    /// # 返回
    /// - `Ok(RescheduleResult)`: 重排产结果（包含plan_items和统计信息）
    /// - `Err`: 重排失败
    ///
    /// # 红线
    /// - 必须调用完整引擎链
    /// - 未适温材料不进入当日产能池
    /// - 冻结区材料只读，不可修改
    pub fn execute_reschedule(
        &self,
        version_id: &str,
        date_range: (NaiveDate, NaiveDate),
        machine_codes: &[String],
        is_dry_run: bool,
        strategy: ScheduleStrategy,
        strategy_params: Option<CustomStrategyParameters>,
    ) -> Result<RescheduleResult, Box<dyn Error>> {
        // 检查是否已经在 tokio 运行时中
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // 已经在运行时中，使用 block_in_place 来运行异步代码
            tokio::task::block_in_place(|| {
                handle.block_on(async move {
                    self.execute_reschedule_async(
                        version_id,
                        date_range,
                        machine_codes,
                        is_dry_run,
                        strategy,
                        strategy_params,
                    )
                    .await
                })
            })
        } else {
            // 不在运行时中，创建新的运行时
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async move {
                self.execute_reschedule_async(
                    version_id,
                    date_range,
                    machine_codes,
                    is_dry_run,
                    strategy,
                    strategy_params,
                )
                .await
            })
        }
    }

    /// 异步执行重排产（内部实现）
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `date_range`: 日期范围 (start, end)
    /// - `machine_codes`: 机组列表
    ///
    /// # 返回
    /// - `Ok(RescheduleResult)`: 重排产结果
    /// - `Err`: 重排失败
    async fn execute_reschedule_async(
        &self,
        version_id: &str,
        date_range: (NaiveDate, NaiveDate),
        machine_codes: &[String],
        is_dry_run: bool,
        strategy: ScheduleStrategy,
        strategy_params: Option<CustomStrategyParameters>,
    ) -> Result<RescheduleResult, Box<dyn Error>> {
        // ===== Step 1: 查询冻结区材料（冻结区保护红线） =====
        let frozen_items = self.item_repo.find_frozen_items(version_id)?;

        let orchestrator = match strategy_params {
            Some(params) => ScheduleOrchestrator::new_with_strategy_parameters(
                self.config_manager.clone(),
                strategy,
                params,
            ),
            None => ScheduleOrchestrator::new_with_strategy(self.config_manager.clone(), strategy),
        };

        // ===== Step 2: 初始化统计 =====
        let mut all_plan_items = Vec::new();
        let mut mature_count = 0;
        let mut immature_count = 0;
        let mut total_capacity_used = 0.0;
        let mut overflow_days = 0;

        // 跟踪已排产的材料ID，避免重复排产
        let mut scheduled_material_ids: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        // 路径规则：待人工确认（由重算生成，按版本+机组+material 去重，plan_date=首次遇到的日期）
        let mut path_override_pending_records: Vec<PathOverridePendingRecord> = Vec::new();

        // 将冻结区材料加入已排产集合
        for item in &frozen_items {
            scheduled_material_ids.insert(item.material_id.clone());
        }

        // ===== PathRule / RollCycle 初始化 =====
        let parse_bool = |raw: Option<String>, default: bool| -> bool {
            match raw.as_deref().map(|s| s.trim().to_lowercase()) {
                Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "y" | "on") => true,
                Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "n" | "off") => false,
                _ => default,
            }
        };
        let parse_f64 = |raw: Option<String>, default: f64| -> f64 {
            raw.as_deref()
                .and_then(|s| s.trim().parse::<f64>().ok())
                .filter(|v| v.is_finite())
                .unwrap_or(default)
        };
        let parse_i32 = |raw: Option<String>, default: i32| -> i32 {
            raw.as_deref()
                .and_then(|s| s.trim().parse::<i32>().ok())
                .unwrap_or(default)
        };
        let parse_urgent_levels =
            |raw: Option<String>, default: Vec<UrgentLevel>| -> Vec<UrgentLevel> {
                let Some(raw) = raw else {
                    return default;
                };
                let mut levels = Vec::new();
                for token in raw.split(',').map(|s| s.trim().to_uppercase()) {
                    let level = match token.as_str() {
                        "L0" => Some(UrgentLevel::L0),
                        "L1" => Some(UrgentLevel::L1),
                        "L2" => Some(UrgentLevel::L2),
                        "L3" => Some(UrgentLevel::L3),
                        _ => None,
                    };
                    if let Some(l) = level {
                        if !levels.contains(&l) {
                            levels.push(l);
                        }
                    }
                }
                if levels.is_empty() {
                    default
                } else {
                    levels
                }
            };

        let path_rule_config = PathRuleConfig {
            enabled: parse_bool(
                self.config_manager
                    .get_global_config_value("path_rule_enabled")
                    .ok()
                    .flatten(),
                true,
            ),
            width_tolerance_mm: parse_f64(
                self.config_manager
                    .get_global_config_value("path_width_tolerance_mm")
                    .ok()
                    .flatten(),
                50.0,
            ),
            thickness_tolerance_mm: parse_f64(
                self.config_manager
                    .get_global_config_value("path_thickness_tolerance_mm")
                    .ok()
                    .flatten(),
                1.0,
            ),
            override_allowed_urgency_levels: parse_urgent_levels(
                self.config_manager
                    .get_global_config_value("path_override_allowed_urgency_levels")
                    .ok()
                    .flatten(),
                vec![UrgentLevel::L2, UrgentLevel::L3],
            ),
        };
        let path_rule_engine = PathRuleEngine::new(path_rule_config.clone());
        let path_rule_engine_ref = if path_rule_config.enabled {
            Some(&path_rule_engine)
        } else {
            None
        };

        let seed_s2_config = SeedS2Config {
            percentile: parse_f64(
                self.config_manager
                    .get_global_config_value("seed_s2_percentile")
                    .ok()
                    .flatten(),
                0.95,
            )
            .clamp(0.0, 1.0),
            small_sample_threshold: parse_i32(
                self.config_manager
                    .get_global_config_value("seed_s2_small_sample_threshold")
                    .ok()
                    .flatten(),
                10,
            )
            .max(1),
        };
        let anchor_resolver = AnchorResolver::new(seed_s2_config);

        let roll_suggest_threshold_t = parse_f64(
            self.config_manager
                .get_global_config_value(config_keys::ROLL_SUGGEST_THRESHOLD_T)
                .ok()
                .flatten(),
            1500.0,
        );
        let roll_hard_limit_t = parse_f64(
            self.config_manager
                .get_global_config_value(config_keys::ROLL_HARD_LIMIT_T)
                .ok()
                .flatten(),
            2500.0,
        );
        let (roll_suggest_threshold_t, roll_hard_limit_t) =
            if roll_hard_limit_t > roll_suggest_threshold_t {
                (roll_suggest_threshold_t, roll_hard_limit_t)
            } else {
                (1500.0, 2500.0)
            };

        // ===== Step 3: 多日循环 =====
        let (start_date, end_date) = date_range;

        // 为本次版本准备活跃换辊周期（用于持久化锚点）
        let mut active_campaigns: HashMap<String, RollerCampaign> = HashMap::new();
        for machine_code in machine_codes {
            let campaign = if is_dry_run {
                let mut c = RollerCampaign::new(
                    version_id.to_string(),
                    machine_code.clone(),
                    1,
                    start_date,
                    Some(roll_suggest_threshold_t),
                    Some(roll_hard_limit_t),
                );
                c.anchor_source = Some(AnchorSource::None);
                c
            } else {
                match self
                    .roller_campaign_repo
                    .find_active_campaign(version_id, machine_code)?
                {
                    Some(c) => c,
                    None => {
                        let mut c = RollerCampaign::new(
                            version_id.to_string(),
                            machine_code.clone(),
                            1,
                            start_date,
                            Some(roll_suggest_threshold_t),
                            Some(roll_hard_limit_t),
                        );
                        c.anchor_source = Some(AnchorSource::None);
                        self.roller_campaign_repo.create(&c)?;
                        c
                    }
                }
            };
            active_campaigns.insert(machine_code.clone(), campaign);
        }

        let mut current_date = start_date;

        while current_date <= end_date {
            // ===== Step 4: 多机组循环 =====
            for machine_code in machine_codes {
                // ----- 4.1 查询候选材料 -----
                let materials = self.material_master_repo.find_by_machine(machine_code)?;

                // ----- 4.2 查询材料状态 -----
                let states: Vec<MaterialState> = materials
                    .iter()
                    .filter_map(|m| {
                        self.material_state_repo
                            .find_by_id(&m.material_id)
                            .ok()
                            .flatten()
                    })
                    .collect();

                // ----- 4.3 过滤适温材料（适温约束红线） -----
                // 同时排除已排产的材料，避免重复排产
                let total_materials = materials.len();
                let (ready_materials, ready_states): (Vec<_>, Vec<_>) = materials
                    .into_iter()
                    .zip(states.into_iter())
                    .filter(|(material, state)| {
                        // 排除已排产的材料
                        if scheduled_material_ids.contains(&material.material_id) {
                            return false;
                        }
                        // 只有READY, LOCKED, FORCE_RELEASE可以排产
                        state.sched_state == SchedState::Ready
                            || state.sched_state == SchedState::Locked
                            || state.sched_state == SchedState::ForceRelease
                    })
                    .unzip();

                // 统计成熟/未成熟数量
                let ready_count = ready_materials.len();
                mature_count += ready_count;
                immature_count += total_materials - ready_count;

                // ----- 4.4 查询或创建产能池 -----
                let mut capacity_pool = self
                    .capacity_repo
                    .find_by_machine_and_date(version_id, machine_code, current_date)?
                    .unwrap_or_else(|| {
                        Self::create_default_capacity_pool(version_id, machine_code, current_date)
                    });

                // ----- 4.5 提取当日冻结项 -----
                let frozen_for_today: Vec<PlanItem> = frozen_items
                    .iter()
                    .filter(|item| item.plan_date == current_date && &item.machine_code == machine_code)
                    .cloned()
                    .collect();

                // 无候选且无冻结项：跳过本次排产
                if ready_materials.is_empty() && frozen_for_today.is_empty() {
                    continue;
                }

                // ----- 4.5.1 解析当日初始锚点（用于 PathRuleEngine 门控） -----
                let (initial_anchor, initial_anchor_material_id) = if path_rule_config.enabled {
                    let campaign = active_campaigns
                        .get_mut(machine_code.as_str())
                        .ok_or_else(|| format!("active roll campaign missing: {}", machine_code))?;

                    // A) 若当日存在冻结项，则锚点优先取冻结区最后一块（seq_no 最大）
                    let mut anchor = None;
                    let mut anchor_material_id = None;

                    if let Some(last_frozen) = frozen_for_today.iter().max_by_key(|i| i.seq_no) {
                        if let Some(master) =
                            self.material_master_repo.find_by_id(&last_frozen.material_id)?
                        {
                            let w = master.width_mm.unwrap_or(0.0);
                            let t = master.thickness_mm.unwrap_or(0.0);
                            if w.is_finite() && t.is_finite() && w > 0.0 && t > 0.0 {
                                anchor = Some(Anchor {
                                    width_mm: w,
                                    thickness_mm: t,
                                });
                                anchor_material_id = Some(last_frozen.material_id.clone());
                            }
                        }
                    }

                    // B) 否则尝试使用已持久化的 campaign 锚点
                    if anchor.is_none() && campaign.has_valid_anchor() {
                        let w = campaign.path_anchor_width_mm.unwrap_or(0.0);
                        let t = campaign.path_anchor_thickness_mm.unwrap_or(0.0);
                        if w.is_finite() && t.is_finite() && w > 0.0 && t > 0.0 {
                            anchor = Some(Anchor {
                                width_mm: w,
                                thickness_mm: t,
                            });
                            anchor_material_id = campaign.path_anchor_material_id.clone();
                        }
                    }

                    // C) 若仍无锚点：按优先级解析（FrozenLast/LockedLast/UserConfirmedLast/SeedS2）
                    if anchor.is_none() {
                        // 冻结区最后一块（全量冻结区口径）
                        let frozen_last = frozen_items
                            .iter()
                            .filter(|i| &i.machine_code == machine_code)
                            .max_by_key(|i| (i.plan_date, i.seq_no));
                        let frozen_summaries: Vec<MaterialSummary> = match frozen_last {
                            Some(item) => {
                                if let Some(m) = self.material_master_repo.find_by_id(&item.material_id)?
                                {
                                    let w = m.width_mm.unwrap_or(0.0);
                                    let t = m.thickness_mm.unwrap_or(0.0);
                                    if w.is_finite() && t.is_finite() && w > 0.0 && t > 0.0 {
                                        vec![MaterialSummary {
                                            material_id: item.material_id.clone(),
                                            width_mm: w,
                                            thickness_mm: t,
                                            seq_no: item.seq_no,
                                            user_confirmed_at: None,
                                        }]
                                    } else {
                                        Vec::new()
                                    }
                                } else {
                                    Vec::new()
                                }
                            }
                            None => Vec::new(),
                        };

                        // 锁定区最后一块（本日候选中 sched_state=LOCKED）
                        let locked_summaries: Vec<MaterialSummary> = ready_materials
                            .iter()
                            .zip(ready_states.iter())
                            .filter(|(_, s)| s.sched_state == SchedState::Locked)
                            .filter_map(|(m, s)| {
                                let w = m.width_mm.unwrap_or(0.0);
                                let t = m.thickness_mm.unwrap_or(0.0);
                                if !(w.is_finite() && t.is_finite() && w > 0.0 && t > 0.0) {
                                    return None;
                                }
                                Some(MaterialSummary {
                                    material_id: m.material_id.clone(),
                                    width_mm: w,
                                    thickness_mm: t,
                                    seq_no: s.seq_no.unwrap_or(0),
                                    user_confirmed_at: None,
                                })
                            })
                            .collect();

                        // 人工确认队列（机组口径，按 user_confirmed_at 排序）
                        let user_confirmed_summaries: Vec<MaterialSummary> = self
                            .material_state_repo
                            .list_user_confirmed_materials(machine_code)?
                            .into_iter()
                            .filter_map(|u| {
                                let w = u.width_mm;
                                let t = u.thickness_mm;
                                if !(w.is_finite() && t.is_finite() && w > 0.0 && t > 0.0) {
                                    return None;
                                }
                                Some(MaterialSummary {
                                    material_id: u.material_id,
                                    width_mm: w,
                                    thickness_mm: t,
                                    seq_no: u.seq_no.unwrap_or(0),
                                    user_confirmed_at: u.user_confirmed_at,
                                })
                            })
                            .collect();

                        // 候选池（用于 SeedS2）
                        let candidate_summaries: Vec<MaterialSummary> = ready_materials
                            .iter()
                            .zip(ready_states.iter())
                            .filter_map(|(m, _s)| {
                                let w = m.width_mm.unwrap_or(0.0);
                                let t = m.thickness_mm.unwrap_or(0.0);
                                if !(w.is_finite() && t.is_finite() && w > 0.0 && t > 0.0) {
                                    return None;
                                }
                                Some(MaterialSummary {
                                    material_id: m.material_id.clone(),
                                    width_mm: w,
                                    thickness_mm: t,
                                    seq_no: 0,
                                    user_confirmed_at: None,
                                })
                            })
                            .collect();

                        let resolved = anchor_resolver.resolve(
                            &frozen_summaries,
                            &locked_summaries,
                            &user_confirmed_summaries,
                            &candidate_summaries,
                        );

                        if let Some(a) = resolved.anchor {
                            campaign.update_anchor(
                                resolved.material_id.clone(),
                                a.width_mm,
                                a.thickness_mm,
                                resolved.source,
                            );

                            if !is_dry_run {
                                self.roller_campaign_repo.update_campaign_anchor(
                                    version_id,
                                    machine_code,
                                    campaign.campaign_no,
                                    campaign.path_anchor_material_id.as_deref(),
                                    campaign.path_anchor_width_mm,
                                    campaign.path_anchor_thickness_mm,
                                    campaign.anchor_source.unwrap_or(AnchorSource::None),
                                )?;
                            }

                            anchor = Some(a);
                            anchor_material_id = resolved.material_id;
                        } else {
                            campaign.reset_anchor();
                            if !is_dry_run {
                                self.roller_campaign_repo.update_campaign_anchor(
                                    version_id,
                                    machine_code,
                                    campaign.campaign_no,
                                    None,
                                    None,
                                    None,
                                    AnchorSource::None,
                                )?;
                            }
                            anchor = None;
                            anchor_material_id = None;
                        }
                    }

                    (anchor, anchor_material_id)
                } else {
                    (None, None)
                };

                // ----- 4.6 查询结构目标配比 -----
                let target_ratio = self
                    .config_manager
                    .get_target_ratio()
                    .await
                    .unwrap_or_else(|e| {
                        tracing::warn!("加载目标配比配置失败: {}, 使用空配置", e);
                        HashMap::new()
                    });
                let deviation_threshold = self
                    .config_manager
                    .get_deviation_threshold()
                    .await
                    .unwrap_or_else(|e| {
                        tracing::warn!("加载偏差阈值配置失败: {}, 使用默认值 0.1", e);
                        0.1
                    });

                // ----- 4.7 创建编排器并执行单日排产 -----
                let schedule_result = orchestrator
                    .execute_single_day_schedule_with_path_rule(
                        ready_materials,
                        ready_states,
                        &mut capacity_pool,
                        frozen_for_today,
                        &target_ratio,
                        deviation_threshold,
                        current_date,
                        version_id,
                        path_rule_engine_ref,
                        initial_anchor,
                        initial_anchor_material_id,
                    )
                    .await?;

                // ----- 4.7.2 收集路径规则待确认（由 CapacityFiller 产生，供上层落库/汇总） -----
                if !schedule_result.path_override_pending.is_empty() {
                    for p in &schedule_result.path_override_pending {
                        path_override_pending_records.push(PathOverridePendingRecord {
                            version_id: version_id.to_string(),
                            machine_code: machine_code.clone(),
                            plan_date: current_date,
                            material_id: p.material_id.clone(),
                            violation_type: p.violation_type.clone(),
                            urgent_level: p.urgent_level.clone(),
                            width_mm: p.width_mm,
                            thickness_mm: p.thickness_mm,
                            anchor_width_mm: p.anchor_width_mm,
                            anchor_thickness_mm: p.anchor_thickness_mm,
                            width_delta_mm: p.width_delta_mm,
                            thickness_delta_mm: p.thickness_delta_mm,
                        });
                    }
                }

                // ----- 4.7.1 持久化 RollCycle 锚点（供后续日期与前端查询使用） -----
                if path_rule_config.enabled {
                    if let Some(campaign) = active_campaigns.get_mut(machine_code.as_str()) {
                        if let Some(anchor) = schedule_result.roll_cycle_anchor {
                            campaign.path_anchor_width_mm = Some(anchor.width_mm);
                            campaign.path_anchor_thickness_mm = Some(anchor.thickness_mm);
                            campaign.path_anchor_material_id =
                                schedule_result.roll_cycle_anchor_material_id.clone();
                        }

                        if campaign.anchor_source.is_none() {
                            campaign.anchor_source = Some(AnchorSource::None);
                        }

                        if !is_dry_run {
                            self.roller_campaign_repo.update_campaign_anchor(
                                version_id,
                                machine_code,
                                campaign.campaign_no,
                                campaign.path_anchor_material_id.as_deref(),
                                campaign.path_anchor_width_mm,
                                campaign.path_anchor_thickness_mm,
                                campaign.anchor_source.unwrap_or(AnchorSource::None),
                            )?;
                        }
                    }
                }

                // ----- 4.8 收集排产结果 -----
                // 将新排产的材料ID加入已排产集合，避免后续日期重复排产
                for item in &schedule_result.plan_items {
                    scheduled_material_ids.insert(item.material_id.clone());
                }
                all_plan_items.extend(schedule_result.plan_items);
                total_capacity_used += schedule_result.updated_capacity_pool.used_capacity_t;

                // 统计超限天数
                if schedule_result.updated_capacity_pool.overflow_t > 0.0 {
                    overflow_days += 1;
                }

                // ----- 4.9 更新产能池（写回数据库） -----
                if !is_dry_run {
                    self.capacity_repo
                        .upsert_single(&schedule_result.updated_capacity_pool)?;
                }

                // ----- 4.10 持久化修改的材料状态（urgent_level, rush_level 等） -----
                // Orchestrator 更新了 eligible_materials 中的状态，必须持久化到数据库
                // 否则紧急等级判定结果会丢失
                let updated_states: Vec<MaterialState> = schedule_result
                    .eligible_materials
                    .into_iter()
                    .map(|(_, state)| state)
                    .collect();

                if !is_dry_run && !updated_states.is_empty() {
                    self.material_state_repo
                        .batch_insert_material_state(updated_states)?;
                    tracing::debug!(
                        machine_code = %capacity_pool.machine_code,
                        plan_date = %capacity_pool.plan_date,
                        "材料状态已持久化（包含紧急等级更新）"
                    );
                }
            }

            current_date += chrono::Duration::days(1);
        }

        // ===== Step 4.11: 持久化路径规则待确认（仅生产模式） =====
        if !is_dry_run {
            if let Err(e) = self.path_override_pending_repo.ensure_schema() {
                tracing::warn!("path_override_pending 表初始化失败(将继续返回重算结果): {}", e);
            } else if !path_override_pending_records.is_empty() {
                match self
                    .path_override_pending_repo
                    .insert_ignore_many(&path_override_pending_records)
                {
                    Ok(inserted) => {
                        tracing::info!(
                            version_id = %version_id,
                            inserted = inserted,
                            total_collected = path_override_pending_records.len(),
                            "路径规则待确认已落库"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            version_id = %version_id,
                            "路径规则待确认落库失败(将继续返回重算结果): {}",
                            e
                        );
                    }
                }
            }
        }

        // ===== Step 5: 返回结果 =====
        Ok(RescheduleResult {
            plan_items: all_plan_items,
            mature_count,
            immature_count,
            total_capacity_used,
            overflow_days,
        })
    }
}


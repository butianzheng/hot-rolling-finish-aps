use super::*;

impl PlanApi {
    // ==========================================
    // 策略草案接口（多策略对比）
    // ==========================================

    /// 生成多策略草案（dry-run 试算，草案落库持久化）
    ///
    /// # 说明
    /// - 排产计算采用 dry-run 模式：不写 plan_item / risk_snapshot / capacity_pool / material_state；
    /// - 草案本身会写入 decision_strategy_draft（避免刷新/重启丢失；支持并发/审计）；
    /// - 草案发布时必须再走一次生产模式重算（生成正式版本），保证审计与可追溯。
    pub fn generate_strategy_drafts(
        &self,
        base_version_id: &str,
        plan_date_from: NaiveDate,
        plan_date_to: NaiveDate,
        strategies: Vec<String>,
        operator: &str,
    ) -> ApiResult<GenerateStrategyDraftsResponse> {
        if base_version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("基准版本ID不能为空".to_string()));
        }
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }
        if strategies.is_empty() {
            return Err(ApiError::InvalidInput("策略列表不能为空".to_string()));
        }

        let (from, to) = if plan_date_to < plan_date_from {
            (plan_date_to, plan_date_from)
        } else {
            (plan_date_from, plan_date_to)
        };

        let range_days = (to - from).num_days();
        if range_days > 60 {
            return Err(ApiError::InvalidInput("时间跨度过大，最多支持60天".to_string()));
        }

        // 校验基准版本存在
        let base_version = self
            .plan_version_repo
            .find_by_id(base_version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", base_version_id)))?;

        // 仅允许针对当前激活版本生成草案，避免“草案发布”时基准漂移导致不可复现。
        let active_version = self
            .plan_version_repo
            .find_active_version(&base_version.plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::InvalidInput("当前方案没有激活版本，无法生成草案".to_string()))?;

        if active_version.version_id != base_version_id {
            return Err(ApiError::VersionConflict(format!(
                "基准版本已变更：草案基于 {}，当前激活版本为 {}。请刷新后重新生成草案。",
                base_version_id, active_version.version_id
            )));
        }

        // 基准版本在时间范围内的快照（用于 diff）
        let base_items_in_range = self
            .plan_item_repo
            .find_by_date_range(base_version_id, from, to)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 冻结项（locked_in_plan=1）在范围内需要计入草案快照，否则会被误判为“挤出”
        let frozen_items_in_range: Vec<PlanItem> = self
            .plan_item_repo
            .find_frozen_items(base_version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .into_iter()
            .filter(|item| item.plan_date >= from && item.plan_date <= to)
            .collect();

        // 与 RecalcEngine 默认一致：固定三条机组（后续可改为从配置/机组表动态加载）
        let machine_codes = vec!["H032".to_string(), "H033".to_string(), "H034".to_string()];

        let now = chrono::Local::now().naive_local();
        let expires_at = now + chrono::Duration::hours(72);
        let mut summaries = Vec::new();

        let mut seen: HashSet<String> = HashSet::new();
        for raw_strategy_key in strategies {
            let raw_strategy_key = raw_strategy_key.trim().to_string();
            if raw_strategy_key.is_empty() {
                continue;
            }
            if !seen.insert(raw_strategy_key.clone()) {
                continue;
            }

            let profile = self
                .recalc_engine
                .resolve_strategy_profile(&raw_strategy_key)
                .map_err(|e| ApiError::InvalidInput(format!("策略解析失败（{}）: {}", raw_strategy_key, e)))?;

            let draft_id = uuid::Uuid::new_v4().to_string();

            let reschedule = self
                .recalc_engine
                .execute_reschedule(
                    base_version_id,
                    (from, to),
                    &machine_codes,
                    true,
                    profile.base_strategy,
                    profile.parameters.clone(),
                )
                .map_err(|e| ApiError::InternalError(format!("生成草案失败: {}", e)))?;

            let mature_count = reschedule.mature_count;
            let immature_count = reschedule.immature_count;
            let total_capacity_used_t = reschedule.total_capacity_used;
            let overflow_days = reschedule.overflow_days;
            let reschedule_items = reschedule.plan_items;

            let mut draft_items_in_range: Vec<PlanItem> = Vec::with_capacity(
                frozen_items_in_range.len() + reschedule_items.len(),
            );

            for mut item in frozen_items_in_range.clone() {
                item.version_id = draft_id.clone();
                draft_items_in_range.push(item);
            }

            let frozen_items_count = frozen_items_in_range.len();
            let mut calc_items_count = 0usize;

            for mut item in reschedule_items.into_iter() {
                if item.plan_date < from || item.plan_date > to {
                    continue;
                }
                item.version_id = draft_id.clone();
                draft_items_in_range.push(item);
                calc_items_count += 1;
            }

            let (
                moved_count,
                added_count,
                removed_count,
                squeezed_out_count,
                diff_items,
                diff_items_total,
                diff_items_truncated,
            ) = Self::diff_plan_items_detail(&base_items_in_range, &draft_items_in_range);

            let summary = StrategyDraftSummary {
                draft_id: draft_id.clone(),
                base_version_id: base_version_id.to_string(),
                strategy: profile.strategy_key.clone(),
                plan_items_count: draft_items_in_range.len(),
                frozen_items_count,
                calc_items_count,
                mature_count,
                immature_count,
                total_capacity_used_t,
                overflow_days,
                moved_count,
                added_count,
                removed_count,
                squeezed_out_count,
                message: format!(
                    "{} | 排产{}(冻结{}+新排{}) | 成熟{} 未成熟{} | 预计产量{:.1}t | 超限机组日{} | 移动{} 新增{} 挤出{}",
                    profile.title_cn.as_str(),
                    draft_items_in_range.len(),
                    frozen_items_count,
                    calc_items_count,
                    mature_count,
                    immature_count,
                    total_capacity_used_t,
                    overflow_days,
                    moved_count,
                    added_count,
                    squeezed_out_count
                ),
            };

            let params_json = profile.parameters_json();
            let params_json = if params_json.is_null() {
                None
            } else {
                Some(params_json.to_string())
            };

            let summary_json = serde_json::to_string(&summary)
                .map_err(|e| ApiError::InternalError(format!("序列化草案摘要失败: {}", e)))?;
            let diff_items_json = serde_json::to_string(&diff_items)
                .map_err(|e| ApiError::InternalError(format!("序列化草案变更明细失败: {}", e)))?;

            let entity = StrategyDraftEntity {
                draft_id: draft_id.clone(),
                base_version_id: base_version_id.to_string(),
                plan_date_from: from,
                plan_date_to: to,
                strategy_key: profile.strategy_key.clone(),
                strategy_base: profile.base_strategy.as_str().to_string(),
                strategy_title_cn: profile.title_cn.clone(),
                strategy_params_json: params_json,
                status: StrategyDraftStatus::Draft,
                created_by: operator.to_string(),
                created_at: now,
                expires_at,
                published_as_version_id: None,
                published_by: None,
                published_at: None,
                locked_by: None,
                locked_at: None,
                summary_json,
                diff_items_json,
                diff_items_total: diff_items_total as i64,
                diff_items_truncated,
            };

            self.strategy_draft_repo
                .insert(&entity)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

            summaries.push(summary);
        }

        let draft_count = summaries.len();

        Ok(GenerateStrategyDraftsResponse {
            base_version_id: base_version_id.to_string(),
            plan_date_from: from,
            plan_date_to: to,
            drafts: summaries,
            message: format!("已生成{}个策略草案", draft_count),
        })
    }

    /// 发布策略草案：生成正式版本（落库）
    pub fn apply_strategy_draft(
        &self,
        draft_id: &str,
        operator: &str,
    ) -> ApiResult<ApplyStrategyDraftResponse> {
        if draft_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("草案ID不能为空".to_string()));
        }
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }

        // best-effort: 先尝试将过期草案标记为 EXPIRED，避免误发布
        if let Err(e) = self.strategy_draft_repo.expire_if_needed(draft_id) {
            tracing::warn!("expire_if_needed failed: {}", e);
        }

        let record = self
            .strategy_draft_repo
            .find_by_id(draft_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("草案{}不存在或已过期", draft_id)))?;

        if record.status != StrategyDraftStatus::Draft {
            return Err(ApiError::InvalidInput(format!(
                "草案状态不允许发布: {}",
                record.status.as_str()
            )));
        }

        // 并发保护：发布前加锁（best-effort）
        let lock_rows = self
            .strategy_draft_repo
            .try_lock_for_publish(draft_id, operator)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        if lock_rows == 0 {
            return Err(ApiError::VersionConflict(
                "草案已被其他用户锁定、已过期或状态已变更，请刷新后重试".to_string(),
            ));
        }

        // 校验基准版本仍为激活版本，避免基准漂移导致“发布结果不可复现”
        let base_version = self
            .plan_version_repo
            .find_by_id(&record.base_version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", record.base_version_id)))?;

        let active_version = self
            .plan_version_repo
            .find_active_version(&base_version.plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::InvalidInput("当前方案没有激活版本，无法发布草案".to_string()))?;

        if active_version.version_id != record.base_version_id {
            return Err(ApiError::VersionConflict(format!(
                "基准版本已变更：草案基于 {}，当前激活版本为 {}。请重新生成草案后再发布。",
                record.base_version_id, active_version.version_id
            )));
        }

        let window_days_i64 = (record.plan_date_to - record.plan_date_from).num_days();
        if window_days_i64 < 0 {
            return Err(ApiError::InvalidInput("草案日期范围非法".to_string()));
        }
        if window_days_i64 > 60 {
            return Err(ApiError::InvalidInput("草案时间跨度过大，最多支持60天".to_string()));
        }
        let window_days = window_days_i64 as i32;

        // 从草案快照中恢复策略 profile（避免发布时策略漂移导致不可复现）
        let base_strategy = record
            .strategy_base
            .parse::<ScheduleStrategy>()
            .map_err(|e| ApiError::InvalidInput(format!("草案策略解析失败: {}", e)))?;
        let parameters = match record.strategy_params_json.as_deref() {
            Some(raw) if !raw.trim().is_empty() && raw.trim() != "null" => {
                Some(serde_json::from_str(raw).map_err(|e| {
                    ApiError::InvalidInput(format!("草案参数解析失败: {}", e))
                })?)
            }
            _ => None,
        };
        let profile = ResolvedStrategyProfile {
            strategy_key: record.strategy_key.clone(),
            base_strategy,
            title_cn: record.strategy_title_cn.clone(),
            parameters,
        };

        let result = match self.recalc_engine.recalc_full_with_profile(
            &base_version.plan_id,
            record.plan_date_from,
            window_days,
            operator,
            false,
            profile.clone(),
        ) {
            Ok(v) => v,
            Err(e) => {
                // best-effort: 释放锁，避免草案长期处于 locked 状态
                if let Err(unlock_err) =
                    self.strategy_draft_repo.unlock(draft_id, operator)
                {
                    tracing::warn!("unlock draft failed: {}", unlock_err);
                }
                return Err(ApiError::InternalError(format!("发布草案失败: {}", e)));
            }
        };

        // 标记草案已发布（best-effort：版本已生成，失败也不应阻塞主流程）
        let published_at = chrono::Local::now().naive_local();
        if let Err(e) = self.strategy_draft_repo.mark_published(
            draft_id,
            &result.version_id,
            operator,
            published_at,
        ) {
            tracing::warn!("mark_published failed: {}", e);
        }

        // 审计记录：发布草案属于“决策行为”，需要落 ActionLog
        let log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(result.version_id.clone()),
            action_type: "APPLY_STRATEGY_DRAFT".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "draft_id": draft_id,
                "base_version_id": record.base_version_id,
                "plan_date_from": record.plan_date_from.to_string(),
                "plan_date_to": record.plan_date_to.to_string(),
                "window_days": window_days,
                "strategy": profile.strategy_key,
                "strategy_base": profile.base_strategy.as_str(),
                "strategy_title_cn": profile.title_cn,
                "parameters": profile.parameters_json(),
            })),
            impact_summary_json: Some(serde_json::json!({
                "plan_items_count": result.total_items,
                "frozen_items_count": result.frozen_items,
                "mature_count": result.mature_count,
                "immature_count": result.immature_count,
                "elapsed_ms": result.elapsed_ms,
            })),
            machine_code: None,
            date_range_start: Some(record.plan_date_from),
            date_range_end: Some(record.plan_date_to),
            detail: Some(format!("发布策略草案: {} ({})", record.strategy_title_cn.as_str(), draft_id)),
        };

        self.action_log_repo
            .insert(&log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(ApplyStrategyDraftResponse {
            version_id: result.version_id,
            success: true,
            message: "草案已发布，已生成正式版本".to_string(),
        })
    }

    /// 查询策略草案变更明细（用于前端解释对比）
    pub fn get_strategy_draft_detail(
        &self,
        draft_id: &str,
    ) -> ApiResult<GetStrategyDraftDetailResponse> {
        if draft_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("草案ID不能为空".to_string()));
        }

        // best-effort: 先尝试将过期草案标记为 EXPIRED
        if let Err(e) = self.strategy_draft_repo.expire_if_needed(draft_id) {
            tracing::warn!("expire_if_needed failed: {}", e);
        }

        let record = self
            .strategy_draft_repo
            .find_by_id(draft_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("草案{}不存在或已过期", draft_id)))?;

        let mut diff_items: Vec<StrategyDraftDiffItem> = serde_json::from_str(&record.diff_items_json)
            .map_err(|e| ApiError::InternalError(format!("解析草案变更明细失败: {}", e)))?;

        let mut message = if record.diff_items_truncated {
            format!(
                "变更明细过多，已截断展示 {}/{} 条",
                diff_items.len(),
                record.diff_items_total
            )
        } else {
            "OK".to_string()
        };

        // best-effort: 为“挤出”项补充 material_state 快照，减少前端逐条查库
        let squeezed_ids: Vec<String> = diff_items
            .iter()
            .filter(|it| it.change_type == "SQUEEZED_OUT")
            .map(|it| it.material_id.clone())
            .collect();
        if !squeezed_ids.is_empty() {
            match self
                .material_state_repo
                .find_snapshots_by_material_ids(&squeezed_ids)
            {
                Ok(list) => {
                    let map: HashMap<String, MaterialStateSnapshotLite> = list
                        .into_iter()
                        .map(|s| (s.material_id.clone(), s))
                        .collect();
                    for it in diff_items.iter_mut() {
                        if it.change_type != "SQUEEZED_OUT" {
                            continue;
                        }
                        it.material_state_snapshot = map.get(&it.material_id).cloned();
                    }
                }
                Err(e) => {
                    message = format!("{}（material_state 快照加载失败：{}）", message, e);
                }
            }
        }

        Ok(GetStrategyDraftDetailResponse {
            draft_id: draft_id.to_string(),
            base_version_id: record.base_version_id,
            plan_date_from: record.plan_date_from,
            plan_date_to: record.plan_date_to,
            strategy: record.strategy_key,
            diff_items,
            diff_items_total: record.diff_items_total as usize,
            diff_items_truncated: record.diff_items_truncated,
            message,
        })
    }

    /// 列出并恢复指定基准版本 + 日期范围内的草案（默认：每个策略仅返回最新一条）
    pub fn list_strategy_drafts(
        &self,
        base_version_id: &str,
        plan_date_from: NaiveDate,
        plan_date_to: NaiveDate,
        status_filter: Option<String>,
        limit: Option<i64>,
    ) -> ApiResult<ListStrategyDraftsResponse> {
        if base_version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("基准版本ID不能为空".to_string()));
        }

        let (from, to) = if plan_date_to < plan_date_from {
            (plan_date_to, plan_date_from)
        } else {
            (plan_date_from, plan_date_to)
        };

        let range_days = (to - from).num_days();
        if range_days > 60 {
            return Err(ApiError::InvalidInput("时间跨度过大，最多支持60天".to_string()));
        }

        let status = status_filter
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(StrategyDraftStatus::parse);

        let rows = self
            .strategy_draft_repo
            .list_by_base_version_and_range(base_version_id, from, to, status, limit.unwrap_or(200))
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let now = chrono::Local::now().naive_local();
        let mut seen: HashSet<String> = HashSet::new();
        let mut drafts: Vec<StrategyDraftSummary> = Vec::new();
        let mut expired_count = 0usize;
        let mut parse_failed = 0usize;

        for record in rows.into_iter() {
            // best-effort: 避免把已过期但未标记的草案返回给前端
            if record.status == StrategyDraftStatus::Draft && record.expires_at <= now {
                expired_count += 1;
                let _ = self.strategy_draft_repo.expire_if_needed(&record.draft_id);
                continue;
            }

            // 每个策略只取最新一条（query 已按 created_at DESC 排序）
            if !seen.insert(record.strategy_key.clone()) {
                continue;
            }

            match serde_json::from_str::<StrategyDraftSummary>(&record.summary_json) {
                Ok(mut summary) => {
                    // 防御：以 DB 为准覆盖关键字段，避免历史数据格式漂移
                    summary.draft_id = record.draft_id;
                    summary.base_version_id = record.base_version_id;
                    summary.strategy = record.strategy_key;
                    drafts.push(summary);
                }
                Err(e) => {
                    parse_failed += 1;
                    tracing::warn!(
                        "failed to parse decision_strategy_draft.summary_json: draft_id={}, err={}",
                        record.draft_id,
                        e
                    );
                }
            }
        }

        let mut message = format!("已找到{}个草案", drafts.len());
        if expired_count > 0 {
            message = format!("{}（{}个已过期）", message, expired_count);
        }
        if parse_failed > 0 {
            message = format!("{}（{}个解析失败）", message, parse_failed);
        }

        Ok(ListStrategyDraftsResponse {
            base_version_id: base_version_id.to_string(),
            plan_date_from: from,
            plan_date_to: to,
            drafts,
            message,
        })
    }

    /// 清理过期草案（默认保留 7 天，最大 90 天）
    pub fn cleanup_expired_strategy_drafts(
        &self,
        keep_days: i64,
    ) -> ApiResult<CleanupStrategyDraftsResponse> {
        let deleted_count = self
            .strategy_draft_repo
            .cleanup_expired(keep_days)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(CleanupStrategyDraftsResponse {
            deleted_count,
            message: format!("已清理{}条过期草案", deleted_count),
        })
    }

    /// 获取预设策略列表（用于前端展示/默认对比）
    pub fn get_strategy_presets(&self) -> ApiResult<Vec<StrategyPreset>> {
        Ok(vec![
            StrategyPreset {
                strategy: ScheduleStrategy::Balanced,
                title: "均衡方案".to_string(),
                description: "在交付/产能/库存之间保持均衡".to_string(),
                default_parameters: serde_json::json!({
                    "order_keys": [
                        { "key": "sched_state", "order": "FORCE_RELEASE > LOCKED", "note": "红线：强制放行/锁定优先" },
                        { "key": "stock_age_days", "order": "desc", "note": "库龄大优先（冷料优先）" },
                        { "key": "rolling_output_age_days", "order": "desc", "note": "出炉更久优先" },
                        { "key": "due_date", "order": "asc", "note": "交期更早优先" },
                    ],
                    "parameter_template": {
                        "urgent_weight": 10,
                        "capacity_weight": 3,
                        "cold_stock_weight": 2,
                        "due_date_weight": 2,
                        "rolling_output_age_weight": 1,
                        "cold_stock_age_threshold_days": 30,
                        "overflow_tolerance_pct": null,
                    },
                    "notes": [
                        "预设策略按固定排序规则执行，不读取 parameter_template。",
                        "自定义策略 custom:* 若设置参数，会在等级内排序中按加权评分排序；同分时回落到基于预设策略排序。",
                    ],
                }),
            },
            StrategyPreset {
                strategy: ScheduleStrategy::UrgentFirst,
                title: "紧急优先".to_string(),
                description: "优先保障 L3/L2 紧急订单".to_string(),
                default_parameters: serde_json::json!({
                    "order_keys": [
                        { "key": "sched_state", "order": "FORCE_RELEASE > LOCKED", "note": "红线：强制放行/锁定优先" },
                        { "key": "urgent_level", "order": "desc", "note": "L3 > L2 > L1 > L0" },
                        { "key": "due_date", "order": "asc", "note": "交期更早优先" },
                        { "key": "stock_age_days", "order": "desc", "note": "库龄大优先（冷料兜底）" },
                        { "key": "rolling_output_age_days", "order": "desc", "note": "出炉更久优先" },
                    ],
                    "parameter_template": {
                        "urgent_weight": 60,
                        "capacity_weight": 1,
                        "cold_stock_weight": 2,
                        "due_date_weight": 10,
                        "rolling_output_age_weight": 1,
                        "cold_stock_age_threshold_days": 0,
                        "overflow_tolerance_pct": null,
                    },
                    "notes": [
                        "预设策略按 fixed order_keys 执行，适合紧急单保障。",
                        "若你需要更细粒度的“紧急内排序”，可复制 parameter_template 作为自定义策略起点。",
                    ],
                }),
            },
            StrategyPreset {
                strategy: ScheduleStrategy::CapacityFirst,
                title: "产能优先".to_string(),
                description: "优先提升产能利用率，减少溢出".to_string(),
                default_parameters: serde_json::json!({
                    "order_keys": [
                        { "key": "sched_state", "order": "FORCE_RELEASE > LOCKED", "note": "红线：强制放行/锁定优先" },
                        { "key": "weight_t", "order": "desc", "note": "吨位更大优先（更容易填满产能）" },
                        { "key": "due_date", "order": "asc", "note": "交期兜底" },
                        { "key": "stock_age_days", "order": "desc", "note": "库龄兜底（冷料）" },
                    ],
                    "parameter_template": {
                        "urgent_weight": 5,
                        "capacity_weight": 20,
                        "cold_stock_weight": 1,
                        "due_date_weight": 3,
                        "rolling_output_age_weight": 0,
                        "cold_stock_age_threshold_days": 0,
                        "overflow_tolerance_pct": null,
                    },
                    "notes": [
                        "预设策略更偏“快速吃满产能”，适用于产能压力较大时。",
                    ],
                }),
            },
            StrategyPreset {
                strategy: ScheduleStrategy::ColdStockFirst,
                title: "冷料消化".to_string(),
                description: "优先消化冷料/压库物料".to_string(),
                default_parameters: serde_json::json!({
                    "order_keys": [
                        { "key": "sched_state", "order": "FORCE_RELEASE > LOCKED", "note": "红线：强制放行/锁定优先" },
                        { "key": "stock_age_days", "order": "desc", "note": "库龄大优先（冷料优先）" },
                        { "key": "rolling_output_age_days", "order": "desc", "note": "出炉更久优先" },
                        { "key": "due_date", "order": "asc", "note": "交期兜底" },
                    ],
                    "parameter_template": {
                        "urgent_weight": 3,
                        "capacity_weight": 1,
                        "cold_stock_weight": 10,
                        "due_date_weight": 1,
                        "rolling_output_age_weight": 5,
                        "cold_stock_age_threshold_days": 60,
                        "overflow_tolerance_pct": null,
                    },
                    "notes": [
                        "预设策略更偏“去库龄/去压库”，适用于库存消化专项。",
                    ],
                }),
            },
        ])
    }

    fn diff_plan_items(
        items_a: &[PlanItem],
        items_b: &[PlanItem],
    ) -> (usize, usize, usize, usize) {
        let (moved_count, added_count, removed_count, squeezed_out_count, _, _, _) =
            Self::diff_plan_items_detail(items_a, items_b);
        (moved_count, added_count, removed_count, squeezed_out_count)
    }

    fn diff_plan_items_detail(
        items_a: &[PlanItem],
        items_b: &[PlanItem],
    ) -> (
        usize,
        usize,
        usize,
        usize,
        Vec<StrategyDraftDiffItem>,
        usize,
        bool,
    ) {
        const MAX_DIFF_ITEMS: usize = 5000;

        // 逻辑保持与 compare_versions 一致：只统计 moved/added/removed/squeezed_out
        //
        // 性能注意：
        // - 在 50k+ 数据量下，diff_items_total 可能非常大；
        // - 若先收集全量 diff_items 再 sort/截断，会导致明显的 CPU 与内存放大；
        // - 这里采用“计数全量 + 仅保留前 MAX_DIFF_ITEMS 作为明细预览”的策略。
        let map_a: HashMap<String, &PlanItem> =
            items_a.iter().map(|item| (item.material_id.clone(), item)).collect();
        let map_b: HashMap<String, &PlanItem> =
            items_b.iter().map(|item| (item.material_id.clone(), item)).collect();

        let mut moved_count = 0usize;
        let mut added_count = 0usize;
        let mut squeezed_out_count = 0usize;
        let mut diff_items: Vec<StrategyDraftDiffItem> = Vec::new();
        diff_items.reserve(MAX_DIFF_ITEMS.min(items_a.len().saturating_add(items_b.len())));

        // moved/squeezed_out：按 items_a 的顺序遍历，保证明细预览稳定
        for item_a in items_a.iter() {
            let material_id = item_a.material_id.as_str();
            if let Some(item_b) = map_b.get(material_id) {
                if item_a.plan_date != item_b.plan_date || item_a.machine_code != item_b.machine_code {
                    moved_count += 1;
                    if diff_items.len() < MAX_DIFF_ITEMS {
                        diff_items.push(StrategyDraftDiffItem {
                            material_id: material_id.to_string(),
                            change_type: "MOVED".to_string(),
                            from_plan_date: Some(item_a.plan_date),
                            from_machine_code: Some(item_a.machine_code.clone()),
                            from_seq_no: Some(item_a.seq_no),
                            to_plan_date: Some(item_b.plan_date),
                            to_machine_code: Some(item_b.machine_code.clone()),
                            to_seq_no: Some(item_b.seq_no),
                            to_assign_reason: item_b.assign_reason.clone(),
                            to_urgent_level: item_b.urgent_level.clone(),
                            to_sched_state: item_b.sched_state.clone(),
                            material_state_snapshot: None,
                        });
                    }
                }
            } else {
                squeezed_out_count += 1;
                if diff_items.len() < MAX_DIFF_ITEMS {
                    diff_items.push(StrategyDraftDiffItem {
                        material_id: material_id.to_string(),
                        change_type: "SQUEEZED_OUT".to_string(),
                        from_plan_date: Some(item_a.plan_date),
                        from_machine_code: Some(item_a.machine_code.clone()),
                        from_seq_no: Some(item_a.seq_no),
                        to_plan_date: None,
                        to_machine_code: None,
                        to_seq_no: None,
                        to_assign_reason: None,
                        to_urgent_level: None,
                        to_sched_state: None,
                        material_state_snapshot: None,
                    });
                }
            }
        }

        // added：按 items_b 的顺序遍历，保证明细预览稳定
        for item_b in items_b.iter() {
            if map_a.contains_key(item_b.material_id.as_str()) {
                continue;
            }
            added_count += 1;
            if diff_items.len() < MAX_DIFF_ITEMS {
                diff_items.push(StrategyDraftDiffItem {
                    material_id: item_b.material_id.clone(),
                    change_type: "ADDED".to_string(),
                    from_plan_date: None,
                    from_machine_code: None,
                    from_seq_no: None,
                    to_plan_date: Some(item_b.plan_date),
                    to_machine_code: Some(item_b.machine_code.clone()),
                    to_seq_no: Some(item_b.seq_no),
                    to_assign_reason: item_b.assign_reason.clone(),
                    to_urgent_level: item_b.urgent_level.clone(),
                    to_sched_state: item_b.sched_state.clone(),
                    material_state_snapshot: None,
                });
            }
        }

        // 固定排序：变更类型 -> 日期 -> 机组 -> material_id（仅对预览明细排序，避免全量 sort）
        let type_rank = |t: &str| match t {
            "MOVED" => 0i32,
            "ADDED" => 1i32,
            "SQUEEZED_OUT" => 2i32,
            _ => 9i32,
        };

        diff_items.sort_by(|a, b| {
            let ra = type_rank(&a.change_type);
            let rb = type_rank(&b.change_type);
            if ra != rb {
                return ra.cmp(&rb);
            }

            let da = a.to_plan_date.or(a.from_plan_date);
            let db = b.to_plan_date.or(b.from_plan_date);
            if da != db {
                return da.cmp(&db);
            }

            let ma = a
                .to_machine_code
                .as_deref()
                .or(a.from_machine_code.as_deref())
                .unwrap_or("");
            let mb = b
                .to_machine_code
                .as_deref()
                .or(b.from_machine_code.as_deref())
                .unwrap_or("");
            if ma != mb {
                return ma.cmp(mb);
            }

            a.material_id.cmp(&b.material_id)
        });

        let diff_items_total = moved_count + added_count + squeezed_out_count;
        let diff_items_truncated = diff_items_total > MAX_DIFF_ITEMS;

        let removed_count = squeezed_out_count;
        (
            moved_count,
            added_count,
            removed_count,
            squeezed_out_count,
            diff_items,
            diff_items_total,
            diff_items_truncated,
        )
    }

    // ==========================================

}

use super::*;

impl PlanApi {
    // ==========================================
    // 版本管理接口
    // ==========================================

    /// 创建新版本
    ///
    /// # 参数
    /// - plan_id: 方案ID
    /// - window_days: 窗口天数
    /// - frozen_from_date: 冻结区起始日期（可选）
    /// - note: 备注（可选）
    /// - created_by: 创建人
    ///
    /// # 返回
    /// - Ok(String): 版本ID
    /// - Err(ApiError): API错误
    pub fn create_version(
        &self,
        plan_id: String,
        window_days: i32,
        frozen_from_date: Option<NaiveDate>,
        note: Option<String>,
        created_by: String,
    ) -> ApiResult<String> {
        // 参数验证
        if plan_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("方案ID不能为空".to_string()));
        }
        if !(1..=60).contains(&window_days) {
            return Err(ApiError::InvalidInput(
                "窗口天数必须在1-60之间".to_string(),
            ));
        }

        // 检查Plan是否存在
        let plan = self
            .plan_repo
            .find_by_id(&plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        if plan.is_none() {
            return Err(ApiError::NotFound(format!("方案{}不存在", plan_id)));
        }

        // 创建配置快照JSON（用于版本回滚/对比口径）
        // 注意：元信息（例如中文命名/备注）统一写入 __meta_*，避免污染“配置差异”与回滚恢复。
        let config_snapshot_json = Some(
            self.config_manager
                .get_config_snapshot()
                .map_err(|e| ApiError::InternalError(e.to_string()))?,
        );

        // 创建PlanVersion实例（version_no 由仓储层在事务内分配，避免并发冲突）
        let mut version = PlanVersion {
            version_id: uuid::Uuid::new_v4().to_string(),
            plan_id: plan_id.clone(),
            version_no: 0,
            status: PlanVersionStatus::Draft,
            frozen_from_date,
            recalc_window_days: Some(window_days),
            config_snapshot_json,
            created_by: Some(created_by.clone()),
            created_at: chrono::Local::now().naive_local(),
            revision: 1,
        };

        // 保存到数据库
        self.plan_version_repo
            .create_with_next_version_no(&mut version)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 写入备注/命名等元信息（不改变表结构，写到 config_snapshot_json.__meta_*）
        if let Some(note_text) = note.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
            // best-effort: meta 更新失败不影响版本创建，但会影响“命名显示/回滚可解释性”
            if let Ok(mut map) =
                serde_json::from_str::<std::collections::HashMap<String, String>>(
                    version.config_snapshot_json.as_deref().unwrap_or("{}"),
                )
            {
                map.insert("__meta_version_name_cn".to_string(), note_text.to_string());
                map.insert("__meta_note".to_string(), note_text.to_string());
                map.insert(
                    "__meta_note_created_at".to_string(),
                    chrono::Local::now().to_rfc3339(),
                );

                if let Ok(next_json) = serde_json::to_string(&map) {
                    version.config_snapshot_json = Some(next_json);
                    if let Err(e) = self.plan_version_repo.update(&version) {
                        tracing::warn!("创建版本后写入 meta 失败: {}", e);
                    }
                }
            }
        }

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version.version_id.clone()),
            action_type: "CREATE_VERSION".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: created_by,
            payload_json: Some(serde_json::json!({
                "plan_id": plan_id,
                "version_no": version.version_no,
                "window_days": window_days,
                "frozen_from_date": frozen_from_date.map(|d| d.to_string()),
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("创建版本: V{}", version.version_no)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(version.version_id)
    }

    /// 查询版本列表
    ///
    /// # 参数
    /// - plan_id: 方案ID
    ///
    /// # 返回
    /// - Ok(Vec<PlanVersion>): 版本列表
    /// - Err(ApiError): API错误
    pub fn list_versions(&self, plan_id: &str) -> ApiResult<Vec<PlanVersion>> {
        if plan_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("方案ID不能为空".to_string()));
        }

        self.plan_version_repo
            .find_by_plan_id(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 删除版本（仅允许删除非激活版本）
    pub fn delete_version(&self, version_id: &str, operator: &str) -> ApiResult<()> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }

        let version = self
            .plan_version_repo
            .find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        if version.status == PlanVersionStatus::Active {
            return Err(ApiError::BusinessRuleViolation(
                "不能删除激活版本，请先激活其他版本或将其归档".to_string(),
            ));
        }

        // 显式删除关联数据（避免依赖 SQLite foreign_keys 配置）
        // 0. 删除策略草稿（strategy_draft 表有外键引用）
        let deleted_drafts = self
            .strategy_draft_repo
            .delete_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 1. 在删除 plan_item 之前，查询受影响的产能池
        let affected_capacity_keys = self
            .get_affected_capacity_keys_for_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 2. 删除 plan_item
        let deleted_items = self
            .plan_item_repo
            .delete_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 3. 同步重置受影响的产能池
        if !affected_capacity_keys.is_empty() {
            tracing::info!(
                "版本删除后开始重置产能池: version_id={}, 涉及 {} 个(机组,日期)组合",
                version_id,
                affected_capacity_keys.len()
            );
            if let Err(e) = self.reset_capacity_pools(version_id, &affected_capacity_keys) {
                tracing::warn!("产能池重置失败: {}, 继续执行", e);
            }
        }

        // 4. 删除风险快照
        let deleted_risks = self
            .risk_snapshot_repo
            .delete_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 4.5 解绑 action_log（action_log.version_id 有外键约束，删除 plan_version 前必须置空）
        let detached_action_logs = self
            .action_log_repo
            .detach_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        self.plan_version_repo
            .delete(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            // 版本已删除，action_log.version_id 不能再引用已不存在的 plan_version（否则会触发外键约束失败）
            version_id: None,
            action_type: "DELETE_VERSION".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "version_id": version_id,
                "plan_id": version.plan_id,
                "version_no": version.version_no,
                "deleted_plan_items": deleted_items,
                "deleted_risk_snapshots": deleted_risks,
                "deleted_strategy_drafts": deleted_drafts,
                "detached_action_logs": detached_action_logs,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("删除版本: {}", version_id)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// 激活版本
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - operator: 操作人
    ///
    /// # 返回
    /// - Ok(()): 成功
    /// - Err(ApiError): API错误
    pub fn activate_version(&self, version_id: &str, operator: &str) -> ApiResult<()> {
        // 参数验证
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        // 检查版本是否存在
        let version = self
            .plan_version_repo
            .find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        // 同一方案只能有一个激活版本：仓储层在事务中完成归档+激活
        self.plan_version_repo
            .activate_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version_id.to_string()),
            action_type: "ACTIVATE_VERSION".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "version_id": version_id,
                "plan_id": version.plan_id,
                "version_no": version.version_no,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("激活版本: {}", version_id)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 同步刷新产能池以匹配该版本的 plan_item
        tracing::info!("版本激活后开始同步刷新产能池: version_id={}", version_id);
        if let Err(e) = self.recalculate_capacity_pool_for_version(version_id) {
            tracing::warn!("产能池同步刷新失败: {}, 继续执行", e);
            // 不阻断流程，仅记录警告
        }

        // 触发决策视图全量刷新
        let event = ScheduleEvent::full_scope(
            version_id.to_string(),
            ScheduleEventType::ManualTrigger, // 版本激活属于手动触发
            Some(format!("Version activated by {}", operator)),
        );

        match self.event_publisher.publish(event) {
            Ok(task_id) => {
                if !task_id.is_empty() {
                    tracing::info!("版本激活后决策视图刷新事件已发布: task_id={}, version_id={}", task_id, version_id);
                }
            }
            Err(e) => {
                tracing::warn!("版本激活后决策视图刷新事件发布失败: {}", e);
            }
        }

        Ok(())
    }

    /// 根据版本的 plan_item 重新计算产能池
    ///
    /// # 说明
    /// 当版本切换时，需要同步刷新产能池数据，确保：
    /// - used_capacity_t = 该版本 plan_item 的实际 weight_t 总和
    /// - overflow_t = max(0, used_capacity_t - limit_capacity_t)
    ///
    /// # 参数
    /// - version_id: 版本ID
    ///
    /// # 返回
    /// - Ok(()): 成功
    /// - Err(ApiError): API错误
    fn recalculate_capacity_pool_for_version(&self, version_id: &str) -> ApiResult<()> {
        // 1. 获取版本的日期窗口
        let version = self.plan_version_repo.find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        let window_days = version.recalc_window_days.unwrap_or(30);
        let frozen_date = version.frozen_from_date
            .unwrap_or_else(|| chrono::Local::now().date_naive());
        let end_date = frozen_date + chrono::Duration::days(window_days as i64);

        // 2. 先清零窗口内所有 capacity_pool 的 used_capacity_t 和 overflow_t
        // 这确保不会有残留值（避免"利用率高但已排为0"的异常显示）
        self.capacity_repo.reset_used_in_date_range(version_id, frozen_date, end_date)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        tracing::info!(
            "已清零产能池 used_capacity_t: version_id={}, date_range=[{}, {}]",
            version_id, frozen_date, end_date
        );

        // 3. 使用 SQL 聚合按 (machine_code, plan_date) 统计吨位，避免拉取全量 plan_item（50k+ 性能瓶颈）
        let updates = self
            .plan_item_repo
            .sum_weight_by_machine_and_date_range(version_id, frozen_date, end_date)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        tracing::info!(
            "产能池同步：version_id={}, date_range=[{}, {}], agg_rows={}",
            version_id,
            frozen_date,
            end_date,
            updates.len()
        );

        // 4. 批量更新 capacity_pool 的 used_capacity_t + overflow_t（不读取全量 capacity_pool）
        let updated_rows = self
            .capacity_repo
            .update_used_and_overflow_batch(version_id, &updates)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        tracing::info!(
            "产能池同步完成: version_id={}, updated_rows={}",
            version_id,
            updated_rows
        );

        Ok(())
    }

    /// 查询版本中所有 plan_item 的 (machine_code, plan_date) 组合
    ///
    /// # 说明
    /// 用于在删除 plan_item 之前，获取受影响的产能池位置，以便后续重置
    fn get_affected_capacity_keys_for_version(
        &self,
        version_id: &str,
    ) -> ApiResult<Vec<(String, NaiveDate)>> {
        self.plan_item_repo
            .list_machine_date_keys(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 重置产能池（将 used_capacity_t 和 overflow_t 清零）
    ///
    /// # 说明
    /// 当删除 plan_item 后，需要同步重置产能池数据，确保数据一致性
    ///
    /// # 参数
    /// - version_id: 版本ID (P1-1: 版本化改造)
    /// - keys: (machine_code, plan_date) 列表
    fn reset_capacity_pools(&self, version_id: &str, keys: &[(String, NaiveDate)]) -> ApiResult<()> {
        for (machine_code, plan_date) in keys {
            // 查询产能池（如果不存在则跳过）
            if let Some(mut capacity_pool) = self
                .capacity_repo
                .find_by_machine_and_date(version_id, machine_code, *plan_date)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            {
                // 重置 used_capacity_t 和 overflow_t
                capacity_pool.used_capacity_t = 0.0;
                capacity_pool.overflow_t = 0.0;

                // 持久化
                self.capacity_repo
                    .upsert_single(&capacity_pool)
                    .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// 版本回滚（激活历史版本，并按需恢复该版本记录的配置快照）
    ///
    /// 规则：
    /// - 仅允许回滚到同一 plan 的历史版本
    /// - 写入 ActionLog（包含 from/to/version_no/恢复配置数量/原因）
    /// - 发布刷新事件（触发决策读模型刷新）
    pub fn rollback_version(
        &self,
        plan_id: &str,
        target_version_id: &str,
        operator: &str,
        reason: &str,
    ) -> ApiResult<RollbackVersionResponse> {
        if plan_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("方案ID不能为空".to_string()));
        }
        if target_version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("目标版本ID不能为空".to_string()));
        }
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("回滚原因不能为空".to_string()));
        }

        // 校验 plan 存在（便于输出可读信息）
        let plan = self
            .plan_repo
            .find_by_id(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("方案{}不存在", plan_id)))?;

        // 校验目标版本存在且属于该 plan
        let target = self
            .plan_version_repo
            .find_by_id(target_version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", target_version_id)))?;

        if target.plan_id != plan_id {
            return Err(ApiError::BusinessRuleViolation(format!(
                "目标版本不属于该方案：plan_id={}, target.plan_id={}",
                plan_id, target.plan_id
            )));
        }

        let current_active = self
            .plan_version_repo
            .find_active_version(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let from_version_id = current_active.as_ref().map(|v| v.version_id.clone());
        let from_version_no = current_active.as_ref().map(|v| v.version_no);

        // 1) 尝试恢复配置（优先：保证后续重算/解释口径一致）
        let mut restored_config_count: Option<usize> = None;
        let mut config_restore_skipped: Option<String> = None;

        match target
            .config_snapshot_json
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            None => {
                config_restore_skipped = Some("目标版本缺少 config_snapshot_json，已跳过配置恢复".to_string());
            }
            Some(snapshot_json) => {
                // 防御：识别“备注型快照”（旧实现可能把 note 写入 config_snapshot_json）
                // - 若对象内非 __meta_* 键过少且包含 note，则认为不是 config_kv 快照，避免把 note 写入 config_kv。
                let mut should_skip = None::<String>;
                match serde_json::from_str::<serde_json::Value>(snapshot_json) {
                    Ok(serde_json::Value::Object(obj)) => {
                        let non_meta_key_count = obj
                            .keys()
                            .filter(|k| !k.starts_with("__meta_"))
                            .count();
                        if non_meta_key_count == 0 {
                            should_skip = Some("目标版本配置快照为空对象，已跳过配置恢复".to_string());
                        } else if non_meta_key_count <= 2 && obj.contains_key("note") {
                            should_skip = Some(
                                "目标版本 config_snapshot_json 更像备注信息（含 note），已跳过配置恢复".to_string(),
                            );
                        }
                    }
                    Ok(_) => {
                        should_skip =
                            Some("目标版本 config_snapshot_json 不是 JSON 对象，已跳过配置恢复".to_string());
                    }
                    Err(e) => {
                        should_skip = Some(format!(
                            "目标版本 config_snapshot_json 解析失败（{}），已跳过配置恢复",
                            e
                        ));
                    }
                }

                if let Some(msg) = should_skip {
                    config_restore_skipped = Some(msg);
                } else {
                    let count = self
                        .config_manager
                        .restore_config_from_snapshot(snapshot_json)
                        .map_err(|e| ApiError::InternalError(format!("恢复配置失败: {}", e)))?;
                    restored_config_count = Some(count);
                }
            }
        }

        // 2) 激活目标版本（事务内归档其他 ACTIVE）
        self.plan_version_repo
            .activate_version(target_version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 3) 写入审计日志
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(target_version_id.to_string()),
            action_type: "ROLLBACK_VERSION".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "plan_id": plan_id,
                "plan_name": plan.plan_name,
                "from_version_id": from_version_id,
                "from_version_no": from_version_no,
                "to_version_id": target_version_id,
                "to_version_no": target.version_no,
                "restored_config_count": restored_config_count,
                "config_restore_skipped": config_restore_skipped,
                "reason": reason,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!(
                "版本回滚: {:?} -> V{} | {}",
                from_version_no,
                target.version_no,
                reason.trim()
            )),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 4) 触发决策视图全量刷新（回滚属于手动触发）
        let event = ScheduleEvent::full_scope(
            target_version_id.to_string(),
            ScheduleEventType::ManualTrigger,
            Some(format!("rollback_version by {} | {}", operator, reason.trim())),
        );

        if let Err(e) = self.event_publisher.publish(event) {
            tracing::warn!("版本回滚后决策刷新事件发布失败: {}", e);
        }

        Ok(RollbackVersionResponse {
            plan_id: plan_id.to_string(),
            from_version_id,
            to_version_id: target_version_id.to_string(),
            restored_config_count,
            config_restore_skipped,
            message: "回滚完成".to_string(),
        })
    }

    /// 手动触发决策读模型刷新（P0-2）
    ///
    /// 说明：
    /// - 这是“可重试”的兜底入口：当决策数据刷新失败或用户怀疑数据过期时，可手动触发一次全量刷新。
    /// - 实际执行依赖 event_publisher（默认由 Decision 层 RefreshQueueAdapter 提供）。
    pub fn manual_refresh_decision(
        &self,
        version_id: &str,
        operator: &str,
    ) -> ApiResult<ManualRefreshDecisionResponse> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }

        // 校验版本存在（避免写入无效 action_log）
        let _ = self
            .plan_version_repo
            .find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        let event = ScheduleEvent::full_scope(
            version_id.to_string(),
            ScheduleEventType::ManualTrigger,
            Some(format!("manual_refresh_decision by {}", operator)),
        );

        let mut task_id: Option<String> = None;
        let mut message = String::new();
        let mut success = true;

        match self.event_publisher.publish(event) {
            Ok(id) => {
                if id.trim().is_empty() {
                    success = false;
                    message = "已收到刷新请求，但当前未配置决策刷新组件（可能不会执行）".to_string();
                } else {
                    task_id = Some(id.clone());
                    message = format!("已触发决策刷新: task_id={}", id);
                }
            }
            Err(e) => {
                success = false;
                message = format!("触发决策刷新失败: {}", e);
            }
        }

        // 记录 ActionLog（best-effort）
        let log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version_id.to_string()),
            action_type: "MANUAL_REFRESH_DECISION".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "version_id": version_id,
                "task_id": task_id,
                "success": success,
                "message": message,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some("手动触发决策读模型刷新".to_string()),
        };

        if let Err(e) = self.action_log_repo.insert(&log) {
            tracing::warn!("记录操作日志失败: {}", e);
        }

        Ok(ManualRefreshDecisionResponse {
            version_id: version_id.to_string(),
            task_id,
            success,
            message,
        })
    }

    // ==========================================

}

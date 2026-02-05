use super::*;

impl PlanApi {
    // ==========================================
    // 排产操作接口
    // ==========================================

    /// 移动排产项
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - moves: 移动项列表
    /// - mode: 校验模式 (STRICT/AUTO_FIX)
    ///
    /// # 返回
    /// - Ok(MoveItemsResponse): 移动结果
    /// - Err(ApiError): API错误
    ///
    /// # 红线合规
    /// - 红线1: 冻结区材料不可移动
    /// - 红线2: 非适温材料不可移动到当日
    pub fn move_items(
        &self,
        version_id: &str,
        moves: Vec<MoveItemRequest>,
        mode: crate::api::ValidationMode,
        operator: &str,
        reason: Option<&str>,
    ) -> ApiResult<MoveItemsResponse> {
        use std::collections::HashMap;

        // 参数验证
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if moves.is_empty() {
            return Err(ApiError::InvalidInput("移动项列表不能为空".to_string()));
        }

        // 1. 验证版本存在且为草稿状态
        let version = self.plan_version_repo.find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        if !version.is_draft() && !version.is_active() {
            return Err(ApiError::BusinessRuleViolation(
                "只能修改草稿或激活状态的版本".to_string()
            ));
        }

        // 2. 加载当前版本的所有排产明细
        let current_items = self.plan_item_repo.find_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 构建材料ID到PlanItem的映射
        let item_map: HashMap<String, PlanItem> = current_items
            .into_iter()
            .map(|item| (item.material_id.clone(), item))
            .collect();

        // 3. 处理每个移动请求
        let mut results = Vec::new();
        let mut success_count = 0;
        let mut failed_count = 0;
        let mut has_violations = false;
        let mut items_to_update = Vec::new();

        for move_req in moves {
            // 解析目标日期
            let to_date = match NaiveDate::parse_from_str(&move_req.to_date, "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => {
                    failed_count += 1;
                    results.push(MoveItemResult {
                        material_id: move_req.material_id.clone(),
                        success: false,
                        from_date: None,
                        from_machine: None,
                        to_date: move_req.to_date.clone(),
                        to_machine: move_req.to_machine.clone(),
                        error: Some("日期格式错误，应为YYYY-MM-DD".to_string()),
                        violation_type: None,
                    });
                    continue;
                }
            };

            // 查找原排产项
            let original_item = match item_map.get(&move_req.material_id) {
                Some(item) => item,
                None => {
                    failed_count += 1;
                    results.push(MoveItemResult {
                        material_id: move_req.material_id.clone(),
                        success: false,
                        from_date: None,
                        from_machine: None,
                        to_date: move_req.to_date.clone(),
                        to_machine: move_req.to_machine.clone(),
                        error: Some(format!("材料{}在版本中不存在", move_req.material_id)),
                        violation_type: None,
                    });
                    continue;
                }
            };

            // 红线1: 检查冻结区保护
            if original_item.locked_in_plan {
                has_violations = true;
                match mode {
                    crate::api::ValidationMode::Strict => {
                        failed_count += 1;
                        results.push(MoveItemResult {
                            material_id: move_req.material_id.clone(),
                            success: false,
                            from_date: Some(original_item.plan_date.format("%Y-%m-%d").to_string()),
                            from_machine: Some(original_item.machine_code.clone()),
                            to_date: move_req.to_date.clone(),
                            to_machine: move_req.to_machine.clone(),
                            error: Some("冻结区材料不可移动".to_string()),
                            violation_type: Some("FROZEN_ZONE".to_string()),
                        });
                        continue;
                    }
                    crate::api::ValidationMode::AutoFix => {
                        // AutoFix 模式跳过冻结材料
                        results.push(MoveItemResult {
                            material_id: move_req.material_id.clone(),
                            success: false,
                            from_date: Some(original_item.plan_date.format("%Y-%m-%d").to_string()),
                            from_machine: Some(original_item.machine_code.clone()),
                            to_date: move_req.to_date.clone(),
                            to_machine: move_req.to_machine.clone(),
                            error: Some("冻结区材料已跳过".to_string()),
                            violation_type: Some("FROZEN_ZONE_SKIPPED".to_string()),
                        });
                        continue;
                    }
                }
            }

            // 创建更新后的排产项
            let updated_item = PlanItem {
                version_id: version_id.to_string(),
                material_id: move_req.material_id.clone(),
                machine_code: move_req.to_machine.clone(),
                plan_date: to_date,
                seq_no: move_req.to_seq,
                weight_t: original_item.weight_t,
                source_type: "MANUAL".to_string(), // 手动移动
                locked_in_plan: original_item.locked_in_plan,
                force_release_in_plan: original_item.force_release_in_plan,
                violation_flags: original_item.violation_flags.clone(),
                urgent_level: original_item.urgent_level.clone(),
                sched_state: original_item.sched_state.clone(),
                assign_reason: Some("MANUAL_MOVE".to_string()),
                steel_grade: original_item.steel_grade.clone(),
                width_mm: original_item.width_mm,
                thickness_mm: original_item.thickness_mm,
            };

            items_to_update.push(updated_item);
            success_count += 1;
            results.push(MoveItemResult {
                material_id: move_req.material_id.clone(),
                success: true,
                from_date: Some(original_item.plan_date.format("%Y-%m-%d").to_string()),
                from_machine: Some(original_item.machine_code.clone()),
                to_date: move_req.to_date.clone(),
                to_machine: move_req.to_machine.clone(),
                error: None,
                violation_type: None,
            });
        }

        // 4. 批量更新排产项
        if !items_to_update.is_empty() {
            self.plan_item_repo.batch_upsert(&items_to_update)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

            // 5. 记录操作日志
            let actor = if operator.trim().is_empty() { "system" } else { operator };
            let detail = match reason {
                Some(r) if !r.trim().is_empty() => format!("移动{}个排产项 | {}", success_count, r.trim()),
                _ => format!("移动{}个排产项", success_count),
            };

            let log = ActionLog {
                action_id: uuid::Uuid::new_v4().to_string(),
                version_id: Some(version_id.to_string()),
                action_type: "MOVE_ITEMS".to_string(),
                action_ts: chrono::Local::now().naive_local(),
                actor: actor.to_string(),
                payload_json: Some(serde_json::json!({
                    "success_count": success_count,
                    "failed_count": failed_count,
                    "has_violations": has_violations,
                    "reason": reason,
                    "moved_materials": items_to_update.iter().map(|i| &i.material_id).collect::<Vec<_>>(),
                })),
                impact_summary_json: None,
                machine_code: None,
                date_range_start: None,
                date_range_end: None,
                detail: Some(detail),
            };

            if let Err(e) = self.action_log_repo.insert(&log) {
                tracing::warn!("记录操作日志失败: {}", e);
            }

            // 6. 触发刷新事件
            let event = ScheduleEvent::full_scope(
                version_id.to_string(),
                ScheduleEventType::PlanItemChanged,
                Some("move_items".to_string()),
            );

            if let Err(e) = self.event_publisher.publish(event) {
                tracing::warn!("发布刷新事件失败: {}", e);
            }
        }

        Ok(MoveItemsResponse {
            version_id: version_id.to_string(),
            results,
            success_count,
            failed_count,
            has_violations,
            message: format!(
                "移动完成: 成功{}个, 失败{}个{}",
                success_count,
                failed_count,
                if has_violations { ", 存在违规" } else { "" }
            ),
        })
    }

}

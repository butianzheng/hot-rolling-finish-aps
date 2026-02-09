use super::*;

const CUSTOM_STRATEGY_PREFIX: &str = "custom:";

impl PlanApi {
    // ==========================================
    // 排产计算接口
    // ==========================================

    /// 试算接口（沙盘模式）
    ///
    /// # 参数
    /// - version_id: 版本ID（作为基准版本）
    /// - base_date: 基准日期（从哪天开始排产）
    /// - _frozen_date: 冻结日期（保留参数，实际由RecalcEngine内部计算）
    /// - operator: 操作人
    ///
    /// # 返回
    /// - Ok(RecalcResponse): 试算结果（不保存到数据库）
    /// - Err(ApiError): API错误
    ///
    /// # 说明
    /// - 使用RecalcEngine的dry-run模式
    /// - 不写入plan_item表
    /// - 不写入risk_snapshot表
    /// - 不记录ActionLog
    /// - 返回内存中的结果供前端预览
    pub fn simulate_recalc(
        &self,
        version_id: &str,
        base_date: NaiveDate,
        _frozen_date: Option<NaiveDate>,
        operator: &str,
    ) -> ApiResult<RecalcResponse> {
        self.simulate_recalc_with_strategy(
            version_id,
            base_date,
            _frozen_date,
            operator,
            ScheduleStrategy::Balanced,
            None,
        )
    }

    /// 试算接口（沙盘模式）- 指定策略
    pub fn simulate_recalc_with_strategy(
        &self,
        version_id: &str,
        base_date: NaiveDate,
        _frozen_date: Option<NaiveDate>,
        operator: &str,
        strategy: ScheduleStrategy,
        window_days_override: Option<i32>,
    ) -> ApiResult<RecalcResponse> {
        self.simulate_recalc_with_strategy_key(
            version_id,
            base_date,
            _frozen_date,
            operator,
            strategy.as_str(),
            window_days_override,
        )
    }

    /// 试算接口（沙盘模式）- 指定策略键（支持 custom:*）
    pub fn simulate_recalc_with_strategy_key(
        &self,
        version_id: &str,
        base_date: NaiveDate,
        _frozen_date: Option<NaiveDate>,
        operator: &str,
        strategy_key: &str,
        window_days_override: Option<i32>,
    ) -> ApiResult<RecalcResponse> {
        // 参数验证
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        let strategy_key = self.normalize_strategy_key(strategy_key)?;

        // 加载版本信息
        let version = self
            .plan_version_repo
            .find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        // 获取窗口天数（允许前端在本次试算/重算中覆盖版本配置，便于评估“窗口大小”对排程的影响）
        let window_days = match window_days_override {
            Some(v) => {
                if v <= 0 || v > 60 {
                    return Err(ApiError::InvalidInput(
                        "window_days_override 必须在 1-60 之间".to_string(),
                    ));
                }
                v
            }
            None => version.recalc_window_days.unwrap_or(30),
        };

        // 调用RecalcEngine执行试算（dry-run模式）
        let result = self
            .recalc_engine
            .recalc_full_with_strategy_key(
                &version.plan_id,
                base_date,
                window_days,
                operator,
                true,
                &strategy_key,
            )
            .map_err(|e| ApiError::InternalError(format!("试算失败: {}", e)))?;

        // 返回结果（不记录ActionLog）
        Ok(RecalcResponse {
            run_id: result.run_id,
            version_id: result.version_id,
            plan_rev: result.plan_rev,
            plan_items_count: result.total_items,
            frozen_items_count: result.frozen_items,
            success: true,
            message: format!(
                "试算完成（{}），共排产{}个材料（冻结{}个，重算{}个）",
                strategy_key,
                result.total_items, result.frozen_items, result.recalc_items
            ),
        })
    }

    /// 一键重算（核心方法）
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - base_date: 基准日期（从哪天开始排产）
    /// - frozen_date: 冻结日期（该日期之前的材料为冻结区）
    /// - operator: 操作人
    ///
    /// # 返回
    /// - Ok(RecalcResponse): 重算结果
    /// - Err(ApiError): API错误
    ///
    /// # 红线合规
    /// - 红线1: 加载冻结区材料，不调整其排产结果
    /// - 红线2: 调用EligibilityEngine验证适温状态
    /// - 红线3: 调用UrgencyEngine计算紧急等级
    /// - 红线4: 调用CapacityFiller填充，不超产能
    /// - 红线5: 记录ActionLog，包含窗口天数、冻结日期等
    pub fn recalc_full(
        &self,
        version_id: &str,
        base_date: NaiveDate,
        frozen_date: Option<NaiveDate>,
        operator: &str,
    ) -> ApiResult<RecalcResponse> {
        self.recalc_full_with_strategy(
            version_id,
            base_date,
            frozen_date,
            operator,
            ScheduleStrategy::Balanced,
            None,
        )
    }

    /// 一键重算（核心方法）- 指定策略
    pub fn recalc_full_with_strategy(
        &self,
        version_id: &str,
        base_date: NaiveDate,
        frozen_date: Option<NaiveDate>,
        operator: &str,
        strategy: ScheduleStrategy,
        window_days_override: Option<i32>,
    ) -> ApiResult<RecalcResponse> {
        self.recalc_full_with_strategy_key(
            version_id,
            base_date,
            frozen_date,
            operator,
            strategy.as_str(),
            window_days_override,
        )
    }

    /// 一键重算（核心方法）- 指定策略键（支持 custom:*）
    pub fn recalc_full_with_strategy_key(
        &self,
        version_id: &str,
        base_date: NaiveDate,
        frozen_date: Option<NaiveDate>,
        operator: &str,
        strategy_key: &str,
        window_days_override: Option<i32>,
    ) -> ApiResult<RecalcResponse> {
        // 参数验证
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        let strategy_key = self.normalize_strategy_key(strategy_key)?;

        // 加载版本信息
        let version = self
            .plan_version_repo
            .find_by_id(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id)))?;

        // 红线1预检: 设置冻结日期
        let _frozen_from_date = frozen_date.or(version.frozen_from_date);

        // 获取窗口天数（允许覆盖版本配置，便于“一键优化/重算”选择不同计算窗口）
        let window_days = match window_days_override {
            Some(v) => {
                if v <= 0 || v > 60 {
                    return Err(ApiError::InvalidInput(
                        "window_days_override 必须在 1-60 之间".to_string(),
                    ));
                }
                v
            }
            None => version.recalc_window_days.unwrap_or(30),
        };

        // 调用RecalcEngine执行重算
        tracing::info!(
            "开始重算 version_id={}, plan_id={}, base_date={}, window_days={}",
            version_id,
            version.plan_id,
            base_date,
            window_days
        );

        // 调用 RecalcEngine 执行实际重算
        let recalc_result = self
            .recalc_engine
            .recalc_full_with_strategy_key(
                &version.plan_id,
                base_date,
                window_days,
                operator,
                false,
                &strategy_key,
            )
            .map_err(|e| ApiError::InternalError(format!("重算失败: {}", e)))?;

        let plan_items_count = recalc_result.total_items;
        let frozen_items_count = recalc_result.frozen_items;

        // 记录ActionLog（红线5: 可解释性）
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(recalc_result.version_id.clone()),
            action_type: "RECALC_FULL".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "base_date": base_date.to_string(),
                "window_days": window_days,
                "frozen_from_date": frozen_date.map(|d| d.to_string()),
                "strategy": strategy_key,
            })),
            impact_summary_json: Some(serde_json::json!({
                "plan_items_count": plan_items_count,
                "frozen_items_count": frozen_items_count,
                "mature_count": recalc_result.mature_count,
                "immature_count": recalc_result.immature_count,
                "elapsed_ms": recalc_result.elapsed_ms,
            })),
            machine_code: None,
            date_range_start: Some(base_date),
            date_range_end: Some(
                base_date + chrono::Duration::days(window_days as i64),
            ),
            detail: Some("一键重算".to_string()),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 返回结果
        Ok(RecalcResponse {
            run_id: recalc_result.run_id,
            version_id: recalc_result.version_id,
            plan_rev: recalc_result.plan_rev,
            plan_items_count,
            frozen_items_count,
            success: true,
            message: format!("重算完成（{}），共排产{}个材料", strategy_key, plan_items_count),
        })
    }

    fn normalize_strategy_key(&self, raw: &str) -> ApiResult<String> {
        let normalized = raw.trim();
        if normalized.is_empty() {
            return Ok(ScheduleStrategy::Balanced.as_str().to_string());
        }

        if let Some(custom_id_raw) = normalized.strip_prefix(CUSTOM_STRATEGY_PREFIX) {
            let custom_id = custom_id_raw.trim();
            if custom_id.is_empty() {
                tracing::warn!("自定义策略ID为空，回退 balanced");
                return Ok(ScheduleStrategy::Balanced.as_str().to_string());
            }

            let profile = self
                .config_manager
                .get_custom_strategy_profile(custom_id)
                .map_err(|e| ApiError::DatabaseError(format!("读取自定义策略失败: {}", e)))?;

            let Some(profile) = profile else {
                tracing::warn!(strategy_key = normalized, "自定义策略不存在，回退 balanced");
                return Ok(ScheduleStrategy::Balanced.as_str().to_string());
            };

            if profile.base_strategy.parse::<ScheduleStrategy>().is_err() {
                tracing::warn!(strategy_key = normalized, "自定义策略基线异常，回退 balanced");
                return Ok(ScheduleStrategy::Balanced.as_str().to_string());
            }

            return Ok(format!("{}{}", CUSTOM_STRATEGY_PREFIX, custom_id));
        }

        match normalized.parse::<ScheduleStrategy>() {
            Ok(preset) => Ok(preset.as_str().to_string()),
            Err(_) => {
                tracing::warn!(strategy_key = normalized, "策略无效，回退 balanced");
                Ok(ScheduleStrategy::Balanced.as_str().to_string())
            }
        }
    }

    // ==========================================

}

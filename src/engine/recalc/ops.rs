use super::{RecalcEngine, RecalcResult, ResolvedStrategyProfile};
use crate::domain::plan::{PlanItem, PlanVersion};
use crate::domain::types::{PlanVersionStatus};
use crate::engine::events::ScheduleEventType;
use crate::engine::strategy::ScheduleStrategy;
use chrono::NaiveDate;
use std::error::Error;
use tracing::instrument;
use uuid::Uuid;

impl RecalcEngine {
    pub fn recalc_full_with_strategy_key(
        &self,
        plan_id: &str,
        base_date: NaiveDate,
        window_days: i32,
        operator: &str,
        is_dry_run: bool,
        strategy_key: &str,
    ) -> Result<RecalcResult, Box<dyn Error>> {
        let profile = self.resolve_strategy_profile(strategy_key)?;
        self.recalc_full_with_profile(plan_id, base_date, window_days, operator, is_dry_run, profile)
    }

    #[instrument(skip(self), fields(plan_id = %plan_id, window_days = %window_days, is_dry_run = %is_dry_run))]
    pub fn recalc_full(
        &self,
        plan_id: &str,
        base_date: NaiveDate,
        window_days: i32,
        operator: &str,
        is_dry_run: bool,
        strategy: ScheduleStrategy,
    ) -> Result<RecalcResult, Box<dyn Error>> {
        let profile = ResolvedStrategyProfile {
            strategy_key: strategy.as_str().to_string(),
            base_strategy: strategy,
            title_cn: strategy.title_cn().to_string(),
            parameters: None,
        };
        self.recalc_full_with_profile(plan_id, base_date, window_days, operator, is_dry_run, profile)
    }

    #[instrument(skip(self, profile), fields(plan_id = %plan_id, window_days = %window_days, is_dry_run = %is_dry_run, strategy = %profile.strategy_key))]
    pub(crate) fn recalc_full_with_profile(
        &self,
        plan_id: &str,
        base_date: NaiveDate,
        window_days: i32,
        operator: &str,
        is_dry_run: bool,
        profile: ResolvedStrategyProfile,
    ) -> Result<RecalcResult, Box<dyn Error>> {
        // 1. 查询激活版本 (如果存在)
        let base_version = self.version_repo.find_active_version(plan_id)?;

        // 2. 创建新版本（试算模式下也创建临时版本用于计算）
        let mut new_version = if is_dry_run {
            // 试算模式：创建临时版本对象（不写库）
            PlanVersion {
                version_id: Uuid::new_v4().to_string(),
                plan_id: plan_id.to_string(),
                version_no: 0, // 试算版本号为0
                status: PlanVersionStatus::Draft, // 试算也用 Draft 状态
                frozen_from_date: None,
                recalc_window_days: Some(window_days),
                config_snapshot_json: Some(format!("试算 (操作人: {})", operator)),
                created_by: Some(operator.to_string()),
                created_at: chrono::Utc::now().naive_utc(),
                revision: 0,
            }
        } else {
            // 生产模式：创建并保存版本
            self.create_derived_version(
                plan_id,
                base_version.as_ref().map(|v| v.version_id.as_str()),
                window_days,
                Some(format!("一键重算 (操作人: {})", operator)),
                operator,
            )?
        };

        // 2.1 生产模式：为新版本写入“中文命名”到 config_snapshot_json（不改表结构）
        if !is_dry_run {
            let version_name_cn =
                Self::build_version_name_cn(&profile.title_cn, base_date, new_version.version_no);
            let snapshot_json = Self::upsert_version_meta_snapshot(
                new_version.config_snapshot_json.take(),
                &version_name_cn,
                &profile.strategy_key,
                profile.base_strategy,
                profile.parameters.as_ref(),
            )?;
            new_version.config_snapshot_json = Some(snapshot_json);
        }

        // 3. 计算冻结区起始日期
        let frozen_from_date = self.calculate_frozen_from_date(base_date);
        new_version.frozen_from_date = Some(frozen_from_date);

        // 4. 如果有基准版本，复制冻结区（仅生产模式）
        let frozen_count = if !is_dry_run {
            if let Some(base_ver) = &base_version {
                self.copy_frozen_zone(&base_ver.version_id, &new_version.version_id, frozen_from_date)?
            } else {
                0
            }
        } else {
            0
        };

        // 4.1 清理本版本的路径规则待确认（避免复算/局部重排造成脏数据）
        if !is_dry_run {
            if let Err(e) = self
                .path_override_pending_repo
                .delete_by_version(&new_version.version_id)
            {
                tracing::warn!(
                    version_id = %new_version.version_id,
                    "清理 path_override_pending 失败(将继续重算): {}",
                    e
                );
            }
        }

        // 5. 执行重排 (计算区)
        let end_date = base_date + chrono::Duration::days(window_days as i64);
        let machine_codes = vec!["H032".to_string(), "H033".to_string(), "H034".to_string()];
        let reschedule_result = self.execute_reschedule(
            &new_version.version_id,
            (base_date, end_date),
            &machine_codes,
            is_dry_run,
            profile.base_strategy,
            profile.parameters.clone(),
        )?;

        // 6. 提取统计信息
        let plan_items = reschedule_result.plan_items;
        let mature_count = reschedule_result.mature_count;
        let immature_count = reschedule_result.immature_count;

        // 7. 保存明细（仅生产模式）
        let inserted_count = if !is_dry_run && !plan_items.is_empty() {
            self.item_repo.batch_insert(&plan_items)?
        } else {
            plan_items.len()
        };

        // 8. 更新版本的frozen_from_date（仅生产模式）
        if !is_dry_run {
            self.version_repo.update(&new_version)?;
        }

        // 9. 生成风险快照（仅生产模式）
        if !is_dry_run {
            match self.generate_risk_snapshots(
                &new_version.version_id,
                base_date,
                end_date,
                &machine_codes,
            ) {
                Ok(count) => {
                    tracing::info!(
                        "已生成风险快照: version_id={}, count={}",
                        new_version.version_id,
                        count
                    );
                }
                Err(e) => {
                    tracing::warn!("生成风险快照失败: {}, 继续执行", e);
                }
            }
        }

        // 10. 记录操作日志（仅生产模式，TODO: 阶段3实施）
        // if !is_dry_run {
        //     ActionLogRepository.insert()
        // }

        // 11. 激活新版本（仅生产模式且auto_activate=true）
        if !is_dry_run && self.config.auto_activate {
            self.version_repo.activate_version(&new_version.version_id)?;
        }

        // 12. 触发决策视图刷新（仅生产模式）
        if !is_dry_run {
            self.trigger_decision_refresh(
                &new_version.version_id,
                ScheduleEventType::PlanItemChanged,
                Some(base_date),
                Some(end_date),
                Some(&machine_codes),
            )?;
        }

        // 13. 构建返回结果
        Ok(RecalcResult {
            version_id: new_version.version_id.clone(),
            version_no: new_version.version_no,
            total_items: inserted_count,
            mature_count,
            immature_count,
            frozen_items: frozen_count,
            recalc_items: inserted_count - frozen_count,
            elapsed_ms: 0, // TODO: 添加计时
        })
    }

    /// 局部重排 (指定日期范围)
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `start_date`: 重排起始日期
    /// - `end_date`: 重排结束日期
    /// - `operator`: 操作人
    /// - `is_dry_run`: 是否为试算模式（true=不落库，false=落库）
    ///
    /// # 返回
    /// - `Ok(RecalcResult)`: 重排成功
    /// - `Err`: 重排失败
    ///
    /// # 红线
    /// - 不删除冻结区明细
    /// - 日期范围外的明细不受影响
    ///
    /// # 试算模式 (is_dry_run=true)
    /// - 不删除现有plan_item
    /// - 不写入新的plan_item
    /// - 返回内存中的结果供前端预览
    pub fn recalc_partial(
        &self,
        version_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        _operator: &str,
        is_dry_run: bool,
        strategy: ScheduleStrategy,
    ) -> Result<RecalcResult, Box<dyn Error>> {
        // 1. 查询版本
        let version = self
            .version_repo
            .find_by_id(version_id)?
            .ok_or("Version not found")?;

        // 2. 查询冻结区明细 (用于统计)
        let frozen_items = self.item_repo.find_frozen_items(version_id)?;
        let frozen_in_range_count = frozen_items
            .iter()
            .filter(|i| i.plan_date >= start_date && i.plan_date <= end_date)
            .count();

        // 3. 删除日期范围的非冻结明细（仅生产模式）
        if !is_dry_run {
            // 注: delete_by_date_range会删除所有明细，业务层需确保冻结区不被删除
            // 这里我们先查询冻结区明细，删除后再重新插入
            let frozen_to_keep: Vec<PlanItem> = frozen_items
                .into_iter()
                .filter(|i| i.plan_date >= start_date && i.plan_date <= end_date)
                .collect();

            let _deleted_count =
                self.item_repo
                    .delete_by_date_range(version_id, start_date, end_date)?;

            // 4. 重新插入冻结区明细
            if !frozen_to_keep.is_empty() {
                self.item_repo.batch_insert(&frozen_to_keep)?;
            }
        }

        // 5. 执行重排 (计算区)
        let machine_codes = vec!["H032".to_string(), "H033".to_string(), "H034".to_string()];
        let reschedule_result = self.execute_reschedule(
            version_id,
            (start_date, end_date),
            &machine_codes,
            is_dry_run,
            strategy,
            None,
        )?;

        // 6. 提取统计信息
        let plan_items = reschedule_result.plan_items;
        let mature_count = reschedule_result.mature_count;
        let immature_count = reschedule_result.immature_count;

        // 7. 保存新明细（仅生产模式）
        let inserted_count = if !is_dry_run && !plan_items.is_empty() {
            self.item_repo.batch_insert(&plan_items)?
        } else {
            plan_items.len()
        };

        // 8. 更新风险快照（仅生产模式，TODO: 阶段3实施）
        // if !is_dry_run {
        //     RiskEngine.generate_snapshot()
        // }

        // 9. 记录操作日志（仅生产模式，TODO: 阶段3实施）
        // if !is_dry_run {
        //     ActionLogRepository.insert()
        // }

        // 10. 触发决策视图刷新（仅生产模式）
        if !is_dry_run {
            self.trigger_decision_refresh(
                version_id,
                ScheduleEventType::PlanItemChanged,
                Some(start_date),
                Some(end_date),
                Some(&machine_codes),
            )?;
        }

        // 11. 构建返回结果
        Ok(RecalcResult {
            version_id: version.version_id.clone(),
            version_no: version.version_no,
            total_items: inserted_count,
            mature_count,
            immature_count,
            frozen_items: frozen_in_range_count,
            recalc_items: inserted_count - frozen_in_range_count,
            elapsed_ms: 0, // TODO: 添加计时
        })
    }

    /// 联动窗口重排
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `trigger_date`: 触发日期
    /// - `cascade_days`: 联动天数
    /// - `operator`: 操作人
    /// - `is_dry_run`: 是否为试算模式（true=不落库，false=落库）
    ///
    /// # 返回
    /// - `Ok(RecalcResult)`: 重排成功
    /// - `Err`: 重排失败
    pub fn recalc_cascade(
        &self,
        version_id: &str,
        trigger_date: NaiveDate,
        cascade_days: i32,
        operator: &str,
        is_dry_run: bool,
        strategy: ScheduleStrategy,
    ) -> Result<RecalcResult, Box<dyn Error>> {
        // 计算联动范围
        let start_date = trigger_date;
        let end_date = trigger_date + chrono::Duration::days(cascade_days as i64);

        // 调用局部重排
        self.recalc_partial(version_id, start_date, end_date, operator, is_dry_run, strategy)
    }
}


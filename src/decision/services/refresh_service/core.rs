use super::*;

impl DecisionRefreshService {

    /// 刷新所有决策视图（P1 版本：只刷新 D1 和 D4）
    ///
    /// # 参数
    /// - `scope`: 刷新范围
    /// - `trigger`: 触发类型
    /// - `trigger_source`: 触发源（操作人/系统组件）
    ///
    /// # 返回
    /// - Ok(refresh_id): 刷新任务 ID
    /// - Err: 刷新失败错误
    pub fn refresh_all(
        &self,
        scope: RefreshScope,
        trigger: RefreshTrigger,
        trigger_source: Option<String>,
    ) -> Result<String, Box<dyn Error>> {
        let refresh_id = Uuid::new_v4().to_string();
        let started_at = Utc::now().to_rfc3339();

        let mut conn = self.conn.lock()
            .map_err(|e| format!("锁获取失败: {}", e))?;
        let tx = conn.transaction()?;

        // 记录刷新开始
        self.log_refresh_start(
            &tx,
            &refresh_id,
            &scope.version_id,
            &trigger,
            trigger_source.as_deref(),
            scope.is_full_refresh,
            &started_at,
        )?;

        let mut refreshed_tables = Vec::new();
        let mut total_rows_affected = 0;

        // 刷新 D1: 哪天最危险
        if self.should_refresh_d1(&trigger) {
            let rows = self.refresh_d1(&tx, &scope)?;
            refreshed_tables.push("decision_day_summary".to_string());
            total_rows_affected += rows;
        }

        // 刷新 D4: 哪个机组最堵
        if self.should_refresh_d4(&trigger) {
            let rows = self.refresh_d4(&tx, &scope)?;
            refreshed_tables.push("decision_machine_bottleneck".to_string());
            total_rows_affected += rows;
        }

        // ==========================================
        // P2 阶段: D2-D6 刷新已重构,直接使用 Transaction
        // ==========================================
        // 修复说明: 已将 refresh_d2~d6 方法改为直接使用 Transaction,
        // 不再创建新的 Repository,避免死锁问题。
        // ==========================================

        // 刷新 D2: 哪些紧急单无法完成
        if self.should_refresh_d2(&trigger) {
            let rows = self.refresh_d2(&tx, &scope)?;
            refreshed_tables.push("decision_order_failure_set".to_string());
            total_rows_affected += rows;
        }

        // 刷新 D3: 哪些冷料压库
        if self.should_refresh_d3(&trigger) {
            let rows = self.refresh_d3(&tx, &scope)?;
            refreshed_tables.push("decision_cold_stock_profile".to_string());
            total_rows_affected += rows;
        }

        // 刷新 D5: 换辊是否异常
        if self.should_refresh_d5(&trigger) {
            let rows = self.refresh_d5(&tx, &scope)?;
            refreshed_tables.push("decision_roll_campaign_alert".to_string());
            total_rows_affected += rows;
        }

        // 刷新 D6: 是否存在产能优化空间
        if self.should_refresh_d6(&trigger) {
            let rows = self.refresh_d6(&tx, &scope)?;
            refreshed_tables.push("decision_capacity_opportunity".to_string());
            total_rows_affected += rows;
        }

        // 记录刷新完成
        let completed_at = Utc::now().to_rfc3339();
        let duration_ms = (chrono::DateTime::parse_from_rfc3339(&completed_at)?
            .timestamp_millis()
            - chrono::DateTime::parse_from_rfc3339(&started_at)?.timestamp_millis())
            as i64;

        self.log_refresh_complete(
            &tx,
            &refresh_id,
            &refreshed_tables,
            total_rows_affected,
            &completed_at,
            duration_ms,
        )?;

        tx.commit()?;

        tracing::info!(
            "决策视图刷新完成: refresh_id={}, tables={:?}, rows={}",
            refresh_id,
            refreshed_tables,
            total_rows_affected
        );

        Ok(refresh_id)
    }

    /// 判断是否应该刷新 D1
    pub(super) fn should_refresh_d1(&self, trigger: &RefreshTrigger) -> bool {
        matches!(
            trigger,
            RefreshTrigger::RiskSnapshotUpdated
                | RefreshTrigger::PlanItemChanged
                | RefreshTrigger::VersionCreated
                | RefreshTrigger::ManualRefresh
        )
    }

    /// 判断是否应该刷新 D4
    pub(super) fn should_refresh_d4(&self, trigger: &RefreshTrigger) -> bool {
        matches!(
            trigger,
            RefreshTrigger::CapacityPoolChanged
                | RefreshTrigger::PlanItemChanged
                | RefreshTrigger::MaterialStateChanged
                | RefreshTrigger::RhythmTargetChanged
                | RefreshTrigger::VersionCreated
                | RefreshTrigger::ManualRefresh
        )
    }

    /// 判断是否应该刷新 D2: 哪些紧急单无法完成
    pub(super) fn should_refresh_d2(&self, trigger: &RefreshTrigger) -> bool {
        matches!(
            trigger,
            RefreshTrigger::PlanItemChanged
                | RefreshTrigger::MaterialStateChanged
                | RefreshTrigger::RiskSnapshotUpdated
                | RefreshTrigger::VersionCreated
                | RefreshTrigger::ManualRefresh
        )
    }

    /// 判断是否应该刷新 D3: 哪些冷料压库
    pub(super) fn should_refresh_d3(&self, trigger: &RefreshTrigger) -> bool {
        matches!(
            trigger,
            RefreshTrigger::MaterialStateChanged
                | RefreshTrigger::PlanItemChanged
                | RefreshTrigger::VersionCreated
                | RefreshTrigger::ManualRefresh
        )
    }

    /// 判断是否应该刷新 D5: 换辊是否异常
    pub(super) fn should_refresh_d5(&self, trigger: &RefreshTrigger) -> bool {
        matches!(
            trigger,
            RefreshTrigger::RollCampaignChanged
                | RefreshTrigger::MaterialStateChanged
                | RefreshTrigger::PlanItemChanged
                | RefreshTrigger::VersionCreated
                | RefreshTrigger::ManualRefresh
        )
    }

    /// 判断是否应该刷新 D6: 是否存在产能优化空间
    pub(super) fn should_refresh_d6(&self, trigger: &RefreshTrigger) -> bool {
        matches!(
            trigger,
            RefreshTrigger::CapacityPoolChanged
                | RefreshTrigger::PlanItemChanged
                | RefreshTrigger::MaterialStateChanged
                | RefreshTrigger::VersionCreated
                | RefreshTrigger::ManualRefresh
        )
    }

}

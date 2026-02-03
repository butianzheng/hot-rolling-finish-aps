use super::*;

impl PlanApi {
    // ==========================================
    // 方案管理接口
    // ==========================================

    /// 创建排产方案
    ///
    /// # 参数
    /// - plan_name: 方案名称
    /// - created_by: 创建人
    ///
    /// # 返回
    /// - Ok(String): 方案ID
    /// - Err(ApiError): API错误
    pub fn create_plan(&self, plan_name: String, created_by: String) -> ApiResult<String> {
        // 参数验证
        if plan_name.trim().is_empty() {
            return Err(ApiError::InvalidInput("方案名称不能为空".to_string()));
        }
        if created_by.trim().is_empty() {
            return Err(ApiError::InvalidInput("创建人不能为空".to_string()));
        }

        // 创建Plan实例
        let plan = Plan {
            plan_id: uuid::Uuid::new_v4().to_string(),
            plan_name,
            plan_type: "BASELINE".to_string(),
            base_plan_id: None,
            created_by: created_by.clone(),
            created_at: chrono::Local::now().naive_local(),
            updated_at: chrono::Local::now().naive_local(),
        };

        // 保存到数据库
        self.plan_repo
            .create(&plan)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None,
            action_type: "CREATE_PLAN".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: created_by,
            payload_json: Some(serde_json::json!({
                "plan_id": plan.plan_id,
                "plan_name": plan.plan_name,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("创建方案: {}", plan.plan_name)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(plan.plan_id)
    }

    /// 查询方案列表
    ///
    /// # 返回
    /// - Ok(Vec<Plan>): 方案列表
    /// - Err(ApiError): API错误
    pub fn list_plans(&self) -> ApiResult<Vec<Plan>> {
        self.plan_repo
            .list_all()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 查询方案详情
    ///
    /// # 参数
    /// - plan_id: 方案ID
    ///
    /// # 返回
    /// - Ok(Some(Plan)): 方案详情
    /// - Ok(None): 方案不存在
    /// - Err(ApiError): API错误
    pub fn get_plan_detail(&self, plan_id: &str) -> ApiResult<Option<Plan>> {
        if plan_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("方案ID不能为空".to_string()));
        }

        self.plan_repo
            .find_by_id(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 查询最近创建的激活版本ID（跨方案）
    ///
    /// 用途：前端启动时自动回填工作版本，避免“已有激活版本但界面提示未选择”。
    pub fn get_latest_active_version_id(&self) -> ApiResult<Option<String>> {
        self.plan_version_repo
            .find_latest_active_version_id()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 删除排产方案（同时删除其版本与明细）
    ///
    /// 注意：该操作为破坏性操作，仅建议在开发/测试数据管理中使用。
    pub fn delete_plan(&self, plan_id: &str, operator: &str) -> ApiResult<()> {
        if plan_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("方案ID不能为空".to_string()));
        }
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }

        // 校验存在性并取名称用于审计记录
        let plan = self
            .plan_repo
            .find_by_id(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("方案{}不存在", plan_id)))?;

        // 显式删除关联数据（避免依赖 SQLite foreign_keys 配置）
        let versions = self
            .plan_version_repo
            .find_by_plan_id(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let mut deleted_items = 0usize;
        let mut deleted_risks = 0usize;
        let mut deleted_drafts = 0usize;
        let mut detached_action_logs = 0usize;

        for v in &versions {
            deleted_drafts += self
                .strategy_draft_repo
                .delete_by_version(&v.version_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

            deleted_items += self
                .plan_item_repo
                .delete_by_version(&v.version_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
            deleted_risks += self
                .risk_snapshot_repo
                .delete_by_version(&v.version_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

            detached_action_logs += self
                .action_log_repo
                .detach_version(&v.version_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

            self.plan_version_repo
                .delete(&v.version_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        }

        self.plan_repo
            .delete(plan_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None,
            action_type: "DELETE_PLAN".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "plan_id": plan.plan_id,
                "plan_name": plan.plan_name,
                "deleted_versions": versions.len(),
                "deleted_plan_items": deleted_items,
                "deleted_risk_snapshots": deleted_risks,
                "deleted_strategy_drafts": deleted_drafts,
                "detached_action_logs": detached_action_logs,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("删除方案: {}", plan.plan_name)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    // ==========================================

}

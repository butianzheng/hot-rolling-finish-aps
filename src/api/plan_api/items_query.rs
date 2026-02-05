use super::*;

impl PlanApi {
    // ==========================================
    // 明细查询接口
    // ==========================================

    /// 查询排产明细（按版本）
    ///
    /// # 参数
    /// - version_id: 版本ID
    ///
    /// # 返回
    /// - Ok(Vec<PlanItem>): 排产明细列表
    /// - Err(ApiError): API错误
    pub fn list_plan_items(&self, version_id: &str) -> ApiResult<Vec<PlanItem>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        let mut items = self.plan_item_repo
            .find_by_version(version_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        self.enrich_plan_items(&mut items);
        Ok(items)
    }

    /// 查询排产明细（可选过滤 + 分页）
    ///
    /// 说明：
    /// - 该接口用于“增量加载/按时间窗加载”，避免前端一次性拉取全量 plan_item；
    /// - 不改变旧接口 `list_plan_items` 的语义，便于逐步迁移。
    pub fn list_plan_items_filtered(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        plan_date_from: Option<NaiveDate>,
        plan_date_to: Option<NaiveDate>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> ApiResult<Vec<PlanItem>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        if let Some(limit) = limit {
            if limit <= 0 || limit > 20_000 {
                return Err(ApiError::InvalidInput(
                    "limit必须在1-20000之间".to_string(),
                ));
            }
        }
        if let Some(offset) = offset {
            if offset < 0 {
                return Err(ApiError::InvalidInput("offset不能为负数".to_string()));
            }
        }

        self.plan_item_repo
            .find_by_filters_paged(
                version_id,
                machine_code,
                plan_date_from,
                plan_date_to,
                limit,
                offset,
            )
            .map(|mut items| {
                self.enrich_plan_items(&mut items);
                items
            })
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 查询排产明细（按日期）
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - plan_date: 排产日期
    ///
    /// # 返回
    /// - Ok(Vec<PlanItem>): 排产明细列表
    /// - Err(ApiError): API错误
    pub fn list_items_by_date(
        &self,
        version_id: &str,
        plan_date: NaiveDate,
    ) -> ApiResult<Vec<PlanItem>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        self.plan_item_repo
            .find_by_date(version_id, plan_date)
            .map(|mut items| {
                self.enrich_plan_items(&mut items);
                items
            })
            .map_err(|e| ApiError::DatabaseError(e.to_string()))
    }

    /// 查询版本排产日期边界（min/max）及数量
    ///
    /// 用途：
    /// - Workbench AUTO 日期范围计算（避免拉取全量 plan_item）
    pub fn get_plan_item_date_bounds(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
    ) -> ApiResult<PlanItemDateBoundsResponse> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        let (min_date, max_date, total_count) = self
            .plan_item_repo
            .get_plan_date_bounds(version_id, machine_code)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(PlanItemDateBoundsResponse {
            version_id: version_id.to_string(),
            machine_code: machine_code.map(|s| s.to_string()),
            min_plan_date: min_date,
            max_plan_date: max_date,
            total_count,
        })
    }

    /// 从 material_state 和 material_master 批量补充排产明细的快照字段
    ///
    /// 补充字段：urgent_level, sched_state (来自 material_state)，steel_grade (来自 material_master.steel_mark)
    fn enrich_plan_items(&self, items: &mut [PlanItem]) {
        if items.is_empty() {
            return;
        }

        let material_ids: Vec<String> = items.iter().map(|it| it.material_id.clone()).collect();

        // 1. 从 material_state 获取 urgent_level, sched_state
        if let Ok(snapshots) = self.material_state_repo.find_snapshots_by_material_ids(&material_ids) {
            let state_map: HashMap<String, MaterialStateSnapshotLite> = snapshots
                .into_iter()
                .map(|s| (s.material_id.clone(), s))
                .collect();

            for item in items.iter_mut() {
                if let Some(snap) = state_map.get(&item.material_id) {
                    if item.urgent_level.is_none() {
                        item.urgent_level = snap.urgent_level.clone();
                    }
                    if item.sched_state.is_none() {
                        item.sched_state = snap.sched_state.clone();
                    }
                }
            }
        }

        // 2. 从 material_master 获取 steel_mark/宽度/厚度（用于前端展示完整规格信息）
        if let Ok(spec_map) = self.material_master_repo.find_spec_lite_by_ids(&material_ids) {
            for item in items.iter_mut() {
                if let Some(spec) = spec_map.get(&item.material_id) {
                    if item.steel_grade.is_none() {
                        if let Some(mark) = spec.steel_mark.as_ref() {
                            item.steel_grade = Some(mark.clone());
                        }
                    }
                    if item.width_mm.is_none() {
                        item.width_mm = spec.width_mm;
                    }
                    if item.thickness_mm.is_none() {
                        item.thickness_mm = spec.thickness_mm;
                    }
                }
            }
        }
    }

    // ==========================================

}

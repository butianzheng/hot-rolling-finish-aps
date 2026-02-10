use super::*;

impl PlanApi {
    // ==========================================
    // 版本对比接口
    // ==========================================

    /// 版本对比
    ///
    /// # 参数
    /// - version_id_a: 版本A ID
    /// - version_id_b: 版本B ID
    ///
    /// # 返回
    /// - Ok(VersionComparisonResult): 对比结果
    /// - Err(ApiError): API错误
    pub fn compare_versions(
        &self,
        version_id_a: &str,
        version_id_b: &str,
    ) -> ApiResult<VersionComparisonResult> {
        // 参数验证
        if version_id_a.trim().is_empty() || version_id_b.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        // 1. 加载两个版本的排产明细
        let items_a = self
            .plan_item_repo
            .find_by_version(version_id_a)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        let items_b = self
            .plan_item_repo
            .find_by_version(version_id_b)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 2. 构建材料ID到PlanItem的映射
        use std::collections::HashMap;
        let map_a: HashMap<String, &crate::domain::plan::PlanItem> = items_a
            .iter()
            .map(|item| (item.material_id.clone(), item))
            .collect();
        let map_b: HashMap<String, &crate::domain::plan::PlanItem> = items_b
            .iter()
            .map(|item| (item.material_id.clone(), item))
            .collect();

        // 3. 计算差异
        let mut moved_count = 0;
        let mut added_count = 0;
        let mut squeezed_out_count = 0;

        // 遍历版本A的材料
        for (material_id, item_a) in map_a.iter() {
            if let Some(item_b) = map_b.get(material_id) {
                // 材料在两个版本中都存在
                if item_a.plan_date != item_b.plan_date
                    || item_a.machine_code != item_b.machine_code
                {
                    // 日期或机组变化 = 移动
                    moved_count += 1;
                }
            } else {
                // 材料只在A中，不在B中 = 被挤出
                squeezed_out_count += 1;
            }
        }

        // 遍历版本B的材料
        for (material_id, _item_b) in map_b.iter() {
            if !map_a.contains_key(material_id) {
                // 材料只在B中，不在A中 = 新增
                added_count += 1;
            }
        }

        // 删除数量 = 被挤出数量
        let removed_count = squeezed_out_count;

        // 4. 加载两个版本信息（用于配置对比）
        let version_a = self
            .plan_version_repo
            .find_by_id(version_id_a)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id_a)))?;
        let version_b = self
            .plan_version_repo
            .find_by_id(version_id_b)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id_b)))?;

        // 5. 对比配置快照
        let config_changes = self.compare_config_snapshots(
            version_a.config_snapshot_json.as_deref(),
            version_b.config_snapshot_json.as_deref(),
        )?;

        // 6. 对比风险快照（按日期聚合）
        let risk_delta = self.build_risk_delta(version_id_a, version_id_b)?;

        // 7. 对比产能变化（按机组+日期）
        let capacity_delta = self.build_capacity_delta(version_id_a, version_id_b)?;

        Ok(VersionComparisonResult {
            version_id_a: version_id_a.to_string(),
            version_id_b: version_id_b.to_string(),
            moved_count,
            added_count,
            removed_count,
            squeezed_out_count,
            risk_delta,
            capacity_delta,
            config_changes,
            message: format!(
                "版本对比完成: 移动{}个, 新增{}个, 删除{}个, 挤出{}个",
                moved_count, added_count, removed_count, squeezed_out_count
            ),
        })
    }

    /// 构建风险变化（按日期）
    ///
    /// 口径：
    /// - 使用 risk_snapshot 风险等级映射为分值后，按日期取机组均值
    /// - 变化值 = 版本乙 - 版本甲（缺失版本按 0 参与 delta 计算）
    fn build_risk_delta(
        &self,
        version_id_a: &str,
        version_id_b: &str,
    ) -> ApiResult<Option<Vec<RiskDelta>>> {
        let snapshots_a = self
            .risk_snapshot_repo
            .find_by_version_id(version_id_a)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        let snapshots_b = self
            .risk_snapshot_repo
            .find_by_version_id(version_id_b)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        if snapshots_a.is_empty() && snapshots_b.is_empty() {
            return Ok(None);
        }

        let mut daily_a: std::collections::BTreeMap<NaiveDate, (f64, usize)> =
            std::collections::BTreeMap::new();
        let mut daily_b: std::collections::BTreeMap<NaiveDate, (f64, usize)> =
            std::collections::BTreeMap::new();

        for snapshot in snapshots_a.iter() {
            let score = Self::risk_level_to_score(snapshot.risk_level);
            let entry = daily_a.entry(snapshot.snapshot_date).or_insert((0.0, 0));
            entry.0 += score;
            entry.1 += 1;
        }

        for snapshot in snapshots_b.iter() {
            let score = Self::risk_level_to_score(snapshot.risk_level);
            let entry = daily_b.entry(snapshot.snapshot_date).or_insert((0.0, 0));
            entry.0 += score;
            entry.1 += 1;
        }

        let mut all_dates: std::collections::BTreeSet<NaiveDate> =
            daily_a.keys().cloned().collect();
        all_dates.extend(daily_b.keys().cloned());

        if all_dates.is_empty() {
            return Ok(None);
        }

        let avg_score = |daily: &std::collections::BTreeMap<NaiveDate, (f64, usize)>,
                         date: &NaiveDate|
         -> Option<f64> {
            daily.get(date).map(|(sum, count)| {
                if *count == 0 {
                    0.0
                } else {
                    *sum / *count as f64
                }
            })
        };

        let rows = all_dates
            .into_iter()
            .map(|date| {
                let risk_score_a = avg_score(&daily_a, &date);
                let risk_score_b = avg_score(&daily_b, &date);
                RiskDelta {
                    date: date.format("%Y-%m-%d").to_string(),
                    risk_score_a,
                    risk_score_b,
                    risk_score_delta: risk_score_b.unwrap_or(0.0) - risk_score_a.unwrap_or(0.0),
                }
            })
            .collect::<Vec<_>>();

        Ok(Some(rows))
    }

    /// 构建产能变化（按机组+日期）
    ///
    /// 口径：
    /// - 对比 capacity_pool.used_capacity_t
    /// - used_capacity_a/b 为空表示该版本缺失该键
    /// - 变化值 = 版本乙 - 版本甲（缺失按 0 参与 delta 计算）
    fn build_capacity_delta(
        &self,
        version_id_a: &str,
        version_id_b: &str,
    ) -> ApiResult<Option<Vec<CapacityDelta>>> {
        let pools_a = self
            .capacity_repo
            .find_by_version_id(version_id_a)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        let pools_b = self
            .capacity_repo
            .find_by_version_id(version_id_b)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        if pools_a.is_empty() && pools_b.is_empty() {
            return Ok(None);
        }

        let mut used_a: std::collections::BTreeMap<(NaiveDate, String), f64> =
            std::collections::BTreeMap::new();
        let mut used_b: std::collections::BTreeMap<(NaiveDate, String), f64> =
            std::collections::BTreeMap::new();

        for pool in pools_a.into_iter() {
            used_a.insert((pool.plan_date, pool.machine_code), pool.used_capacity_t);
        }
        for pool in pools_b.into_iter() {
            used_b.insert((pool.plan_date, pool.machine_code), pool.used_capacity_t);
        }

        let mut all_keys: std::collections::BTreeSet<(NaiveDate, String)> =
            used_a.keys().cloned().collect();
        all_keys.extend(used_b.keys().cloned());

        if all_keys.is_empty() {
            return Ok(None);
        }

        let rows = all_keys
            .into_iter()
            .map(|(date, machine_code)| {
                let key = (date, machine_code.clone());
                let used_capacity_a = used_a.get(&key).copied();
                let used_capacity_b = used_b.get(&key).copied();

                CapacityDelta {
                    machine_code,
                    date: date.format("%Y-%m-%d").to_string(),
                    used_capacity_a,
                    used_capacity_b,
                    capacity_delta: used_capacity_b.unwrap_or(0.0) - used_capacity_a.unwrap_or(0.0),
                }
            })
            .collect::<Vec<_>>();

        Ok(Some(rows))
    }

    fn risk_level_to_score(level: crate::domain::types::RiskLevel) -> f64 {
        match level {
            crate::domain::types::RiskLevel::Red => 90.0,
            crate::domain::types::RiskLevel::Orange => 70.0,
            crate::domain::types::RiskLevel::Yellow => 40.0,
            crate::domain::types::RiskLevel::Green => 20.0,
        }
    }

    /// 版本对比 KPI 汇总（聚合接口，避免前端全量拉取 plan_item 再本地计算）
    ///
    /// 说明：
    /// - plan_item 侧：使用 SQL 聚合（count/sum/min/max + diff counts）
    /// - risk_snapshot 侧：基于既有读模型聚合（mature/immature、overflow_days/overflow_t 等）
    pub fn compare_versions_kpi(
        &self,
        version_id_a: &str,
        version_id_b: &str,
    ) -> ApiResult<VersionComparisonKpiResult> {
        if version_id_a.trim().is_empty() || version_id_b.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        // 版本存在性校验（避免 silent 0）
        let _version_a = self
            .plan_version_repo
            .find_by_id(version_id_a)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id_a)))?;

        let _version_b = self
            .plan_version_repo
            .find_by_id(version_id_b)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("版本{}不存在", version_id_b)))?;

        let agg_a = self
            .plan_item_repo
            .get_version_agg(version_id_a)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        let agg_b = self
            .plan_item_repo
            .get_version_agg(version_id_b)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let diff_counts = self
            .plan_item_repo
            .get_versions_diff_counts(version_id_a, version_id_b)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let build_risk_kpi = |version_id: &str| -> ApiResult<VersionRiskKpi> {
            let snapshots = self
                .risk_snapshot_repo
                .find_by_version_id(version_id)
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

            if snapshots.is_empty() {
                return Ok(VersionRiskKpi::empty());
            }

            let mut snapshot_date_from: Option<NaiveDate> = None;
            let mut snapshot_date_to: Option<NaiveDate> = None;
            let mut overflow_dates: HashSet<NaiveDate> = HashSet::new();

            let mut overflow_t = 0.0;
            let mut used_capacity_t = 0.0;
            let mut target_capacity_t = 0.0;
            let mut limit_capacity_t = 0.0;
            let mut mature_backlog_t = 0.0;
            let mut immature_backlog_t = 0.0;
            let mut urgent_total_t = 0.0;

            for s in snapshots.iter() {
                snapshot_date_from = match snapshot_date_from {
                    Some(d) => Some(std::cmp::min(d, s.snapshot_date)),
                    None => Some(s.snapshot_date),
                };
                snapshot_date_to = match snapshot_date_to {
                    Some(d) => Some(std::cmp::max(d, s.snapshot_date)),
                    None => Some(s.snapshot_date),
                };

                if s.overflow_t > 0.0 {
                    overflow_dates.insert(s.snapshot_date);
                }

                overflow_t += s.overflow_t;
                used_capacity_t += s.used_capacity_t;
                target_capacity_t += s.target_capacity_t;
                limit_capacity_t += s.limit_capacity_t;
                mature_backlog_t += s.mature_backlog_t;
                immature_backlog_t += s.immature_backlog_t;
                urgent_total_t += s.urgent_total_t;
            }

            let capacity_util_pct = if target_capacity_t > 0.0 {
                (used_capacity_t / target_capacity_t) * 100.0
            } else {
                0.0
            };

            Ok(VersionRiskKpi {
                overflow_days: overflow_dates.len(),
                overflow_t,
                used_capacity_t,
                target_capacity_t,
                limit_capacity_t,
                capacity_util_pct,
                mature_backlog_t,
                immature_backlog_t,
                urgent_total_t,
                snapshot_date_from,
                snapshot_date_to,
            })
        };

        let risk_a = build_risk_kpi(version_id_a)?;
        let risk_b = build_risk_kpi(version_id_b)?;

        let missing_risk_snapshot = risk_a.is_empty() || risk_b.is_empty();
        let message = if missing_risk_snapshot {
            "KPI 汇总完成（部分版本缺少 risk_snapshot，相关指标将返回 null）".to_string()
        } else {
            "KPI 汇总完成".to_string()
        };

        Ok(VersionComparisonKpiResult {
            version_id_a: version_id_a.to_string(),
            version_id_b: version_id_b.to_string(),
            kpi_a: VersionKpiSummary::from_aggs(agg_a, risk_a),
            kpi_b: VersionKpiSummary::from_aggs(agg_b, risk_b),
            diff_counts: VersionDiffCounts {
                moved_count: diff_counts.moved_count,
                added_count: diff_counts.added_count,
                removed_count: diff_counts.removed_count,
                squeezed_out_count: diff_counts.squeezed_out_count,
            },
            message,
        })
    }

    /// 对比配置快照
    ///
    /// # 参数
    /// - snapshot_a: 版本A的配置快照JSON
    /// - snapshot_b: 版本B的配置快照JSON
    ///
    /// # 返回
    /// - Ok(Option<Vec<ConfigChange>>): 配置变化列表
    /// - Err(ApiError): 解析失败
    fn compare_config_snapshots(
        &self,
        snapshot_a: Option<&str>,
        snapshot_b: Option<&str>,
    ) -> ApiResult<Option<Vec<ConfigChange>>> {
        use std::collections::HashMap;

        // 如果两个快照都不存在，返回None
        if snapshot_a.is_none() && snapshot_b.is_none() {
            return Ok(None);
        }

        // 解析快照A
        let config_a: HashMap<String, String> = if let Some(json) = snapshot_a {
            serde_json::from_str(json)
                .map_err(|e| ApiError::InvalidInput(format!("解析配置快照A失败: {}", e)))?
        } else {
            HashMap::new()
        };

        // 解析快照B
        let config_b: HashMap<String, String> = if let Some(json) = snapshot_b {
            serde_json::from_str(json)
                .map_err(|e| ApiError::InvalidInput(format!("解析配置快照B失败: {}", e)))?
        } else {
            HashMap::new()
        };

        // 过滤元信息字段（例如版本中文命名），避免污染“配置差异”视图。
        let mut config_a = config_a;
        let mut config_b = config_b;
        config_a.retain(|k, _| !k.starts_with("__meta_"));
        config_b.retain(|k, _| !k.starts_with("__meta_"));

        // 收集所有配置键
        let mut all_keys: std::collections::HashSet<String> = config_a.keys().cloned().collect();
        all_keys.extend(config_b.keys().cloned());

        // 对比配置
        let mut changes = Vec::new();
        for key in all_keys {
            let value_a = config_a.get(&key).cloned();
            let value_b = config_b.get(&key).cloned();

            // 只记录有变化的配置
            if value_a != value_b {
                changes.push(ConfigChange {
                    key: key.clone(),
                    value_a,
                    value_b,
                });
            }
        }

        if changes.is_empty() {
            Ok(None)
        } else {
            Ok(Some(changes))
        }
    }

    // ==========================================
}

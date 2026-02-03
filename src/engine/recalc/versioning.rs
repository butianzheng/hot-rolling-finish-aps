use super::RecalcEngine;
use crate::config::strategy_profile::CustomStrategyParameters;
use crate::domain::plan::{PlanItem, PlanVersion};
use crate::domain::types::PlanVersionStatus;
use crate::engine::strategy::ScheduleStrategy;
use chrono::NaiveDate;
use std::collections::HashMap;
use std::error::Error;
use uuid::Uuid;

impl RecalcEngine {
    pub(super) fn build_version_name_cn(
        strategy_title_cn: &str,
        base_date: NaiveDate,
        version_no: i32,
    ) -> String {
        let date_part = base_date.format("%m%d").to_string();
        format!("{}-{}-{:03}", strategy_title_cn, date_part, version_no)
    }

    pub(super) fn upsert_version_meta_snapshot(
        snapshot_json: Option<String>,
        version_name_cn: &str,
        strategy_key: &str,
        base_strategy: ScheduleStrategy,
        strategy_params: Option<&CustomStrategyParameters>,
    ) -> Result<String, Box<dyn Error>> {
        let mut map: HashMap<String, String> = match snapshot_json.as_deref() {
            Some(raw) => match serde_json::from_str(raw) {
                Ok(v) => v,
                Err(_) => {
                    let mut m = HashMap::new();
                    m.insert("__meta_config_snapshot_raw".to_string(), raw.to_string());
                    m
                }
            },
            None => HashMap::new(),
        };

        map.insert(
            "__meta_version_name_cn".to_string(),
            version_name_cn.to_string(),
        );
        map.insert("__meta_strategy".to_string(), strategy_key.to_string());
        map.insert(
            "__meta_strategy_base".to_string(),
            base_strategy.as_str().to_string(),
        );

        if let Some(p) = strategy_params {
            let raw = serde_json::to_string(p)?;
            map.insert("__meta_strategy_params_json".to_string(), raw);
        }

        Ok(serde_json::to_string(&map)?)
    }

    /// 创建派生版本 (基于现有版本)
    ///
    /// # 参数
    /// - `plan_id`: 方案ID
    /// - `base_version_id`: 基准版本ID (可选，如果为None则基于激活版本)
    /// - `window_days`: 计算窗口天数
    /// - `note`: 备注
    /// - `operator`: 操作人
    ///
    /// # 返回
    /// - `Ok(PlanVersion)`: 新版本
    /// - `Err`: 创建失败
    pub fn create_derived_version(
        &self,
        plan_id: &str,
        _base_version_id: Option<&str>,
        window_days: i32,
        _note: Option<String>,
        operator: &str,
    ) -> Result<PlanVersion, Box<dyn Error>> {
        // 1. 获取配置快照
        let config_snapshot = self.config_manager.get_config_snapshot()?;

        // 2. 创建PlanVersion对象（version_no 由仓储层在事务内分配，避免并发冲突）
        let mut version = PlanVersion {
            version_id: Uuid::new_v4().to_string(),
            plan_id: plan_id.to_string(),
            version_no: 0,
            status: PlanVersionStatus::Draft,
            frozen_from_date: None, // 将在recalc_full中设置
            recalc_window_days: Some(window_days),
            config_snapshot_json: Some(config_snapshot), // 存储配置快照
            created_by: Some(operator.to_string()),
            created_at: chrono::Utc::now().naive_utc(),
            revision: 0, // 乐观锁：初始修订号
        };

        // 3. 保存版本
        self.version_repo.create_with_next_version_no(&mut version)?;

        Ok(version)
    }

    /// 复制冻结区 (从旧版本到新版本)
    ///
    /// # 参数
    /// - `from_version_id`: 源版本ID
    /// - `to_version_id`: 目标版本ID
    /// - `frozen_from_date`: 冻结区起始日期 (< frozen_from_date的明细被复制)
    ///
    /// # 返回
    /// - `Ok(count)`: 复制的明细数量
    /// - `Err`: 复制失败
    ///
    /// # 红线
    /// - 只复制 locked_in_plan = true 的明细
    /// - 只复制 plan_date < frozen_from_date 的明细
    pub fn copy_frozen_zone(
        &self,
        from_version_id: &str,
        to_version_id: &str,
        frozen_from_date: NaiveDate,
    ) -> Result<usize, Box<dyn Error>> {
        // 1. 查询源版本的冻结区明细
        let frozen_items = self.item_repo.find_frozen_items(from_version_id)?;

        // 2. 过滤: 只复制 plan_date < frozen_from_date 的明细
        let items_to_copy: Vec<PlanItem> = frozen_items
            .into_iter()
            .filter(|item| item.plan_date < frozen_from_date)
            .map(|mut item| {
                // 修改version_id为目标版本
                item.version_id = to_version_id.to_string();
                // 确保source_type为FROZEN
                item.source_type = "FROZEN".to_string();
                item
            })
            .collect();

        // 3. 批量插入
        let count = self.item_repo.batch_insert(&items_to_copy)?;

        Ok(count)
    }

    /// 计算冻结区起始日期
    ///
    /// # 参数
    /// - `base_date`: 基准日期
    ///
    /// # 返回
    /// - 冻结区起始日期 (base_date - frozen_days_before_today)
    pub(super) fn calculate_frozen_from_date(&self, base_date: NaiveDate) -> NaiveDate {
        base_date - chrono::Duration::days(self.config.frozen_days_before_today as i64)
    }
}


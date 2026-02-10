use super::RecalcEngine;
use crate::config::config_keys;
use crate::domain::material::MaterialState;
use crate::domain::plan::PlanItem;
use chrono::NaiveDate;
use std::error::Error;

impl RecalcEngine {
    /// 收集统计信息
    ///
    /// # 参数
    /// - `items`: 排产明细列表
    ///
    /// # 返回
    /// - (scheduled_count, frozen_count)
    fn collect_statistics(&self, items: &[PlanItem]) -> (usize, usize) {
        let scheduled_count = items.len();
        let frozen_count = items.iter().filter(|i| i.locked_in_plan).count();
        (scheduled_count, frozen_count)
    }

    /// 生成风险快照（批量）
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `start_date`: 起始日期
    /// - `end_date`: 结束日期
    /// - `machine_codes`: 机组代码列表
    ///
    /// # 返回
    /// - Ok(usize): 成功生成的快照数量
    /// - Err: 生成失败
    ///
    /// # 说明
    /// - 为每个机组、每个日期生成一个风险快照
    /// - 使用 RiskEngine 计算风险等级
    /// - 批量插入到 risk_snapshot 表
    pub(super) fn generate_risk_snapshots(
        &self,
        version_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        machine_codes: &[String],
    ) -> Result<usize, Box<dyn Error>> {
        use std::collections::HashMap;

        // 超限严重阈值（默认 10%）；兼容用户按百分比输入（如 5 表示 5%）
        let overflow_red_threshold_pct = self
            .config_manager
            .get_global_config_value(config_keys::OVERFLOW_PCT)
            .ok()
            .flatten()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .map(|v| if v > 1.0 && v <= 100.0 { v / 100.0 } else { v })
            .filter(|v| *v > 0.0 && *v <= 1.0)
            .unwrap_or(0.1);

        // 1. 查询版本的所有 plan_item（已排产材料）
        let all_plan_items = self.item_repo.find_by_version(version_id)?;

        // 2. 对每个机组，查询材料主数据和材料状态
        let mut snapshots = Vec::new();

        for machine_code in machine_codes {
            // 2.1 查询该机组的材料主数据
            let materials = self.material_master_repo.find_by_machine(machine_code)?;

            // 2.2 构建材料重量映射
            let material_weights: HashMap<String, f64> = materials
                .iter()
                .map(|m| (m.material_id.clone(), m.weight_t.unwrap_or(0.0)))
                .collect();

            // 2.3 查询材料状态
            let material_states: Vec<MaterialState> = materials
                .iter()
                .filter_map(|m| {
                    self.material_state_repo
                        .find_by_id(&m.material_id)
                        .ok()
                        .flatten()
                })
                .collect();

            // 2.4 遍历日期，生成快照
            let mut current_date = start_date;
            while current_date <= end_date {
                // 查询或创建产能池
                let pool = self
                    .capacity_repo
                    .find_by_machine_and_date(version_id, machine_code, current_date)?
                    .unwrap_or_else(|| {
                        Self::create_default_capacity_pool(version_id, machine_code, current_date)
                    });

                // 筛选当日当机组的排产明细
                let scheduled_items: Vec<PlanItem> = all_plan_items
                    .iter()
                    .filter(|item| {
                        item.machine_code == *machine_code && item.plan_date == current_date
                    })
                    .cloned()
                    .collect();

                // 生成快照
                let snapshot = self.risk_engine.generate_snapshot(
                    version_id,
                    machine_code,
                    current_date,
                    &pool,
                    &scheduled_items,
                    &material_states,
                    &material_weights,
                    None,
                    overflow_red_threshold_pct,
                );

                snapshots.push(snapshot);

                current_date = current_date
                    .checked_add_signed(chrono::Duration::days(1))
                    .ok_or("Date overflow")?;
            }
        }

        // 3. 批量插入快照
        let count = self.risk_snapshot_repo.batch_insert(snapshots)?;

        Ok(count)
    }
}

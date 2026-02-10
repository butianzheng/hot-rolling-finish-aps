// ==========================================
// D5 模块 - 换辊异常监控
// ==========================================
// 职责: 刷新换辊状态告警（时间线仿真 + 阈值判定）
// 输入: 版本ID + 机组列表 + 计划项
// 输出: decision_roll_campaign_alert 表
// ==========================================

mod alert;
mod campaign_state;
mod schema_check;
mod thresholds;
mod timeline;

use super::*;
use alert::calculate_alert;
use campaign_state::CampaignStreamState;
use schema_check::table_has_column;
use thresholds::{parse_dt_best_effort, read_global_i32, read_global_real, ymd_to_start_at};
use timeline::{produce_weight_until, simulate_to_as_of, PlanItemLite};

impl DecisionRefreshService {
    /// 刷新 D5: 换辊是否异常
    ///
    /// # 职责
    /// - 基于时间线仿真估算换辊批次状态
    /// - 计算当前累计重量、预计阈值触达时间
    /// - 生成告警级别和建议操作
    ///
    /// # 参数
    /// - `tx`: SQLite 事务
    /// - `scope`: 刷新范围（版本ID + 可选机组列表）
    ///
    /// # 返回
    /// - `Ok(usize)`: 成功插入的告警记录数
    /// - `Err`: 刷新失败
    ///
    /// # 说明
    /// - 新口径：换辊从"风险/约束"调整为"设备时间监控"
    /// - 周期起点：来自 roll_campaign_plan.initial_start_at（可人工微调）；缺省用该机组最早 plan_date 00:00。
    /// - 当前累计：按计划项时间线（plan_item + hourly_capacity_t）估算到 as_of（刷新时刻）。
    /// - 周期重置：默认在到达软限制时触发换辊（并产生停机时长）；用于避免"全版本求和导致 300%+"的问题。
    /// - 计划换辊时刻：允许通过 roll_campaign_plan.next_change_at 覆盖（只影响"下一次换辊"提示，不直接改排程）。
    pub(super) fn refresh_d5(
        &self,
        tx: &Transaction,
        scope: &RefreshScope,
    ) -> Result<usize, Box<dyn Error>> {
        // ==========================================
        // 1. 数据库表结构兼容性检查与初始化
        // ==========================================

        // 确保 roll_campaign_plan 表存在
        let _ = tx.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS roll_campaign_plan (
              version_id TEXT NOT NULL,
              machine_code TEXT NOT NULL,
              initial_start_at TEXT NOT NULL,
              next_change_at TEXT,
              downtime_minutes INTEGER,
              updated_at TEXT NOT NULL DEFAULT (datetime('now')),
              updated_by TEXT,
              PRIMARY KEY (version_id, machine_code)
            );
            "#,
        );

        // 添加扩展列（兼容旧数据库）
        if !table_has_column(tx, "decision_roll_campaign_alert", "campaign_start_at") {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN campaign_start_at TEXT",
                [],
            );
        }
        if !table_has_column(tx, "decision_roll_campaign_alert", "planned_change_at") {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN planned_change_at TEXT",
                [],
            );
        }
        if !table_has_column(
            tx,
            "decision_roll_campaign_alert",
            "planned_downtime_minutes",
        ) {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN planned_downtime_minutes INTEGER",
                [],
            );
        }
        if !table_has_column(
            tx,
            "decision_roll_campaign_alert",
            "estimated_soft_reach_at",
        ) {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN estimated_soft_reach_at TEXT",
                [],
            );
        }
        if !table_has_column(
            tx,
            "decision_roll_campaign_alert",
            "estimated_hard_reach_at",
        ) {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN estimated_hard_reach_at TEXT",
                [],
            );
        }

        // ==========================================
        // 2. 删除旧数据
        // ==========================================

        let delete_sql = if let Some(machines) = &scope.affected_machines {
            format!(
                "DELETE FROM decision_roll_campaign_alert WHERE version_id = ?1 AND machine_code IN ({})",
                machines.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
            )
        } else {
            "DELETE FROM decision_roll_campaign_alert WHERE version_id = ?1".to_string()
        };

        if let Some(machines) = &scope.affected_machines {
            let mut params: Vec<&dyn rusqlite::ToSql> = vec![&scope.version_id];
            let machine_refs: Vec<&dyn rusqlite::ToSql> =
                machines.iter().map(|m| m as &dyn rusqlite::ToSql).collect();
            params.extend(machine_refs);
            tx.execute(&delete_sql, rusqlite::params_from_iter(params))?;
        } else {
            tx.execute(&delete_sql, rusqlite::params![&scope.version_id])?;
        }

        // ==========================================
        // 3. 读取全局阈值配置
        // ==========================================

        let suggest_threshold_t = read_global_real(tx, "roll_suggest_threshold_t")?
            .filter(|v| *v > 0.0)
            .unwrap_or(1500.0);
        let hard_limit_t = read_global_real(tx, "roll_hard_limit_t")?
            .filter(|v| *v > 0.0)
            .unwrap_or(2500.0);
        let default_downtime_minutes = read_global_i32(tx, "roll_change_downtime_minutes")?
            .filter(|v| *v > 0)
            .unwrap_or(45);

        // ==========================================
        // 4. 获取机组列表及小时产能
        // ==========================================

        let has_hourly_capacity = table_has_column(tx, "machine_master", "hourly_capacity_t");
        let has_is_active = table_has_column(tx, "machine_master", "is_active");

        let mut machine_sql = String::new();
        if has_hourly_capacity {
            machine_sql.push_str(
                "SELECT machine_code, COALESCE(hourly_capacity_t, 0) AS hourly_capacity_t FROM machine_master",
            );
        } else {
            machine_sql.push_str("SELECT machine_code, 0 AS hourly_capacity_t FROM machine_master");
        }
        if has_is_active {
            machine_sql.push_str(" WHERE is_active = 1");
        }

        // 过滤指定机组
        if let Some(machines) = &scope.affected_machines {
            if !machines.is_empty() {
                let placeholders = machines.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                if machine_sql.contains(" WHERE ") {
                    machine_sql.push_str(&format!(" AND machine_code IN ({})", placeholders));
                } else {
                    machine_sql.push_str(&format!(" WHERE machine_code IN ({})", placeholders));
                }
            }
        }

        machine_sql.push_str(" ORDER BY machine_code ASC");

        let mut machine_stmt = tx.prepare(&machine_sql)?;
        let machines: Vec<(String, f64)> = if let Some(machines) = &scope.affected_machines {
            if machines.is_empty() {
                machine_stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
                    })?
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                let params: Vec<&dyn rusqlite::ToSql> =
                    machines.iter().map(|m| m as &dyn rusqlite::ToSql).collect();
                machine_stmt
                    .query_map(rusqlite::params_from_iter(params), |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
                    })?
                    .collect::<Result<Vec<_>, _>>()?
            }
        } else {
            machine_stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?
        };

        if machines.is_empty() {
            return Ok(0);
        }

        // ==========================================
        // 5. 检查 plan_item 列兼容性
        // ==========================================

        let has_weight_t = table_has_column(tx, "plan_item", "weight_t");
        let has_seq_no = table_has_column(tx, "plan_item", "seq_no");

        if !has_weight_t {
            // 无法估算时间线（缺少重量），直接跳过刷新
            return Ok(0);
        }

        // ==========================================
        // 6. 遍历机组，仿真时间线并生成告警
        // ==========================================

        let as_of = Local::now().naive_local();
        let mut inserted = 0usize;

        for (machine_code, hourly_capacity_t) in machines {
            // 6.1 查询机组计划覆盖配置
            let plan_row: Option<(String, Option<String>, Option<i32>)> = tx
                .query_row(
                    r#"
                    SELECT initial_start_at, next_change_at, downtime_minutes
                    FROM roll_campaign_plan
                    WHERE version_id = ?1 AND machine_code = ?2
                    "#,
                    rusqlite::params![&scope.version_id, &machine_code],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?;

            // 6.2 查询计划项
            let order_clause = if has_seq_no {
                "ORDER BY plan_date ASC, seq_no ASC"
            } else {
                "ORDER BY plan_date ASC"
            };
            let pi_sql = format!(
                "SELECT plan_date, weight_t FROM plan_item WHERE version_id = ?1 AND machine_code = ?2 {}",
                order_clause
            );
            let mut pi_stmt = tx.prepare(&pi_sql)?;
            let pi_iter =
                pi_stmt.query_map(rusqlite::params![&scope.version_id, &machine_code], |row| {
                    let plan_date: String = row.get(0)?;
                    let weight_t: f64 = row.get(1)?;
                    let start_at = ymd_to_start_at(&plan_date).unwrap_or_else(|| as_of);
                    Ok(PlanItemLite {
                        earliest_start_at: start_at,
                        weight_t,
                    })
                })?;

            let mut items: Vec<PlanItemLite> = pi_iter.collect::<Result<Vec<_>, _>>()?;
            items.retain(|i| i.weight_t > 0.0);
            items.sort_by_key(|i| i.earliest_start_at);

            // 6.3 确定初始起始时间和停机时长
            let default_start_at = items
                .first()
                .map(|i| i.earliest_start_at)
                .or_else(|| as_of.date().and_hms_opt(0, 0, 0))
                .unwrap_or(as_of);

            let initial_start_at = plan_row
                .as_ref()
                .and_then(|(s, _, _)| parse_dt_best_effort(s))
                .unwrap_or(default_start_at);

            let override_next_change_at = plan_row
                .as_ref()
                .and_then(|(_, v, _)| v.as_deref())
                .and_then(parse_dt_best_effort);

            let override_downtime_minutes = plan_row.and_then(|(_, _, m)| m).filter(|v| *v > 0);
            let planned_downtime_minutes =
                override_downtime_minutes.unwrap_or(default_downtime_minutes);

            let rate_t_per_sec = if hourly_capacity_t > 0.0 {
                hourly_capacity_t / 3600.0
            } else {
                0.0
            };

            // 6.4 仿真时间线到当前时刻（as_of）
            let state_at_as_of = if rate_t_per_sec > 0.0 && !items.is_empty() {
                simulate_to_as_of(
                    &items,
                    rate_t_per_sec,
                    initial_start_at,
                    suggest_threshold_t,
                    planned_downtime_minutes as i64,
                    as_of,
                )
            } else {
                // 无法仿真时间线（无产能或无计划项），回退到按日期聚合
                let cum_weight_t: f64 = items
                    .iter()
                    .filter(|i| {
                        i.earliest_start_at >= initial_start_at && i.earliest_start_at <= as_of
                    })
                    .map(|i| i.weight_t)
                    .sum();
                CampaignStreamState {
                    item_index: 0,
                    remaining_weight_t: 0.0,
                    current_time: as_of,
                    campaign_no: 1,
                    campaign_start_at: initial_start_at,
                    cum_weight_t,
                }
            };

            // 6.5 估算软/硬限制触达时间
            let (soft_reach, hard_reach) = if rate_t_per_sec > 0.0 && !items.is_empty() {
                let soft_need = (suggest_threshold_t - state_at_as_of.cum_weight_t).max(0.0);
                let hard_need = (hard_limit_t - state_at_as_of.cum_weight_t).max(0.0);

                let base_state_for_future = CampaignStreamState {
                    item_index: state_at_as_of.item_index,
                    remaining_weight_t: state_at_as_of.remaining_weight_t,
                    current_time: state_at_as_of.current_time,
                    campaign_no: state_at_as_of.campaign_no,
                    campaign_start_at: state_at_as_of.campaign_start_at,
                    cum_weight_t: state_at_as_of.cum_weight_t,
                };

                let soft_reach = if suggest_threshold_t > 0.0 {
                    produce_weight_until(
                        &items,
                        base_state_for_future.clone(),
                        rate_t_per_sec,
                        soft_need,
                    )
                    .map(|s| s.current_time)
                } else {
                    None
                };
                let hard_reach = if hard_limit_t > 0.0 {
                    produce_weight_until(
                        &items,
                        base_state_for_future.clone(),
                        rate_t_per_sec,
                        hard_need,
                    )
                    .map(|s| s.current_time)
                } else {
                    None
                };
                (soft_reach, hard_reach)
            } else {
                (None, None)
            };

            // 6.6 确定计划换辊时间
            let planned_change_at = override_next_change_at
                .filter(|dt| *dt > as_of && *dt >= state_at_as_of.campaign_start_at)
                .or(soft_reach);

            // 6.7 计算告警级别
            let alert_result = calculate_alert(
                state_at_as_of.cum_weight_t,
                suggest_threshold_t,
                hard_limit_t,
                soft_reach,
                hard_reach,
                planned_change_at,
            );

            let estimated_change_date =
                hard_reach.map(|dt| dt.date().format("%Y-%m-%d").to_string());

            // 6.8 插入告警记录
            tx.execute(
                r#"
                INSERT INTO decision_roll_campaign_alert (
                    version_id,
                    machine_code,
                    campaign_no,
                    cum_weight_t,
                    suggest_threshold_t,
                    hard_limit_t,
                    alert_level,
                    reason,
                    distance_to_suggest,
                    distance_to_hard,
                    utilization_rate,
                    estimated_change_date,
                    needs_immediate_change,
                    suggested_actions,
                    campaign_start_at,
                    planned_change_at,
                    planned_downtime_minutes,
                    estimated_soft_reach_at,
                    estimated_hard_reach_at,
                    refreshed_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, datetime('now')
                )
                "#,
                rusqlite::params![
                    &scope.version_id,
                    &machine_code,
                    state_at_as_of.campaign_no,
                    state_at_as_of.cum_weight_t,
                    suggest_threshold_t,
                    hard_limit_t,
                    alert_result.level.as_str(),
                    alert_result.reason,
                    suggest_threshold_t - state_at_as_of.cum_weight_t,
                    hard_limit_t - state_at_as_of.cum_weight_t,
                    if suggest_threshold_t > 0.0 {
                        state_at_as_of.cum_weight_t / suggest_threshold_t
                    } else {
                        0.0
                    },
                    estimated_change_date,
                    if alert_result.needs_immediate_change { 1 } else { 0 },
                    alert_result.suggested_actions,
                    state_at_as_of.campaign_start_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                    planned_change_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
                    planned_downtime_minutes,
                    soft_reach.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
                    hard_reach.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
                ],
            )?;

            inserted += 1;
        }

        Ok(inserted)
    }
}

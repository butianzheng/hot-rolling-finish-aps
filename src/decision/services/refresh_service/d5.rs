use super::*;

impl DecisionRefreshService {

    /// 刷新 D5: 换辊是否异常
    pub(super) fn refresh_d5(
        &self,
        tx: &Transaction,
        scope: &RefreshScope,
    ) -> Result<usize, Box<dyn Error>> {
        // ==========================================
        // 新口径：换辊从“风险/约束”调整为“设备时间监控”
        // - 周期起点：来自 roll_campaign_plan.initial_start_at（可人工微调）；缺省用该机组最早 plan_date 00:00。
        // - 当前累计：按计划项时间线（plan_item + hourly_capacity_t）估算到 as_of（刷新时刻）。
        // - 周期重置：默认在到达软限制时触发换辊（并产生停机时长）；用于避免“全版本求和导致 300%+”的问题。
        // - 计划换辊时刻：允许通过 roll_campaign_plan.next_change_at 覆盖（只影响“下一次换辊”提示，不直接改排程）。
        // ==========================================

        fn table_has_column(tx: &Transaction, table: &str, col: &str) -> bool {
            if table.trim().is_empty() || col.trim().is_empty() {
                return false;
            }
            // NOTE: `pragma_table_info(?1)` is not reliably parameterizable across SQLite builds.
            // Since `table` is an internal constant, we safely inline it.
            let table_escaped = table.replace('\'', "''");
            let sql = format!(
                "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?1",
                table_escaped
            );
            tx.query_row(&sql, rusqlite::params![col], |row| row.get::<_, i32>(0))
                .map(|v| v > 0)
                .unwrap_or(false)
        }

        // Ensure D5 extension columns exist (best-effort, keeps old DB compatible).
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

        if table_has_column(tx, "decision_roll_campaign_alert", "campaign_start_at") == false {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN campaign_start_at TEXT",
                [],
            );
        }
        if table_has_column(tx, "decision_roll_campaign_alert", "planned_change_at") == false {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN planned_change_at TEXT",
                [],
            );
        }
        if table_has_column(tx, "decision_roll_campaign_alert", "planned_downtime_minutes") == false
        {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN planned_downtime_minutes INTEGER",
                [],
            );
        }
        if table_has_column(tx, "decision_roll_campaign_alert", "estimated_soft_reach_at") == false
        {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN estimated_soft_reach_at TEXT",
                [],
            );
        }
        if table_has_column(tx, "decision_roll_campaign_alert", "estimated_hard_reach_at") == false
        {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN estimated_hard_reach_at TEXT",
                [],
            );
        }

        // 1) 删除旧数据
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
            let machine_refs: Vec<&dyn rusqlite::ToSql> = machines
                .iter()
                .map(|m| m as &dyn rusqlite::ToSql)
                .collect();
            params.extend(machine_refs);
            tx.execute(&delete_sql, rusqlite::params_from_iter(params))?;
        } else {
            tx.execute(&delete_sql, rusqlite::params![&scope.version_id])?;
        }

        // 2) 读取全局阈值与默认停机时长
        let read_global_real = |key: &str| -> Result<Option<f64>, rusqlite::Error> {
            tx.query_row(
                "SELECT value FROM config_kv WHERE scope_id = 'global' AND key = ?1 LIMIT 1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map(|opt| opt.and_then(|s| s.trim().parse::<f64>().ok()))
        };

        let read_global_i32 = |key: &str| -> Result<Option<i32>, rusqlite::Error> {
            tx.query_row(
                "SELECT value FROM config_kv WHERE scope_id = 'global' AND key = ?1 LIMIT 1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map(|opt| opt.and_then(|s| s.trim().parse::<i32>().ok()))
        };

        let suggest_threshold_t = read_global_real("roll_suggest_threshold_t")
            .unwrap_or(None)
            .filter(|v| *v > 0.0)
            .unwrap_or(1500.0);
        let hard_limit_t = read_global_real("roll_hard_limit_t")
            .unwrap_or(None)
            .filter(|v| *v > 0.0)
            .unwrap_or(2500.0);
        let default_downtime_minutes = read_global_i32("roll_change_downtime_minutes")
            .unwrap_or(None)
            .filter(|v| *v > 0)
            .unwrap_or(45);

        // 3) 获取机组列表 + 小时产能
        let has_hourly_capacity = table_has_column(tx, "machine_master", "hourly_capacity_t");
        let has_is_active = table_has_column(tx, "machine_master", "is_active");

        let mut machine_sql = String::new();
        if has_hourly_capacity {
            machine_sql.push_str("SELECT machine_code, COALESCE(hourly_capacity_t, 0) AS hourly_capacity_t FROM machine_master");
        } else {
            machine_sql.push_str("SELECT machine_code, 0 AS hourly_capacity_t FROM machine_master");
        }
        if has_is_active {
            machine_sql.push_str(" WHERE is_active = 1");
        }

        // Filter machines if scope provided.
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
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)))?
                .collect::<Result<Vec<_>, _>>()?
        };
        if machines.is_empty() {
            return Ok(0);
        }

        // 4) 读取 plan_item 列存在性（兼容测试库/旧库）
        let has_weight_t = table_has_column(tx, "plan_item", "weight_t");
        let has_seq_no = table_has_column(tx, "plan_item", "seq_no");

        if !has_weight_t {
            // 无法估算时间线（缺少重量），直接跳过刷新，避免 UI 因校验失败崩溃。
            return Ok(0);
        }

        #[derive(Debug, Clone)]
        struct PlanItemLite {
            earliest_start_at: NaiveDateTime,
            weight_t: f64,
        }

        fn parse_dt_best_effort(raw: &str) -> Option<NaiveDateTime> {
            let s = raw.trim();
            if s.is_empty() {
                return None;
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                return Some(dt);
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
                return Some(dt);
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
                return Some(dt);
            }
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                return Some(dt.naive_local());
            }
            None
        }

        fn ymd_to_start_at(ymd: &str) -> Option<NaiveDateTime> {
            let d = NaiveDate::parse_from_str(ymd, "%Y-%m-%d").ok()?;
            d.and_hms_opt(0, 0, 0)
        }

        #[derive(Debug, Clone)]
        struct StreamState {
            item_index: usize,
            remaining_weight_t: f64,
            current_time: NaiveDateTime,
            campaign_no: i32,
            campaign_start_at: NaiveDateTime,
            cum_weight_t: f64,
        }

        fn produce_weight_until(
            items: &[PlanItemLite],
            mut state: StreamState,
            rate_t_per_sec: f64,
            additional_weight_t: f64,
        ) -> Option<StreamState> {
            if additional_weight_t <= 0.0 {
                return Some(state);
            }
            if rate_t_per_sec <= 0.0 {
                return None;
            }

            let mut need = additional_weight_t;
            while need > 1e-9 {
                if state.item_index >= items.len() {
                    return None;
                }
                let item = &items[state.item_index];
                if state.current_time < item.earliest_start_at {
                    state.current_time = item.earliest_start_at;
                }

                let take = state.remaining_weight_t.min(need);
                let seconds_f = take / rate_t_per_sec;
                let seconds = seconds_f.round().max(0.0) as i64;
                state.current_time += chrono::Duration::seconds(seconds);
                state.remaining_weight_t -= take;
                need -= take;

                if state.remaining_weight_t <= 1e-9 {
                    state.item_index += 1;
                    if state.item_index < items.len() {
                        state.remaining_weight_t = items[state.item_index].weight_t;
                    } else {
                        state.remaining_weight_t = 0.0;
                    }
                }
            }
            Some(state)
        }

        fn simulate_to_as_of(
            items: &[PlanItemLite],
            rate_t_per_sec: f64,
            initial_start_at: NaiveDateTime,
            suggest_threshold_t: f64,
            downtime_minutes: i64,
            as_of: NaiveDateTime,
        ) -> StreamState {
            let mut state = StreamState {
                item_index: 0,
                remaining_weight_t: items.first().map(|i| i.weight_t).unwrap_or(0.0),
                current_time: items.first().map(|i| i.earliest_start_at).unwrap_or(as_of),
                campaign_no: 1,
                campaign_start_at: initial_start_at,
                cum_weight_t: 0.0,
            };

            // If as_of is before the schedule starts, clamp current_time to as_of and exit early.
            if state.current_time > as_of {
                state.current_time = as_of;
                return state;
            }

            let mut campaign_active = state.current_time >= initial_start_at;
            if !campaign_active && state.current_time < initial_start_at && as_of >= initial_start_at {
                // Campaign becomes active sometime before/as_of (possibly during idle); we will handle more precisely below.
            }

            while state.current_time < as_of {
                if state.item_index >= items.len() {
                    // No more production; idle until as_of.
                    state.current_time = as_of;
                    break;
                }

                let item = &items[state.item_index];
                let item_start = if state.current_time < item.earliest_start_at {
                    item.earliest_start_at
                } else {
                    state.current_time
                };

                // Idle gap before next item.
                if state.current_time < item_start {
                    if !campaign_active && initial_start_at <= item_start && initial_start_at <= as_of {
                        campaign_active = true;
                        state.campaign_start_at = initial_start_at;
                    }

                    if as_of < item_start {
                        state.current_time = as_of;
                        break;
                    }
                    state.current_time = item_start;
                }

                // If campaign starts during this item's processing, split at initial_start_at.
                if !campaign_active && state.current_time < initial_start_at && initial_start_at <= as_of {
                    // Produce until initial_start_at (does not count into cum_weight_t)
                    let seconds_until = (initial_start_at - state.current_time).num_seconds();
                    if seconds_until > 0 && rate_t_per_sec > 0.0 {
                        let producible = (seconds_until as f64) * rate_t_per_sec;
                        let produced = state.remaining_weight_t.min(producible);
                        let actual_seconds = (produced / rate_t_per_sec).round().max(0.0) as i64;
                        state.current_time += chrono::Duration::seconds(actual_seconds);
                        state.remaining_weight_t -= produced;
                        if state.remaining_weight_t <= 1e-9 {
                            state.item_index += 1;
                            if state.item_index < items.len() {
                                state.remaining_weight_t = items[state.item_index].weight_t;
                            } else {
                                state.remaining_weight_t = 0.0;
                            }
                        }
                    }

                    if state.current_time >= initial_start_at {
                        campaign_active = true;
                        state.campaign_start_at = initial_start_at;
                    }

                    continue;
                }

                // No production capacity
                if rate_t_per_sec <= 0.0 {
                    state.current_time = as_of;
                    break;
                }

                // Process the current item in small segments: (as_of boundary) and (soft-threshold boundary).
                let mut seg_start = state.current_time;
                while seg_start < as_of && state.remaining_weight_t > 1e-9 {
                    let seconds_to_finish_item =
                        (state.remaining_weight_t / rate_t_per_sec).round().max(0.0) as i64;
                    let finish_time = seg_start + chrono::Duration::seconds(seconds_to_finish_item);
                    let mut next_event_time = finish_time;

                    // Stop at as_of
                    if as_of < next_event_time {
                        next_event_time = as_of;
                    }

                    // Soft limit reach -> triggers roll change (auto), but only when campaign is active.
                    if campaign_active && suggest_threshold_t > 0.0 {
                        let remaining_to_soft = suggest_threshold_t - state.cum_weight_t;
                        if remaining_to_soft >= 0.0 && state.remaining_weight_t >= remaining_to_soft {
                            let sec_to_soft =
                                (remaining_to_soft / rate_t_per_sec).round().max(0.0) as i64;
                            let soft_time = seg_start + chrono::Duration::seconds(sec_to_soft);
                            if soft_time < next_event_time {
                                next_event_time = soft_time;
                            }
                        }
                    }

                    let delta_seconds = (next_event_time - seg_start).num_seconds().max(0);
                    let produced = (delta_seconds as f64) * rate_t_per_sec;
                    let produced = produced.min(state.remaining_weight_t).max(0.0);

                    if campaign_active {
                        state.cum_weight_t += produced;
                    }

                    state.remaining_weight_t -= produced;
                    state.current_time = next_event_time;
                    seg_start = next_event_time;

                    // Reached as_of
                    if state.current_time >= as_of {
                        break;
                    }

                    // Finished item
                    if (finish_time - state.current_time).num_seconds().abs() <= 1 {
                        state.item_index += 1;
                        if state.item_index < items.len() {
                            state.remaining_weight_t = items[state.item_index].weight_t;
                        } else {
                            state.remaining_weight_t = 0.0;
                        }
                        break;
                    }

                    // Soft threshold reached -> downtime + reset (if downtime fits before as_of)
                    if campaign_active && suggest_threshold_t > 0.0 {
                        let reached_soft = (state.cum_weight_t - suggest_threshold_t).abs() <= 1e-6
                            || state.cum_weight_t >= suggest_threshold_t;
                        if reached_soft {
                            let downtime_end = state.current_time + chrono::Duration::minutes(downtime_minutes);
                            if downtime_end > as_of {
                                // as_of within downtime: stop here, keep current campaign as-is.
                                state.current_time = as_of;
                                return state;
                            }

                            // Apply downtime and start next campaign
                            state.current_time = downtime_end;
                            state.campaign_no += 1;
                            state.cum_weight_t = 0.0;
                            state.campaign_start_at = state.current_time;
                            // Continue processing remaining weight (same item) after downtime
                            seg_start = state.current_time;
                            continue;
                        }
                    }
                }
            }

            state.current_time = as_of;
            state
        }

        let as_of = Local::now().naive_local();
        let mut inserted = 0usize;

        for (machine_code, hourly_capacity_t) in machines {
            // Query machine plan overrides
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

            // Plan items for this machine
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
            let pi_iter = pi_stmt.query_map(
                rusqlite::params![&scope.version_id, &machine_code],
                |row| {
                    let plan_date: String = row.get(0)?;
                    let weight_t: f64 = row.get(1)?;
                    let start_at = ymd_to_start_at(&plan_date).unwrap_or_else(|| as_of);
                    Ok(PlanItemLite {
                        earliest_start_at: start_at,
                        weight_t,
                    })
                },
            )?;

            let mut items: Vec<PlanItemLite> = pi_iter.collect::<Result<Vec<_>, _>>()?;
            items.retain(|i| i.weight_t > 0.0);
            items.sort_by_key(|i| i.earliest_start_at);

            // Default initial_start_at from earliest plan_date
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
            let planned_downtime_minutes = override_downtime_minutes.unwrap_or(default_downtime_minutes);

            let rate_t_per_sec = if hourly_capacity_t > 0.0 {
                hourly_capacity_t / 3600.0
            } else {
                0.0
            };

            // If no capacity (rate<=0), we cannot estimate timestamps. Fallback to a simple tonnage
            // aggregation (by plan_date) to keep D5 usable for legacy/test DBs.
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
                let cum_weight_t: f64 = items
                    .iter()
                    .filter(|i| i.earliest_start_at >= initial_start_at && i.earliest_start_at <= as_of)
                    .map(|i| i.weight_t)
                    .sum();
                StreamState {
                    item_index: 0,
                    remaining_weight_t: 0.0,
                    current_time: as_of,
                    campaign_no: 1,
                    campaign_start_at: initial_start_at,
                    cum_weight_t,
                }
            };

            let (soft_reach, hard_reach) = if rate_t_per_sec > 0.0 && !items.is_empty() {
                // Estimate reach times from as_of (no further auto resets)
                let soft_need = (suggest_threshold_t - state_at_as_of.cum_weight_t).max(0.0);
                let hard_need = (hard_limit_t - state_at_as_of.cum_weight_t).max(0.0);

                let base_state_for_future = StreamState {
                    item_index: state_at_as_of.item_index,
                    remaining_weight_t: state_at_as_of.remaining_weight_t,
                    current_time: state_at_as_of.current_time,
                    campaign_no: state_at_as_of.campaign_no,
                    campaign_start_at: state_at_as_of.campaign_start_at,
                    cum_weight_t: state_at_as_of.cum_weight_t,
                };

                let soft_reach = if suggest_threshold_t > 0.0 {
                    produce_weight_until(&items, base_state_for_future.clone(), rate_t_per_sec, soft_need)
                        .map(|s| s.current_time)
                } else {
                    None
                };
                let hard_reach = if hard_limit_t > 0.0 {
                    produce_weight_until(&items, base_state_for_future.clone(), rate_t_per_sec, hard_need)
                        .map(|s| s.current_time)
                } else {
                    None
                };
                (soft_reach, hard_reach)
            } else {
                (None, None)
            };

            let planned_change_at = override_next_change_at
                .filter(|dt| *dt > as_of && *dt >= state_at_as_of.campaign_start_at)
                .or(soft_reach);

            let will_exceed_soft_before_change = match (soft_reach, planned_change_at) {
                (Some(s), Some(p)) => s < p,
                _ => false,
            };

            let will_hard_stop_before_change = match (hard_reach, planned_change_at) {
                (Some(h), Some(p)) => h <= p,
                _ => false,
            };

            let utilization_rate = if suggest_threshold_t > 0.0 {
                state_at_as_of.cum_weight_t / suggest_threshold_t
            } else {
                0.0
            };

            let mut alert_level = "NONE".to_string();
            let mut reason = "换辊状态正常".to_string();

            if will_hard_stop_before_change {
                alert_level = "EMERGENCY".to_string();
                reason = "计划换辊时间晚于预计硬限制触达，存在硬停止风险".to_string();
            } else if state_at_as_of.cum_weight_t >= hard_limit_t {
                alert_level = "EMERGENCY".to_string();
                reason = format!(
                    "已超过硬限制 {:.1} 吨，必须立即换辊",
                    hard_limit_t
                );
            } else if will_exceed_soft_before_change || state_at_as_of.cum_weight_t >= suggest_threshold_t {
                alert_level = "CRITICAL".to_string();
                reason = format!("已达到/超过建议阈值 {:.1} 吨", suggest_threshold_t);
            } else if utilization_rate >= 0.95 {
                alert_level = "CRITICAL".to_string();
                reason = format!(
                    "接近建议阈值 ({:.1}%)，建议尽快安排换辊",
                    utilization_rate * 100.0
                );
            } else if utilization_rate >= 0.85 {
                alert_level = "WARNING".to_string();
                reason = format!(
                    "接近建议阈值 ({:.1}%)，请关注",
                    utilization_rate * 100.0
                );
            }

            let needs_immediate_change = matches!(alert_level.as_str(), "EMERGENCY")
                || utilization_rate >= 0.95
                || will_hard_stop_before_change;

            let suggested_actions = if will_hard_stop_before_change {
                r#"["调整计划换辊时间（避免硬停止）","考虑提前换辊或增加停机时长"]"#.to_string()
            } else if matches!(alert_level.as_str(), "EMERGENCY") {
                r#"["立即换辊"]"#.to_string()
            } else if matches!(alert_level.as_str(), "CRITICAL") {
                r#"["尽快安排换辊（优先在计划停机）"]"#.to_string()
            } else if matches!(alert_level.as_str(), "WARNING") {
                r#"["关注换辊时间与阈值触达"]"#.to_string()
            } else {
                "[]".to_string()
            };

            let estimated_change_date = hard_reach
                .map(|dt| dt.date().format("%Y-%m-%d").to_string());

            // Insert one row per machine (current campaign as_of).
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
                    alert_level,
                    reason,
                    suggest_threshold_t - state_at_as_of.cum_weight_t,
                    hard_limit_t - state_at_as_of.cum_weight_t,
                    utilization_rate,
                    estimated_change_date,
                    if needs_immediate_change { 1 } else { 0 },
                    suggested_actions,
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

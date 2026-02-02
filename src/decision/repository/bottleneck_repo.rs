// ==========================================
// 热轧精整排产系统 - D4 机组堵塞仓储
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md - D4 用例
// 职责: 查询机组堵塞概况数据
// ==========================================
// P2 阶段: 优先从 decision_machine_bottleneck 读模型表读取
//         如果读模型表为空，回退到 capacity_pool/plan_item 实时计算
// ==========================================

use crate::decision::common::sql_builder::SqlQueryBuilder;
use crate::decision::use_cases::d4_machine_bottleneck::{
    BottleneckHeatmap, MachineBottleneckProfile,
};
use rusqlite::{params, Connection};
use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::sync::{Arc, Mutex};

/// D4 机组堵塞仓储
///
/// 职责: 查询机组堵塞概况数据
/// 策略: 优先从 decision_machine_bottleneck 读模型表读取，回退到 capacity_pool/plan_item 实时计算
pub struct BottleneckRepository {
    conn: Arc<Mutex<Connection>>,
}

impl BottleneckRepository {
    /// 创建新的 BottleneckRepository 实例
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 查询机组堵塞概况
    ///
    /// 策略: 优先从 decision_machine_bottleneck 读模型表读取，如果为空则回退到实时计算
    ///
    /// # 参数
    /// - version_id: 方案版本 ID
    /// - machine_code: 机组代码（可选）
    /// - start_date: 开始日期
    /// - end_date: 结束日期
    ///
    /// # 返回
    /// - Ok(Vec<MachineBottleneckProfile>): 机组堵塞概况列表，按堵塞分数降序排列
    /// - Err: 数据库错误
    pub fn get_bottleneck_profile(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<MachineBottleneckProfile>, Box<dyn Error>> {
        // 优先尝试从读模型表读取
        if let Ok(profiles) = self.get_bottleneck_from_read_model(version_id, machine_code, start_date, end_date) {
            if !profiles.is_empty() {
                tracing::debug!(
                    version_id = version_id,
                    count = profiles.len(),
                    "D4: 从 decision_machine_bottleneck 读模型表读取"
                );
                return Ok(profiles);
            }
        }

        // 回退到实时计算
        tracing::debug!(
            version_id = version_id,
            "D4: 回退到 capacity_pool/plan_item 实时计算"
        );
        self.get_bottleneck_realtime(version_id, machine_code, start_date, end_date)
    }

    /// 从 decision_machine_bottleneck 读模型表读取（P2 优先路径）
    fn get_bottleneck_from_read_model(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<MachineBottleneckProfile>, Box<dyn Error>> {
        let conn = self.conn.lock().expect("锁获取失败");

        let machine_filter = machine_code.map(|_| "machine_code = ?");

        let sql = SqlQueryBuilder::new(
            r#"SELECT
                machine_code,
                plan_date,
                bottleneck_score,
                bottleneck_level,
                bottleneck_types,
                reasons,
                remaining_capacity_t,
                capacity_utilization,
                needs_roll_change,
                structure_violations,
                pending_materials,
                suggested_actions
            FROM decision_machine_bottleneck"#,
        )
        .where_clause("version_id = ?")
        .where_clause("plan_date >= ?")
        .where_clause("plan_date <= ?")
        .and_if(machine_filter)
        .order_by("bottleneck_score DESC")
        .build();

        let mut stmt = conn.prepare(&sql)?;

        // 构建参数
        let mut profiles = if let Some(mc) = machine_code {
            stmt.query_map(params![version_id, start_date, end_date, mc], |row| {
                Self::map_read_model_row(row, version_id)
            })?
            .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![version_id, start_date, end_date], |row| {
                Self::map_read_model_row(row, version_id)
            })?
            .collect::<Result<Vec<_>, _>>()?
        };

        // 补齐读模型缺失字段（scheduled/pending weight 等）
        // 说明：decision_machine_bottleneck 是 P2 读模型，但早期版本未落全字段。
        //       为保证前端展示口径一致，这里用 plan_item/material_state 进行轻量 enrich。
        Self::enrich_read_model_profiles(&conn, version_id, machine_code, start_date, end_date, &mut profiles)?;

        Ok(profiles)
    }

    /// 映射读模型表行到 MachineBottleneckProfile
    fn map_read_model_row(row: &rusqlite::Row, version_id: &str) -> rusqlite::Result<MachineBottleneckProfile> {
        let machine_code: String = row.get(0)?;
        let plan_date: String = row.get(1)?;
        let bottleneck_score: f64 = row.get(2)?;
        let bottleneck_level: String = row.get(3)?;
        let bottleneck_types: String = row.get(4)?;
        let reasons: String = row.get(5)?;
        let remaining_capacity_t: f64 = row.get(6)?;
        let capacity_utilization: f64 = row.get(7)?;
        let needs_roll_change: i32 = row.get(8)?;
        let structure_violations: i32 = row.get(9)?;
        let pending_materials: i32 = row.get(10)?;
        let suggested_actions: Option<String> = row.get(11)?;

        let mut profile = MachineBottleneckProfile::new(
            version_id.to_string(),
            machine_code,
            plan_date,
        );

        profile.bottleneck_score = bottleneck_score;
        profile.bottleneck_level = bottleneck_level;
        profile.remaining_capacity_t = remaining_capacity_t;
        profile.capacity_utilization = capacity_utilization;
        profile.needs_roll_change = needs_roll_change != 0;
        profile.structure_violations = structure_violations;
        profile.pending_materials = pending_materials;

        // 解析 bottleneck_types (String -> BottleneckType)
        if let Ok(type_strings) = serde_json::from_str::<Vec<String>>(&bottleneck_types) {
            use crate::decision::use_cases::d4_machine_bottleneck::BottleneckType;
            profile.bottleneck_types = type_strings
                .into_iter()
                .filter_map(|s| match s.as_str() {
                    "Capacity" => Some(BottleneckType::Capacity),
                    "Structure" => Some(BottleneckType::Structure),
                    "RollChange" => Some(BottleneckType::RollChange),
                    "ColdStock" => Some(BottleneckType::ColdStock),
                    "Mixed" => Some(BottleneckType::Mixed),
                    _ => None,
                })
                .collect();
        }

        // 解析 reasons
        // 兼容字段名：
        // - 早期 refresh 写入: {code, description, severity, affected_materials}
        // - 旧注释/草案:      {code, description, severity, impact_t}
        // - DTO 口径:        {code, msg, weight, affected_count}
        if let Ok(reason_list) = serde_json::from_str::<Vec<serde_json::Value>>(&reasons) {
            for reason in reason_list {
                let code = reason.get("code").and_then(|v| v.as_str());
                let description = reason
                    .get("description")
                    .or_else(|| reason.get("msg"))
                    .and_then(|v| v.as_str());
                let severity = reason
                    .get("severity")
                    .or_else(|| reason.get("weight"))
                    .and_then(|v| v.as_f64());
                let affected_materials = reason
                    .get("affected_materials")
                    .or_else(|| reason.get("affected_count"))
                    .or_else(|| reason.get("impact_t"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;

                if let (Some(code), Some(description), Some(severity)) = (code, description, severity) {
                    profile.reasons.push(crate::decision::use_cases::d4_machine_bottleneck::BottleneckReason {
                        code: code.to_string(),
                        description: description.to_string(),
                        severity,
                        affected_materials,
                    });
                }
            }
        }

        // 解析 suggested_actions
        if let Some(actions) = suggested_actions {
            if let Ok(action_list) = serde_json::from_str::<Vec<String>>(&actions) {
                for action in action_list {
                    profile.add_suggested_action(action);
                }
            }
        }

        Ok(profile)
    }

    /// 读模型补齐字段：scheduled/pending material & weight
    ///
    /// 背景：decision_machine_bottleneck 表中只有 pending_materials（且早期可能口径不准），
    ///       未存 scheduled_materials / weight_t / pending_weight_t。
    ///       这里用 plan_item/material_state 做补齐，避免前端看到“利用率很高但材料数全为 0 / 原因为空”。
    fn enrich_read_model_profiles(
        conn: &Connection,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: &str,
        end_date: &str,
        profiles: &mut [MachineBottleneckProfile],
    ) -> Result<(), Box<dyn Error>> {
        // 1) scheduled: 来自 plan_item (按机组×日期聚合)
        let mut scheduled_map: HashMap<(String, String), (i32, f64)> = HashMap::new();
        if let Some(mc) = machine_code {
            let mut stmt = conn.prepare(
                r#"
                SELECT
                    machine_code,
                    plan_date,
                    COUNT(*) AS material_count,
                    COALESCE(SUM(weight_t), 0) AS total_weight_t
                FROM plan_item
                WHERE version_id = ?1
                  AND plan_date BETWEEN ?2 AND ?3
                  AND machine_code = ?4
                GROUP BY machine_code, plan_date
                "#,
            )?;
            let rows = stmt.query_map(params![version_id, start_date, end_date, mc], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i32>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            })?;
            for row in rows {
                let (machine_code, plan_date, count, weight_t) = row?;
                scheduled_map.insert((machine_code, plan_date), (count, weight_t));
            }
        } else {
            let mut stmt = conn.prepare(
                r#"
                SELECT
                    machine_code,
                    plan_date,
                    COUNT(*) AS material_count,
                    COALESCE(SUM(weight_t), 0) AS total_weight_t
                FROM plan_item
                WHERE version_id = ?1
                  AND plan_date BETWEEN ?2 AND ?3
                GROUP BY machine_code, plan_date
                "#,
            )?;
            let rows = stmt.query_map(params![version_id, start_date, end_date], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i32>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            })?;
            for row in rows {
                let (machine_code, plan_date, count, weight_t) = row?;
                scheduled_map.insert((machine_code, plan_date), (count, weight_t));
            }
        }

        // 2) pending: 口径调整为“缺口（到当日仍未排入 <= 当日 的量）”
        //    缺口随日期变化：gap(D) = max(0, demand_ready_cum(<=D) - scheduled_cum(<=D))
        //    demand_ready_cum 按 effective_earliest_date 累计（FORCE_RELEASE 视为 start_date）。
        //    注：由于 capacity_pool 不是按 version_id 隔离，读模型也可能不全字段，这里在仓储层统一补齐。
        let machine_codes: Vec<String> = {
            let mut list: Vec<String> = profiles.iter().map(|p| p.machine_code.clone()).collect();
            list.sort();
            list.dedup();
            list
        };

        let mut profile_dates: HashMap<String, BTreeSet<String>> = HashMap::new();
        for p in profiles.iter() {
            profile_dates
                .entry(p.machine_code.clone())
                .or_default()
                .insert(p.plan_date.clone());
        }

        let scheduled_before_map =
            Self::query_scheduled_before_date(conn, version_id, &machine_codes, start_date)?;
        let demand_incr_map =
            Self::query_ready_demand_increments(conn, version_id, &machine_codes, start_date, end_date)?;
        let gap_map = Self::compute_gap_map(
            &machine_codes,
            &profile_dates,
            &scheduled_before_map,
            &scheduled_map,
            &demand_incr_map,
        );

        // 3) 写回 profiles
        for profile in profiles.iter_mut() {
            if let Some((count, weight_t)) = scheduled_map.get(&(
                profile.machine_code.clone(),
                profile.plan_date.clone(),
            )) {
                profile.scheduled_materials = *count;
                profile.scheduled_weight_t = *weight_t;
            } else {
                profile.scheduled_materials = 0;
                profile.scheduled_weight_t = 0.0;
            }

            if let Some((count, weight_t)) =
                gap_map.get(&(profile.machine_code.clone(), profile.plan_date.clone()))
            {
                profile.pending_materials = *count;
                profile.pending_weight_t = *weight_t;
            } else {
                profile.pending_materials = 0;
                profile.pending_weight_t = 0.0;
            }
        }

        Ok(())
    }

    fn query_scheduled_before_date(
        conn: &Connection,
        version_id: &str,
        machine_codes: &[String],
        start_date: &str,
    ) -> Result<HashMap<String, (i32, f64)>, Box<dyn Error>> {
        if machine_codes.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders = vec!["?"; machine_codes.len()].join(", ");
        let sql = format!(
            r#"
            SELECT
                machine_code,
                COUNT(*) AS material_count,
                COALESCE(SUM(weight_t), 0) AS total_weight_t
            FROM plan_item
            WHERE version_id = ?
              AND plan_date < ?
              AND machine_code IN ({})
            GROUP BY machine_code
            "#,
            placeholders
        );

        let mut params: Vec<String> = Vec::with_capacity(2 + machine_codes.len());
        params.push(version_id.to_string());
        params.push(start_date.to_string());
        params.extend(machine_codes.iter().cloned());
        let params_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();

        let mut map: HashMap<String, (i32, f64)> = HashMap::new();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_refs.iter()), |row| {
            Ok((
                row.get::<_, String>(0)?, // machine_code
                row.get::<_, i32>(1)?,    // material_count
                row.get::<_, f64>(2)?,    // total_weight_t
            ))
        })?;
        for row in rows {
            let (machine_code, count, weight_t) = row?;
            map.insert(machine_code, (count, weight_t));
        }

        Ok(map)
    }

    fn query_ready_demand_increments(
        conn: &Connection,
        version_id: &str,
        machine_codes: &[String],
        start_date: &str,
        end_date: &str,
    ) -> Result<HashMap<(String, String), (i32, f64)>, Box<dyn Error>> {
        if machine_codes.is_empty() {
            return Ok(HashMap::new());
        }

        let has_force_release_flag = Self::table_has_column(conn, "material_state", "force_release_flag")?;
        let has_earliest_sched_date = Self::table_has_column(conn, "material_state", "earliest_sched_date")?;

        let placeholders = vec!["?"; machine_codes.len()].join(", ");

        let force_release_flag_clause = if has_force_release_flag {
            "OR COALESCE(ms.force_release_flag, 0) != 0"
        } else {
            ""
        };

        let effective_earliest_expr = if has_earliest_sched_date {
            format!(
                r#"
                CASE
                    WHEN c.force_release_in_plan != 0
                      OR COALESCE(ms.sched_state, '') = 'FORCE_RELEASE'
                      {force_release_flag_clause}
                    THEN ?
                    WHEN ms.earliest_sched_date IS NULL
                      OR TRIM(ms.earliest_sched_date) = ''
                      OR ms.earliest_sched_date < ?
                    THEN ?
                    ELSE ms.earliest_sched_date
                END AS effective_earliest_date
                "#,
                force_release_flag_clause = force_release_flag_clause
            )
        } else {
            // 兼容旧 schema：缺少 earliest_sched_date 时，视为全部从 start_date 起可排
            format!(
                r#"
                CASE
                    WHEN c.force_release_in_plan != 0
                      OR COALESCE(ms.sched_state, '') = 'FORCE_RELEASE'
                      {force_release_flag_clause}
                    THEN ?
                    ELSE ?
                END AS effective_earliest_date
                "#,
                force_release_flag_clause = force_release_flag_clause
            )
        };

        let sql = format!(
            r#"
            WITH candidates_raw AS (
                -- 1) 版本内已排（无论 material_state 当前是什么状态，都应纳入“需求池”）
                SELECT
                    pi.material_id AS material_id,
                    pi.machine_code AS machine_code,
                    MAX(pi.force_release_in_plan) AS force_release_in_plan
                FROM plan_item pi
                WHERE pi.version_id = ?
                  AND pi.machine_code IN ({machines})
                GROUP BY pi.material_id, pi.machine_code

                UNION ALL

                -- 2) 当前未排且可排（READY/FORCE_RELEASE/LOCKED）
                SELECT
                    ms.material_id AS material_id,
                    mm.next_machine_code AS machine_code,
                    0 AS force_release_in_plan
                FROM material_state ms
                INNER JOIN material_master mm ON ms.material_id = mm.material_id
                WHERE ms.sched_state IN ('READY', 'FORCE_RELEASE', 'LOCKED')
                  AND mm.next_machine_code IN ({machines})
                  AND mm.next_machine_code IS NOT NULL
                  AND mm.next_machine_code != ''
            ),
            candidates AS (
                SELECT
                    material_id,
                    machine_code,
                    MAX(force_release_in_plan) AS force_release_in_plan
                FROM candidates_raw
                GROUP BY material_id, machine_code
            ),
            normalized AS (
                SELECT
                    c.machine_code AS machine_code,
                    c.material_id AS material_id,
                    COALESCE(mm.weight_t, 0) AS weight_t,
                    {effective_earliest_expr}
                FROM candidates c
                INNER JOIN material_master mm ON c.material_id = mm.material_id
                LEFT JOIN material_state ms ON c.material_id = ms.material_id
            )
            SELECT
                machine_code,
                effective_earliest_date,
                COUNT(*) AS material_count,
                COALESCE(SUM(weight_t), 0) AS total_weight_t
            FROM normalized
            WHERE machine_code IS NOT NULL
              AND machine_code != ''
              AND effective_earliest_date <= ?
            GROUP BY machine_code, effective_earliest_date
            "#,
            machines = placeholders
            ,
            effective_earliest_expr = effective_earliest_expr
        );

        // 参数顺序：
        // - plan_item: version_id, machine_codes...
        // - material_state: machine_codes...
        // - normalized:
        //   - 有 earliest_sched_date：start_date, start_date, start_date
        //   - 无 earliest_sched_date：start_date, start_date
        // - filter: end_date
        let normalized_params_len = if has_earliest_sched_date { 3 } else { 2 };
        let mut params: Vec<String> =
            Vec::with_capacity(1 + machine_codes.len() * 2 + normalized_params_len + 1);
        params.push(version_id.to_string());
        params.extend(machine_codes.iter().cloned());
        params.extend(machine_codes.iter().cloned());
        params.push(start_date.to_string());
        if has_earliest_sched_date {
            params.push(start_date.to_string());
            params.push(start_date.to_string());
        } else {
            params.push(start_date.to_string());
        }
        params.push(end_date.to_string());
        let params_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();

        let mut map: HashMap<(String, String), (i32, f64)> = HashMap::new();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_refs.iter()), |row| {
            Ok((
                row.get::<_, String>(0)?, // machine_code
                row.get::<_, String>(1)?, // effective_earliest_date
                row.get::<_, i32>(2)?,    // material_count
                row.get::<_, f64>(3)?,    // total_weight_t
            ))
        })?;
        for row in rows {
            let (machine_code, effective_date, count, weight_t) = row?;
            map.insert((machine_code, effective_date), (count, weight_t));
        }

        Ok(map)
    }

    fn table_has_column(
        conn: &Connection,
        table: &str,
        column: &str,
    ) -> Result<bool, Box<dyn Error>> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn compute_gap_map(
        machine_codes: &[String],
        profile_dates: &HashMap<String, BTreeSet<String>>,
        scheduled_before_map: &HashMap<String, (i32, f64)>,
        scheduled_daily_map: &HashMap<(String, String), (i32, f64)>,
        demand_incr_map: &HashMap<(String, String), (i32, f64)>,
    ) -> HashMap<(String, String), (i32, f64)> {
        if machine_codes.is_empty() {
            return HashMap::new();
        }

        // 组装每个机组需要参与前缀累计的日期集合：
        // - profiles 中出现的日期（用于最终回填）
        // - scheduled/demand 的发生日期（即使该日无 capacity_pool，也会影响后续累计）
        let mut dates_by_machine: HashMap<String, BTreeSet<String>> = profile_dates.clone();
        for ((machine_code, plan_date), _) in scheduled_daily_map.iter() {
            dates_by_machine
                .entry(machine_code.clone())
                .or_default()
                .insert(plan_date.clone());
        }
        for ((machine_code, effective_date), _) in demand_incr_map.iter() {
            dates_by_machine
                .entry(machine_code.clone())
                .or_default()
                .insert(effective_date.clone());
        }

        let mut gap_map: HashMap<(String, String), (i32, f64)> = HashMap::new();

        for machine_code in machine_codes {
            let (before_cnt, before_weight) = scheduled_before_map
                .get(machine_code)
                .copied()
                .unwrap_or((0, 0.0));

            let mut scheduled_cnt_cum: i64 = before_cnt as i64;
            let mut scheduled_weight_cum: f64 = before_weight;
            let mut demand_cnt_cum: i64 = 0;
            let mut demand_weight_cum: f64 = 0.0;

            let Some(dates) = dates_by_machine.get(machine_code) else {
                continue;
            };

            for date in dates {
                if let Some((cnt, w)) =
                    demand_incr_map.get(&(machine_code.clone(), date.clone()))
                {
                    demand_cnt_cum += *cnt as i64;
                    demand_weight_cum += *w;
                }
                if let Some((cnt, w)) =
                    scheduled_daily_map.get(&(machine_code.clone(), date.clone()))
                {
                    scheduled_cnt_cum += *cnt as i64;
                    scheduled_weight_cum += *w;
                }

                let gap_cnt = (demand_cnt_cum - scheduled_cnt_cum).max(0) as i32;
                let mut gap_weight = (demand_weight_cum - scheduled_weight_cum).max(0.0);
                if gap_weight.abs() < 1e-9 {
                    gap_weight = 0.0;
                }

                gap_map.insert((machine_code.clone(), date.clone()), (gap_cnt, gap_weight));
            }
        }

        gap_map
    }

    /// 从 capacity_pool/plan_item 表实时计算（P1 回退路径）
    fn get_bottleneck_realtime(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<MachineBottleneckProfile>, Box<dyn Error>> {
        let conn = self.conn.lock().expect("锁获取失败");

        // 按机组-日聚合数据
        let mut bottleneck_map: HashMap<(String, String), MachineBottleneckAggregateData> =
            HashMap::new();

        // 根据是否指定 machine_code 选择不同的查询路径
        if let Some(mc) = machine_code {
            self.query_capacity_for_machine(&conn, version_id, mc, start_date, end_date, &mut bottleneck_map)?;
        } else {
            self.query_capacity_for_all(&conn, version_id, start_date, end_date, &mut bottleneck_map)?;
        }

        // 查询 plan_item 表以获取已排材料数据
        self.enrich_with_plan_items(&conn, version_id, start_date, end_date, &mut bottleneck_map)?;

        // 查询 material_state 表以获取真实的待排材料数据
        self.enrich_with_pending_materials(&conn, version_id, start_date, end_date, &mut bottleneck_map)?;

        // 转换为 MachineBottleneckProfile 并排序
        let mut profiles: Vec<MachineBottleneckProfile> = bottleneck_map
            .into_values()
            .map(|data| data.into_profile(version_id.to_string()))
            .collect();

        // 按堵塞分数降序排序
        profiles.sort_by(|a, b| {
            b.bottleneck_score
                .partial_cmp(&a.bottleneck_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(profiles)
    }

    /// 查询指定机组的产能数据
    fn query_capacity_for_machine(
        &self,
        conn: &Connection,
        version_id: &str,
        machine_code: &str,
        start_date: &str,
        end_date: &str,
        bottleneck_map: &mut HashMap<(String, String), MachineBottleneckAggregateData>,
    ) -> Result<(), Box<dyn Error>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code,
                plan_date,
                target_capacity_t,
                limit_capacity_t,
                used_capacity_t,
                overflow_t,
                frozen_capacity_t,
                accumulated_tonnage_t,
                roll_campaign_id
            FROM capacity_pool
            WHERE version_id = ?1
              AND machine_code = ?2
              AND plan_date BETWEEN ?3 AND ?4
            ORDER BY plan_date ASC
            "#,
        )?;

        let rows = stmt.query_map(params![version_id, machine_code, start_date, end_date], |row| {
            Ok((
                row.get::<_, String>(0)?, // machine_code
                row.get::<_, String>(1)?, // plan_date
                row.get::<_, f64>(2)?,    // target_capacity_t
                row.get::<_, f64>(3)?,    // limit_capacity_t
                row.get::<_, f64>(4)?,    // used_capacity_t
                row.get::<_, f64>(5)?,    // overflow_t
                row.get::<_, f64>(6)?,    // frozen_capacity_t
                row.get::<_, f64>(7)?,    // accumulated_tonnage_t
                row.get::<_, Option<String>>(8)?, // roll_campaign_id
            ))
        })?;

        self.process_capacity_rows(rows, bottleneck_map)?;
        Ok(())
    }

    /// 查询所有机组的产能数据
    fn query_capacity_for_all(
        &self,
        conn: &Connection,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        bottleneck_map: &mut HashMap<(String, String), MachineBottleneckAggregateData>,
    ) -> Result<(), Box<dyn Error>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code,
                plan_date,
                target_capacity_t,
                limit_capacity_t,
                used_capacity_t,
                overflow_t,
                frozen_capacity_t,
                accumulated_tonnage_t,
                roll_campaign_id
            FROM capacity_pool
            WHERE version_id = ?1
              AND plan_date BETWEEN ?2 AND ?3
            ORDER BY machine_code ASC, plan_date ASC
            "#,
        )?;

        let rows = stmt.query_map(params![version_id, start_date, end_date], |row| {
            Ok((
                row.get::<_, String>(0)?, // machine_code
                row.get::<_, String>(1)?, // plan_date
                row.get::<_, f64>(2)?,    // target_capacity_t
                row.get::<_, f64>(3)?,    // limit_capacity_t
                row.get::<_, f64>(4)?,    // used_capacity_t
                row.get::<_, f64>(5)?,    // overflow_t
                row.get::<_, f64>(6)?,    // frozen_capacity_t
                row.get::<_, f64>(7)?,    // accumulated_tonnage_t
                row.get::<_, Option<String>>(8)?, // roll_campaign_id
            ))
        })?;

        self.process_capacity_rows(rows, bottleneck_map)?;
        Ok(())
    }

    /// 处理产能查询结果行
    fn process_capacity_rows(
        &self,
        rows: rusqlite::MappedRows<impl FnMut(&rusqlite::Row) -> rusqlite::Result<(String, String, f64, f64, f64, f64, f64, f64, Option<String>)>>,
        bottleneck_map: &mut HashMap<(String, String), MachineBottleneckAggregateData>,
    ) -> Result<(), Box<dyn Error>> {
        for row_result in rows {
            let (
                machine_code,
                plan_date,
                target_capacity_t,
                limit_capacity_t,
                used_capacity_t,
                overflow_t,
                frozen_capacity_t,
                accumulated_tonnage_t,
                roll_campaign_id,
            ) = row_result?;

            let key = (machine_code.clone(), plan_date.clone());
            let entry = bottleneck_map
                .entry(key)
                .or_insert_with(|| MachineBottleneckAggregateData::new(machine_code, plan_date));

            entry.set_capacity_data(
                target_capacity_t,
                limit_capacity_t,
                used_capacity_t,
                overflow_t,
                frozen_capacity_t,
            );

            entry.set_roll_campaign_data(accumulated_tonnage_t, roll_campaign_id);
        }
        Ok(())
    }

    /// 从 plan_item 表查询已排材料数据并填充到聚合数据中
    fn enrich_with_plan_items(
        &self,
        conn: &Connection,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        bottleneck_map: &mut HashMap<(String, String), MachineBottleneckAggregateData>,
    ) -> Result<(), Box<dyn Error>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code,
                plan_date,
                COUNT(*) as material_count,
                SUM(weight_t) as total_weight_t,
                SUM(CASE WHEN violation_flags IS NOT NULL AND violation_flags != '' THEN 1 ELSE 0 END) as violation_count
            FROM plan_item
            WHERE version_id = ?1
              AND plan_date BETWEEN ?2 AND ?3
            GROUP BY machine_code, plan_date
            "#,
        )?;

        let rows = stmt.query_map(params![version_id, start_date, end_date], |row| {
            Ok((
                row.get::<_, String>(0)?,  // machine_code
                row.get::<_, String>(1)?,  // plan_date
                row.get::<_, i32>(2)?,     // material_count (已排材料数)
                row.get::<_, f64>(3)?,     // total_weight_t (已排材料重量)
                row.get::<_, i32>(4)?,     // violation_count
            ))
        })?;

        for row_result in rows {
            let (machine_code, plan_date, scheduled_count, scheduled_weight, violation_count) =
                row_result?;

            let key = (machine_code, plan_date);
            if let Some(entry) = bottleneck_map.get_mut(&key) {
                // plan_item 数据代表已排材料，直接赋值给 scheduled 字段
                // pending 字段将从 material_state 表查询，不在此处赋值
                entry.set_plan_item_data(
                    0,                   // pending_materials（暂不赋值，由 enrich_with_pending_materials 填充）
                    0.0,                 // pending_weight_t（暂不赋值）
                    violation_count,
                    scheduled_count,     // scheduled_materials（已排材料数）
                    scheduled_weight,    // scheduled_weight_t（已排材料重量）
                );
            }
        }

        Ok(())
    }

    /// 计算“缺口（到当日仍未排入 <= 当日 的量）”并填充到聚合数据中
    ///
    /// # 说明
    /// - gap(D) = max(0, demand_ready_cum(<=D) - scheduled_cum(<=D))
    /// - demand_ready_cum：按 effective_earliest_date 累计（FORCE_RELEASE 视为 start_date）
    /// - scheduled_cum：来自 plan_item（按 version_id 累计）
    ///
    /// # 业务含义
    /// - 缺口是按机组×日期统计的，会随日期推进逐步收敛（若后续日期排入足够材料）
    fn enrich_with_pending_materials(
        &self,
        conn: &Connection,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        bottleneck_map: &mut HashMap<(String, String), MachineBottleneckAggregateData>,
    ) -> Result<(), Box<dyn Error>> {
        let machine_codes: Vec<String> = {
            let mut list: Vec<String> = bottleneck_map.keys().map(|(mc, _)| mc.clone()).collect();
            list.sort();
            list.dedup();
            list
        };
        if machine_codes.is_empty() {
            return Ok(());
        }

        let mut profile_dates: HashMap<String, BTreeSet<String>> = HashMap::new();
        for ((mc, plan_date), _) in bottleneck_map.iter() {
            profile_dates
                .entry(mc.clone())
                .or_default()
                .insert(plan_date.clone());
        }

        // scheduled: plan_item（按机组×日期聚合，用于 prefix sum）
        let scheduled_daily_map =
            Self::query_scheduled_by_machine_date(conn, version_id, &machine_codes, start_date, end_date)?;
        let scheduled_before_map =
            Self::query_scheduled_before_date(conn, version_id, &machine_codes, start_date)?;

        // demand increments: READY/FORCE_RELEASE/LOCKED + 本版本 plan_item 的 union
        let demand_incr_map =
            Self::query_ready_demand_increments(conn, version_id, &machine_codes, start_date, end_date)?;

        let gap_map = Self::compute_gap_map(
            &machine_codes,
            &profile_dates,
            &scheduled_before_map,
            &scheduled_daily_map,
            &demand_incr_map,
        );

        for ((machine_code, plan_date), entry) in bottleneck_map.iter_mut() {
            if let Some((gap_cnt, gap_weight)) =
                gap_map.get(&(machine_code.clone(), plan_date.clone()))
            {
                entry.pending_materials = *gap_cnt;
                entry.pending_weight_t = *gap_weight;
            } else {
                entry.pending_materials = 0;
                entry.pending_weight_t = 0.0;
            }
        }

        Ok(())
    }

    fn query_scheduled_by_machine_date(
        conn: &Connection,
        version_id: &str,
        machine_codes: &[String],
        start_date: &str,
        end_date: &str,
    ) -> Result<HashMap<(String, String), (i32, f64)>, Box<dyn Error>> {
        if machine_codes.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders = vec!["?"; machine_codes.len()].join(", ");
        let sql = format!(
            r#"
            SELECT
                machine_code,
                plan_date,
                COUNT(*) AS material_count,
                COALESCE(SUM(weight_t), 0) AS total_weight_t
            FROM plan_item
            WHERE version_id = ?
              AND plan_date BETWEEN ? AND ?
              AND machine_code IN ({})
            GROUP BY machine_code, plan_date
            "#,
            placeholders
        );

        let mut params: Vec<String> = Vec::with_capacity(3 + machine_codes.len());
        params.push(version_id.to_string());
        params.push(start_date.to_string());
        params.push(end_date.to_string());
        params.extend(machine_codes.iter().cloned());
        let params_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();

        let mut map: HashMap<(String, String), (i32, f64)> = HashMap::new();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_refs.iter()), |row| {
            Ok((
                row.get::<_, String>(0)?, // machine_code
                row.get::<_, String>(1)?, // plan_date
                row.get::<_, i32>(2)?,    // material_count
                row.get::<_, f64>(3)?,    // total_weight_t
            ))
        })?;
        for row in rows {
            let (machine_code, plan_date, count, weight_t) = row?;
            map.insert((machine_code, plan_date), (count, weight_t));
        }

        Ok(map)
    }

    /// 查询最堵塞的 N 个机组-日组合
    pub fn get_top_bottlenecks(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        top_n: usize,
    ) -> Result<Vec<MachineBottleneckProfile>, Box<dyn Error>> {
        let mut profiles = self.get_bottleneck_profile(version_id, None, start_date, end_date)?;
        profiles.truncate(top_n);
        Ok(profiles)
    }

    /// 获取机组堵塞热力图数据
    pub fn get_bottleneck_heatmap(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<BottleneckHeatmap, Box<dyn Error>> {
        let profiles = self.get_bottleneck_profile(version_id, None, start_date, end_date)?;

        let mut heatmap = BottleneckHeatmap::new(
            version_id.to_string(),
            start_date.to_string(),
            end_date.to_string(),
        );

        for profile in profiles {
            heatmap.add_cell(
                profile.machine_code,
                profile.plan_date,
                profile.bottleneck_score,
                profile.bottleneck_level,
            );
        }

        Ok(heatmap)
    }
}

/// 机组堵塞聚合数据（中间结构）
struct MachineBottleneckAggregateData {
    machine_code: String,
    plan_date: String,
    target_capacity_t: f64,
    limit_capacity_t: f64,
    used_capacity_t: f64,
    overflow_t: f64,
    frozen_capacity_t: f64,
    accumulated_tonnage_t: f64,
    roll_campaign_id: Option<String>,
    pending_materials: i32,
    pending_weight_t: f64,
    structure_violations: i32,
    scheduled_materials: i32,
    scheduled_weight_t: f64,
}

impl MachineBottleneckAggregateData {
    fn new(machine_code: String, plan_date: String) -> Self {
        Self {
            machine_code,
            plan_date,
            target_capacity_t: 0.0,
            limit_capacity_t: 0.0,
            used_capacity_t: 0.0,
            overflow_t: 0.0,
            frozen_capacity_t: 0.0,
            accumulated_tonnage_t: 0.0,
            roll_campaign_id: None,
            pending_materials: 0,
            pending_weight_t: 0.0,
            structure_violations: 0,
            scheduled_materials: 0,
            scheduled_weight_t: 0.0,
        }
    }

    fn set_capacity_data(
        &mut self,
        target_capacity_t: f64,
        limit_capacity_t: f64,
        used_capacity_t: f64,
        overflow_t: f64,
        frozen_capacity_t: f64,
    ) {
        self.target_capacity_t = target_capacity_t;
        self.limit_capacity_t = limit_capacity_t;
        self.used_capacity_t = used_capacity_t;
        self.overflow_t = overflow_t;
        self.frozen_capacity_t = frozen_capacity_t;
    }

    fn set_roll_campaign_data(
        &mut self,
        accumulated_tonnage_t: f64,
        roll_campaign_id: Option<String>,
    ) {
        self.accumulated_tonnage_t = accumulated_tonnage_t;
        self.roll_campaign_id = roll_campaign_id;
    }

    fn set_plan_item_data(
        &mut self,
        pending_materials: i32,
        pending_weight_t: f64,
        structure_violations: i32,
        scheduled_materials: i32,
        scheduled_weight_t: f64,
    ) {
        self.pending_materials = pending_materials;
        self.pending_weight_t = pending_weight_t;
        self.structure_violations = structure_violations;
        self.scheduled_materials = scheduled_materials;
        self.scheduled_weight_t = scheduled_weight_t;
    }

    fn into_profile(self, version_id: String) -> MachineBottleneckProfile {
        let mut profile =
            MachineBottleneckProfile::new(version_id, self.machine_code, self.plan_date);

        // 计算产能利用率
        let capacity_utilization = if self.limit_capacity_t > 0.0 {
            self.used_capacity_t / self.limit_capacity_t
        } else {
            0.0
        };

        // 计算剩余产能
        let remaining_capacity_t = self.limit_capacity_t - self.used_capacity_t;

        // 设置产能信息
        profile.set_capacity_info(remaining_capacity_t, capacity_utilization);

        // 设置结构信息
        profile.set_structure_info(self.structure_violations);

        // 设置待排材料数量和重量
        profile.pending_materials = self.pending_materials;
        profile.pending_weight_t = self.pending_weight_t;

        // 设置已排材料数量和重量
        profile.scheduled_materials = self.scheduled_materials;
        profile.scheduled_weight_t = self.scheduled_weight_t;

        // 添加堵塞原因
        // 数据一致性校验：如果显示超限但无已排材料，添加警告原因
        if self.overflow_t > 0.0 && self.scheduled_materials == 0 {
            profile.add_reason(
                "DATA_INCONSISTENCY_WARNING".to_string(),
                format!(
                    "数据不一致：容量池显示超限 {:.1}t，但无已排材料。请检查容量配置或排程状态",
                    self.overflow_t
                ),
                0.3,
                0,
            );
        }

        if self.overflow_t > 0.0 {
            profile.add_reason(
                "CAPACITY_OVERFLOW".to_string(),
                format!(
                    "产能池超限 {:.1}t，利用率 {:.1}%",
                    self.overflow_t,
                    capacity_utilization * 100.0
                ),
                0.9,
                0,
            );
        }

        if capacity_utilization >= 0.95 && capacity_utilization < 1.0 {
            profile.add_reason(
                "HIGH_UTILIZATION".to_string(),
                format!("产能利用率高 {:.1}%", capacity_utilization * 100.0),
                0.7,
                0,
            );
        }

        if self.structure_violations > 0 {
            profile.add_reason(
                "STRUCTURE_CONFLICT".to_string(),
                format!(
                    "结构矛盾导致 {} 个材料无法排入",
                    self.structure_violations
                ),
                0.8,
                self.structure_violations,
            );
        }

        if self.pending_materials > 20 {
            profile.add_reason(
                "HIGH_PENDING_COUNT".to_string(),
                format!("缺口材料数量较多 {} 个（到当日仍未排入≤当日）", self.pending_materials),
                0.5,
                self.pending_materials,
            );
        }

        if remaining_capacity_t < 100.0 && self.pending_weight_t > 0.0 {
            profile.add_reason(
                "LOW_REMAINING_CAPACITY".to_string(),
                format!(
                    "剩余产能不足 {:.1}t，缺口 {:.1}t（到当日仍未排入≤当日）",
                    remaining_capacity_t, self.pending_weight_t
                ),
                0.6,
                0,
            );
        }

        // 添加建议措施
        if profile.is_severe() {
            if self.overflow_t > 0.0 {
                profile.add_suggested_action("调整产能池上限".to_string());
            }
            if self.structure_violations > 0 {
                profile.add_suggested_action("优先处理结构冲突材料".to_string());
            }
            if self.pending_materials > 20 {
                profile.add_suggested_action("将部分材料转移至其他机组或延后至后续日期".to_string());
            }
        }

        profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();

        // 创建 capacity_pool 表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS capacity_pool (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                target_capacity_t REAL NOT NULL,
                limit_capacity_t REAL NOT NULL,
                used_capacity_t REAL NOT NULL DEFAULT 0.0,
                overflow_t REAL NOT NULL DEFAULT 0.0,
                frozen_capacity_t REAL NOT NULL DEFAULT 0.0,
                accumulated_tonnage_t REAL NOT NULL DEFAULT 0.0,
                roll_campaign_id TEXT,
                PRIMARY KEY (version_id, machine_code, plan_date)
            )
            "#,
            [],
        )
        .unwrap();

        // 创建 plan_item 表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS plan_item (
                version_id TEXT NOT NULL,
                material_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                seq_no INTEGER NOT NULL,
                weight_t REAL NOT NULL,
                source_type TEXT NOT NULL,
                locked_in_plan INTEGER NOT NULL DEFAULT 0,
                force_release_in_plan INTEGER NOT NULL DEFAULT 0,
                violation_flags TEXT,
                PRIMARY KEY (version_id, material_id)
            )
            "#,
            [],
        )
        .unwrap();

        // 创建 material_master 表（用于待排材料查询）
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS material_master (
                material_id TEXT PRIMARY KEY,
                manufacturing_order_id TEXT,
                contract_no TEXT,
                due_date TEXT,
                next_machine_code TEXT,
                rework_machine_code TEXT,
                current_machine_code TEXT,
                width_mm REAL,
                thickness_mm REAL,
                length_m REAL,
                weight_t REAL,
                available_width_mm REAL,
                steel_mark TEXT,
                slab_id TEXT,
                material_status_code_src TEXT,
                status_updated_at TEXT,
                output_age_days_raw INTEGER,
                stock_age_days INTEGER,
                contract_nature TEXT,
                weekly_delivery_flag TEXT,
                export_flag TEXT,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )
            "#,
            [],
        )
        .unwrap();

        // 创建 material_state 表（用于待排材料查询）
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS material_state (
                material_id TEXT PRIMARY KEY,
                sched_state TEXT NOT NULL DEFAULT 'READY',
                lock_flag INTEGER NOT NULL DEFAULT 0,
                force_release_flag INTEGER NOT NULL DEFAULT 0,
                urgent_level TEXT NOT NULL DEFAULT 'L0',
                urgent_reason TEXT,
                rush_level TEXT DEFAULT 'L0',
                rolling_output_age_days INTEGER DEFAULT 0,
                ready_in_days INTEGER DEFAULT 0,
                earliest_sched_date TEXT,
                stock_age_days INTEGER DEFAULT 0,
                scheduled_date TEXT,
                scheduled_machine_code TEXT,
                seq_no INTEGER,
                manual_urgent_flag INTEGER NOT NULL DEFAULT 0,
                in_frozen_zone INTEGER NOT NULL DEFAULT 0,
                last_calc_version_id TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_by TEXT
            )
            "#,
            [],
        )
        .unwrap();

        Arc::new(Mutex::new(conn))
    }

    fn insert_test_capacity_data(conn: &Connection) {
        // H032: 高利用率
        conn.execute(
            r#"
            INSERT INTO capacity_pool (
                version_id, machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                "V001",
                "H032",
                "2026-01-24",
                1500.0,
                2000.0,
                1950.0,
                0.0,
                100.0,
                15000.0,
                "RC001"
            ],
        )
        .unwrap();

        // H033: 产能超载
        conn.execute(
            r#"
            INSERT INTO capacity_pool (
                version_id, machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                "V001",
                "H033",
                "2026-01-24",
                1500.0,
                2000.0,
                2300.0,
                300.0,
                150.0,
                18000.0,
                "RC002"
            ],
        )
        .unwrap();
    }

    fn insert_test_plan_items(conn: &Connection) {
        // H032: 10 个材料，其中 2 个有结构违规
        for i in 1..=10 {
            let violation_flags = if i <= 2 { "STRUCT_CONFLICT" } else { "" };
            conn.execute(
                r#"
                INSERT INTO plan_item (
                    version_id, material_id, machine_code, plan_date, seq_no, weight_t,
                    source_type, locked_in_plan, force_release_in_plan, violation_flags
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    "V001",
                    format!("MAT{:03}", i),
                    "H032",
                    "2026-01-24",
                    i,
                    150.0,
                    "AUTO",
                    0,
                    0,
                    violation_flags
                ],
            )
            .unwrap();
        }

        // H033: 25 个材料，其中 5 个有结构违规
        for i in 11..=35 {
            let violation_flags = if i <= 15 { "STRUCT_CONFLICT" } else { "" };
            conn.execute(
                r#"
                INSERT INTO plan_item (
                    version_id, material_id, machine_code, plan_date, seq_no, weight_t,
                    source_type, locked_in_plan, force_release_in_plan, violation_flags
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    "V001",
                    format!("MAT{:03}", i),
                    "H033",
                    "2026-01-24",
                    i - 10,
                    100.0,
                    "AUTO",
                    0,
                    0,
                    violation_flags
                ],
            )
            .unwrap();
        }
    }

    fn insert_test_material_master(conn: &Connection) {
        // H032: 插入 10 个已排材料（对应 plan_item）
        for i in 1..=10 {
            conn.execute(
                r#"
                INSERT INTO material_master (
                    material_id, current_machine_code, next_machine_code, weight_t,
                    manufacturing_order_id, contract_no, due_date,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "H031",
                    "H032",
                    150.0,
                    format!("MO{:03}", i),
                    format!("C{:03}", i),
                    "2026-02-01",
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:00:00Z"
                ],
            )
            .unwrap();
        }

        // H033: 插入 25 个已排材料（对应 plan_item）
        for i in 11..=35 {
            conn.execute(
                r#"
                INSERT INTO material_master (
                    material_id, current_machine_code, next_machine_code, weight_t,
                    manufacturing_order_id, contract_no, due_date,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "H031",
                    "H033",
                    100.0,
                    format!("MO{:03}", i),
                    format!("C{:03}", i),
                    "2026-02-01",
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:00:00Z"
                ],
            )
            .unwrap();
        }

        // H032: 插入 3 个待排材料（READY 状态的 material_master 记录）
        for i in 36..=38 {
            conn.execute(
                r#"
                INSERT INTO material_master (
                    material_id, current_machine_code, next_machine_code, weight_t,
                    manufacturing_order_id, contract_no, due_date,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "H031",
                    "H032",
                    120.0,
                    format!("MO{:03}", i),
                    format!("C{:03}", i),
                    "2026-02-05",
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:00:00Z"
                ],
            )
            .unwrap();
        }

        // H033: 插入 5 个待排材料（READY 状态的 material_master 记录）
        for i in 39..=43 {
            conn.execute(
                r#"
                INSERT INTO material_master (
                    material_id, current_machine_code, next_machine_code, weight_t,
                    manufacturing_order_id, contract_no, due_date,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "H031",
                    "H033",
                    80.0,
                    format!("MO{:03}", i),
                    format!("C{:03}", i),
                    "2026-02-05",
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:00:00Z"
                ],
            )
            .unwrap();
        }
    }

    fn insert_test_material_state(conn: &Connection) {
        // H032: 3 个待排材料（READY 状态）
        for i in 36..=38 {
            conn.execute(
                r#"
                INSERT INTO material_state (
                    material_id, sched_state, lock_flag, force_release_flag,
                    urgent_level, updated_at, updated_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "READY",
                    0,
                    0,
                    "L0",
                    "2026-01-01T00:00:00Z",
                    "SYSTEM"
                ],
            )
            .unwrap();
        }

        // H033: 5 个待排材料（READY 状态）
        for i in 39..=43 {
            conn.execute(
                r#"
                INSERT INTO material_state (
                    material_id, sched_state, lock_flag, force_release_flag,
                    urgent_level, updated_at, updated_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "READY",
                    0,
                    0,
                    "L0",
                    "2026-01-01T00:00:00Z",
                    "SYSTEM"
                ],
            )
            .unwrap();
        }

        // 已排材料（SCHEDULED 状态）
        for i in 1..=35 {
            let (machine_code, plan_date) = if i <= 10 {
                ("H032", "2026-01-24")
            } else {
                ("H033", "2026-01-24")
            };
            conn.execute(
                r#"
                INSERT INTO material_state (
                    material_id, sched_state, lock_flag, force_release_flag,
                    urgent_level, scheduled_machine_code, scheduled_date,
                    updated_at, updated_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "SCHEDULED",
                    0,
                    0,
                    "L0",
                    machine_code,
                    plan_date,
                    "2026-01-01T00:00:00Z",
                    "SYSTEM"
                ],
            )
            .unwrap();
        }
    }

    #[test]
    fn test_get_bottleneck_profile() {
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().expect("锁获取失败");
            insert_test_capacity_data(&conn);
            insert_test_plan_items(&conn);
            insert_test_material_master(&conn);
            insert_test_material_state(&conn);
        }

        let repo = BottleneckRepository::new(conn_arc);
        let profiles = repo
            .get_bottleneck_profile("V001", None, "2026-01-24", "2026-01-24")
            .unwrap();

        assert_eq!(profiles.len(), 2);

        // 第一个应该是 H033（产能超载，堵塞分数最高）
        let h033 = &profiles[0];
        assert_eq!(h033.machine_code, "H033");
        assert!(h033.bottleneck_score > 0.0);
        assert!(h033.is_severe());
        // pending_materials 口径：缺口（到当日仍未排入≤当日）
        assert_eq!(h033.pending_materials, 5);
        assert_eq!(h033.structure_violations, 5);
        // scheduled_materials 来自 plan_item
        assert_eq!(h033.scheduled_materials, 25);

        // 第二个应该是 H032（高利用率）
        let h032 = &profiles[1];
        assert_eq!(h032.machine_code, "H032");
        assert!(h032.bottleneck_score > 0.0);
        // pending_materials 口径：缺口（到当日仍未排入≤当日）
        assert_eq!(h032.pending_materials, 3);
        assert_eq!(h032.structure_violations, 2);
        // scheduled_materials 来自 plan_item
        assert_eq!(h032.scheduled_materials, 10);
    }

    #[test]
    fn test_get_top_bottlenecks() {
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().expect("锁获取失败");
            insert_test_capacity_data(&conn);
            insert_test_plan_items(&conn);
            insert_test_material_master(&conn);
            insert_test_material_state(&conn);
        }

        let repo = BottleneckRepository::new(conn_arc);
        let profiles = repo
            .get_top_bottlenecks("V001", "2026-01-24", "2026-01-24", 1)
            .unwrap();

        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].machine_code, "H033");
    }

    #[test]
    fn test_get_bottleneck_heatmap() {
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().expect("锁获取失败");
            insert_test_capacity_data(&conn);
            insert_test_plan_items(&conn);
            insert_test_material_master(&conn);
            insert_test_material_state(&conn);
        }

        let repo = BottleneckRepository::new(conn_arc);
        let heatmap = repo
            .get_bottleneck_heatmap("V001", "2026-01-24", "2026-01-24")
            .unwrap();

        assert_eq!(heatmap.machines.len(), 2);
        assert_eq!(heatmap.data.len(), 2);
        assert!(heatmap.max_score > 0.0);
        assert!(heatmap.avg_score > 0.0);

        // 验证可以获取特定机组-日的分数
        let h033_score = heatmap.get_score("H033", "2026-01-24");
        assert!(h033_score.is_some());
        assert!(h033_score.unwrap() > 0.0);
    }

    #[test]
    fn test_filter_by_machine_code() {
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().expect("锁获取失败");
            insert_test_capacity_data(&conn);
            insert_test_plan_items(&conn);
            insert_test_material_master(&conn);
            insert_test_material_state(&conn);
        }

        let repo = BottleneckRepository::new(conn_arc);
        let profiles = repo
            .get_bottleneck_profile("V001", Some("H032"), "2026-01-24", "2026-01-24")
            .unwrap();

        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].machine_code, "H032");
        assert_eq!(profiles[0].pending_materials, 3);  // H032 有 3 个待排材料
        assert_eq!(profiles[0].scheduled_materials, 10);  // H032 有 10 个已排材料
    }

    #[test]
    fn test_pending_materials_gap_by_date() {
        // 新测试：验证缺口（到当日仍未排入≤当日）按日期随排产累计收敛
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().expect("锁获取失败");
            // capacity_pool: H032 两天
            conn.execute(
                r#"
                INSERT INTO capacity_pool (
                    version_id, machine_code, plan_date, target_capacity_t, limit_capacity_t,
                    used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
                ) VALUES
                    ('V001', 'H032', '2026-01-24', 1500.0, 2000.0, 1500.0, 0.0, 0.0, 0.0, NULL),
                    ('V001', 'H032', '2026-01-25', 1500.0, 2000.0, 1500.0, 0.0, 0.0, 0.0, NULL)
                "#,
                [],
            )
            .unwrap();

            // material_master: 4 件需求（其中 1 件未排）
            for i in 1..=4 {
                conn.execute(
                    r#"
                    INSERT INTO material_master (
                        material_id, current_machine_code, next_machine_code, weight_t,
                        manufacturing_order_id, contract_no, due_date,
                        created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    params![
                        format!("MAT{:03}", i),
                        "H031",
                        "H032",
                        100.0,
                        format!("MO{:03}", i),
                        format!("C{:03}", i),
                        "2026-02-01",
                        "2026-01-01T00:00:00Z",
                        "2026-01-01T00:00:00Z"
                    ],
                )
                .unwrap();
            }

            // material_state: 3 件 2026-01-24 起可排，1 件 2026-01-25 起可排
            for i in 1..=4 {
                let earliest = if i == 3 { "2026-01-25" } else { "2026-01-24" };
                let sched_state = if i == 1 { "SCHEDULED" } else { "READY" };
                conn.execute(
                    r#"
                    INSERT INTO material_state (
                        material_id, sched_state, lock_flag, force_release_flag,
                        urgent_level, earliest_sched_date,
                        updated_at, updated_by
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    params![
                        format!("MAT{:03}", i),
                        sched_state,
                        0,
                        0,
                        "L0",
                        earliest,
                        "2026-01-01T00:00:00Z",
                        "SYSTEM"
                    ],
                )
                .unwrap();
            }

            // plan_item: 2026-01-24 排 1 件；2026-01-25 排 2 件（第 4 件不排）
            conn.execute(
                r#"
                INSERT INTO plan_item (
                    version_id, material_id, machine_code, plan_date, seq_no, weight_t,
                    source_type, locked_in_plan, force_release_in_plan, violation_flags
                ) VALUES
                    ('V001', 'MAT001', 'H032', '2026-01-24', 1, 100.0, 'AUTO', 0, 0, ''),
                    ('V001', 'MAT002', 'H032', '2026-01-25', 1, 100.0, 'AUTO', 0, 0, ''),
                    ('V001', 'MAT003', 'H032', '2026-01-25', 2, 100.0, 'AUTO', 0, 0, '')
                "#,
                [],
            )
            .unwrap();
        }

        let repo = BottleneckRepository::new(conn_arc);
        let profiles = repo
            .get_bottleneck_profile("V001", Some("H032"), "2026-01-24", "2026-01-25")
            .unwrap();

        assert_eq!(profiles.len(), 2);

        let mut by_date: HashMap<String, MachineBottleneckProfile> = HashMap::new();
        for p in profiles {
            by_date.insert(p.plan_date.clone(), p);
        }

        let p_24 = by_date.get("2026-01-24").expect("missing 2026-01-24");
        // 2026-01-24：需求 3（MAT001/MAT002/MAT004），已排 1（MAT001）→ 缺口 2
        assert_eq!(p_24.pending_materials, 2);
        assert_eq!(p_24.pending_weight_t, 200.0);
        assert_eq!(p_24.scheduled_materials, 1);
        assert_eq!(p_24.scheduled_weight_t, 100.0);

        let p_25 = by_date.get("2026-01-25").expect("missing 2026-01-25");
        // 2026-01-25：需求 4（+MAT003），已排累计 3（+MAT002/MAT003）→ 缺口 1（MAT004）
        assert_eq!(p_25.pending_materials, 1);
        assert_eq!(p_25.pending_weight_t, 100.0);
        assert_eq!(p_25.scheduled_materials, 2);
        assert_eq!(p_25.scheduled_weight_t, 200.0);
    }

    #[test]
    fn test_data_inconsistency_warning() {
        // 新测试：验证数据一致性校验（超限但无已排材料）
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().expect("锁获取失败");
            // 插入产能超限的 capacity_pool 数据
            conn.execute(
                r#"
                INSERT INTO capacity_pool (
                    version_id, machine_code, plan_date, target_capacity_t, limit_capacity_t,
                    used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    "V001",
                    "H034",
                    "2026-01-25",
                    1500.0,
                    2000.0,
                    2300.0,
                    300.0,  // 超限
                    0.0,
                    0.0,
                    "RC003"
                ],
            )
            .unwrap();
            // 故意不插入 plan_item（没有已排材料）
        }

        let repo = BottleneckRepository::new(conn_arc);
        let profiles = repo
            .get_bottleneck_profile("V001", Some("H034"), "2026-01-25", "2026-01-25")
            .unwrap();

        assert_eq!(profiles.len(), 1);
        let h034 = &profiles[0];

        // 应该包含数据不一致警告原因
        let warning_reason = h034.reasons.iter()
            .find(|r| r.code == "DATA_INCONSISTENCY_WARNING");
        assert!(warning_reason.is_some(), "应该包含数据不一致警告");
        assert_eq!(h034.scheduled_materials, 0);  // 没有已排材料
    }

    #[test]
    fn test_read_model_reason_parsing_and_enrich_fields() {
        // 回归测试：读模型 reasons 字段使用 affected_materials 时应能正常解析；
        // 同时补齐 scheduled/pending 的数量与重量，避免前端出现“利用率很高但材料数为 0 / 原因为空”。
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().expect("锁获取失败");

            // 创建读模型表（最小字段集）
            conn.execute(
                r#"
                CREATE TABLE IF NOT EXISTS decision_machine_bottleneck (
                    version_id TEXT NOT NULL,
                    machine_code TEXT NOT NULL,
                    plan_date TEXT NOT NULL,
                    bottleneck_score REAL NOT NULL,
                    bottleneck_level TEXT NOT NULL,
                    bottleneck_types TEXT NOT NULL,
                    reasons TEXT NOT NULL,
                    remaining_capacity_t REAL NOT NULL,
                    capacity_utilization REAL NOT NULL,
                    needs_roll_change INTEGER NOT NULL DEFAULT 0,
                    structure_violations INTEGER NOT NULL DEFAULT 0,
                    pending_materials INTEGER NOT NULL DEFAULT 0,
                    suggested_actions TEXT
                )
                "#,
                [],
            )
            .unwrap();

            conn.execute(
                r#"
                INSERT INTO decision_machine_bottleneck (
                    version_id, machine_code, plan_date,
                    bottleneck_score, bottleneck_level,
                    bottleneck_types, reasons,
                    remaining_capacity_t, capacity_utilization,
                    needs_roll_change, structure_violations, pending_materials, suggested_actions
                ) VALUES (
                    'V001', 'H034', '2026-01-31',
                    95.0, 'CRITICAL',
                    '["Capacity"]',
                    '[{"code":"CAPACITY_UTILIZATION","description":"产能利用率: 114.7%","severity":1.147,"affected_materials":0}]',
                    -100.0, 1.147,
                    0, 0, 0, '[]'
                )
                "#,
                [],
            )
            .unwrap();

            // 插入 plan_item（已排 2 件，共 200t）
            for i in 1..=2 {
                // 对齐真实 schema：plan_item.material_id 通常引用 material_master.material_id
                conn.execute(
                    r#"
                    INSERT INTO material_master (
                        material_id, current_machine_code, next_machine_code, weight_t,
                        manufacturing_order_id, contract_no, due_date,
                        created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    params![
                        format!("PM{:03}", i),
                        "H031",
                        "H034",
                        100.0,
                        format!("MO_PM{:03}", i),
                        format!("C_PM{:03}", i),
                        "2026-02-01",
                        "2026-01-01T00:00:00Z",
                        "2026-01-01T00:00:00Z"
                    ],
                )
                .unwrap();

                conn.execute(
                    r#"
                    INSERT INTO plan_item (
                        version_id, material_id, machine_code, plan_date, seq_no, weight_t,
                        source_type, locked_in_plan, force_release_in_plan, violation_flags
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    params![
                        "V001",
                        format!("PM{:03}", i),
                        "H034",
                        "2026-01-31",
                        i,
                        100.0,
                        "AUTO",
                        0,
                        0,
                        ""
                    ],
                )
                .unwrap();
            }

            // 插入待排材料（READY 1 件，共 50t，next_machine_code = H034）
            conn.execute(
                r#"
                INSERT INTO material_master (
                    material_id, current_machine_code, next_machine_code, weight_t,
                    manufacturing_order_id, contract_no, due_date,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    "PENDING_001",
                    "H031",
                    "H034",
                    50.0,
                    "MO_P001",
                    "C_P001",
                    "2026-02-01",
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:00:00Z"
                ],
            )
            .unwrap();

            conn.execute(
                r#"
                INSERT INTO material_state (
                    material_id, sched_state, lock_flag, force_release_flag,
                    urgent_level, updated_at, updated_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    "PENDING_001",
                    "READY",
                    0,
                    0,
                    "L0",
                    "2026-01-01T00:00:00Z",
                    "SYSTEM"
                ],
            )
            .unwrap();
        }

        let repo = BottleneckRepository::new(conn_arc);
        let profiles = repo
            .get_bottleneck_profile("V001", Some("H034"), "2026-01-31", "2026-01-31")
            .unwrap();

        assert_eq!(profiles.len(), 1);
        let p = &profiles[0];
        assert_eq!(p.machine_code, "H034");
        assert_eq!(p.plan_date, "2026-01-31");
        assert_eq!(p.bottleneck_level, "CRITICAL");
        assert_eq!(p.scheduled_materials, 2);
        assert_eq!(p.scheduled_weight_t, 200.0);
        assert_eq!(p.pending_materials, 1);
        assert_eq!(p.pending_weight_t, 50.0);
        assert!(
            p.reasons.iter().any(|r| r.code == "CAPACITY_UTILIZATION"),
            "应能解析读模型 reasons 字段"
        );
    }
}

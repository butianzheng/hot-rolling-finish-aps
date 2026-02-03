use super::core::BottleneckRepository;
use crate::decision::common::sql_builder::SqlQueryBuilder;
use crate::decision::use_cases::d4_machine_bottleneck::MachineBottleneckProfile;
use rusqlite::{params, Connection};
use std::collections::{BTreeSet, HashMap};
use std::error::Error;

impl BottleneckRepository {
    /// 从 decision_machine_bottleneck 读模型表读取（P2 优先路径）
    pub(super) fn get_bottleneck_from_read_model(
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

}

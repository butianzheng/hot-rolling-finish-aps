use super::core::BottleneckRepository;
use rusqlite::Connection;
use std::collections::{BTreeSet, HashMap};
use std::error::Error;

impl BottleneckRepository {
    pub(super) fn query_scheduled_before_date(
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

    pub(super) fn query_ready_demand_increments(
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

    pub(super) fn compute_gap_map(
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

    pub(super) fn query_scheduled_by_machine_date(
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
}

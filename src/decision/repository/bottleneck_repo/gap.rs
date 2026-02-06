use super::core::BottleneckRepository;
use rusqlite::Connection;
use std::collections::{BTreeSet, HashMap};
use std::error::Error;

impl BottleneckRepository {
    pub(super) fn query_unscheduled_ready_increments(
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

        let force_release_flag_expr = if has_force_release_flag {
            "c.force_release_flag"
        } else {
            "0"
        };

        let earliest_sched_field = if has_earliest_sched_date {
            "c.earliest_sched_date"
        } else {
            "NULL"
        };

        let effective_earliest_expr = if has_earliest_sched_date {
            format!(
                r#"
                CASE
                    WHEN COALESCE(c.sched_state, '') = 'FORCE_RELEASE'
                      OR COALESCE({force_release_flag_expr}, 0) != 0
                    THEN ?
                    WHEN {earliest_field} IS NULL
                      OR TRIM({earliest_field}) = ''
                      OR {earliest_field} < ?
                    THEN ?
                    ELSE {earliest_field}
                END AS effective_earliest_date
                "#,
                force_release_flag_expr = force_release_flag_expr,
                earliest_field = earliest_sched_field
            )
        } else {
            // 兼容旧 schema：缺少 earliest_sched_date 时，视为全部从 start_date 起可排
            format!(
                r#"
                CASE
                    WHEN COALESCE(c.sched_state, '') = 'FORCE_RELEASE'
                      OR COALESCE({force_release_flag_expr}, 0) != 0
                    THEN ?
                    ELSE ?
                END AS effective_earliest_date
                "#,
                force_release_flag_expr = force_release_flag_expr
            )
        };

        let earliest_select = if has_earliest_sched_date {
            "ms.earliest_sched_date AS earliest_sched_date"
        } else {
            "NULL AS earliest_sched_date"
        };

        let force_release_select = if has_force_release_flag {
            "ms.force_release_flag AS force_release_flag"
        } else {
            "0 AS force_release_flag"
        };

        let sql = format!(
            r#"
            WITH candidates AS (
                SELECT
                    ms.material_id AS material_id,
                    mm.next_machine_code AS machine_code,
                    COALESCE(mm.weight_t, 0) AS weight_t,
                    ms.sched_state AS sched_state,
                    {force_release_select},
                    {earliest_select}
                FROM material_state ms
                INNER JOIN material_master mm ON ms.material_id = mm.material_id
                LEFT JOIN plan_item pi
                  ON pi.version_id = ?
                 AND pi.material_id = ms.material_id
                WHERE ms.sched_state IN ('READY', 'FORCE_RELEASE', 'LOCKED')
                  AND mm.next_machine_code IN ({machines})
                  AND mm.next_machine_code IS NOT NULL
                  AND mm.next_machine_code != ''
                  AND pi.material_id IS NULL
            ),
            normalized AS (
                SELECT
                    c.machine_code,
                    c.weight_t,
                    {effective_earliest_expr}
                FROM candidates c
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
            machines = placeholders,
            effective_earliest_expr = effective_earliest_expr,
            earliest_select = earliest_select,
            force_release_select = force_release_select
        );

        // 参数顺序：
        // - plan_item join: version_id
        // - material_state: machine_codes...
        // - normalized:
        //   - 有 earliest_sched_date：start_date, start_date, start_date
        //   - 无 earliest_sched_date：start_date, start_date
        // - filter: end_date
        let normalized_params_len = if has_earliest_sched_date { 3 } else { 2 };
        let mut params: Vec<String> =
            Vec::with_capacity(1 + machine_codes.len() + normalized_params_len + 1);
        params.push(version_id.to_string());
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

    pub(super) fn compute_pending_map(
        machine_codes: &[String],
        profile_dates: &HashMap<String, BTreeSet<String>>,
        pending_incr_map: &HashMap<(String, String), (i32, f64)>,
    ) -> HashMap<(String, String), (i32, f64)> {
        if machine_codes.is_empty() {
            return HashMap::new();
        }

        // 组装每个机组需要参与前缀累计的日期集合：
        // - profiles 中出现的日期（用于最终回填）
        // - pending 的发生日期（即使该日无 capacity_pool，也会影响后续累计）
        let mut dates_by_machine: HashMap<String, BTreeSet<String>> = profile_dates.clone();
        for ((machine_code, effective_date), _) in pending_incr_map.iter() {
            dates_by_machine
                .entry(machine_code.clone())
                .or_default()
                .insert(effective_date.clone());
        }

        let mut pending_map: HashMap<(String, String), (i32, f64)> = HashMap::new();

        for machine_code in machine_codes {
            let mut pending_cnt_cum: i64 = 0;
            let mut pending_weight_cum: f64 = 0.0;

            let Some(dates) = dates_by_machine.get(machine_code) else {
                continue;
            };

            for date in dates {
                if let Some((cnt, w)) =
                    pending_incr_map.get(&(machine_code.clone(), date.clone()))
                {
                    pending_cnt_cum += *cnt as i64;
                    pending_weight_cum += *w;
                }

                let mut pending_weight = pending_weight_cum;
                if pending_weight.abs() < 1e-9 {
                    pending_weight = 0.0;
                }

                pending_map.insert(
                    (machine_code.clone(), date.clone()),
                    (pending_cnt_cum.max(0) as i32, pending_weight),
                );
            }
        }

        pending_map
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

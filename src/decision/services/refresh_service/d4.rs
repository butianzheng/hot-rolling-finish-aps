use super::*;

impl DecisionRefreshService {

    /// 刷新 D4: 哪个机组最堵
    pub(super) fn refresh_d4(
        &self,
        tx: &Transaction,
        scope: &RefreshScope,
    ) -> Result<usize, Box<dyn Error>> {
        // 构建删除条件和参数
        let mut delete_conditions = vec!["version_id = ?1".to_string()];
        let mut insert_conditions = vec![];
        let mut params = vec![scope.version_id.clone()];

        if !scope.is_full_refresh {
            // 增量刷新：根据受影响的机组和日期范围
            if let Some(machines) = &scope.affected_machines {
                if !machines.is_empty() {
                    let placeholders: Vec<String> = (0..machines.len())
                        .map(|i| format!("?{}", params.len() + i + 1))
                        .collect();
                    delete_conditions.push(format!("machine_code IN ({})", placeholders.join(", ")));
                    insert_conditions.push(format!("cp.machine_code IN ({})", placeholders.join(", ")));
                    params.extend(machines.clone());
                }
            }

            if let Some((start_date, end_date)) = &scope.affected_date_range {
                let start_idx = params.len() + 1;
                let end_idx = params.len() + 2;
                delete_conditions.push(format!("plan_date BETWEEN ?{} AND ?{}", start_idx, end_idx));
                insert_conditions.push(format!("cp.plan_date BETWEEN ?{} AND ?{}", start_idx, end_idx));
                params.push(start_date.clone());
                params.push(end_date.clone());
            }
        }

        // 删除旧数据
        let delete_sql = format!(
            "DELETE FROM decision_machine_bottleneck WHERE {}",
            delete_conditions.join(" AND ")
        );
        let params_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
        tx.execute(&delete_sql, rusqlite::params_from_iter(params_refs.iter()))?;

        // 从 capacity_pool 和 plan_item 计算机组堵塞概况
        let has_cp_version_id: i32 = tx.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('capacity_pool') WHERE name = 'version_id'",
            [],
            |row| row.get(0),
        )?;

        let mut capacity_where_conditions: Vec<String> = Vec::new();
        if has_cp_version_id > 0 {
            capacity_where_conditions.push("cp.version_id = ?1".to_string());
        }
        capacity_where_conditions.extend(insert_conditions);

        let capacity_where_clause = if capacity_where_conditions.is_empty() {
            String::new()
        } else {
            format!("\n            WHERE {}", capacity_where_conditions.join(" AND "))
        };

        // 节奏（品种大类）偏差：需要 plan_rhythm_target 表；若不存在则走旧逻辑（仅产能）
        let has_rhythm_table: i32 = tx.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='plan_rhythm_target'",
            [],
            |row| row.get(0),
        )?;
        let has_product_category_col: i32 = tx.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('material_master') WHERE name = 'product_category'",
            [],
            |row| row.get(0),
        )?;

        let insert_sql = if has_rhythm_table == 0 {
            // 旧逻辑：仅产能口径（bottleneck_types 使用 [] 避免前端枚举漂移）
            format!(
                r#"
                INSERT OR REPLACE INTO decision_machine_bottleneck (
                    version_id,
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
                    suggested_actions,
                    refreshed_at
                )
                SELECT
                    ?1 AS version_id,
                    cp.machine_code,
                    cp.plan_date,
                    CASE
                        WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) >= 1.0 THEN 95.0
                        WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) >= 0.9 THEN 75.0
                        WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) >= 0.8 THEN 50.0
                        ELSE 25.0
                    END AS bottleneck_score,
                    CASE
                        WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) >= 1.0 THEN 'CRITICAL'
                        WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) >= 0.9 THEN 'HIGH'
                        WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) >= 0.8 THEN 'MEDIUM'
                        WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) >= 0.5 THEN 'LOW'
                        ELSE 'NONE'
                    END AS bottleneck_level,
                    CASE
                        WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) >= 0.9 THEN '["Capacity"]'
                        ELSE '[]'
                    END AS bottleneck_types,
                    json_array(
                        json_object(
                            'code', 'CAPACITY_UTILIZATION',
                            'description', '产能利用率: ' || CAST(ROUND((cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0)) * 100, 1) AS TEXT) || '%',
                            'severity', CAST(cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) AS REAL),
                            'affected_materials', 0
                        )
                    ) AS reasons,
                    cp.target_capacity_t - cp.used_capacity_t AS remaining_capacity_t,
                    cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) AS capacity_utilization,
                    0 AS needs_roll_change,
                    COALESCE(pi.violation_count, 0) AS structure_violations,
                    0 AS pending_materials,
                    '[]' AS suggested_actions,
                    datetime('now') AS refreshed_at
                FROM capacity_pool cp
                LEFT JOIN (
                    SELECT
                        machine_code,
                        plan_date,
                        SUM(CASE WHEN violation_flags IS NOT NULL AND violation_flags != '' THEN 1 ELSE 0 END) AS violation_count
                    FROM plan_item
                    WHERE version_id = ?1
                    GROUP BY machine_code, plan_date
                ) pi ON cp.machine_code = pi.machine_code AND cp.plan_date = pi.plan_date
                {}
                "#,
                capacity_where_clause
            )
        } else {
            // 新逻辑：增加“每日生产节奏（品种大类）”偏差的结构性堵塞信号
            let category_expr = if has_product_category_col > 0 {
                "COALESCE(mm.product_category, '未分类')"
            } else {
                "COALESCE(mm.steel_mark, '未分类')"
            };

            format!(
                r#"
                WITH cfg AS (
                    SELECT COALESCE(
                        NULLIF(
                            CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'rhythm_deviation_threshold') AS REAL),
                            0.0
                        ),
                        NULLIF(
                            CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'deviation_threshold') AS REAL),
                            0.0
                        ),
                        0.1
                    ) AS dev_th
                ),
                target AS (
                    SELECT
                        machine_code,
                        plan_date,
                        target_json
                    FROM plan_rhythm_target
                    WHERE version_id = ?1
                      AND dimension = 'PRODUCT_CATEGORY'
                      AND target_json IS NOT NULL
                      AND TRIM(target_json) != ''
                      AND TRIM(target_json) != '{{}}'
                ),
                actual_total AS (
                    SELECT
                        pi.machine_code,
                        pi.plan_date,
                        COALESCE(SUM(pi.weight_t), 0) AS total_weight_t
                    FROM plan_item pi
                    WHERE pi.version_id = ?1
                    GROUP BY pi.machine_code, pi.plan_date
                ),
                actual AS (
                    SELECT
                        pi.machine_code,
                        pi.plan_date,
                        {category_expr} AS category,
                        COALESCE(SUM(pi.weight_t), 0) / NULLIF(at.total_weight_t, 0) AS actual_ratio
                    FROM plan_item pi
                    JOIN material_master mm ON mm.material_id = pi.material_id
                    JOIN actual_total at ON at.machine_code = pi.machine_code AND at.plan_date = pi.plan_date
                    WHERE pi.version_id = ?1
                      AND at.total_weight_t > 0
                    GROUP BY pi.machine_code, pi.plan_date, category
                ),
                target_kv AS (
                    SELECT
                        t.machine_code,
                        t.plan_date,
                        je.key AS category,
                        CAST(je.value AS REAL) AS target_ratio
                    FROM target t, json_each(t.target_json) je
                ),
                diff_target_keys AS (
                    SELECT
                        tk.machine_code,
                        tk.plan_date,
                        ABS(COALESCE(a.actual_ratio, 0) - tk.target_ratio) AS diff
                    FROM target_kv tk
                    LEFT JOIN actual a
                      ON a.machine_code = tk.machine_code
                     AND a.plan_date = tk.plan_date
                     AND a.category = tk.category
                ),
                diff_actual_only AS (
                    SELECT
                        a.machine_code,
                        a.plan_date,
                        CASE WHEN tk.category IS NULL THEN a.actual_ratio ELSE 0 END AS diff
                    FROM actual a
                    LEFT JOIN target_kv tk
                      ON tk.machine_code = a.machine_code
                     AND tk.plan_date = a.plan_date
                     AND tk.category = a.category
                ),
                maxdiff AS (
                    SELECT
                        machine_code,
                        plan_date,
                        MAX(diff) AS max_deviation
                    FROM (
                        SELECT * FROM diff_target_keys
                        UNION ALL
                        SELECT * FROM diff_actual_only
                    )
                    GROUP BY machine_code, plan_date
                )
                INSERT OR REPLACE INTO decision_machine_bottleneck (
                    version_id,
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
                    suggested_actions,
                    refreshed_at
                )
                SELECT
                    ?1 AS version_id,
                    base.machine_code,
                    base.plan_date,
                    max(base.cap_score, base.struct_score) AS bottleneck_score,
                    CASE
                        WHEN max(base.cap_score, base.struct_score) >= 90 THEN 'CRITICAL'
                        WHEN max(base.cap_score, base.struct_score) >= 75 THEN 'HIGH'
                        WHEN max(base.cap_score, base.struct_score) >= 50 THEN 'MEDIUM'
                        WHEN max(base.cap_score, base.struct_score) >= 30 THEN 'LOW'
                        ELSE 'NONE'
                    END AS bottleneck_level,
                    CASE
                        WHEN base.cap_flag = 1 AND base.struct_flag = 1 THEN '["Capacity","Structure"]'
                        WHEN base.cap_flag = 1 THEN '["Capacity"]'
                        WHEN base.struct_flag = 1 THEN '["Structure"]'
                        ELSE '[]'
                    END AS bottleneck_types,
                    CASE
                        WHEN base.struct_flag = 1 THEN json_array(json(base.capacity_reason), json(base.structure_reason))
                        ELSE json_array(json(base.capacity_reason))
                    END AS reasons,
                    base.remaining_capacity_t,
                    base.capacity_utilization,
                    0 AS needs_roll_change,
                    base.structure_violations,
                    0 AS pending_materials,
                    '[]' AS suggested_actions,
                    datetime('now') AS refreshed_at
                FROM (
                    SELECT
                        cp.machine_code,
                        cp.plan_date,
                        cp.target_capacity_t - cp.used_capacity_t AS remaining_capacity_t,
                        cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) AS capacity_utilization,
                        CASE
                            WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) >= 1.0 THEN 95.0
                            WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) >= 0.9 THEN 75.0
                            WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) >= 0.8 THEN 50.0
                            ELSE 25.0
                        END AS cap_score,
                        CASE WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) >= 0.9 THEN 1 ELSE 0 END AS cap_flag,
                        COALESCE(md.max_deviation, 0.0) AS rhythm_max_deviation,
                        cfg.dev_th AS dev_th,
                        CASE
                            WHEN COALESCE(md.max_deviation, 0.0) >= cfg.dev_th * 3 THEN 90.0
                            WHEN COALESCE(md.max_deviation, 0.0) >= cfg.dev_th * 2 THEN 75.0
                            WHEN COALESCE(md.max_deviation, 0.0) >= cfg.dev_th THEN 55.0
                            ELSE 0.0
                        END AS struct_score,
                        CASE WHEN COALESCE(md.max_deviation, 0.0) >= cfg.dev_th THEN 1 ELSE 0 END AS struct_flag,
                        COALESCE(pi.violation_count, 0) + CASE WHEN COALESCE(md.max_deviation, 0.0) >= cfg.dev_th THEN 1 ELSE 0 END AS structure_violations,
                        json_object(
                            'code', 'CAPACITY_UTILIZATION',
                            'description', '产能利用率: ' || CAST(ROUND((cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0)) * 100, 1) AS TEXT) || '%',
                            'severity', CAST(cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) AS REAL),
                            'affected_materials', 0
                        ) AS capacity_reason,
                        json_object(
                            'code', 'RHYTHM_DEVIATION',
                            'description', '节奏最大偏差: ' || CAST(ROUND(COALESCE(md.max_deviation, 0.0) * 100.0, 1) AS TEXT) || '%（阈值 ' || CAST(ROUND(cfg.dev_th * 100.0, 1) AS TEXT) || '%）',
                            'severity', CAST(COALESCE(md.max_deviation, 0.0) AS REAL),
                            'affected_materials', 0
                        ) AS structure_reason
                    FROM capacity_pool cp
                    CROSS JOIN cfg
                    LEFT JOIN (
                        SELECT
                            machine_code,
                            plan_date,
                            SUM(CASE WHEN violation_flags IS NOT NULL AND violation_flags != '' THEN 1 ELSE 0 END) AS violation_count
                        FROM plan_item
                        WHERE version_id = ?1
                        GROUP BY machine_code, plan_date
                    ) pi ON cp.machine_code = pi.machine_code AND cp.plan_date = pi.plan_date
                    LEFT JOIN maxdiff md ON cp.machine_code = md.machine_code AND cp.plan_date = md.plan_date
                    {capacity_where_clause}
                ) base
                "#,
                category_expr = category_expr,
                capacity_where_clause = capacity_where_clause
            )
        };

        let params_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
        let rows_affected = tx.execute(&insert_sql, rusqlite::params_from_iter(params_refs.iter()))?;

        Ok(rows_affected)
    }

}

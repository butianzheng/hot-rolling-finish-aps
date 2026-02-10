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
                    delete_conditions
                        .push(format!("machine_code IN ({})", placeholders.join(", ")));
                    insert_conditions
                        .push(format!("cp.machine_code IN ({})", placeholders.join(", ")));
                    params.extend(machines.clone());
                }
            }

            if let Some((start_date, end_date)) = &scope.affected_date_range {
                let start_idx = params.len() + 1;
                let end_idx = params.len() + 2;
                delete_conditions
                    .push(format!("plan_date BETWEEN ?{} AND ?{}", start_idx, end_idx));
                insert_conditions.push(format!(
                    "cp.plan_date BETWEEN ?{} AND ?{}",
                    start_idx, end_idx
                ));
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
            format!(
                "\n            WHERE {}",
                capacity_where_conditions.join(" AND ")
            )
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
            // 仅产能 + 结构违规口径（评分/等级参数可配置）
            format!(
                r#"
                WITH cfg AS (
                    SELECT
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_capacity_hard_threshold') AS REAL), 0.0), 0.95) AS cap_hard,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_capacity_full_threshold') AS REAL), 0.0), 1.0) AS cap_full,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_structure_violation_full_count') AS REAL), 0.0), 10.0) AS violation_full_cnt,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_bottleneck_low_threshold') AS REAL), 0.0), 0.3) AS low_th,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_bottleneck_medium_threshold') AS REAL), 0.0), 0.6) AS med_th,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_bottleneck_high_threshold') AS REAL), 0.0), 0.9) AS high_th,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_bottleneck_critical_threshold') AS REAL), 0.0), 0.95) AS critical_th
                ),
                base AS (
                    SELECT
                        cp.machine_code,
                        cp.plan_date,
                        cp.limit_capacity_t - cp.used_capacity_t AS remaining_capacity_t,
                        cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) AS capacity_utilization,
                        COALESCE(pi.violation_count, 0) AS violation_count,
                        cfg.low_th,
                        cfg.med_th,
                        cfg.high_th,
                        cfg.critical_th,
                        CASE
                            WHEN cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) IS NULL THEN 0.0
                            WHEN cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) <= cfg.cap_hard THEN 0.0
                            WHEN cfg.cap_full <= cfg.cap_hard THEN 1.0
                            ELSE MIN(1.0, (cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) - cfg.cap_hard) / (cfg.cap_full - cfg.cap_hard))
                        END AS cap_sev,
                        CASE
                            WHEN cfg.violation_full_cnt <= 0 THEN 0.0
                            ELSE MIN(1.0, COALESCE(pi.violation_count, 0) / cfg.violation_full_cnt)
                        END AS violation_sev,
                        json_object(
                            'code', 'CAPACITY_UTILIZATION',
                            'description', '产能利用率: ' || CAST(ROUND((cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0)) * 100, 1) AS TEXT) || '%（硬阈值 ' || CAST(ROUND(cfg.cap_hard * 100.0, 1) AS TEXT) || '%）',
                            'severity', CAST(
                                CASE
                                    WHEN cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) IS NULL THEN 0.0
                                    WHEN cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) <= cfg.cap_hard THEN 0.0
                                    WHEN cfg.cap_full <= cfg.cap_hard THEN 1.0
                                    ELSE MIN(1.0, (cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) - cfg.cap_hard) / (cfg.cap_full - cfg.cap_hard))
                                END AS REAL
                            ),
                            'affected_materials', 0
                        ) AS capacity_reason,
                        json_object(
                            'code', 'STRUCTURE_VIOLATION',
                            'description', '结构违规: ' || CAST(COALESCE(pi.violation_count, 0) AS TEXT) || ' 个',
                            'severity', CAST(
                                CASE
                                    WHEN cfg.violation_full_cnt <= 0 THEN 0.0
                                    ELSE MIN(1.0, COALESCE(pi.violation_count, 0) / cfg.violation_full_cnt)
                                END AS REAL
                            ),
                            'affected_materials', COALESCE(pi.violation_count, 0)
                        ) AS violation_reason
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
                    {}
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
                    MAX(base.cap_sev, base.violation_sev) * 100.0 AS bottleneck_score,
                    CASE
                        WHEN MAX(base.cap_sev, base.violation_sev) >= base.critical_th THEN 'CRITICAL'
                        WHEN MAX(base.cap_sev, base.violation_sev) >= base.high_th THEN 'HIGH'
                        WHEN MAX(base.cap_sev, base.violation_sev) >= base.med_th THEN 'MEDIUM'
                        WHEN MAX(base.cap_sev, base.violation_sev) >= base.low_th THEN 'LOW'
                        ELSE 'NONE'
                    END AS bottleneck_level,
                    CASE
                        WHEN base.cap_sev > 0 AND base.violation_sev > 0 THEN '["Capacity","Structure"]'
                        WHEN base.cap_sev > 0 THEN '["Capacity"]'
                        WHEN base.violation_sev > 0 THEN '["Structure"]'
                        ELSE '[]'
                    END AS bottleneck_types,
                    CASE
                        WHEN base.violation_sev > 0 THEN json_array(json(base.capacity_reason), json(base.violation_reason))
                        ELSE json_array(json(base.capacity_reason))
                    END AS reasons,
                    base.remaining_capacity_t,
                    base.capacity_utilization,
                    0 AS needs_roll_change,
                    base.violation_count AS structure_violations,
                    0 AS pending_materials,
                    '[]' AS suggested_actions,
                    datetime('now') AS refreshed_at
                FROM base
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
                    SELECT
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_capacity_hard_threshold') AS REAL), 0.0), 0.95) AS cap_hard,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_capacity_full_threshold') AS REAL), 0.0), 1.0) AS cap_full,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_structure_dev_threshold') AS REAL), 0.0), 0.1) AS dev_th,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_structure_dev_full_multiplier') AS REAL), 0.0), 2.0) AS dev_full_mul,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_structure_small_category_threshold') AS REAL), 0.0), 0.05) AS small_cat_th,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_structure_violation_full_count') AS REAL), 0.0), 10.0) AS violation_full_cnt,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_bottleneck_low_threshold') AS REAL), 0.0), 0.3) AS low_th,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_bottleneck_medium_threshold') AS REAL), 0.0), 0.6) AS med_th,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_bottleneck_high_threshold') AS REAL), 0.0), 0.9) AS high_th,
                        COALESCE(NULLIF(CAST((SELECT value FROM config_kv WHERE scope_id = 'global' AND key = 'd4_bottleneck_critical_threshold') AS REAL), 0.0), 0.95) AS critical_th
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
                target_dates AS (
                    SELECT DISTINCT machine_code, plan_date FROM target
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
                        tk.category,
                        tk.target_ratio,
                        COALESCE(a.actual_ratio, 0) AS actual_ratio,
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
                        a.category,
                        0 AS target_ratio,
                        a.actual_ratio AS actual_ratio,
                        a.actual_ratio AS diff
                    FROM actual a
                    LEFT JOIN target_kv tk
                      ON tk.machine_code = a.machine_code
                     AND tk.plan_date = a.plan_date
                     AND tk.category = a.category
                    WHERE tk.category IS NULL
                ),
                diff_all AS (
                    SELECT * FROM diff_target_keys
                    UNION ALL
                    SELECT * FROM diff_actual_only
                ),
                diff_filtered AS (
                    SELECT
                        d.machine_code,
                        d.plan_date,
                        d.diff
                    FROM diff_all d
                    CROSS JOIN cfg
                    WHERE cfg.small_cat_th <= 0
                       OR (CASE WHEN d.target_ratio >= d.actual_ratio THEN d.target_ratio ELSE d.actual_ratio END) >= cfg.small_cat_th
                ),
                sumdiff AS (
                    SELECT
                        machine_code,
                        plan_date,
                        COALESCE(SUM(diff), 0) AS sum_diff
                    FROM diff_filtered
                    GROUP BY machine_code, plan_date
                ),
                weighted_dev AS (
                    SELECT
                        td.machine_code,
                        td.plan_date,
                        CASE
                            WHEN COALESCE(at.total_weight_t, 0) <= 0 THEN 0.0
                            ELSE COALESCE(sd.sum_diff, 0) / 2.0
                        END AS weighted_dev
                    FROM target_dates td
                    LEFT JOIN sumdiff sd
                      ON sd.machine_code = td.machine_code
                     AND sd.plan_date = td.plan_date
                    LEFT JOIN actual_total at
                      ON at.machine_code = td.machine_code
                     AND at.plan_date = td.plan_date
                ),
                base AS (
                    SELECT
                        cp.machine_code,
                        cp.plan_date,
                        cp.limit_capacity_t - cp.used_capacity_t AS remaining_capacity_t,
                        cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) AS capacity_utilization,
                        COALESCE(pi.violation_count, 0) AS violation_count,
                        COALESCE(wd.weighted_dev, 0.0) AS weighted_dev,
                        cfg.low_th,
                        cfg.med_th,
                        cfg.high_th,
                        cfg.critical_th,
                        CASE
                            WHEN cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) IS NULL THEN 0.0
                            WHEN cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) <= cfg.cap_hard THEN 0.0
                            WHEN cfg.cap_full <= cfg.cap_hard THEN 1.0
                            ELSE MIN(1.0, (cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) - cfg.cap_hard) / (cfg.cap_full - cfg.cap_hard))
                        END AS cap_sev,
                        CASE
                            WHEN cfg.dev_th <= 0 THEN MIN(1.0, COALESCE(wd.weighted_dev, 0.0))
                            WHEN COALESCE(wd.weighted_dev, 0.0) <= cfg.dev_th THEN 0.0
                            WHEN cfg.dev_full_mul <= 1 THEN 1.0
                            ELSE MIN(1.0, (COALESCE(wd.weighted_dev, 0.0) - cfg.dev_th) / (cfg.dev_th * (cfg.dev_full_mul - 1.0)))
                        END AS struct_dev_sev,
                        CASE
                            WHEN cfg.violation_full_cnt <= 0 THEN 0.0
                            ELSE MIN(1.0, COALESCE(pi.violation_count, 0) / cfg.violation_full_cnt)
                        END AS violation_sev,
                        json_object(
                            'code', 'CAPACITY_UTILIZATION',
                            'description', '产能利用率: ' || CAST(ROUND((cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0)) * 100, 1) AS TEXT) || '%（硬阈值 ' || CAST(ROUND(cfg.cap_hard * 100.0, 1) AS TEXT) || '%）',
                            'severity', CAST(
                                CASE
                                    WHEN cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) IS NULL THEN 0.0
                                    WHEN cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) <= cfg.cap_hard THEN 0.0
                                    WHEN cfg.cap_full <= cfg.cap_hard THEN 1.0
                                    ELSE MIN(1.0, (cp.used_capacity_t / NULLIF(cp.limit_capacity_t, 0) - cfg.cap_hard) / (cfg.cap_full - cfg.cap_hard))
                                END AS REAL
                            ),
                            'affected_materials', 0
                        ) AS capacity_reason,
                        json_object(
                            'code', 'STRUCTURE_DEVIATION',
                            'description', '加权节奏偏差: ' || CAST(ROUND(COALESCE(wd.weighted_dev, 0.0) * 100.0, 1) AS TEXT) || '%（阈值 ' || CAST(ROUND(cfg.dev_th * 100.0, 1) AS TEXT) || '%，小类<' || CAST(ROUND(cfg.small_cat_th * 100.0, 1) AS TEXT) || '%忽略）',
                            'severity', CAST(
                                CASE
                                    WHEN cfg.dev_th <= 0 THEN MIN(1.0, COALESCE(wd.weighted_dev, 0.0))
                                    WHEN COALESCE(wd.weighted_dev, 0.0) <= cfg.dev_th THEN 0.0
                                    WHEN cfg.dev_full_mul <= 1 THEN 1.0
                                    ELSE MIN(1.0, (COALESCE(wd.weighted_dev, 0.0) - cfg.dev_th) / (cfg.dev_th * (cfg.dev_full_mul - 1.0)))
                                END AS REAL
                            ),
                            'affected_materials', 0
                        ) AS structure_reason,
                        json_object(
                            'code', 'STRUCTURE_VIOLATION',
                            'description', '结构违规: ' || CAST(COALESCE(pi.violation_count, 0) AS TEXT) || ' 个',
                            'severity', CAST(
                                CASE
                                    WHEN cfg.violation_full_cnt <= 0 THEN 0.0
                                    ELSE MIN(1.0, COALESCE(pi.violation_count, 0) / cfg.violation_full_cnt)
                                END AS REAL
                            ),
                            'affected_materials', COALESCE(pi.violation_count, 0)
                        ) AS violation_reason
                    FROM capacity_pool cp
                    CROSS JOIN cfg
                    LEFT JOIN weighted_dev wd
                      ON wd.machine_code = cp.machine_code
                     AND wd.plan_date = cp.plan_date
                    LEFT JOIN (
                        SELECT
                            machine_code,
                            plan_date,
                            SUM(CASE WHEN violation_flags IS NOT NULL AND violation_flags != '' THEN 1 ELSE 0 END) AS violation_count
                        FROM plan_item
                        WHERE version_id = ?1
                        GROUP BY machine_code, plan_date
                    ) pi ON cp.machine_code = pi.machine_code AND cp.plan_date = pi.plan_date
                    {capacity_where_clause}
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
                    MAX(base.cap_sev, base.struct_dev_sev, base.violation_sev) * 100.0 AS bottleneck_score,
                    CASE
                        WHEN MAX(base.cap_sev, base.struct_dev_sev, base.violation_sev) >= base.critical_th THEN 'CRITICAL'
                        WHEN MAX(base.cap_sev, base.struct_dev_sev, base.violation_sev) >= base.high_th THEN 'HIGH'
                        WHEN MAX(base.cap_sev, base.struct_dev_sev, base.violation_sev) >= base.med_th THEN 'MEDIUM'
                        WHEN MAX(base.cap_sev, base.struct_dev_sev, base.violation_sev) >= base.low_th THEN 'LOW'
                        ELSE 'NONE'
                    END AS bottleneck_level,
                    CASE
                        WHEN base.cap_sev > 0 AND (base.struct_dev_sev > 0 OR base.violation_sev > 0) THEN '["Capacity","Structure"]'
                        WHEN base.cap_sev > 0 THEN '["Capacity"]'
                        WHEN base.struct_dev_sev > 0 OR base.violation_sev > 0 THEN '["Structure"]'
                        ELSE '[]'
                    END AS bottleneck_types,
                    CASE
                        WHEN base.struct_dev_sev > 0 AND base.violation_sev > 0 THEN json_array(json(base.capacity_reason), json(base.structure_reason), json(base.violation_reason))
                        WHEN base.struct_dev_sev > 0 THEN json_array(json(base.capacity_reason), json(base.structure_reason))
                        WHEN base.violation_sev > 0 THEN json_array(json(base.capacity_reason), json(base.violation_reason))
                        ELSE json_array(json(base.capacity_reason))
                    END AS reasons,
                    base.remaining_capacity_t,
                    base.capacity_utilization,
                    0 AS needs_roll_change,
                    base.violation_count AS structure_violations,
                    0 AS pending_materials,
                    '[]' AS suggested_actions,
                    datetime('now') AS refreshed_at
                FROM base
                "#,
                category_expr = category_expr,
                capacity_where_clause = capacity_where_clause
            )
        };

        let params_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
        let rows_affected =
            tx.execute(&insert_sql, rusqlite::params_from_iter(params_refs.iter()))?;

        Ok(rows_affected)
    }
}

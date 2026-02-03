use super::*;

impl DecisionRefreshService {

    /// 刷新 D3: 哪些冷料压库
    pub(super) fn refresh_d3(
        &self,
        tx: &Transaction,
        scope: &RefreshScope,
    ) -> Result<usize, Box<dyn Error>> {
        // 1. 删除旧数据
        let delete_sql = if let Some(machines) = &scope.affected_machines {
            format!(
                "DELETE FROM decision_cold_stock_profile WHERE version_id = ?1 AND machine_code IN ({})",
                machines.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
            )
        } else {
            "DELETE FROM decision_cold_stock_profile WHERE version_id = ?1".to_string()
        };

        if let Some(machines) = &scope.affected_machines {
            let mut params: Vec<&dyn rusqlite::ToSql> = vec![&scope.version_id];
            let machine_refs: Vec<&dyn rusqlite::ToSql> = machines.iter()
                .map(|m| m as &dyn rusqlite::ToSql)
                .collect();
            params.extend(machine_refs);
            tx.execute(&delete_sql, rusqlite::params_from_iter(params))?;
        } else {
            tx.execute(&delete_sql, rusqlite::params![&scope.version_id])?;
        }

        // 2. 检查 material_state 表是否包含 D3 计算所需字段
        //
        // 真实库的 material_state 通常包含:
        // - machine_code
        // - stock_age_days
        // - is_mature
        // - weight_t
        //
        // 若缺失这些字段(例如某些测试环境),跳过刷新返回 0，避免整批刷新失败。
        let required_cols = ["machine_code", "weight_t", "is_mature", "stock_age_days"];
        for col in required_cols {
            let check_sql =
                "SELECT COUNT(*) FROM pragma_table_info('material_state') WHERE name = ?1";
            let has_col: i32 = tx.query_row(check_sql, rusqlite::params![col], |row| row.get(0))?;
            if has_col == 0 {
                return Ok(0);
            }
        }

        // 3. 从 material_state 聚合计算冷料压库情况
        // 口径：未适温(is_mature=0) 且当前版本未排产的材料，按机组 + 库龄分桶聚合。
        let mut where_conditions: Vec<String> = vec![
            "ms.is_mature = 0".to_string(),
            "ms.machine_code IS NOT NULL".to_string(),
            "NOT EXISTS (SELECT 1 FROM plan_item pi WHERE pi.version_id = ?1 AND pi.material_id = ms.material_id)".to_string(),
        ];
        let mut params: Vec<String> = vec![scope.version_id.clone()];

        if let Some(machines) = &scope.affected_machines {
            if !machines.is_empty() {
                let placeholders: Vec<String> =
                    (0..machines.len()).map(|i| format!("?{}", i + 2)).collect();
                where_conditions.push(format!("ms.machine_code IN ({})", placeholders.join(", ")));
                params.extend(machines.clone());
            }
        }

        let where_clause = format!("WHERE {}", where_conditions.join(" AND "));

        let insert_sql = format!(
            r#"
            WITH base AS (
                SELECT
                    ms.machine_code AS machine_code,
                    CASE
                        WHEN COALESCE(ms.stock_age_days, 0) < 0 THEN 0
                        ELSE COALESCE(ms.stock_age_days, 0)
                    END AS age_days,
                    ms.weight_t AS weight_t
                FROM material_state ms
                {}
            ),
            agg AS (
                SELECT
                    machine_code,
                    CASE
                        WHEN age_days <= 7 THEN '0-7'
                        WHEN age_days <= 14 THEN '8-14'
                        WHEN age_days <= 30 THEN '15-30'
                        ELSE '30+'
                    END AS age_bin,
                    CASE
                        WHEN age_days <= 7 THEN 0
                        WHEN age_days <= 14 THEN 8
                        WHEN age_days <= 30 THEN 15
                        ELSE 30
                    END AS age_min_days,
                    CASE
                        WHEN age_days <= 7 THEN 7
                        WHEN age_days <= 14 THEN 14
                        WHEN age_days <= 30 THEN 30
                        ELSE NULL
                    END AS age_max_days,
                    COUNT(*) AS count,
                    SUM(weight_t) AS weight_t,
                    AVG(age_days) AS avg_age_days
                FROM base
                GROUP BY machine_code, age_bin
            ),
            scored AS (
                SELECT
                    agg.*,
                    (
                        (
                            CASE
                                WHEN age_min_days >= 30 THEN 1.0
                                WHEN age_min_days >= 15 THEN 0.7
                                WHEN age_min_days >= 8 THEN 0.4
                                ELSE 0.2
                            END
                        ) * 0.6
                        +
                        (
                            CASE
                                WHEN count <= 5 THEN 0.3
                                WHEN count <= 10 THEN 0.6
                                WHEN count <= 20 THEN 0.8
                                ELSE 1.0
                            END
                        ) * 0.4
                    ) * 100.0 AS pressure_score
                FROM agg
            )
            INSERT INTO decision_cold_stock_profile (
                version_id,
                machine_code,
                age_bin,
                age_min_days,
                age_max_days,
                count,
                weight_t,
                avg_age_days,
                pressure_score,
                pressure_level,
                reasons,
                structure_gap,
                estimated_ready_date,
                can_force_release,
                suggested_actions,
                refreshed_at
            )
            SELECT
                ?1 AS version_id,
                machine_code,
                age_bin,
                age_min_days,
                age_max_days,
                count,
                weight_t,
                avg_age_days,
                pressure_score,
                CASE
                    WHEN pressure_score >= 80.0 THEN 'CRITICAL'
                    WHEN pressure_score >= 60.0 THEN 'HIGH'
                    WHEN pressure_score >= 40.0 THEN 'MEDIUM'
                    ELSE 'LOW'
                END AS pressure_level,
                json_array(
                    '冷料未适温',
                    '平均库龄: ' || CAST(ROUND(avg_age_days, 1) AS TEXT) || ' 天'
                ) AS reasons,
                '无' AS structure_gap,
                NULL AS estimated_ready_date,
                0 AS can_force_release,
                '[]' AS suggested_actions,
                datetime('now') AS refreshed_at
            FROM scored
            "#,
            where_clause
        );

        let params_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
        let rows_affected =
            tx.execute(&insert_sql, rusqlite::params_from_iter(params_refs.iter()))?;

        Ok(rows_affected)
    }

}

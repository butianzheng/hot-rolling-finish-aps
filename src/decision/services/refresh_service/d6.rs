use super::*;

impl DecisionRefreshService {
    /// 刷新 D6: 是否存在产能优化空间
    pub(super) fn refresh_d6(
        &self,
        tx: &Transaction,
        scope: &RefreshScope,
    ) -> Result<usize, Box<dyn Error>> {
        // 构建删除条件和插入条件
        let mut delete_conditions = vec!["version_id = ?1".to_string()];
        let mut insert_conditions: Vec<String> = Vec::new();
        let mut params: Vec<String> = vec![scope.version_id.clone()];

        if let Some(machines) = &scope.affected_machines {
            let placeholders: Vec<String> =
                (0..machines.len()).map(|i| format!("?{}", i + 2)).collect();
            delete_conditions.push(format!("machine_code IN ({})", placeholders.join(", ")));
            insert_conditions.push(format!("cp.machine_code IN ({})", placeholders.join(", ")));
            params.extend(machines.clone());
        }

        if let Some((start_date, end_date)) = &scope.affected_date_range {
            let start_idx = params.len() + 1;
            let end_idx = params.len() + 2;
            delete_conditions.push(format!("plan_date BETWEEN ?{} AND ?{}", start_idx, end_idx));
            insert_conditions.push(format!(
                "cp.plan_date BETWEEN ?{} AND ?{}",
                start_idx, end_idx
            ));
            params.push(start_date.clone());
            params.push(end_date.clone());
        }

        // 1. 删除旧数据
        let delete_sql = format!(
            "DELETE FROM decision_capacity_opportunity WHERE {}",
            delete_conditions.join(" AND ")
        );
        let params_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
        tx.execute(&delete_sql, rusqlite::params_from_iter(params_refs.iter()))?;

        // 2. 构建 INSERT WHERE 条件
        insert_conditions
            .push("cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) < 0.9".to_string());
        let has_cp_version_id: i32 = tx.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('capacity_pool') WHERE name = 'version_id'",
            [],
            |row| row.get(0),
        )?;
        if has_cp_version_id > 0 {
            insert_conditions.push("cp.version_id = ?1".to_string());
        }
        let insert_where_clause =
            format!("\n            WHERE {}", insert_conditions.join(" AND "));

        // 3. 从 capacity_pool 计算产能优化空间
        let insert_sql = format!(
            r#"
            INSERT INTO decision_capacity_opportunity (
                version_id,
                machine_code,
                plan_date,
                slack_t,
                soft_adjust_space_t,
                utilization_rate,
                binding_constraints,
                opportunity_level,
                sensitivity,
                suggested_optimizations,
                refreshed_at
            )
            SELECT
                ?1 AS version_id,
                cp.machine_code,
                cp.plan_date,
                cp.target_capacity_t - cp.used_capacity_t AS slack_t,
                CASE
                    WHEN cp.target_capacity_t - cp.used_capacity_t > 100 THEN cp.target_capacity_t - cp.used_capacity_t - 50
                    ELSE 0.0
                END AS soft_adjust_space_t,
                cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) AS utilization_rate,
                '[]' AS binding_constraints,
                CASE
                    WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) < 0.5 THEN 'HIGH'
                    WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) < 0.7 THEN 'MEDIUM'
                    WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) < 0.9 THEN 'LOW'
                    ELSE 'NONE'
                END AS opportunity_level,
                'Normal' AS sensitivity,
                CASE
                    WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) < 0.5 THEN '["考虑调整产能计划"]'
                    WHEN cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) < 0.7 THEN '["可适当增加排产"]'
                    ELSE '[]'
                END AS suggested_optimizations,
                datetime('now') AS refreshed_at
            FROM capacity_pool cp{}
            "#,
            insert_where_clause
        );

        let rows_affected =
            tx.execute(&insert_sql, rusqlite::params_from_iter(params_refs.iter()))?;

        Ok(rows_affected)
    }
}

use super::*;

impl DecisionRefreshService {

    /// 刷新 D1: 哪天最危险
    pub(super) fn refresh_d1(
        &self,
        tx: &Transaction,
        scope: &RefreshScope,
    ) -> Result<usize, Box<dyn Error>> {
        // D1 历史实现依赖 risk_snapshot 聚合；但在部分环境中 risk_snapshot 尚未生成，
        // 会导致“驾驶舱/决策看板 D1 风险热力图无数据”。这里增加基于 capacity_pool 的兜底聚合。

        // 构建删除条件和查询条件（增量刷新时支持机组/日期过滤）
        let mut delete_conditions = vec!["version_id = ?1".to_string()];
        let mut risk_conditions = vec!["version_id = ?1".to_string()];
        let mut cp_conditions: Vec<String> = Vec::new();
        let mut params: Vec<String> = vec![scope.version_id.clone()];

        if !scope.is_full_refresh {
            if let Some(machines) = &scope.affected_machines {
                if !machines.is_empty() {
                    let placeholders: Vec<String> = (0..machines.len())
                        .map(|i| format!("?{}", params.len() + i + 1))
                        .collect();
                    risk_conditions.push(format!("machine_code IN ({})", placeholders.join(", ")));
                    cp_conditions.push(format!("cp.machine_code IN ({})", placeholders.join(", ")));
                    params.extend(machines.clone());
                }
            }

            if let Some((start_date, end_date)) = &scope.affected_date_range {
                let start_idx = params.len() + 1;
                let end_idx = params.len() + 2;
                delete_conditions.push(format!("plan_date BETWEEN ?{} AND ?{}", start_idx, end_idx));
                risk_conditions.push(format!(
                    "snapshot_date BETWEEN ?{} AND ?{}",
                    start_idx, end_idx
                ));
                cp_conditions.push(format!("cp.plan_date BETWEEN ?{} AND ?{}", start_idx, end_idx));
                params.push(start_date.clone());
                params.push(end_date.clone());
            }
        }

        // 删除旧数据（全量：按 version_id；增量：按 version_id+日期范围）
        let delete_sql = format!(
            "DELETE FROM decision_day_summary WHERE {}",
            delete_conditions.join(" AND ")
        );
        let params_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
        tx.execute(&delete_sql, rusqlite::params_from_iter(params_refs.iter()))?;

        // 判断 risk_snapshot 是否有可用数据；若无则使用 capacity_pool 兜底聚合。
        let count_sql = format!(
            "SELECT COUNT(*) FROM risk_snapshot WHERE {}",
            risk_conditions.join(" AND ")
        );
        let risk_snapshot_count: i64 = tx.query_row(
            &count_sql,
            rusqlite::params_from_iter(params_refs.iter()),
            |row| row.get(0),
        )?;

        if risk_snapshot_count > 0 {
            let where_clause = format!("\n            WHERE {}", risk_conditions.join(" AND "));
            let insert_sql = format!(
                r#"
                INSERT OR REPLACE INTO decision_day_summary (
                    version_id,
                    plan_date,
                    risk_score,
                    risk_level,
                    capacity_util_pct,
                    top_reasons,
                    affected_machines,
                    bottleneck_machines,
                    has_roll_risk,
                    suggested_actions,
                    refreshed_at
                )
                WITH risk_norm AS (
                    SELECT
                        version_id,
                        snapshot_date,
                        machine_code,
                        used_capacity_t,
                        target_capacity_t,
                        risk_reasons,
                        campaign_status,
                        CASE
                            WHEN risk_level IN ('CRITICAL', 'RED', 'Red') THEN 90.0
                            WHEN risk_level IN ('HIGH', 'ORANGE', 'Orange') THEN 70.0
                            WHEN risk_level IN ('MEDIUM', 'YELLOW', 'Yellow') THEN 40.0
                            WHEN risk_level IN ('LOW', 'GREEN', 'Green') THEN 20.0
                            ELSE 20.0
                        END AS score,
                        CASE
                            WHEN risk_level IN ('CRITICAL', 'RED', 'Red') THEN 'CRITICAL'
                            WHEN risk_level IN ('HIGH', 'ORANGE', 'Orange') THEN 'HIGH'
                            WHEN risk_level IN ('MEDIUM', 'YELLOW', 'Yellow') THEN 'MEDIUM'
                            WHEN risk_level IN ('LOW', 'GREEN', 'Green') THEN 'LOW'
                            ELSE 'LOW'
                        END AS normalized_level
                    FROM risk_snapshot
                    {}
                )
                SELECT
                    version_id,
                    snapshot_date AS plan_date,
                    AVG(score) AS risk_score,
                    CASE
                        WHEN AVG(score) >= 80.0 THEN 'CRITICAL'
                        WHEN AVG(score) >= 60.0 THEN 'HIGH'
                        WHEN AVG(score) >= 30.0 THEN 'MEDIUM'
                        ELSE 'LOW'
                    END AS risk_level,
                    -- capacity_util_pct 在 use case 层口径为 0-1，这里按比率存储（而非百分比）
                    AVG(used_capacity_t / NULLIF(target_capacity_t, 0)) AS capacity_util_pct,
                    json_group_array(
                        json_object(
                            'code', 'CAPACITY_' || normalized_level,
                            'msg', risk_reasons,
                            'weight', 1.0,
                            'severity', CASE
                                WHEN normalized_level = 'CRITICAL' THEN 1.0
                                WHEN normalized_level = 'HIGH' THEN 0.7
                                WHEN normalized_level = 'MEDIUM' THEN 0.4
                                ELSE 0.2
                            END
                        )
                    ) AS top_reasons,
                    COUNT(DISTINCT machine_code) AS affected_machines,
                    SUM(CASE WHEN normalized_level IN ('HIGH', 'CRITICAL') THEN 1 ELSE 0 END) AS bottleneck_machines,
                    MAX(CASE WHEN campaign_status IN ('NEAR_HARD_STOP', 'HARD_STOP') THEN 1 ELSE 0 END) AS has_roll_risk,
                    '[]' AS suggested_actions,
                    datetime('now') AS refreshed_at
                FROM risk_norm
                GROUP BY version_id, snapshot_date
                "#,
                where_clause
            );

            let rows_affected =
                tx.execute(&insert_sql, rusqlite::params_from_iter(params_refs.iter()))?;
            return Ok(rows_affected);
        }

        // capacity_pool 兜底路径（risk_snapshot 为空时）
        let cp_where_clause = if cp_conditions.is_empty() {
            String::new()
        } else {
            format!("\n            WHERE {}", cp_conditions.join(" AND "))
        };

        let insert_sql = format!(
            r#"
            INSERT OR REPLACE INTO decision_day_summary (
                version_id,
                plan_date,
                risk_score,
                risk_level,
                capacity_util_pct,
                top_reasons,
                affected_machines,
                bottleneck_machines,
                has_roll_risk,
                suggested_actions,
                refreshed_at
            )
            SELECT
                ?1 AS version_id,
                cp.plan_date AS plan_date,
                COALESCE(MAX(cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0)), 0.0) * 100.0 AS risk_score,
                CASE
                    WHEN COALESCE(MAX(cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0)), 0.0) >= 0.8 THEN 'CRITICAL'
                    WHEN COALESCE(MAX(cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0)), 0.0) >= 0.6 THEN 'HIGH'
                    WHEN COALESCE(MAX(cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0)), 0.0) >= 0.4 THEN 'MEDIUM'
                    ELSE 'LOW'
                END AS risk_level,
                -- 0-1 比率口径
                COALESCE(MAX(cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0)), 0.0) AS capacity_util_pct,
                json_array(
                    json_object(
                        'code', 'CAPACITY_UTILIZATION',
                        'msg', '产能利用率: ' || CAST(ROUND(COALESCE(MAX(cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0)), 0.0) * 100.0, 1) AS TEXT) || '%',
                        'weight', 1.0,
                        'severity', COALESCE(MAX(cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0)), 0.0)
                    )
                ) AS top_reasons,
                COUNT(DISTINCT cp.machine_code) AS affected_machines,
                SUM(CASE WHEN (cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0)) >= 0.9 THEN 1 ELSE 0 END) AS bottleneck_machines,
                0 AS has_roll_risk,
                '[]' AS suggested_actions,
                datetime('now') AS refreshed_at
            FROM capacity_pool cp{}
            GROUP BY cp.plan_date
            "#,
            cp_where_clause
        );

        let rows_affected =
            tx.execute(&insert_sql, rusqlite::params_from_iter(params_refs.iter()))?;
        Ok(rows_affected)
    }

}

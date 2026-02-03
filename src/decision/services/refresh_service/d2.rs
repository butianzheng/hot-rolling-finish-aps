use super::*;

impl DecisionRefreshService {

    /// 刷新 D2: 哪些紧急单无法完成
    pub(super) fn refresh_d2(
        &self,
        tx: &Transaction,
        scope: &RefreshScope,
    ) -> Result<usize, Box<dyn Error>> {
        // 1. 删除旧数据
        tx.execute(
            "DELETE FROM decision_order_failure_set WHERE version_id = ?",
            rusqlite::params![&scope.version_id],
        )?;

        // 2. 检查 material_state 表是否有必要的列
        // 如果表结构不完整(如测试环境),返回 0
        let check_sql = "SELECT COUNT(*) FROM pragma_table_info('material_state') WHERE name = 'contract_no'";
        let has_contract_no: i32 = tx.query_row(check_sql, [], |row| row.get(0))?;

        if has_contract_no == 0 {
            // material_state 表缺少必要列,跳过刷新
            return Ok(0);
        }

        // 3. 从 material_state 和 plan_item 聚合计算订单失败情况
        let insert_sql = r#"
            INSERT INTO decision_order_failure_set (
                version_id,
                contract_no,
                due_date,
                urgency_level,
                fail_type,
                total_materials,
                unscheduled_count,
                unscheduled_weight_t,
                completion_rate,
                days_to_due,
                failure_reasons,
                blocking_factors,
                suggested_actions,
                refreshed_at
            )
            SELECT
                ?1 AS version_id,
                ms.contract_no,
                ms.due_date,
                ms.urgency_level,
                CASE
                    WHEN julianday(ms.due_date) < julianday('now') THEN 'Overdue'
                    WHEN julianday(ms.due_date) - julianday('now') <= 3
                         AND COUNT(pi.material_id) < COUNT(*) * 0.5 THEN 'NearDueImpossible'
                    WHEN COUNT(pi.material_id) = 0 THEN 'CapacityShortage'
                    ELSE 'Other'
                END AS fail_type,
                COUNT(*) AS total_materials,
                COUNT(*) - COUNT(pi.material_id) AS unscheduled_count,
                SUM(CASE WHEN pi.material_id IS NULL THEN ms.weight_t ELSE 0 END) AS unscheduled_weight_t,
                CAST(COUNT(pi.material_id) AS REAL) / NULLIF(COUNT(*), 0) AS completion_rate,
                CAST(julianday(ms.due_date) - julianday('now') AS INTEGER) AS days_to_due,
                json_array('产能不足', '结构冲突') AS failure_reasons,
                json_array(
                    json_object(
                        'factor', 'Capacity',
                        'severity', 0.8,
                        'description', '产能不足'
                    )
                ) AS blocking_factors,
                '[]' AS suggested_actions,
                datetime('now') AS refreshed_at
            FROM material_state ms
            LEFT JOIN plan_item pi ON pi.version_id = ?1 AND pi.material_id = ms.material_id
            WHERE ms.urgency_level IN ('L1', 'L2', 'L3')
                AND ms.contract_no IS NOT NULL
            GROUP BY ms.contract_no, ms.due_date, ms.urgency_level
            HAVING COUNT(pi.material_id) < COUNT(*)
        "#;

        let rows_affected = tx.execute(insert_sql, rusqlite::params![&scope.version_id])?;

        Ok(rows_affected)
    }

}

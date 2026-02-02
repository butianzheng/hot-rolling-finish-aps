// ==========================================
// 热轧精整排产系统 - 决策刷新服务
// ==========================================
// 依据: REFACTOR_PLAN_v1.0.md - P1 阶段
// 职责: 刷新决策读模型表（decision_* 表）
// ==========================================

use chrono::{Local, NaiveDate, NaiveDateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction};
use std::error::Error;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// P2 阶段: refresh_d2~d6 方法已重构,不再需要 Repository import

/// 刷新触发类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshTrigger {
    /// 计划项变更
    PlanItemChanged,
    /// 风险快照更新
    RiskSnapshotUpdated,
    /// 物料状态变更
    MaterialStateChanged,
    /// 产能池变更
    CapacityPoolChanged,
    /// 换辊批次变更
    RollCampaignChanged,
    /// 每日生产节奏目标变更
    RhythmTargetChanged,
    /// 版本创建
    VersionCreated,
    /// 手动刷新
    ManualRefresh,
}

impl RefreshTrigger {
    pub fn as_str(&self) -> &str {
        match self {
            RefreshTrigger::PlanItemChanged => "PlanItemChanged",
            RefreshTrigger::RiskSnapshotUpdated => "RiskSnapshotUpdated",
            RefreshTrigger::MaterialStateChanged => "MaterialStateChanged",
            RefreshTrigger::CapacityPoolChanged => "CapacityPoolChanged",
            RefreshTrigger::RollCampaignChanged => "RollCampaignChanged",
            RefreshTrigger::RhythmTargetChanged => "RhythmTargetChanged",
            RefreshTrigger::VersionCreated => "VersionCreated",
            RefreshTrigger::ManualRefresh => "ManualRefresh",
        }
    }
}

/// 刷新范围
#[derive(Debug, Clone)]
pub struct RefreshScope {
    /// 版本 ID
    pub version_id: String,
    /// 是否全量刷新
    pub is_full_refresh: bool,
    /// 受影响的机组（可选）
    pub affected_machines: Option<Vec<String>>,
    /// 受影响的日期范围（可选）
    pub affected_date_range: Option<(String, String)>,
}

/// 决策刷新服务
pub struct DecisionRefreshService {
    conn: Arc<Mutex<Connection>>,
}

impl DecisionRefreshService {
    /// 创建新的刷新服务实例
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 刷新所有决策视图（P1 版本：只刷新 D1 和 D4）
    ///
    /// # 参数
    /// - `scope`: 刷新范围
    /// - `trigger`: 触发类型
    /// - `trigger_source`: 触发源（操作人/系统组件）
    ///
    /// # 返回
    /// - Ok(refresh_id): 刷新任务 ID
    /// - Err: 刷新失败错误
    pub fn refresh_all(
        &self,
        scope: RefreshScope,
        trigger: RefreshTrigger,
        trigger_source: Option<String>,
    ) -> Result<String, Box<dyn Error>> {
        let refresh_id = Uuid::new_v4().to_string();
        let started_at = Utc::now().to_rfc3339();

        let mut conn = self.conn.lock()
            .map_err(|e| format!("锁获取失败: {}", e))?;
        let tx = conn.transaction()?;

        // 记录刷新开始
        self.log_refresh_start(
            &tx,
            &refresh_id,
            &scope.version_id,
            &trigger,
            trigger_source.as_deref(),
            scope.is_full_refresh,
            &started_at,
        )?;

        let mut refreshed_tables = Vec::new();
        let mut total_rows_affected = 0;

        // 刷新 D1: 哪天最危险
        if self.should_refresh_d1(&trigger) {
            let rows = self.refresh_d1(&tx, &scope)?;
            refreshed_tables.push("decision_day_summary".to_string());
            total_rows_affected += rows;
        }

        // 刷新 D4: 哪个机组最堵
        if self.should_refresh_d4(&trigger) {
            let rows = self.refresh_d4(&tx, &scope)?;
            refreshed_tables.push("decision_machine_bottleneck".to_string());
            total_rows_affected += rows;
        }

        // ==========================================
        // P2 阶段: D2-D6 刷新已重构,直接使用 Transaction
        // ==========================================
        // 修复说明: 已将 refresh_d2~d6 方法改为直接使用 Transaction,
        // 不再创建新的 Repository,避免死锁问题。
        // ==========================================

        // 刷新 D2: 哪些紧急单无法完成
        if self.should_refresh_d2(&trigger) {
            let rows = self.refresh_d2(&tx, &scope)?;
            refreshed_tables.push("decision_order_failure_set".to_string());
            total_rows_affected += rows;
        }

        // 刷新 D3: 哪些冷料压库
        if self.should_refresh_d3(&trigger) {
            let rows = self.refresh_d3(&tx, &scope)?;
            refreshed_tables.push("decision_cold_stock_profile".to_string());
            total_rows_affected += rows;
        }

        // 刷新 D5: 换辊是否异常
        if self.should_refresh_d5(&trigger) {
            let rows = self.refresh_d5(&tx, &scope)?;
            refreshed_tables.push("decision_roll_campaign_alert".to_string());
            total_rows_affected += rows;
        }

        // 刷新 D6: 是否存在产能优化空间
        if self.should_refresh_d6(&trigger) {
            let rows = self.refresh_d6(&tx, &scope)?;
            refreshed_tables.push("decision_capacity_opportunity".to_string());
            total_rows_affected += rows;
        }

        // 记录刷新完成
        let completed_at = Utc::now().to_rfc3339();
        let duration_ms = (chrono::DateTime::parse_from_rfc3339(&completed_at)?
            .timestamp_millis()
            - chrono::DateTime::parse_from_rfc3339(&started_at)?.timestamp_millis())
            as i64;

        self.log_refresh_complete(
            &tx,
            &refresh_id,
            &refreshed_tables,
            total_rows_affected,
            &completed_at,
            duration_ms,
        )?;

        tx.commit()?;

        tracing::info!(
            "决策视图刷新完成: refresh_id={}, tables={:?}, rows={}",
            refresh_id,
            refreshed_tables,
            total_rows_affected
        );

        Ok(refresh_id)
    }

    /// 判断是否应该刷新 D1
    fn should_refresh_d1(&self, trigger: &RefreshTrigger) -> bool {
        matches!(
            trigger,
            RefreshTrigger::RiskSnapshotUpdated
                | RefreshTrigger::PlanItemChanged
                | RefreshTrigger::VersionCreated
                | RefreshTrigger::ManualRefresh
        )
    }

    /// 判断是否应该刷新 D4
    fn should_refresh_d4(&self, trigger: &RefreshTrigger) -> bool {
        matches!(
            trigger,
            RefreshTrigger::CapacityPoolChanged
                | RefreshTrigger::PlanItemChanged
                | RefreshTrigger::MaterialStateChanged
                | RefreshTrigger::RhythmTargetChanged
                | RefreshTrigger::VersionCreated
                | RefreshTrigger::ManualRefresh
        )
    }

    /// 判断是否应该刷新 D2: 哪些紧急单无法完成
    fn should_refresh_d2(&self, trigger: &RefreshTrigger) -> bool {
        matches!(
            trigger,
            RefreshTrigger::PlanItemChanged
                | RefreshTrigger::MaterialStateChanged
                | RefreshTrigger::RiskSnapshotUpdated
                | RefreshTrigger::VersionCreated
                | RefreshTrigger::ManualRefresh
        )
    }

    /// 判断是否应该刷新 D3: 哪些冷料压库
    fn should_refresh_d3(&self, trigger: &RefreshTrigger) -> bool {
        matches!(
            trigger,
            RefreshTrigger::MaterialStateChanged
                | RefreshTrigger::PlanItemChanged
                | RefreshTrigger::VersionCreated
                | RefreshTrigger::ManualRefresh
        )
    }

    /// 判断是否应该刷新 D5: 换辊是否异常
    fn should_refresh_d5(&self, trigger: &RefreshTrigger) -> bool {
        matches!(
            trigger,
            RefreshTrigger::RollCampaignChanged
                | RefreshTrigger::MaterialStateChanged
                | RefreshTrigger::PlanItemChanged
                | RefreshTrigger::VersionCreated
                | RefreshTrigger::ManualRefresh
        )
    }

    /// 判断是否应该刷新 D6: 是否存在产能优化空间
    fn should_refresh_d6(&self, trigger: &RefreshTrigger) -> bool {
        matches!(
            trigger,
            RefreshTrigger::CapacityPoolChanged
                | RefreshTrigger::PlanItemChanged
                | RefreshTrigger::MaterialStateChanged
                | RefreshTrigger::VersionCreated
                | RefreshTrigger::ManualRefresh
        )
    }

    /// 刷新 D1: 哪天最危险
    fn refresh_d1(
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

    /// 刷新 D4: 哪个机组最堵
    fn refresh_d4(
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

    /// 刷新 D2: 哪些紧急单无法完成
    fn refresh_d2(
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

    /// 刷新 D3: 哪些冷料压库
    fn refresh_d3(
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

    /// 刷新 D5: 换辊是否异常
    fn refresh_d5(
        &self,
        tx: &Transaction,
        scope: &RefreshScope,
    ) -> Result<usize, Box<dyn Error>> {
        // ==========================================
        // 新口径：换辊从“风险/约束”调整为“设备时间监控”
        // - 周期起点：来自 roll_campaign_plan.initial_start_at（可人工微调）；缺省用该机组最早 plan_date 00:00。
        // - 当前累计：按计划项时间线（plan_item + hourly_capacity_t）估算到 as_of（刷新时刻）。
        // - 周期重置：默认在到达软限制时触发换辊（并产生停机时长）；用于避免“全版本求和导致 300%+”的问题。
        // - 计划换辊时刻：允许通过 roll_campaign_plan.next_change_at 覆盖（只影响“下一次换辊”提示，不直接改排程）。
        // ==========================================

        fn table_has_column(tx: &Transaction, table: &str, col: &str) -> bool {
            if table.trim().is_empty() || col.trim().is_empty() {
                return false;
            }
            // NOTE: `pragma_table_info(?1)` is not reliably parameterizable across SQLite builds.
            // Since `table` is an internal constant, we safely inline it.
            let table_escaped = table.replace('\'', "''");
            let sql = format!(
                "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?1",
                table_escaped
            );
            tx.query_row(&sql, rusqlite::params![col], |row| row.get::<_, i32>(0))
                .map(|v| v > 0)
                .unwrap_or(false)
        }

        // Ensure D5 extension columns exist (best-effort, keeps old DB compatible).
        let _ = tx.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS roll_campaign_plan (
              version_id TEXT NOT NULL,
              machine_code TEXT NOT NULL,
              initial_start_at TEXT NOT NULL,
              next_change_at TEXT,
              downtime_minutes INTEGER,
              updated_at TEXT NOT NULL DEFAULT (datetime('now')),
              updated_by TEXT,
              PRIMARY KEY (version_id, machine_code)
            );
            "#,
        );

        if table_has_column(tx, "decision_roll_campaign_alert", "campaign_start_at") == false {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN campaign_start_at TEXT",
                [],
            );
        }
        if table_has_column(tx, "decision_roll_campaign_alert", "planned_change_at") == false {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN planned_change_at TEXT",
                [],
            );
        }
        if table_has_column(tx, "decision_roll_campaign_alert", "planned_downtime_minutes") == false
        {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN planned_downtime_minutes INTEGER",
                [],
            );
        }
        if table_has_column(tx, "decision_roll_campaign_alert", "estimated_soft_reach_at") == false
        {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN estimated_soft_reach_at TEXT",
                [],
            );
        }
        if table_has_column(tx, "decision_roll_campaign_alert", "estimated_hard_reach_at") == false
        {
            let _ = tx.execute(
                "ALTER TABLE decision_roll_campaign_alert ADD COLUMN estimated_hard_reach_at TEXT",
                [],
            );
        }

        // 1) 删除旧数据
        let delete_sql = if let Some(machines) = &scope.affected_machines {
            format!(
                "DELETE FROM decision_roll_campaign_alert WHERE version_id = ?1 AND machine_code IN ({})",
                machines.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
            )
        } else {
            "DELETE FROM decision_roll_campaign_alert WHERE version_id = ?1".to_string()
        };

        if let Some(machines) = &scope.affected_machines {
            let mut params: Vec<&dyn rusqlite::ToSql> = vec![&scope.version_id];
            let machine_refs: Vec<&dyn rusqlite::ToSql> = machines
                .iter()
                .map(|m| m as &dyn rusqlite::ToSql)
                .collect();
            params.extend(machine_refs);
            tx.execute(&delete_sql, rusqlite::params_from_iter(params))?;
        } else {
            tx.execute(&delete_sql, rusqlite::params![&scope.version_id])?;
        }

        // 2) 读取全局阈值与默认停机时长
        let read_global_real = |key: &str| -> Result<Option<f64>, rusqlite::Error> {
            tx.query_row(
                "SELECT value FROM config_kv WHERE scope_id = 'global' AND key = ?1 LIMIT 1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map(|opt| opt.and_then(|s| s.trim().parse::<f64>().ok()))
        };

        let read_global_i32 = |key: &str| -> Result<Option<i32>, rusqlite::Error> {
            tx.query_row(
                "SELECT value FROM config_kv WHERE scope_id = 'global' AND key = ?1 LIMIT 1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map(|opt| opt.and_then(|s| s.trim().parse::<i32>().ok()))
        };

        let suggest_threshold_t = read_global_real("roll_suggest_threshold_t")
            .unwrap_or(None)
            .filter(|v| *v > 0.0)
            .unwrap_or(1500.0);
        let hard_limit_t = read_global_real("roll_hard_limit_t")
            .unwrap_or(None)
            .filter(|v| *v > 0.0)
            .unwrap_or(2500.0);
        let default_downtime_minutes = read_global_i32("roll_change_downtime_minutes")
            .unwrap_or(None)
            .filter(|v| *v > 0)
            .unwrap_or(45);

        // 3) 获取机组列表 + 小时产能
        let has_hourly_capacity = table_has_column(tx, "machine_master", "hourly_capacity_t");
        let has_is_active = table_has_column(tx, "machine_master", "is_active");

        let mut machine_sql = String::new();
        if has_hourly_capacity {
            machine_sql.push_str("SELECT machine_code, COALESCE(hourly_capacity_t, 0) AS hourly_capacity_t FROM machine_master");
        } else {
            machine_sql.push_str("SELECT machine_code, 0 AS hourly_capacity_t FROM machine_master");
        }
        if has_is_active {
            machine_sql.push_str(" WHERE is_active = 1");
        }

        // Filter machines if scope provided.
        if let Some(machines) = &scope.affected_machines {
            if !machines.is_empty() {
                let placeholders = machines.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                if machine_sql.contains(" WHERE ") {
                    machine_sql.push_str(&format!(" AND machine_code IN ({})", placeholders));
                } else {
                    machine_sql.push_str(&format!(" WHERE machine_code IN ({})", placeholders));
                }
            }
        }

        machine_sql.push_str(" ORDER BY machine_code ASC");

        let mut machine_stmt = tx.prepare(&machine_sql)?;
        let machines: Vec<(String, f64)> = if let Some(machines) = &scope.affected_machines {
            if machines.is_empty() {
                machine_stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
                    })?
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                let params: Vec<&dyn rusqlite::ToSql> =
                    machines.iter().map(|m| m as &dyn rusqlite::ToSql).collect();
                machine_stmt
                    .query_map(rusqlite::params_from_iter(params), |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
                    })?
                    .collect::<Result<Vec<_>, _>>()?
            }
        } else {
            machine_stmt
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)))?
                .collect::<Result<Vec<_>, _>>()?
        };
        if machines.is_empty() {
            return Ok(0);
        }

        // 4) 读取 plan_item 列存在性（兼容测试库/旧库）
        let has_weight_t = table_has_column(tx, "plan_item", "weight_t");
        let has_seq_no = table_has_column(tx, "plan_item", "seq_no");

        if !has_weight_t {
            // 无法估算时间线（缺少重量），直接跳过刷新，避免 UI 因校验失败崩溃。
            return Ok(0);
        }

        #[derive(Debug, Clone)]
        struct PlanItemLite {
            earliest_start_at: NaiveDateTime,
            weight_t: f64,
        }

        fn parse_dt_best_effort(raw: &str) -> Option<NaiveDateTime> {
            let s = raw.trim();
            if s.is_empty() {
                return None;
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                return Some(dt);
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
                return Some(dt);
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
                return Some(dt);
            }
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                return Some(dt.naive_local());
            }
            None
        }

        fn ymd_to_start_at(ymd: &str) -> Option<NaiveDateTime> {
            let d = NaiveDate::parse_from_str(ymd, "%Y-%m-%d").ok()?;
            d.and_hms_opt(0, 0, 0)
        }

        #[derive(Debug, Clone)]
        struct StreamState {
            item_index: usize,
            remaining_weight_t: f64,
            current_time: NaiveDateTime,
            campaign_no: i32,
            campaign_start_at: NaiveDateTime,
            cum_weight_t: f64,
        }

        fn produce_weight_until(
            items: &[PlanItemLite],
            mut state: StreamState,
            rate_t_per_sec: f64,
            additional_weight_t: f64,
        ) -> Option<StreamState> {
            if additional_weight_t <= 0.0 {
                return Some(state);
            }
            if rate_t_per_sec <= 0.0 {
                return None;
            }

            let mut need = additional_weight_t;
            while need > 1e-9 {
                if state.item_index >= items.len() {
                    return None;
                }
                let item = &items[state.item_index];
                if state.current_time < item.earliest_start_at {
                    state.current_time = item.earliest_start_at;
                }

                let take = state.remaining_weight_t.min(need);
                let seconds_f = take / rate_t_per_sec;
                let seconds = seconds_f.round().max(0.0) as i64;
                state.current_time += chrono::Duration::seconds(seconds);
                state.remaining_weight_t -= take;
                need -= take;

                if state.remaining_weight_t <= 1e-9 {
                    state.item_index += 1;
                    if state.item_index < items.len() {
                        state.remaining_weight_t = items[state.item_index].weight_t;
                    } else {
                        state.remaining_weight_t = 0.0;
                    }
                }
            }
            Some(state)
        }

        fn simulate_to_as_of(
            items: &[PlanItemLite],
            rate_t_per_sec: f64,
            initial_start_at: NaiveDateTime,
            suggest_threshold_t: f64,
            downtime_minutes: i64,
            as_of: NaiveDateTime,
        ) -> StreamState {
            let mut state = StreamState {
                item_index: 0,
                remaining_weight_t: items.first().map(|i| i.weight_t).unwrap_or(0.0),
                current_time: items.first().map(|i| i.earliest_start_at).unwrap_or(as_of),
                campaign_no: 1,
                campaign_start_at: initial_start_at,
                cum_weight_t: 0.0,
            };

            // If as_of is before the schedule starts, clamp current_time to as_of and exit early.
            if state.current_time > as_of {
                state.current_time = as_of;
                return state;
            }

            let mut campaign_active = state.current_time >= initial_start_at;
            if !campaign_active && state.current_time < initial_start_at && as_of >= initial_start_at {
                // Campaign becomes active sometime before/as_of (possibly during idle); we will handle more precisely below.
            }

            while state.current_time < as_of {
                if state.item_index >= items.len() {
                    // No more production; idle until as_of.
                    state.current_time = as_of;
                    break;
                }

                let item = &items[state.item_index];
                let item_start = if state.current_time < item.earliest_start_at {
                    item.earliest_start_at
                } else {
                    state.current_time
                };

                // Idle gap before next item.
                if state.current_time < item_start {
                    if !campaign_active && initial_start_at <= item_start && initial_start_at <= as_of {
                        campaign_active = true;
                        state.campaign_start_at = initial_start_at;
                    }

                    if as_of < item_start {
                        state.current_time = as_of;
                        break;
                    }
                    state.current_time = item_start;
                }

                // If campaign starts during this item's processing, split at initial_start_at.
                if !campaign_active && state.current_time < initial_start_at && initial_start_at <= as_of {
                    // Produce until initial_start_at (does not count into cum_weight_t)
                    let seconds_until = (initial_start_at - state.current_time).num_seconds();
                    if seconds_until > 0 && rate_t_per_sec > 0.0 {
                        let producible = (seconds_until as f64) * rate_t_per_sec;
                        let produced = state.remaining_weight_t.min(producible);
                        let actual_seconds = (produced / rate_t_per_sec).round().max(0.0) as i64;
                        state.current_time += chrono::Duration::seconds(actual_seconds);
                        state.remaining_weight_t -= produced;
                        if state.remaining_weight_t <= 1e-9 {
                            state.item_index += 1;
                            if state.item_index < items.len() {
                                state.remaining_weight_t = items[state.item_index].weight_t;
                            } else {
                                state.remaining_weight_t = 0.0;
                            }
                        }
                    }

                    if state.current_time >= initial_start_at {
                        campaign_active = true;
                        state.campaign_start_at = initial_start_at;
                    }

                    continue;
                }

                // No production capacity
                if rate_t_per_sec <= 0.0 {
                    state.current_time = as_of;
                    break;
                }

                // Process the current item in small segments: (as_of boundary) and (soft-threshold boundary).
                let mut seg_start = state.current_time;
                while seg_start < as_of && state.remaining_weight_t > 1e-9 {
                    let seconds_to_finish_item =
                        (state.remaining_weight_t / rate_t_per_sec).round().max(0.0) as i64;
                    let finish_time = seg_start + chrono::Duration::seconds(seconds_to_finish_item);
                    let mut next_event_time = finish_time;

                    // Stop at as_of
                    if as_of < next_event_time {
                        next_event_time = as_of;
                    }

                    // Soft limit reach -> triggers roll change (auto), but only when campaign is active.
                    if campaign_active && suggest_threshold_t > 0.0 {
                        let remaining_to_soft = suggest_threshold_t - state.cum_weight_t;
                        if remaining_to_soft >= 0.0 && state.remaining_weight_t >= remaining_to_soft {
                            let sec_to_soft =
                                (remaining_to_soft / rate_t_per_sec).round().max(0.0) as i64;
                            let soft_time = seg_start + chrono::Duration::seconds(sec_to_soft);
                            if soft_time < next_event_time {
                                next_event_time = soft_time;
                            }
                        }
                    }

                    let delta_seconds = (next_event_time - seg_start).num_seconds().max(0);
                    let produced = (delta_seconds as f64) * rate_t_per_sec;
                    let produced = produced.min(state.remaining_weight_t).max(0.0);

                    if campaign_active {
                        state.cum_weight_t += produced;
                    }

                    state.remaining_weight_t -= produced;
                    state.current_time = next_event_time;
                    seg_start = next_event_time;

                    // Reached as_of
                    if state.current_time >= as_of {
                        break;
                    }

                    // Finished item
                    if (finish_time - state.current_time).num_seconds().abs() <= 1 {
                        state.item_index += 1;
                        if state.item_index < items.len() {
                            state.remaining_weight_t = items[state.item_index].weight_t;
                        } else {
                            state.remaining_weight_t = 0.0;
                        }
                        break;
                    }

                    // Soft threshold reached -> downtime + reset (if downtime fits before as_of)
                    if campaign_active && suggest_threshold_t > 0.0 {
                        let reached_soft = (state.cum_weight_t - suggest_threshold_t).abs() <= 1e-6
                            || state.cum_weight_t >= suggest_threshold_t;
                        if reached_soft {
                            let downtime_end = state.current_time + chrono::Duration::minutes(downtime_minutes);
                            if downtime_end > as_of {
                                // as_of within downtime: stop here, keep current campaign as-is.
                                state.current_time = as_of;
                                return state;
                            }

                            // Apply downtime and start next campaign
                            state.current_time = downtime_end;
                            state.campaign_no += 1;
                            state.cum_weight_t = 0.0;
                            state.campaign_start_at = state.current_time;
                            // Continue processing remaining weight (same item) after downtime
                            seg_start = state.current_time;
                            continue;
                        }
                    }
                }
            }

            state.current_time = as_of;
            state
        }

        let as_of = Local::now().naive_local();
        let mut inserted = 0usize;

        for (machine_code, hourly_capacity_t) in machines {
            // Query machine plan overrides
            let plan_row: Option<(String, Option<String>, Option<i32>)> = tx
                .query_row(
                    r#"
                    SELECT initial_start_at, next_change_at, downtime_minutes
                    FROM roll_campaign_plan
                    WHERE version_id = ?1 AND machine_code = ?2
                    "#,
                    rusqlite::params![&scope.version_id, &machine_code],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?;

            // Plan items for this machine
            let order_clause = if has_seq_no {
                "ORDER BY plan_date ASC, seq_no ASC"
            } else {
                "ORDER BY plan_date ASC"
            };
            let pi_sql = format!(
                "SELECT plan_date, weight_t FROM plan_item WHERE version_id = ?1 AND machine_code = ?2 {}",
                order_clause
            );
            let mut pi_stmt = tx.prepare(&pi_sql)?;
            let pi_iter = pi_stmt.query_map(
                rusqlite::params![&scope.version_id, &machine_code],
                |row| {
                    let plan_date: String = row.get(0)?;
                    let weight_t: f64 = row.get(1)?;
                    let start_at = ymd_to_start_at(&plan_date).unwrap_or_else(|| as_of);
                    Ok(PlanItemLite {
                        earliest_start_at: start_at,
                        weight_t,
                    })
                },
            )?;

            let mut items: Vec<PlanItemLite> = pi_iter.collect::<Result<Vec<_>, _>>()?;
            items.retain(|i| i.weight_t > 0.0);
            items.sort_by_key(|i| i.earliest_start_at);

            // Default initial_start_at from earliest plan_date
            let default_start_at = items
                .first()
                .map(|i| i.earliest_start_at)
                .or_else(|| as_of.date().and_hms_opt(0, 0, 0))
                .unwrap_or(as_of);

            let initial_start_at = plan_row
                .as_ref()
                .and_then(|(s, _, _)| parse_dt_best_effort(s))
                .unwrap_or(default_start_at);

            let override_next_change_at = plan_row
                .as_ref()
                .and_then(|(_, v, _)| v.as_deref())
                .and_then(parse_dt_best_effort);

            let override_downtime_minutes = plan_row.and_then(|(_, _, m)| m).filter(|v| *v > 0);
            let planned_downtime_minutes = override_downtime_minutes.unwrap_or(default_downtime_minutes);

            let rate_t_per_sec = if hourly_capacity_t > 0.0 {
                hourly_capacity_t / 3600.0
            } else {
                0.0
            };

            // If no capacity (rate<=0), we cannot estimate timestamps. Fallback to a simple tonnage
            // aggregation (by plan_date) to keep D5 usable for legacy/test DBs.
            let state_at_as_of = if rate_t_per_sec > 0.0 && !items.is_empty() {
                simulate_to_as_of(
                    &items,
                    rate_t_per_sec,
                    initial_start_at,
                    suggest_threshold_t,
                    planned_downtime_minutes as i64,
                    as_of,
                )
            } else {
                let cum_weight_t: f64 = items
                    .iter()
                    .filter(|i| i.earliest_start_at >= initial_start_at && i.earliest_start_at <= as_of)
                    .map(|i| i.weight_t)
                    .sum();
                StreamState {
                    item_index: 0,
                    remaining_weight_t: 0.0,
                    current_time: as_of,
                    campaign_no: 1,
                    campaign_start_at: initial_start_at,
                    cum_weight_t,
                }
            };

            let (soft_reach, hard_reach) = if rate_t_per_sec > 0.0 && !items.is_empty() {
                // Estimate reach times from as_of (no further auto resets)
                let soft_need = (suggest_threshold_t - state_at_as_of.cum_weight_t).max(0.0);
                let hard_need = (hard_limit_t - state_at_as_of.cum_weight_t).max(0.0);

                let base_state_for_future = StreamState {
                    item_index: state_at_as_of.item_index,
                    remaining_weight_t: state_at_as_of.remaining_weight_t,
                    current_time: state_at_as_of.current_time,
                    campaign_no: state_at_as_of.campaign_no,
                    campaign_start_at: state_at_as_of.campaign_start_at,
                    cum_weight_t: state_at_as_of.cum_weight_t,
                };

                let soft_reach = if suggest_threshold_t > 0.0 {
                    produce_weight_until(&items, base_state_for_future.clone(), rate_t_per_sec, soft_need)
                        .map(|s| s.current_time)
                } else {
                    None
                };
                let hard_reach = if hard_limit_t > 0.0 {
                    produce_weight_until(&items, base_state_for_future.clone(), rate_t_per_sec, hard_need)
                        .map(|s| s.current_time)
                } else {
                    None
                };
                (soft_reach, hard_reach)
            } else {
                (None, None)
            };

            let planned_change_at = override_next_change_at
                .filter(|dt| *dt > as_of && *dt >= state_at_as_of.campaign_start_at)
                .or(soft_reach);

            let will_exceed_soft_before_change = match (soft_reach, planned_change_at) {
                (Some(s), Some(p)) => s < p,
                _ => false,
            };

            let will_hard_stop_before_change = match (hard_reach, planned_change_at) {
                (Some(h), Some(p)) => h <= p,
                _ => false,
            };

            let utilization_rate = if suggest_threshold_t > 0.0 {
                state_at_as_of.cum_weight_t / suggest_threshold_t
            } else {
                0.0
            };

            let mut alert_level = "NONE".to_string();
            let mut reason = "换辊状态正常".to_string();

            if will_hard_stop_before_change {
                alert_level = "EMERGENCY".to_string();
                reason = "计划换辊时间晚于预计硬限制触达，存在硬停止风险".to_string();
            } else if state_at_as_of.cum_weight_t >= hard_limit_t {
                alert_level = "EMERGENCY".to_string();
                reason = format!(
                    "已超过硬限制 {:.1} 吨，必须立即换辊",
                    hard_limit_t
                );
            } else if will_exceed_soft_before_change || state_at_as_of.cum_weight_t >= suggest_threshold_t {
                alert_level = "CRITICAL".to_string();
                reason = format!("已达到/超过建议阈值 {:.1} 吨", suggest_threshold_t);
            } else if utilization_rate >= 0.95 {
                alert_level = "CRITICAL".to_string();
                reason = format!(
                    "接近建议阈值 ({:.1}%)，建议尽快安排换辊",
                    utilization_rate * 100.0
                );
            } else if utilization_rate >= 0.85 {
                alert_level = "WARNING".to_string();
                reason = format!(
                    "接近建议阈值 ({:.1}%)，请关注",
                    utilization_rate * 100.0
                );
            }

            let needs_immediate_change = matches!(alert_level.as_str(), "EMERGENCY")
                || utilization_rate >= 0.95
                || will_hard_stop_before_change;

            let suggested_actions = if will_hard_stop_before_change {
                r#"["调整计划换辊时间（避免硬停止）","考虑提前换辊或增加停机时长"]"#.to_string()
            } else if matches!(alert_level.as_str(), "EMERGENCY") {
                r#"["立即换辊"]"#.to_string()
            } else if matches!(alert_level.as_str(), "CRITICAL") {
                r#"["尽快安排换辊（优先在计划停机）"]"#.to_string()
            } else if matches!(alert_level.as_str(), "WARNING") {
                r#"["关注换辊时间与阈值触达"]"#.to_string()
            } else {
                "[]".to_string()
            };

            let estimated_change_date = hard_reach
                .map(|dt| dt.date().format("%Y-%m-%d").to_string());

            // Insert one row per machine (current campaign as_of).
            tx.execute(
                r#"
                INSERT INTO decision_roll_campaign_alert (
                    version_id,
                    machine_code,
                    campaign_no,
                    cum_weight_t,
                    suggest_threshold_t,
                    hard_limit_t,
                    alert_level,
                    reason,
                    distance_to_suggest,
                    distance_to_hard,
                    utilization_rate,
                    estimated_change_date,
                    needs_immediate_change,
                    suggested_actions,
                    campaign_start_at,
                    planned_change_at,
                    planned_downtime_minutes,
                    estimated_soft_reach_at,
                    estimated_hard_reach_at,
                    refreshed_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, datetime('now')
                )
                "#,
                rusqlite::params![
                    &scope.version_id,
                    &machine_code,
                    state_at_as_of.campaign_no,
                    state_at_as_of.cum_weight_t,
                    suggest_threshold_t,
                    hard_limit_t,
                    alert_level,
                    reason,
                    suggest_threshold_t - state_at_as_of.cum_weight_t,
                    hard_limit_t - state_at_as_of.cum_weight_t,
                    utilization_rate,
                    estimated_change_date,
                    if needs_immediate_change { 1 } else { 0 },
                    suggested_actions,
                    state_at_as_of.campaign_start_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                    planned_change_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
                    planned_downtime_minutes,
                    soft_reach.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
                    hard_reach.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
                ],
            )?;

            inserted += 1;
        }

        Ok(inserted)
    }

    /// 刷新 D6: 是否存在产能优化空间
    fn refresh_d6(
        &self,
        tx: &Transaction,
        scope: &RefreshScope,
    ) -> Result<usize, Box<dyn Error>> {
        // 构建删除条件和插入条件
        let mut delete_conditions = vec!["version_id = ?1".to_string()];
        let mut insert_conditions: Vec<String> = Vec::new();
        let mut params: Vec<String> = vec![scope.version_id.clone()];

        if let Some(machines) = &scope.affected_machines {
            let placeholders: Vec<String> = (0..machines.len())
                .map(|i| format!("?{}", i + 2))
                .collect();
            delete_conditions.push(format!("machine_code IN ({})", placeholders.join(", ")));
            insert_conditions.push(format!("cp.machine_code IN ({})", placeholders.join(", ")));
            params.extend(machines.clone());
        }

        if let Some((start_date, end_date)) = &scope.affected_date_range {
            let start_idx = params.len() + 1;
            let end_idx = params.len() + 2;
            delete_conditions.push(format!("plan_date BETWEEN ?{} AND ?{}", start_idx, end_idx));
            insert_conditions.push(format!("cp.plan_date BETWEEN ?{} AND ?{}", start_idx, end_idx));
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
        insert_conditions.push("cp.used_capacity_t / NULLIF(cp.target_capacity_t, 0) < 0.9".to_string());
        let has_cp_version_id: i32 = tx.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('capacity_pool') WHERE name = 'version_id'",
            [],
            |row| row.get(0),
        )?;
        if has_cp_version_id > 0 {
            insert_conditions.push("cp.version_id = ?1".to_string());
        }
        let insert_where_clause = format!("\n            WHERE {}", insert_conditions.join(" AND "));

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

        let rows_affected = tx.execute(&insert_sql, rusqlite::params_from_iter(params_refs.iter()))?;

        Ok(rows_affected)
    }

    /// 记录刷新开始
    fn log_refresh_start(
        &self,
        tx: &Transaction,
        refresh_id: &str,
        version_id: &str,
        trigger: &RefreshTrigger,
        trigger_source: Option<&str>,
        is_full_refresh: bool,
        started_at: &str,
    ) -> Result<(), Box<dyn Error>> {
        tx.execute(
            r#"
            INSERT INTO decision_refresh_log (
                refresh_id,
                version_id,
                trigger_type,
                trigger_source,
                is_full_refresh,
                refreshed_tables,
                rows_affected,
                started_at,
                status
            ) VALUES (?1, ?2, ?3, ?4, ?5, '[]', 0, ?6, 'RUNNING')
            "#,
            rusqlite::params![
                refresh_id,
                version_id,
                trigger.as_str(),
                trigger_source,
                if is_full_refresh { 1 } else { 0 },
                started_at,
            ],
        )?;
        Ok(())
    }

    /// 记录刷新完成
    fn log_refresh_complete(
        &self,
        tx: &Transaction,
        refresh_id: &str,
        refreshed_tables: &[String],
        rows_affected: usize,
        completed_at: &str,
        duration_ms: i64,
    ) -> Result<(), Box<dyn Error>> {
        let tables_json = serde_json::to_string(refreshed_tables)?;

        tx.execute(
            r#"
            UPDATE decision_refresh_log
            SET refreshed_tables = ?2,
                rows_affected = ?3,
                completed_at = ?4,
                duration_ms = ?5,
                status = 'SUCCESS'
            WHERE refresh_id = ?1
            "#,
            rusqlite::params![refresh_id, tables_json, rows_affected as i64, completed_at, duration_ms],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();

        // 创建必要的表
        conn.execute_batch(
            r#"
            CREATE TABLE plan_version (
                version_id TEXT PRIMARY KEY
            );

            CREATE TABLE machine_master (
                machine_code TEXT PRIMARY KEY
            );

            CREATE TABLE risk_snapshot (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                snapshot_date TEXT NOT NULL,
                risk_level TEXT NOT NULL,
                risk_reasons TEXT,
                target_capacity_t REAL NOT NULL,
                used_capacity_t REAL NOT NULL,
                limit_capacity_t REAL NOT NULL,
                overflow_t REAL NOT NULL,
                urgent_total_t REAL NOT NULL,
                mature_backlog_t REAL,
                immature_backlog_t REAL,
                campaign_status TEXT,
                created_at TEXT NOT NULL,
                PRIMARY KEY (version_id, machine_code, snapshot_date)
            );

            CREATE TABLE capacity_pool (
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                target_capacity_t REAL NOT NULL,
                limit_capacity_t REAL NOT NULL,
                used_capacity_t REAL NOT NULL DEFAULT 0.0,
                overflow_t REAL NOT NULL DEFAULT 0.0,
                PRIMARY KEY (machine_code, plan_date)
            );

            CREATE TABLE plan_item (
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
            );

            CREATE TABLE decision_day_summary (
                version_id TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                risk_score REAL NOT NULL,
                risk_level TEXT NOT NULL,
                capacity_util_pct REAL NOT NULL,
                top_reasons TEXT NOT NULL,
                affected_machines INTEGER NOT NULL,
                bottleneck_machines INTEGER NOT NULL,
                has_roll_risk INTEGER NOT NULL DEFAULT 0,
                suggested_actions TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, plan_date)
            );

            CREATE TABLE decision_machine_bottleneck (
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
                suggested_actions TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, machine_code, plan_date)
            );

            CREATE TABLE decision_refresh_log (
                refresh_id TEXT PRIMARY KEY,
                version_id TEXT NOT NULL,
                trigger_type TEXT NOT NULL,
                trigger_source TEXT,
                is_full_refresh INTEGER NOT NULL DEFAULT 0,
                affected_machines TEXT,
                affected_date_range TEXT,
                refreshed_tables TEXT NOT NULL,
                rows_affected INTEGER NOT NULL DEFAULT 0,
                started_at TEXT NOT NULL DEFAULT (datetime('now')),
                completed_at TEXT,
                duration_ms INTEGER,
                status TEXT NOT NULL DEFAULT 'RUNNING',
                error_message TEXT
            );

            -- D2: 订单失败表
            CREATE TABLE decision_order_failure_set (
                version_id TEXT NOT NULL,
                contract_no TEXT NOT NULL,
                due_date TEXT NOT NULL,
                urgency_level TEXT NOT NULL,
                fail_type TEXT NOT NULL,
                total_materials INTEGER NOT NULL,
                unscheduled_count INTEGER NOT NULL,
                unscheduled_weight_t REAL NOT NULL,
                completion_rate REAL NOT NULL,
                days_to_due INTEGER NOT NULL,
                failure_reasons TEXT NOT NULL,
                blocking_factors TEXT NOT NULL,
                suggested_actions TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, contract_no)
            );

            -- D3: 冷料压库表
            CREATE TABLE decision_cold_stock_profile (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                age_bin TEXT NOT NULL,
                age_min_days INTEGER NOT NULL,
                age_max_days INTEGER NOT NULL,
                count INTEGER NOT NULL,
                weight_t REAL NOT NULL,
                avg_age_days REAL NOT NULL,
                pressure_score REAL NOT NULL,
                pressure_level TEXT NOT NULL,
                reasons TEXT NOT NULL,
                structure_gap TEXT,
                estimated_ready_date TEXT,
                can_force_release INTEGER NOT NULL DEFAULT 0,
                suggested_actions TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, machine_code, age_bin)
            );

            -- D5: 换辊时间监控表（与真实 schema 对齐）
            CREATE TABLE decision_roll_campaign_alert (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                campaign_no INTEGER NOT NULL,
                cum_weight_t REAL NOT NULL,
                suggest_threshold_t REAL NOT NULL,
                hard_limit_t REAL NOT NULL,
                alert_level TEXT NOT NULL,
                reason TEXT,
                distance_to_suggest REAL NOT NULL,
                distance_to_hard REAL NOT NULL,
                utilization_rate REAL NOT NULL,
                estimated_change_date TEXT,
                needs_immediate_change INTEGER NOT NULL DEFAULT 0,
                suggested_actions TEXT,
                campaign_start_at TEXT,
                planned_change_at TEXT,
                planned_downtime_minutes INTEGER,
                estimated_soft_reach_at TEXT,
                estimated_hard_reach_at TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, machine_code, campaign_no)
            );

            -- D6: 产能优化机会表
            CREATE TABLE decision_capacity_opportunity (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                slack_t REAL NOT NULL,
                soft_adjust_space_t REAL,
                utilization_rate REAL NOT NULL,
                binding_constraints TEXT,
                opportunity_level TEXT NOT NULL,
                sensitivity TEXT,
                suggested_optimizations TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, machine_code, plan_date)
            );

            -- 创建必要的源表
            CREATE TABLE material_state (
                material_id TEXT PRIMARY KEY,
                age_days INTEGER NOT NULL,
                weight_t REAL NOT NULL
            );

            CREATE TABLE roll_campaign (
                campaign_id TEXT PRIMARY KEY,
                machine_code TEXT NOT NULL,
                cum_weight_t REAL NOT NULL
            );

            INSERT INTO plan_version VALUES ('V001');
            INSERT INTO machine_master VALUES ('H032');

            INSERT INTO risk_snapshot VALUES (
                'V001', 'H032', '2026-01-24', 'HIGH', '产能紧张',
                1500.0, 1450.0, 2000.0, 0.0, 800.0, 500.0, 200.0, 'OK',
                datetime('now')
            );

            INSERT INTO capacity_pool VALUES (
                'H032', '2026-01-24', 1500.0, 2000.0, 1450.0, 0.0
            );

            INSERT INTO plan_item VALUES (
                'V001', 'MAT001', 'H032', '2026-01-24', 1, 100.0, 'AUTO', 0, 0, ''
            );
            "#,
        )
        .unwrap();

        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_refresh_all_with_d1_and_d4() {
        let conn = setup_test_db();
        let service = DecisionRefreshService::new(conn.clone());

        let scope = RefreshScope {
            version_id: "V001".to_string(),
            is_full_refresh: true,
            affected_machines: None,
            affected_date_range: None,
        };

        let refresh_id = service
            .refresh_all(scope, RefreshTrigger::ManualRefresh, Some("test".to_string()))
            .unwrap();

        assert!(!refresh_id.is_empty());

        // 验证 D1 数据
        let c = conn.lock().unwrap();
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_day_summary WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // 验证 D4 数据
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_machine_bottleneck WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // 验证刷新日志
        let status: String = c
            .query_row(
                "SELECT status FROM decision_refresh_log WHERE refresh_id = ?1",
                [&refresh_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "SUCCESS");
    }

    #[test]
    fn test_should_refresh_d2() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let service = DecisionRefreshService::new(conn);

        assert!(service.should_refresh_d2(&RefreshTrigger::PlanItemChanged));
        assert!(service.should_refresh_d2(&RefreshTrigger::MaterialStateChanged));
        assert!(service.should_refresh_d2(&RefreshTrigger::RiskSnapshotUpdated));
        assert!(service.should_refresh_d2(&RefreshTrigger::VersionCreated));
        assert!(service.should_refresh_d2(&RefreshTrigger::ManualRefresh));
        assert!(!service.should_refresh_d2(&RefreshTrigger::CapacityPoolChanged));
    }

    #[test]
    fn test_should_refresh_d3() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let service = DecisionRefreshService::new(conn);

        assert!(service.should_refresh_d3(&RefreshTrigger::MaterialStateChanged));
        assert!(service.should_refresh_d3(&RefreshTrigger::PlanItemChanged));
        assert!(service.should_refresh_d3(&RefreshTrigger::VersionCreated));
        assert!(service.should_refresh_d3(&RefreshTrigger::ManualRefresh));
        assert!(!service.should_refresh_d3(&RefreshTrigger::RollCampaignChanged));
    }

    #[test]
    fn test_should_refresh_d5() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let service = DecisionRefreshService::new(conn);

        assert!(service.should_refresh_d5(&RefreshTrigger::RollCampaignChanged));
        assert!(service.should_refresh_d5(&RefreshTrigger::MaterialStateChanged));
        assert!(service.should_refresh_d5(&RefreshTrigger::PlanItemChanged));
        assert!(service.should_refresh_d5(&RefreshTrigger::VersionCreated));
        assert!(service.should_refresh_d5(&RefreshTrigger::ManualRefresh));
        assert!(!service.should_refresh_d5(&RefreshTrigger::CapacityPoolChanged));
    }

    #[test]
    fn test_should_refresh_d6() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let service = DecisionRefreshService::new(conn);

        assert!(service.should_refresh_d6(&RefreshTrigger::CapacityPoolChanged));
        assert!(service.should_refresh_d6(&RefreshTrigger::PlanItemChanged));
        assert!(service.should_refresh_d6(&RefreshTrigger::MaterialStateChanged));
        assert!(service.should_refresh_d6(&RefreshTrigger::VersionCreated));
        assert!(service.should_refresh_d6(&RefreshTrigger::ManualRefresh));
        assert!(!service.should_refresh_d6(&RefreshTrigger::RollCampaignChanged));
    }

    #[test]
    fn test_incremental_refresh_d1_by_date_range() {
        let conn = setup_test_db();
        let c = conn.lock().unwrap();

        // 添加多个日期的数据
        c.execute(
            r#"
            INSERT INTO risk_snapshot VALUES (
                'V001', 'H032', '2026-01-25', 'MEDIUM', '产能正常',
                1500.0, 1200.0, 2000.0, 0.0, 600.0, 400.0, 150.0, 'OK',
                datetime('now')
            )
            "#,
            [],
        )
        .unwrap();
        drop(c);

        let service = DecisionRefreshService::new(conn.clone());

        // 全量刷新
        let scope = RefreshScope {
            version_id: "V001".to_string(),
            is_full_refresh: true,
            affected_machines: None,
            affected_date_range: None,
        };
        service
            .refresh_all(scope, RefreshTrigger::ManualRefresh, Some("test".to_string()))
            .unwrap();

        // 验证有 2 个日期的数据
        let c = conn.lock().unwrap();
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_day_summary WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
        drop(c);

        // 增量刷新：只刷新 2026-01-25
        let scope = RefreshScope {
            version_id: "V001".to_string(),
            is_full_refresh: false,
            affected_machines: None,
            affected_date_range: Some(("2026-01-25".to_string(), "2026-01-25".to_string())),
        };
        service
            .refresh_all(scope, RefreshTrigger::RiskSnapshotUpdated, Some("test".to_string()))
            .unwrap();

        // 验证仍然有 2 个日期的数据
        let c = conn.lock().unwrap();
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_day_summary WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);

        // 验证 2026-01-25 的数据已更新（refreshed_at 应该更新）
        let risk_score: f64 = c
            .query_row(
                "SELECT risk_score FROM decision_day_summary WHERE version_id = 'V001' AND plan_date = '2026-01-25'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(risk_score > 0.0);
    }

    #[test]
    fn test_incremental_refresh_d4_by_machine_and_date() {
        let conn = setup_test_db();
        let c = conn.lock().unwrap();

        // 添加另一个机组和日期的数据
        c.execute_batch(
            r#"
            INSERT INTO machine_master VALUES ('H033');
            INSERT INTO capacity_pool VALUES (
                'H033', '2026-01-24', 1600.0, 2100.0, 1550.0, 0.0
            );
            INSERT INTO capacity_pool VALUES (
                'H032', '2026-01-25', 1500.0, 2000.0, 1300.0, 0.0
            );
            "#,
        )
        .unwrap();
        drop(c);

        let service = DecisionRefreshService::new(conn.clone());

        // 全量刷新
        let scope = RefreshScope {
            version_id: "V001".to_string(),
            is_full_refresh: true,
            affected_machines: None,
            affected_date_range: None,
        };
        service
            .refresh_all(scope, RefreshTrigger::ManualRefresh, Some("test".to_string()))
            .unwrap();

        // 验证有 3 条记录（H032-01-24, H033-01-24, H032-01-25）
        let c = conn.lock().unwrap();
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_machine_bottleneck WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);
        drop(c);

        // 增量刷新：只刷新 H032 机组的 2026-01-25
        let scope = RefreshScope {
            version_id: "V001".to_string(),
            is_full_refresh: false,
            affected_machines: Some(vec!["H032".to_string()]),
            affected_date_range: Some(("2026-01-25".to_string(), "2026-01-25".to_string())),
        };
        service
            .refresh_all(scope, RefreshTrigger::CapacityPoolChanged, Some("test".to_string()))
            .unwrap();

        // 验证仍然有 3 条记录
        let c = conn.lock().unwrap();
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM decision_machine_bottleneck WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);

        // 验证 H032-2026-01-25 的数据存在
        let bottleneck_score: f64 = c
            .query_row(
                "SELECT bottleneck_score FROM decision_machine_bottleneck WHERE version_id = 'V001' AND machine_code = 'H032' AND plan_date = '2026-01-25'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(bottleneck_score > 0.0);
    }
}

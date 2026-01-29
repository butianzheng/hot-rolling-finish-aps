// ==========================================
// 热轧精整排产系统 - 决策刷新队列管理器
// ==========================================
// 依据: REFACTOR_PLAN_v1.0.md - P2 阶段
// 职责: 管理决策视图刷新任务队列，避免并发冲突
// ==========================================

use super::{DecisionRefreshService, RefreshScope, RefreshTrigger};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension};
use std::error::Error;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// 刷新任务状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshStatus {
    /// 等待中
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

impl RefreshStatus {
    pub fn as_str(&self) -> &str {
        match self {
            RefreshStatus::Pending => "PENDING",
            RefreshStatus::Running => "RUNNING",
            RefreshStatus::Completed => "COMPLETED",
            RefreshStatus::Failed => "FAILED",
            RefreshStatus::Cancelled => "CANCELLED",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "PENDING" => RefreshStatus::Pending,
            "RUNNING" => RefreshStatus::Running,
            "COMPLETED" => RefreshStatus::Completed,
            "FAILED" => RefreshStatus::Failed,
            "CANCELLED" => RefreshStatus::Cancelled,
            _ => RefreshStatus::Failed,
        }
    }
}

/// 刷新任务
#[derive(Debug, Clone)]
pub struct RefreshTask {
    /// 任务 ID
    pub task_id: String,
    /// 刷新范围
    pub scope: RefreshScope,
    /// 触发类型
    pub trigger: RefreshTrigger,
    /// 触发源
    pub trigger_source: Option<String>,
    /// 任务状态
    pub status: RefreshStatus,
    /// 重试次数
    pub retry_count: i32,
    /// 最大重试次数
    pub max_retries: i32,
    /// 创建时间
    pub created_at: String,
    /// 开始执行时间
    pub started_at: Option<String>,
    /// 完成时间
    pub completed_at: Option<String>,
    /// 错误信息
    pub error_message: Option<String>,
}

impl RefreshTask {
    /// 创建新的刷新任务
    pub fn new(
        scope: RefreshScope,
        trigger: RefreshTrigger,
        trigger_source: Option<String>,
        max_retries: i32,
    ) -> Self {
        Self {
            task_id: Uuid::new_v4().to_string(),
            scope,
            trigger,
            trigger_source,
            status: RefreshStatus::Pending,
            retry_count: 0,
            max_retries,
            created_at: Utc::now().to_rfc3339(),
            started_at: None,
            completed_at: None,
            error_message: None,
        }
    }

    /// 是否可以重试
    pub fn can_retry(&self) -> bool {
        self.status == RefreshStatus::Failed && self.retry_count < self.max_retries
    }
}

/// 刷新队列管理器
pub struct RefreshQueue {
    conn: Arc<Mutex<Connection>>,
    refresh_service: Arc<DecisionRefreshService>,
}

impl RefreshQueue {
    /// 创建新的刷新队列管理器
    pub fn new(
        conn: Arc<Mutex<Connection>>,
        refresh_service: Arc<DecisionRefreshService>,
    ) -> Result<Self, Box<dyn Error>> {
        let queue = Self {
            conn,
            refresh_service,
        };

        // 确保刷新任务队列表存在
        queue.ensure_queue_table()?;

        Ok(queue)
    }

    /// 确保刷新任务队列表存在
    fn ensure_queue_table(&self) -> Result<(), Box<dyn Error>> {
        let conn = self.conn.lock()
            .map_err(|e| format!("锁获取失败: {}", e))?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS decision_refresh_queue (
                task_id TEXT PRIMARY KEY,
                version_id TEXT NOT NULL,
                trigger_type TEXT NOT NULL,
                trigger_source TEXT,
                is_full_refresh INTEGER NOT NULL DEFAULT 0,
                affected_machines TEXT,
                affected_date_range TEXT,
                status TEXT NOT NULL DEFAULT 'PENDING',
                retry_count INTEGER NOT NULL DEFAULT 0,
                max_retries INTEGER NOT NULL DEFAULT 3,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                started_at TEXT,
                completed_at TEXT,
                error_message TEXT,
                refresh_id TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_refresh_queue_status
              ON decision_refresh_queue(status, created_at);

            CREATE INDEX IF NOT EXISTS idx_refresh_queue_version
              ON decision_refresh_queue(version_id, status);
            "#,
        )?;
        Ok(())
    }

    /// 提交刷新任务到队列
    pub fn enqueue(&self, task: RefreshTask) -> Result<String, Box<dyn Error>> {
        let conn = self.conn.lock()
            .map_err(|e| format!("锁获取失败: {}", e))?;

        // 检查是否已有相同版本的运行中任务
        let running_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM decision_refresh_queue WHERE version_id = ?1 AND status IN ('PENDING', 'RUNNING')",
            [&task.scope.version_id],
            |row| row.get(0),
        )?;

        if running_count > 0 {
            tracing::warn!(
                "版本 {} 已有运行中的刷新任务，新任务将排队等待",
                task.scope.version_id
            );
        }

        // 序列化 affected_machines 和 affected_date_range
        let affected_machines_json = if let Some(machines) = &task.scope.affected_machines {
            Some(serde_json::to_string(machines)?)
        } else {
            None
        };

        let affected_date_range_json = if let Some((start, end)) = &task.scope.affected_date_range
        {
            Some(serde_json::to_string(&(start, end))?)
        } else {
            None
        };

        conn.execute(
            r#"
            INSERT INTO decision_refresh_queue (
                task_id,
                version_id,
                trigger_type,
                trigger_source,
                is_full_refresh,
                affected_machines,
                affected_date_range,
                status,
                retry_count,
                max_retries,
                created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            rusqlite::params![
                task.task_id,
                task.scope.version_id,
                task.trigger.as_str(),
                task.trigger_source,
                if task.scope.is_full_refresh { 1 } else { 0 },
                affected_machines_json,
                affected_date_range_json,
                task.status.as_str(),
                task.retry_count,
                task.max_retries,
                task.created_at,
            ],
        )?;

        tracing::info!("刷新任务已加入队列: task_id={}", task.task_id);

        Ok(task.task_id)
    }

    /// 获取下一个待执行任务
    pub fn dequeue(&self) -> Result<Option<RefreshTask>, Box<dyn Error>> {
        let conn = self.conn.lock()
            .map_err(|e| format!("锁获取失败: {}", e))?;

        // 查找最早的待执行任务（状态为 PENDING）
        let task_opt = conn
            .query_row(
                r#"
                SELECT
                    task_id,
                    version_id,
                    trigger_type,
                    trigger_source,
                    is_full_refresh,
                    affected_machines,
                    affected_date_range,
                    status,
                    retry_count,
                    max_retries,
                    created_at,
                    started_at,
                    completed_at,
                    error_message
                FROM decision_refresh_queue
                WHERE status = 'PENDING'
                ORDER BY created_at ASC
                LIMIT 1
                "#,
                [],
                |row| {
                    let version_id: String = row.get(1)?;
                    let is_full_refresh: i32 = row.get(4)?;
                    let affected_machines_json: Option<String> = row.get(5)?;
                    let affected_date_range_json: Option<String> = row.get(6)?;

                    let affected_machines = if let Some(json) = affected_machines_json {
                        serde_json::from_str(&json).ok()
                    } else {
                        None
                    };

                    let affected_date_range = if let Some(json) = affected_date_range_json {
                        serde_json::from_str(&json).ok()
                    } else {
                        None
                    };

                    let scope = RefreshScope {
                        version_id,
                        is_full_refresh: is_full_refresh != 0,
                        affected_machines,
                        affected_date_range,
                    };

                    let trigger_str: String = row.get(2)?;
                    let trigger = match trigger_str.as_str() {
                        "PlanItemChanged" => RefreshTrigger::PlanItemChanged,
                        "RiskSnapshotUpdated" => RefreshTrigger::RiskSnapshotUpdated,
                        "MaterialStateChanged" => RefreshTrigger::MaterialStateChanged,
                        "CapacityPoolChanged" => RefreshTrigger::CapacityPoolChanged,
                        "RollCampaignChanged" => RefreshTrigger::RollCampaignChanged,
                        "VersionCreated" => RefreshTrigger::VersionCreated,
                        _ => RefreshTrigger::ManualRefresh,
                    };

                    let status_str: String = row.get(7)?;
                    let status = RefreshStatus::from_str(&status_str);

                    Ok(RefreshTask {
                        task_id: row.get(0)?,
                        scope,
                        trigger,
                        trigger_source: row.get(3)?,
                        status,
                        retry_count: row.get(8)?,
                        max_retries: row.get(9)?,
                        created_at: row.get(10)?,
                        started_at: row.get(11)?,
                        completed_at: row.get(12)?,
                        error_message: row.get(13)?,
                    })
                },
            )
            .optional()?;

        if let Some(task) = task_opt {
            // 更新任务状态为 RUNNING
            conn.execute(
                "UPDATE decision_refresh_queue SET status = 'RUNNING', started_at = ?1 WHERE task_id = ?2",
                rusqlite::params![Utc::now().to_rfc3339(), task.task_id],
            )?;

            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    /// 执行刷新任务
    pub fn execute_task(&self, mut task: RefreshTask) -> Result<String, Box<dyn Error>> {
        tracing::info!("开始执行刷新任务: task_id={}", task.task_id);

        // 调用 DecisionRefreshService 执行刷新
        match self.refresh_service.refresh_all(
            task.scope.clone(),
            task.trigger.clone(),
            task.trigger_source.clone(),
        ) {
            Ok(refresh_id) => {
                // 刷新成功
                task.status = RefreshStatus::Completed;
                task.completed_at = Some(Utc::now().to_rfc3339());

                let conn = self.conn.lock()
                    .map_err(|e| format!("锁获取失败: {}", e))?;
                conn.execute(
                    "UPDATE decision_refresh_queue SET status = 'COMPLETED', completed_at = ?1, refresh_id = ?2 WHERE task_id = ?3",
                    rusqlite::params![task.completed_at, refresh_id, task.task_id],
                )?;

                tracing::info!(
                    "刷新任务执行成功: task_id={}, refresh_id={}",
                    task.task_id,
                    refresh_id
                );

                Ok(refresh_id)
            }
            Err(e) => {
                // 刷新失败
                task.status = RefreshStatus::Failed;
                task.error_message = Some(e.to_string());
                task.retry_count += 1;

                let conn = self.conn.lock()
                    .map_err(|e| format!("锁获取失败: {}", e))?;

                // 如果可以重试，更新为 PENDING 状态
                if task.can_retry() {
                    conn.execute(
                        "UPDATE decision_refresh_queue SET status = 'PENDING', error_message = ?1, retry_count = ?2 WHERE task_id = ?3",
                        rusqlite::params![task.error_message, task.retry_count, task.task_id],
                    )?;

                    tracing::info!(
                        "刷新任务将重试: task_id={}, retry_count={}",
                        task.task_id,
                        task.retry_count
                    );
                } else {
                    // 不能重试，更新为 FAILED 状态
                    conn.execute(
                        "UPDATE decision_refresh_queue SET status = 'FAILED', error_message = ?1, retry_count = ?2 WHERE task_id = ?3",
                        rusqlite::params![task.error_message, task.retry_count, task.task_id],
                    )?;

                    tracing::error!(
                        "刷新任务执行失败，达到最大重试次数: task_id={}, retry_count={}",
                        task.task_id,
                        task.retry_count
                    );
                }

                tracing::error!(
                    "刷新任务执行失败: task_id={}, error={}, retry_count={}",
                    task.task_id,
                    e,
                    task.retry_count
                );

                Err(e)
            }
        }
    }

    /// 处理队列中的下一个任务
    pub fn process_next(&self) -> Result<Option<String>, Box<dyn Error>> {
        if let Some(task) = self.dequeue()? {
            match self.execute_task(task) {
                Ok(refresh_id) => Ok(Some(refresh_id)),
                Err(e) => Err(e),
            }
        } else {
            Ok(None)
        }
    }

    /// 处理队列中所有待执行任务
    pub fn process_all(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut refresh_ids = Vec::new();

        loop {
            match self.process_next() {
                Ok(Some(refresh_id)) => {
                    refresh_ids.push(refresh_id);
                }
                Ok(None) => {
                    // 队列为空
                    break;
                }
                Err(e) => {
                    // 任务执行失败，继续处理下一个
                    tracing::error!("处理刷新任务失败: {}", e);
                }
            }
        }

        Ok(refresh_ids)
    }

    /// 获取任务状态
    pub fn get_task_status(&self, task_id: &str) -> Result<Option<RefreshTask>, Box<dyn Error>> {
        let conn = self.conn.lock()
            .map_err(|e| format!("锁获取失败: {}", e))?;

        let task_opt = conn
            .query_row(
                r#"
                SELECT
                    task_id,
                    version_id,
                    trigger_type,
                    trigger_source,
                    is_full_refresh,
                    affected_machines,
                    affected_date_range,
                    status,
                    retry_count,
                    max_retries,
                    created_at,
                    started_at,
                    completed_at,
                    error_message
                FROM decision_refresh_queue
                WHERE task_id = ?1
                "#,
                [task_id],
                |row| {
                    let version_id: String = row.get(1)?;
                    let is_full_refresh: i32 = row.get(4)?;
                    let affected_machines_json: Option<String> = row.get(5)?;
                    let affected_date_range_json: Option<String> = row.get(6)?;

                    let affected_machines = if let Some(json) = affected_machines_json {
                        serde_json::from_str(&json).ok()
                    } else {
                        None
                    };

                    let affected_date_range = if let Some(json) = affected_date_range_json {
                        serde_json::from_str(&json).ok()
                    } else {
                        None
                    };

                    let scope = RefreshScope {
                        version_id,
                        is_full_refresh: is_full_refresh != 0,
                        affected_machines,
                        affected_date_range,
                    };

                    let trigger_str: String = row.get(2)?;
                    let trigger = match trigger_str.as_str() {
                        "PlanItemChanged" => RefreshTrigger::PlanItemChanged,
                        "RiskSnapshotUpdated" => RefreshTrigger::RiskSnapshotUpdated,
                        "MaterialStateChanged" => RefreshTrigger::MaterialStateChanged,
                        "CapacityPoolChanged" => RefreshTrigger::CapacityPoolChanged,
                        "RollCampaignChanged" => RefreshTrigger::RollCampaignChanged,
                        "VersionCreated" => RefreshTrigger::VersionCreated,
                        _ => RefreshTrigger::ManualRefresh,
                    };

                    let status_str: String = row.get(7)?;
                    let status = RefreshStatus::from_str(&status_str);

                    Ok(RefreshTask {
                        task_id: row.get(0)?,
                        scope,
                        trigger,
                        trigger_source: row.get(3)?,
                        status,
                        retry_count: row.get(8)?,
                        max_retries: row.get(9)?,
                        created_at: row.get(10)?,
                        started_at: row.get(11)?,
                        completed_at: row.get(12)?,
                        error_message: row.get(13)?,
                    })
                },
            )
            .optional()?;

        Ok(task_opt)
    }

    /// 取消任务
    pub fn cancel_task(&self, task_id: &str) -> Result<bool, Box<dyn Error>> {
        let conn = self.conn.lock()
            .map_err(|e| format!("锁获取失败: {}", e))?;

        let rows_affected = conn.execute(
            "UPDATE decision_refresh_queue SET status = 'CANCELLED' WHERE task_id = ?1 AND status = 'PENDING'",
            [task_id],
        )?;

        Ok(rows_affected > 0)
    }

    /// 获取队列统计信息
    pub fn get_queue_stats(&self) -> Result<QueueStats, Box<dyn Error>> {
        let conn = self.conn.lock()
            .map_err(|e| format!("锁获取失败: {}", e))?;

        let pending_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM decision_refresh_queue WHERE status = 'PENDING'",
            [],
            |row| row.get(0),
        )?;

        let running_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM decision_refresh_queue WHERE status = 'RUNNING'",
            [],
            |row| row.get(0),
        )?;

        let completed_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM decision_refresh_queue WHERE status = 'COMPLETED'",
            [],
            |row| row.get(0),
        )?;

        let failed_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM decision_refresh_queue WHERE status = 'FAILED'",
            [],
            |row| row.get(0),
        )?;

        Ok(QueueStats {
            pending_count: pending_count as u32,
            running_count: running_count as u32,
            completed_count: completed_count as u32,
            failed_count: failed_count as u32,
        })
    }
}

/// 队列统计信息
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub pending_count: u32,
    pub running_count: u32,
    pub completed_count: u32,
    pub failed_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();

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

            CREATE TABLE decision_cold_stock_profile (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                age_bin TEXT NOT NULL,
                age_min_days INTEGER NOT NULL,
                age_max_days INTEGER,
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

            CREATE TABLE decision_roll_campaign_alert (
                version_id TEXT NOT NULL,
                campaign_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                cum_weight_t REAL NOT NULL,
                hard_limit_t REAL NOT NULL,
                suggest_threshold_t REAL NOT NULL,
                utilization_rate REAL NOT NULL,
                alert_level TEXT NOT NULL,
                needs_immediate_change INTEGER NOT NULL DEFAULT 0,
                remaining_capacity_t REAL NOT NULL,
                estimated_exhaustion_date TEXT,
                suggested_actions TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, campaign_id)
            );

            CREATE TABLE decision_capacity_opportunity (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                slack_t REAL NOT NULL,
                soft_adjust_space_t REAL,
                utilization_rate REAL NOT NULL,
                binding_constraints TEXT NOT NULL,
                opportunity_level TEXT NOT NULL,
                sensitivity TEXT,
                suggested_optimizations TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, machine_code, plan_date)
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
            "#,
        )
        .unwrap();

        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_enqueue_and_dequeue() {
        let conn = setup_test_db();
        let refresh_service = Arc::new(DecisionRefreshService::new(conn.clone()));
        let queue = RefreshQueue::new(conn.clone(), refresh_service).unwrap();

        let task = RefreshTask::new(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: true,
                affected_machines: None,
                affected_date_range: None,
            },
            RefreshTrigger::ManualRefresh,
            Some("test".to_string()),
            3,
        );

        let task_id = queue.enqueue(task).unwrap();
        assert!(!task_id.is_empty());

        let dequeued_task = queue.dequeue().unwrap();
        assert!(dequeued_task.is_some());
        assert_eq!(dequeued_task.unwrap().task_id, task_id);
    }

    #[test]
    fn test_execute_task() {
        let conn = setup_test_db();
        let refresh_service = Arc::new(DecisionRefreshService::new(conn.clone()));
        let queue = RefreshQueue::new(conn.clone(), refresh_service).unwrap();

        let task = RefreshTask::new(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: true,
                affected_machines: None,
                affected_date_range: None,
            },
            RefreshTrigger::ManualRefresh,
            Some("test".to_string()),
            3,
        );

        queue.enqueue(task).unwrap();

        let refresh_id = queue.process_next().unwrap();
        assert!(refresh_id.is_some());
    }

    #[test]
    fn test_queue_stats() {
        let conn = setup_test_db();
        let refresh_service = Arc::new(DecisionRefreshService::new(conn.clone()));
        let queue = RefreshQueue::new(conn.clone(), refresh_service).unwrap();

        let task = RefreshTask::new(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: true,
                affected_machines: None,
                affected_date_range: None,
            },
            RefreshTrigger::ManualRefresh,
            Some("test".to_string()),
            3,
        );

        queue.enqueue(task).unwrap();

        let stats = queue.get_queue_stats().unwrap();
        assert_eq!(stats.pending_count, 1);
        assert_eq!(stats.running_count, 0);
        assert_eq!(stats.completed_count, 0);
    }

    #[test]
    fn test_retry_mechanism() {
        let conn = setup_test_db();

        // 删除 decision_refresh_log 表以引发错误
        {
            let c = conn.lock().unwrap();
            c.execute("DROP TABLE decision_refresh_log", []).unwrap();
        }

        let refresh_service = Arc::new(DecisionRefreshService::new(conn.clone()));
        let queue = RefreshQueue::new(conn.clone(), refresh_service).unwrap();

        // 创建一个会失败的任务（因为 decision_refresh_log 表不存在）
        let task = RefreshTask::new(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: true,
                affected_machines: None,
                affected_date_range: None,
            },
            RefreshTrigger::ManualRefresh,
            Some("test".to_string()),
            3, // 最多重试 3 次（初始执行不算，所以总共执行 4 次）
        );

        let task_id = queue.enqueue(task).unwrap();

        // 第一次执行（失败）- retry_count: 0 -> 1
        let result = queue.process_next();
        assert!(result.is_err());

        // 验证任务状态为 PENDING（准备重试）
        let task_status = queue.get_task_status(&task_id).unwrap();
        assert!(task_status.is_some());
        let task = task_status.unwrap();
        assert_eq!(task.status, RefreshStatus::Pending);
        assert_eq!(task.retry_count, 1);

        // 第二次执行（再次失败）- retry_count: 1 -> 2
        let result = queue.process_next();
        assert!(result.is_err());

        // 验证任务状态为 PENDING（准备最后一次重试）
        let task_status = queue.get_task_status(&task_id).unwrap();
        assert!(task_status.is_some());
        let task = task_status.unwrap();
        assert_eq!(task.status, RefreshStatus::Pending);
        assert_eq!(task.retry_count, 2);

        // 第三次执行（最后一次重试）- retry_count: 2 -> 3
        let result = queue.process_next();
        assert!(result.is_err());

        // 验证任务状态为 FAILED（达到最大重试次数）
        let task_status = queue.get_task_status(&task_id).unwrap();
        assert!(task_status.is_some());
        let task = task_status.unwrap();
        assert_eq!(task.status, RefreshStatus::Failed);
        assert_eq!(task.retry_count, 3);
        assert!(!task.can_retry()); // 不能再重试

        // 验证队列中没有待执行任务
        let next_task = queue.dequeue().unwrap();
        assert!(next_task.is_none());
    }

    #[test]
    fn test_cancel_task() {
        let conn = setup_test_db();
        let refresh_service = Arc::new(DecisionRefreshService::new(conn.clone()));
        let queue = RefreshQueue::new(conn.clone(), refresh_service).unwrap();

        let task = RefreshTask::new(
            RefreshScope {
                version_id: "V001".to_string(),
                is_full_refresh: true,
                affected_machines: None,
                affected_date_range: None,
            },
            RefreshTrigger::ManualRefresh,
            Some("test".to_string()),
            3,
        );

        let task_id = queue.enqueue(task).unwrap();

        // 取消任务
        let cancelled = queue.cancel_task(&task_id).unwrap();
        assert!(cancelled);

        // 验证任务状态为 CANCELLED
        let task_status = queue.get_task_status(&task_id).unwrap();
        assert!(task_status.is_some());
        let task = task_status.unwrap();
        assert_eq!(task.status, RefreshStatus::Cancelled);

        // 验证队列中没有待执行任务
        let next_task = queue.dequeue().unwrap();
        assert!(next_task.is_none());
    }
}

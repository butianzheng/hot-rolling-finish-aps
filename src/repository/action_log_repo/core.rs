use crate::domain::action_log::ActionLog;
use crate::repository::error::{RepositoryError, RepositoryResult};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

// ==========================================
// ActionLogRepository - 操作日志仓储
// ==========================================
// 红线: Repository 不做业务逻辑,只做数据映射
pub struct ActionLogRepository {
    conn: Arc<Mutex<Connection>>,
}

impl ActionLogRepository {
    /// 创建新的操作日志仓储
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 获取数据库连接
    pub(super) fn get_conn(&self) -> RepositoryResult<std::sync::MutexGuard<Connection>> {
        self.conn
            .lock()
            .map_err(|e| RepositoryError::LockError(e.to_string()))
    }

    // ==========================================
    // 写入操作
    // ==========================================

    /// 插入操作日志
    ///
    /// # 参数
    /// - `log`: 操作日志实体
    ///
    /// # 返回
    /// - `Ok(action_id)`: 成功插入,返回action_id
    /// - `Err(...)`: 数据库错误
    pub fn insert(&self, log: &ActionLog) -> RepositoryResult<String> {
        let conn = self.get_conn()?;

        conn.execute(
            r#"
            INSERT INTO action_log (
                action_id, version_id, action_type, action_ts, actor,
                payload_json, impact_summary_json, machine_code,
                date_range_start, date_range_end, detail
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                log.action_id,
                log.version_id,
                log.action_type,
                log.action_ts.format("%Y-%m-%d %H:%M:%S").to_string(),
                log.actor,
                log.payload_json.as_ref().map(|v| v.to_string()),
                log.impact_summary_json.as_ref().map(|v| v.to_string()),
                log.machine_code,
                log.date_range_start.map(|d| d.format("%Y-%m-%d").to_string()),
                log.date_range_end.map(|d| d.format("%Y-%m-%d").to_string()),
                log.detail,
            ],
        )?;

        Ok(log.action_id.clone())
    }

    /// 将指定版本相关的日志从外键关系中“解绑”
    ///
    /// 背景：action_log.version_id 存在外键约束（REFERENCES plan_version）。
    /// 当删除 plan_version 时，需要先将历史日志的 version_id 置空，否则会触发外键约束失败。
    ///
    /// # 返回
    /// - Ok(rows): 被更新的行数
    pub fn detach_version(&self, version_id: &str) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let rows = conn.execute(
            "UPDATE action_log SET version_id = NULL WHERE version_id = ?1",
            params![version_id],
        )?;
        Ok(rows)
    }

    /// 批量插入操作日志
    pub fn batch_insert(&self, logs: Vec<ActionLog>) -> RepositoryResult<usize> {
        let mut conn = self.get_conn()?;
        let tx = conn.transaction()?;

        let mut count = 0;
        for log in logs {
            tx.execute(
                r#"
                INSERT INTO action_log (
                    action_id, version_id, action_type, action_ts, actor,
                    payload_json, impact_summary_json, machine_code,
                    date_range_start, date_range_end, detail
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    log.action_id,
                    log.version_id,
                    log.action_type,
                    log.action_ts.format("%Y-%m-%d %H:%M:%S").to_string(),
                    log.actor,
                    log.payload_json.as_ref().map(|v| v.to_string()),
                    log.impact_summary_json.as_ref().map(|v| v.to_string()),
                    log.machine_code,
                    log.date_range_start.map(|d| d.format("%Y-%m-%d").to_string()),
                    log.date_range_end.map(|d| d.format("%Y-%m-%d").to_string()),
                    log.detail,
                ],
            )?;
            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }

}

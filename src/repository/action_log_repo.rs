// ==========================================
// 热轧精整排产系统 - 操作日志数据仓储
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART A3 审计增强
// 依据: schema_v0.1.sql action_log 表
// 红线: 所有写入必须记录
// ==========================================

use crate::domain::action_log::ActionLog;
use crate::repository::error::{RepositoryError, RepositoryResult};
use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{params, Connection, Result as SqliteResult, Row};
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
    fn get_conn(&self) -> RepositoryResult<std::sync::MutexGuard<Connection>> {
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

    // ==========================================
    // 查询操作
    // ==========================================

    /// 按 action_id 查询单个日志
    pub fn find_by_id(&self, action_id: &str) -> RepositoryResult<Option<ActionLog>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT action_id, version_id, action_type, action_ts, actor,
                   payload_json, impact_summary_json, machine_code,
                   date_range_start, date_range_end, detail
            FROM action_log
            WHERE action_id = ?
            "#,
        )?;

        match stmt.query_row(params![action_id], |row| self.map_row(row)) {
            Ok(log) => Ok(Some(log)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 查询指定版本的所有操作日志
    pub fn find_by_version_id(&self, version_id: &str) -> RepositoryResult<Vec<ActionLog>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT action_id, version_id, action_type, action_ts, actor,
                   payload_json, impact_summary_json, machine_code,
                   date_range_start, date_range_end, detail
            FROM action_log
            WHERE version_id = ?
            ORDER BY action_ts DESC
            "#,
        )?;

        let logs = stmt
            .query_map(params![version_id], |row| self.map_row(row))?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(logs)
    }

    /// 查询指定时间范围的操作日志
    pub fn find_by_time_range(
        &self,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
    ) -> RepositoryResult<Vec<ActionLog>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT action_id, version_id, action_type, action_ts, actor,
                   payload_json, impact_summary_json, machine_code,
                   date_range_start, date_range_end, detail
            FROM action_log
            WHERE action_ts BETWEEN ? AND ?
            ORDER BY action_ts DESC
            "#,
        )?;

        let logs = stmt
            .query_map(
                params![
                    start_time.format("%Y-%m-%d %H:%M:%S").to_string(),
                    end_time.format("%Y-%m-%d %H:%M:%S").to_string(),
                ],
                |row| self.map_row(row),
            )?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(logs)
    }

    /// 查询指定材料ID的操作日志（按时间范围）
    ///
    /// 说明：
    /// - action_log 目前没有专门的 target_id 字段，因此这里通过 detail/payload_json/impact_summary_json 做“包含匹配”。
    /// - 对 JSON 字段使用 `"MATERIAL_ID"` 形式的匹配，尽量避免误匹配（例如 MAT001 不应匹配 MAT0010）。
    pub fn find_by_material_id_in_time_range(
        &self,
        material_id: &str,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
        limit: i32,
    ) -> RepositoryResult<Vec<ActionLog>> {
        let conn = self.get_conn()?;
        let json_token = format!("\"{}\"", material_id);

        let mut stmt = conn.prepare(
            r#"
            SELECT action_id, version_id, action_type, action_ts, actor,
                   payload_json, impact_summary_json, machine_code,
                   date_range_start, date_range_end, detail
            FROM action_log
            WHERE action_ts BETWEEN ? AND ?
              AND (
                instr(COALESCE(detail, ''), ?) > 0
                OR instr(COALESCE(payload_json, ''), ?) > 0
                OR instr(COALESCE(impact_summary_json, ''), ?) > 0
              )
            ORDER BY action_ts DESC
            LIMIT ?
            "#,
        )?;

        let logs = stmt
            .query_map(
                params![
                    start_time.format("%Y-%m-%d %H:%M:%S").to_string(),
                    end_time.format("%Y-%m-%d %H:%M:%S").to_string(),
                    material_id,
                    &json_token,
                    &json_token,
                    limit,
                ],
                |row| self.map_row(row),
            )?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(logs)
    }

    /// 查询指定操作人的日志
    pub fn find_by_actor(&self, actor: &str, limit: i32) -> RepositoryResult<Vec<ActionLog>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT action_id, version_id, action_type, action_ts, actor,
                   payload_json, impact_summary_json, machine_code,
                   date_range_start, date_range_end, detail
            FROM action_log
            WHERE actor = ?
            ORDER BY action_ts DESC
            LIMIT ?
            "#,
        )?;

        let logs = stmt
            .query_map(params![actor, limit], |row| self.map_row(row))?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(logs)
    }

    /// 查询指定操作类型的日志
    pub fn find_by_action_type(
        &self,
        action_type: &str,
        limit: i32,
    ) -> RepositoryResult<Vec<ActionLog>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT action_id, version_id, action_type, action_ts, actor,
                   payload_json, impact_summary_json, machine_code,
                   date_range_start, date_range_end, detail
            FROM action_log
            WHERE action_type = ?
            ORDER BY action_ts DESC
            LIMIT ?
            "#,
        )?;

        let logs = stmt
            .query_map(params![action_type, limit], |row| self.map_row(row))?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(logs)
    }

    /// 查询最近的 N 条日志
    pub fn find_recent(&self, limit: i32) -> RepositoryResult<Vec<ActionLog>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT action_id, version_id, action_type, action_ts, actor,
                   payload_json, impact_summary_json, machine_code,
                   date_range_start, date_range_end, detail
            FROM action_log
            ORDER BY action_ts DESC
            LIMIT ?
            "#,
        )?;

        let logs = stmt
            .query_map(params![limit], |row| self.map_row(row))?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(logs)
    }

    /// 查询最近的 N 条日志（分页）
    pub fn find_recent_paged(&self, limit: i32, offset: i32) -> RepositoryResult<Vec<ActionLog>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT action_id, version_id, action_type, action_ts, actor,
                   payload_json, impact_summary_json, machine_code,
                   date_range_start, date_range_end, detail
            FROM action_log
            ORDER BY action_ts DESC
            LIMIT ?
            OFFSET ?
            "#,
        )?;

        let logs = stmt
            .query_map(params![limit, offset], |row| self.map_row(row))?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(logs)
    }

    /// 查询指定时间范围的操作日志（分页）
    pub fn find_by_time_range_paged(
        &self,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
        limit: i32,
        offset: i32,
    ) -> RepositoryResult<Vec<ActionLog>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT action_id, version_id, action_type, action_ts, actor,
                   payload_json, impact_summary_json, machine_code,
                   date_range_start, date_range_end, detail
            FROM action_log
            WHERE action_ts BETWEEN ? AND ?
            ORDER BY action_ts DESC
            LIMIT ?
            OFFSET ?
            "#,
        )?;

        let logs = stmt
            .query_map(
                params![
                    start_time.format("%Y-%m-%d %H:%M:%S").to_string(),
                    end_time.format("%Y-%m-%d %H:%M:%S").to_string(),
                    limit,
                    offset,
                ],
                |row| self.map_row(row),
            )?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(logs)
    }

    /// 查询影响指定日期范围的操作
    pub fn find_by_impacted_date_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> RepositoryResult<Vec<ActionLog>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT action_id, version_id, action_type, action_ts, actor,
                   payload_json, impact_summary_json, machine_code,
                   date_range_start, date_range_end, detail
            FROM action_log
            WHERE date_range_start IS NOT NULL
              AND date_range_end IS NOT NULL
              AND date_range_start <= ?
              AND date_range_end >= ?
            ORDER BY action_ts DESC
            "#,
        )?;

        let logs = stmt
            .query_map(
                params![
                    end_date.format("%Y-%m-%d").to_string(),
                    start_date.format("%Y-%m-%d").to_string(),
                ],
                |row| self.map_row(row),
            )?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(logs)
    }

    /// 查询指定版本和机组的操作日志
    pub fn find_by_version_and_machine(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> RepositoryResult<Vec<ActionLog>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT action_id, version_id, action_type, action_ts, actor,
                   payload_json, impact_summary_json, machine_code,
                   date_range_start, date_range_end, detail
            FROM action_log
            WHERE version_id = ?
              AND machine_code = ?
            ORDER BY action_ts DESC
            "#,
        )?;

        let logs = stmt
            .query_map(params![version_id, machine_code], |row| self.map_row(row))?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(logs)
    }

    /// 统计指定版本的操作总数
    pub fn count_by_version(&self, version_id: &str) -> RepositoryResult<i32> {
        let conn = self.get_conn()?;

        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM action_log WHERE version_id = ?",
            params![version_id],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// 统计指定操作人的操作总数
    pub fn count_by_actor(&self, actor: &str) -> RepositoryResult<i32> {
        let conn = self.get_conn()?;

        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM action_log WHERE actor = ?",
            params![actor],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    // ==========================================
    // 辅助方法
    // ==========================================

    /// 将数据库行映射为 ActionLog 实体
    fn map_row(&self, row: &Row) -> SqliteResult<ActionLog> {
        let action_id: String = row.get(0)?;
        let version_id: Option<String> = row.get(1)?;
        let action_type: String = row.get(2)?;
        let action_ts_str: String = row.get(3)?;
        let actor: String = row.get(4)?;

        let payload_json_str: Option<String> = row.get(5)?;
        let impact_summary_json_str: Option<String> = row.get(6)?;
        let machine_code: Option<String> = row.get(7)?;
        let date_range_start_str: Option<String> = row.get(8)?;
        let date_range_end_str: Option<String> = row.get(9)?;
        let detail: Option<String> = row.get(10)?;

        // 解析时间戳
        let action_ts = NaiveDateTime::parse_from_str(&action_ts_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e)))?;

        // 解析 JSON 字段
        let payload_json = payload_json_str
            .and_then(|s| serde_json::from_str(&s).ok());

        let impact_summary_json = impact_summary_json_str
            .and_then(|s| serde_json::from_str(&s).ok());

        // 解析日期
        let date_range_start = date_range_start_str
            .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

        let date_range_end = date_range_end_str
            .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

        Ok(ActionLog {
            action_id,
            version_id,
            action_type,
            action_ts,
            actor,
            payload_json,
            impact_summary_json,
            machine_code,
            date_range_start,
            date_range_end,
            detail,
        })
    }
}

// ==========================================
// 单元测试
// ==========================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::action_log::ImpactSummary;
    use chrono::{NaiveDate, NaiveDateTime, Utc};

    fn setup_test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();

        conn.execute(
            r#"
            CREATE TABLE action_log (
                action_id TEXT PRIMARY KEY,
                version_id TEXT NOT NULL,
                action_type TEXT NOT NULL,
                action_ts TEXT NOT NULL,
                actor TEXT NOT NULL,
                payload_json TEXT,
                impact_summary_json TEXT,
                machine_code TEXT,
                date_range_start TEXT,
                date_range_end TEXT,
                detail TEXT
            )
            "#,
            [],
        )
        .unwrap();

        Arc::new(Mutex::new(conn))
    }

    fn make_test_log(action_id: &str, version_id: &str, actor: &str) -> ActionLog {
        ActionLog {
            action_id: action_id.to_string(),
            version_id: Some(version_id.to_string()),
            action_type: "Import".to_string(),
            action_ts: Utc::now().naive_utc(),
            actor: actor.to_string(),
            payload_json: None,
            impact_summary_json: None,
            machine_code: Some("M01".to_string()),
            date_range_start: Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()),
            date_range_end: Some(NaiveDate::from_ymd_opt(2025, 1, 20).unwrap()),
            detail: Some("Test log".to_string()),
        }
    }

    #[test]
    fn test_insert_and_find_by_id() {
        let conn = setup_test_db();
        let repo = ActionLogRepository::new(conn);

        let log = make_test_log("log1", "v1", "user1");
        let result = repo.insert(&log);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "log1");

        let found = repo.find_by_id("log1").unwrap();
        assert!(found.is_some());

        let found_log = found.unwrap();
        assert_eq!(found_log.action_id, "log1");
        assert_eq!(found_log.version_id, Some("v1".to_string()));
        assert_eq!(found_log.actor, "user1");
    }

    #[test]
    fn test_find_by_version_id() {
        let conn = setup_test_db();
        let repo = ActionLogRepository::new(conn);

        let log1 = make_test_log("log1", "v1", "user1");
        let log2 = make_test_log("log2", "v1", "user2");
        let log3 = make_test_log("log3", "v2", "user1");

        repo.insert(&log1).unwrap();
        repo.insert(&log2).unwrap();
        repo.insert(&log3).unwrap();

        let logs = repo.find_by_version_id("v1").unwrap();

        assert_eq!(logs.len(), 2);
        assert!(logs.iter().any(|l| l.action_id == "log1"));
        assert!(logs.iter().any(|l| l.action_id == "log2"));
    }

    #[test]
    fn test_find_by_actor() {
        let conn = setup_test_db();
        let repo = ActionLogRepository::new(conn);

        let log1 = make_test_log("log1", "v1", "user1");
        let log2 = make_test_log("log2", "v1", "user1");
        let log3 = make_test_log("log3", "v1", "user2");

        repo.insert(&log1).unwrap();
        repo.insert(&log2).unwrap();
        repo.insert(&log3).unwrap();

        let logs = repo.find_by_actor("user1", 10).unwrap();

        assert_eq!(logs.len(), 2);
    }

    #[test]
    fn test_find_by_action_type() {
        let conn = setup_test_db();
        let repo = ActionLogRepository::new(conn);

        let mut log1 = make_test_log("log1", "v1", "user1");
        log1.action_type = "Import".to_string();

        let mut log2 = make_test_log("log2", "v1", "user1");
        log2.action_type = "Recalc".to_string();

        let mut log3 = make_test_log("log3", "v1", "user1");
        log3.action_type = "Import".to_string();

        repo.insert(&log1).unwrap();
        repo.insert(&log2).unwrap();
        repo.insert(&log3).unwrap();

        let logs = repo.find_by_action_type("Import", 10).unwrap();

        assert_eq!(logs.len(), 2);
    }

    #[test]
    fn test_find_recent() {
        let conn = setup_test_db();
        let repo = ActionLogRepository::new(conn);

        for i in 1..=5 {
            let log = make_test_log(&format!("log{}", i), "v1", "user1");
            repo.insert(&log).unwrap();
        }

        let logs = repo.find_recent(3).unwrap();

        assert_eq!(logs.len(), 3);
    }

    #[test]
    fn test_batch_insert() {
        let conn = setup_test_db();
        let repo = ActionLogRepository::new(conn);

        let logs = vec![
            make_test_log("log1", "v1", "user1"),
            make_test_log("log2", "v1", "user1"),
            make_test_log("log3", "v1", "user1"),
        ];

        let result = repo.batch_insert(logs);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);

        let all_logs = repo.find_by_version_id("v1").unwrap();
        assert_eq!(all_logs.len(), 3);
    }

    #[test]
    fn test_count_by_version() {
        let conn = setup_test_db();
        let repo = ActionLogRepository::new(conn);

        repo.insert(&make_test_log("log1", "v1", "user1")).unwrap();
        repo.insert(&make_test_log("log2", "v1", "user1")).unwrap();
        repo.insert(&make_test_log("log3", "v2", "user1")).unwrap();

        let count = repo.count_by_version("v1").unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_by_actor() {
        let conn = setup_test_db();
        let repo = ActionLogRepository::new(conn);

        repo.insert(&make_test_log("log1", "v1", "user1")).unwrap();
        repo.insert(&make_test_log("log2", "v1", "user1")).unwrap();
        repo.insert(&make_test_log("log3", "v1", "user2")).unwrap();

        let count = repo.count_by_actor("user1").unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_find_by_impacted_date_range() {
        let conn = setup_test_db();
        let repo = ActionLogRepository::new(conn);

        let mut log1 = make_test_log("log1", "v1", "user1");
        log1.date_range_start = Some(NaiveDate::from_ymd_opt(2025, 1, 10).unwrap());
        log1.date_range_end = Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());

        let mut log2 = make_test_log("log2", "v1", "user1");
        log2.date_range_start = Some(NaiveDate::from_ymd_opt(2025, 1, 20).unwrap());
        log2.date_range_end = Some(NaiveDate::from_ymd_opt(2025, 1, 25).unwrap());

        repo.insert(&log1).unwrap();
        repo.insert(&log2).unwrap();

        let logs = repo
            .find_by_impacted_date_range(
                NaiveDate::from_ymd_opt(2025, 1, 12).unwrap(),
                NaiveDate::from_ymd_opt(2025, 1, 18).unwrap(),
            )
            .unwrap();

        // log1 应该被找到 (10-15 overlaps 12-18)
        // log2 不应该被找到 (20-25 不overlaps 12-18)
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action_id, "log1");
    }

    #[test]
    fn test_find_by_material_id_in_time_range_matches_detail_and_json_token() {
        let conn = setup_test_db();
        let repo = ActionLogRepository::new(conn);

        let t1 = NaiveDateTime::parse_from_str("2026-01-24 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let t2 = NaiveDateTime::parse_from_str("2026-01-24 11:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let t3 = NaiveDateTime::parse_from_str("2026-01-24 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let t4 = NaiveDateTime::parse_from_str("2026-01-24 13:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        // detail 命中
        let mut log_detail = make_test_log("log_detail", "v1", "user1");
        log_detail.action_ts = t1;
        log_detail.detail = Some("move material MAT001 -> H032/2026-01-24".to_string());
        repo.insert(&log_detail).unwrap();

        // payload_json 命中（\"MAT001\" token）
        let mut log_payload = make_test_log("log_payload", "v1", "user1");
        log_payload.action_ts = t2;
        log_payload.payload_json = Some(serde_json::json!({"material_id":"MAT001","op":"move"}));
        log_payload.detail = Some("payload_hit".to_string());
        repo.insert(&log_payload).unwrap();

        // impact_summary_json 命中（\"MAT001\" token）
        let mut log_impact = make_test_log("log_impact", "v1", "user1");
        log_impact.action_ts = t3;
        log_impact.impact_summary_json = Some(serde_json::json!({"moved":["MAT001"],"added":[]}));
        log_impact.detail = Some("impact_hit".to_string());
        repo.insert(&log_impact).unwrap();

        // 相似 material_id（MAT0010）不应被 \"MAT001\" token 误匹配（此处不写 detail，避免 substring 误判）
        let mut log_similar = make_test_log("log_similar", "v1", "user1");
        log_similar.action_ts = t4;
        log_similar.payload_json = Some(serde_json::json!({"material_id":"MAT0010"}));
        log_similar.detail = None;
        repo.insert(&log_similar).unwrap();

        let start_time = NaiveDateTime::parse_from_str("2026-01-24 09:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end_time = NaiveDateTime::parse_from_str("2026-01-24 15:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        let logs = repo
            .find_by_material_id_in_time_range("MAT001", start_time, end_time, 10)
            .unwrap();

        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].action_id, "log_impact");
        assert_eq!(logs[1].action_id, "log_payload");
        assert_eq!(logs[2].action_id, "log_detail");

        let logs_limited = repo
            .find_by_material_id_in_time_range("MAT001", start_time, end_time, 2)
            .unwrap();
        assert_eq!(logs_limited.len(), 2);
    }
}

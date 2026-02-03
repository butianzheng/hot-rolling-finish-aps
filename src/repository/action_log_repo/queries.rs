use super::core::ActionLogRepository;
use crate::domain::action_log::ActionLog;
use crate::repository::error::RepositoryResult;
use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{params, Connection, Result as SqliteResult, Row};

impl ActionLogRepository {
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

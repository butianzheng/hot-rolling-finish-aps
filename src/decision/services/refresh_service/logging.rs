use super::*;

impl DecisionRefreshService {

    /// 记录刷新开始
    pub(super) fn log_refresh_start(
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
    pub(super) fn log_refresh_complete(
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

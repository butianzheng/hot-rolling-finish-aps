// ==========================================
// 热轧精整排产系统 - SQLite 连接初始化
// ==========================================
// 目标:
// - 统一所有 Connection::open 的 PRAGMA 行为，避免“部分模块外键开启/部分不开启”
// - 统一 busy_timeout，减少并发写入时的偶发 busy 错误
// ==========================================

use rusqlite::Connection;
use rusqlite::OptionalExtension;
use std::time::Duration;

/// 默认 busy_timeout（毫秒）
pub const DEFAULT_BUSY_TIMEOUT_MS: u64 = 5_000;

/// 当前代码所期望的 schema_version（与 `migrations/v0.*.sql` 对齐）
///
/// 说明：
/// - 目前项目存在多套“迁移/建库”方式（schema.sql / migrations / scripts/migrations）。
/// - 这里的版本号用于**提示/告警**（不做自动迁移），避免静默在旧库上运行导致隐性错误。
pub const CURRENT_SCHEMA_VERSION: i64 = 6;

/// 配置 SQLite 连接的统一 PRAGMA
///
/// 说明：
/// - foreign_keys 需要“每个连接”单独开启
/// - busy_timeout 需要“每个连接”单独配置
pub fn configure_sqlite_connection(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.busy_timeout(Duration::from_millis(DEFAULT_BUSY_TIMEOUT_MS))?;
    Ok(())
}

/// 打开 SQLite 连接并应用统一配置
pub fn open_sqlite_connection(db_path: &str) -> rusqlite::Result<Connection> {
    let conn = Connection::open(db_path)?;
    configure_sqlite_connection(&conn)?;
    Ok(conn)
}

/// 读取 schema_version（若表不存在则返回 None）
pub fn read_schema_version(conn: &Connection) -> rusqlite::Result<Option<i64>> {
    let has_table: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_version' LIMIT 1",
            [],
            |_row| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    if !has_table {
        return Ok(None);
    }

    let v: Option<i64> = conn.query_row("SELECT MAX(version) FROM schema_version", [], |row| row.get(0))?;
    Ok(v)
}

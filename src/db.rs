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

/// 确保数据库 Schema 存在（首次启动自动建表）
///
/// 职责分工：
/// - `ensure_schema()`: 仅负责"首次建表"（当 schema_version 表不存在时）
/// - `migrations/*.sql`: 负责"增量升级"（人工执行，不自动）
///
/// 行为：
/// - 如果 schema_version 表存在：什么也不做（即使版本过旧，也不自动迁移）
/// - 如果 schema_version 表不存在：执行完整 schema.sql 建表
///
/// 说明：
/// - 这样可以确保"开发环境首次启动"和"生产环境首次部署"都能自动创建完整表结构
/// - 后续版本升级仍然需要人工执行 migrations/*.sql（符合工业系统要求）
pub fn ensure_schema(conn: &Connection) -> rusqlite::Result<()> {
    // 检查 schema_version 表是否存在
    let has_schema_version_table: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_version' LIMIT 1",
            [],
            |_row| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    if has_schema_version_table {
        // schema_version 表已存在，说明数据库已初始化，不做任何操作
        tracing::debug!("schema_version 表已存在，跳过 ensure_schema");
        return Ok(());
    }

    // schema_version 表不存在，执行完整建表脚本
    tracing::info!("schema_version 表不存在，开始执行完整建表脚本");

    let schema_sql = include_str!("../scripts/dev_db/schema.sql");
    conn.execute_batch(schema_sql)?;

    // 插入 schema_version 记录（标记为当前版本）
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
        rusqlite::params![CURRENT_SCHEMA_VERSION, now],
    )?;

    tracing::info!("完整建表脚本执行成功，schema_version={}", CURRENT_SCHEMA_VERSION);
    Ok(())
}

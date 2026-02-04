use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{OptionalExtension, Transaction};

/// 从配置表读取浮点数配置值
///
/// # 参数
/// - `tx`: SQLite 事务
/// - `key`: 配置键
///
/// # 返回
/// - `Ok(Some(value))`: 成功读取并解析
/// - `Ok(None)`: 配置不存在或解析失败
/// - `Err`: 数据库查询失败
pub(super) fn read_global_real(
    tx: &Transaction,
    key: &str,
) -> Result<Option<f64>, rusqlite::Error> {
    tx.query_row(
        "SELECT value FROM config_kv WHERE scope_id = 'global' AND key = ?1 LIMIT 1",
        [key],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map(|opt| opt.and_then(|s| s.trim().parse::<f64>().ok()))
}

/// 从配置表读取整数配置值
///
/// # 参数
/// - `tx`: SQLite 事务
/// - `key`: 配置键
///
/// # 返回
/// - `Ok(Some(value))`: 成功读取并解析
/// - `Ok(None)`: 配置不存在或解析失败
/// - `Err`: 数据库查询失败
pub(super) fn read_global_i32(tx: &Transaction, key: &str) -> Result<Option<i32>, rusqlite::Error> {
    tx.query_row(
        "SELECT value FROM config_kv WHERE scope_id = 'global' AND key = ?1 LIMIT 1",
        [key],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map(|opt| opt.and_then(|s| s.trim().parse::<i32>().ok()))
}

/// 尽力解析日期时间字符串
///
/// # 参数
/// - `raw`: 日期时间字符串
///
/// # 返回
/// - `Some(NaiveDateTime)`: 解析成功
/// - `None`: 解析失败
///
/// # 支持格式
/// - `%Y-%m-%d %H:%M:%S` (例如: 2024-01-01 12:30:00)
/// - `%Y-%m-%d %H:%M` (例如: 2024-01-01 12:30)
/// - `%Y-%m-%dT%H:%M:%S` (例如: 2024-01-01T12:30:00)
/// - RFC3339 (例如: 2024-01-01T12:30:00+08:00)
pub(super) fn parse_dt_best_effort(raw: &str) -> Option<NaiveDateTime> {
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

/// 将日期字符串转换为日期时间（时间部分为 00:00:00）
///
/// # 参数
/// - `ymd`: 日期字符串（格式: YYYY-MM-DD）
///
/// # 返回
/// - `Some(NaiveDateTime)`: 转换成功
/// - `None`: 转换失败
pub(super) fn ymd_to_start_at(ymd: &str) -> Option<NaiveDateTime> {
    let d = NaiveDate::parse_from_str(ymd, "%Y-%m-%d").ok()?;
    d.and_hms_opt(0, 0, 0)
}

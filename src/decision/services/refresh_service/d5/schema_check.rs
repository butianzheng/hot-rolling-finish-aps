use rusqlite::Transaction;

/// 检查表是否存在指定列
///
/// # 参数
/// - `tx`: SQLite 事务
/// - `table`: 表名
/// - `col`: 列名
///
/// # 返回
/// - `true`: 列存在
/// - `false`: 列不存在或查询失败
///
/// # 说明
/// - 使用 `pragma_table_info` 查询表结构
/// - 表名通过 SQL 字符串转义后内联（安全性：表名来自内部常量）
/// - 列名通过参数化查询（防止 SQL 注入）
pub(super) fn table_has_column(tx: &Transaction, table: &str, col: &str) -> bool {
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

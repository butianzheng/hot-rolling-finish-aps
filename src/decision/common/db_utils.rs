// ==========================================
// 热轧精整排产系统 - 数据库工具模块
// ==========================================
// 职责: 提供数据库操作的公共函数
// 目标: 消除 6+ 处重复的 IN 子句构建代码
// ==========================================

use rusqlite::{Connection, Result as SqlResult};

/// 构建 IN 子句的 SQL 片段
///
/// # 功能
/// - 生成 SQL IN 子句的占位符字符串 (?, ?, ?)
/// - 返回完整的 WHERE 子句和参数列表
///
/// # 用途
/// 替换 6+ 处类似代码：
/// ```ignore
/// let placeholders = items.iter().map(|_| "?").collect::<Vec<_>>().join(",");
/// let sql = format!("DELETE FROM table WHERE version_id = ? AND column IN ({})", placeholders);
/// ```
///
/// # 参数
/// - `column_name`: IN 子句应用的列名
/// - `values`: 值列表
///
/// # 返回
/// - 生成的 IN 子句片段，例如: "machine_code IN (?, ?, ?)"
///
/// # 示例
/// ```
/// use hot_rolling_aps::decision::common::db_utils::build_in_clause;
///
/// let machines = vec!["H032".to_string(), "H033".to_string()];
/// let clause = build_in_clause("machine_code", &machines);
/// assert_eq!(clause, "machine_code IN (?, ?)");
///
/// // 空列表返回 FALSE 条件
/// let empty: Vec<String> = vec![];
/// let clause = build_in_clause("machine_code", &empty);
/// assert_eq!(clause, "1 = 0");
/// ```
pub fn build_in_clause<T: AsRef<str>>(column_name: &str, values: &[T]) -> String {
    if values.is_empty() {
        // 空列表时返回永假条件，确保 SQL 语法正确
        return "1 = 0".to_string();
    }

    let placeholders = values.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    format!("{} IN ({})", column_name, placeholders)
}

/// 执行带 IN 子句的 DELETE 语句
///
/// # 功能
/// - 构建并执行带 IN 子句的 DELETE 语句
/// - 自动处理参数绑定
/// - 统一的错误处理
///
/// # 用途
/// 替换所有 refresh_incremental 中的 DELETE 部分，例如：
/// ```ignore
/// let placeholders = machines.iter().map(|_| "?").collect::<Vec<_>>().join(",");
/// let delete_sql = format!(
///     "DELETE FROM table WHERE version_id = ? AND machine_code IN ({})",
///     placeholders
/// );
/// let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&version_id];
/// for item in machines {
///     params_vec.push(item);
/// }
/// conn.execute(&delete_sql, params_vec.as_slice())?;
/// ```
///
/// # 参数
/// - `conn`: 数据库连接
/// - `table_name`: 目标表名
/// - `version_id`: 版本 ID
/// - `filter_column`: 过滤列名（用于 IN 子句）
/// - `filter_values`: 过滤值列表
///
/// # 返回
/// - 成功: 受影响的行数
/// - 失败: rusqlite 错误
///
/// # 示例
/// ```
/// use rusqlite::Connection;
/// use hot_rolling_aps::decision::common::db_utils::execute_delete_with_in_clause;
///
/// let conn = Connection::open_in_memory().unwrap();
/// conn.execute(
///     "CREATE TABLE test_table (version_id TEXT, machine_code TEXT)",
///     []
/// ).unwrap();
///
/// conn.execute(
///     "INSERT INTO test_table VALUES ('V001', 'H032'), ('V001', 'H033')",
///     []
/// ).unwrap();
///
/// let machines = vec!["H032".to_string()];
/// let rows = execute_delete_with_in_clause(
///     &conn,
///     "test_table",
///     "V001",
///     "machine_code",
///     &machines
/// ).unwrap();
///
/// assert_eq!(rows, 1);
/// ```
pub fn execute_delete_with_in_clause(
    conn: &Connection,
    table_name: &str,
    version_id: &str,
    filter_column: &str,
    filter_values: &[String],
) -> SqlResult<usize> {
    if filter_values.is_empty() {
        return Ok(0);
    }

    // 构建 SQL 语句
    let in_clause = build_in_clause(filter_column, filter_values);
    let sql = format!(
        "DELETE FROM {} WHERE version_id = ? AND {}",
        table_name, in_clause
    );

    // 构建参数向量（version_id + 所有过滤值）
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(version_id.to_string())];
    for value in filter_values {
        params.push(Box::new(value.clone()));
    }

    // 执行 DELETE
    conn.execute(&sql, rusqlite::params_from_iter(params))
}

/// 执行带 IN 子句和日期范围的 DELETE 语句
///
/// # 功能
/// - 构建并执行带 IN 子句和日期范围的 DELETE 语句
/// - 用于增量刷新同时按机组和日期过滤的场景
///
/// # 参数
/// - `conn`: 数据库连接
/// - `table_name`: 目标表名
/// - `version_id`: 版本 ID
/// - `machine_column`: 机组列名
/// - `machines`: 机组列表
/// - `date_column`: 日期列名
/// - `start_date`: 开始日期
/// - `end_date`: 结束日期
///
/// # 返回
/// - 成功: 受影响的行数
/// - 失败: rusqlite 错误
///
/// # 示例
/// ```
/// use rusqlite::Connection;
/// use hot_rolling_aps::decision::common::db_utils::execute_delete_with_in_clause_and_date_range;
///
/// let conn = Connection::open_in_memory().unwrap();
/// conn.execute(
///     "CREATE TABLE test_table (version_id TEXT, machine_code TEXT, plan_date TEXT)",
///     []
/// ).unwrap();
///
/// conn.execute(
///     "INSERT INTO test_table VALUES ('V001', 'H032', '2026-01-24'), ('V001', 'H033', '2026-01-25')",
///     []
/// ).unwrap();
///
/// let machines = vec!["H032".to_string()];
/// let rows = execute_delete_with_in_clause_and_date_range(
///     &conn,
///     "test_table",
///     "V001",
///     "machine_code",
///     &machines,
///     "plan_date",
///     "2026-01-24",
///     "2026-01-26"
/// ).unwrap();
///
/// assert_eq!(rows, 1);
/// ```
pub fn execute_delete_with_in_clause_and_date_range(
    conn: &Connection,
    table_name: &str,
    version_id: &str,
    machine_column: &str,
    machines: &[String],
    date_column: &str,
    start_date: &str,
    end_date: &str,
) -> SqlResult<usize> {
    if machines.is_empty() {
        return Ok(0);
    }

    // 构建 SQL 语句
    let in_clause = build_in_clause(machine_column, machines);
    let sql = format!(
        "DELETE FROM {} WHERE version_id = ? AND {} AND {} >= ? AND {} < ?",
        table_name, in_clause, date_column, date_column
    );

    // 构建参数向量
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(version_id.to_string())];
    for machine in machines {
        params.push(Box::new(machine.clone()));
    }
    params.push(Box::new(start_date.to_string()));
    params.push(Box::new(end_date.to_string()));

    // 执行 DELETE
    conn.execute(&sql, rusqlite::params_from_iter(params))
}

// ==========================================
// 单元测试
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    // ==========================================
    // build_in_clause 测试
    // ==========================================

    #[test]
    fn test_build_in_clause_with_values() {
        let values = vec!["H032".to_string(), "H033".to_string(), "H034".to_string()];
        let clause = build_in_clause("machine_code", &values);
        assert_eq!(clause, "machine_code IN (?, ?, ?)");
    }

    #[test]
    fn test_build_in_clause_single_value() {
        let values = vec!["H032".to_string()];
        let clause = build_in_clause("machine_code", &values);
        assert_eq!(clause, "machine_code IN (?)");
    }

    #[test]
    fn test_build_in_clause_empty_returns_false() {
        let values: Vec<String> = vec![];
        let clause = build_in_clause("machine_code", &values);
        assert_eq!(clause, "1 = 0");
    }

    #[test]
    fn test_build_in_clause_different_column() {
        let values = vec!["V001".to_string(), "V002".to_string()];
        let clause = build_in_clause("version_id", &values);
        assert_eq!(clause, "version_id IN (?, ?)");
    }

    // ==========================================
    // execute_delete_with_in_clause 测试
    // ==========================================

    fn setup_test_table(conn: &Connection) {
        conn.execute(
            r#"
            CREATE TABLE decision_test (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                value INTEGER
            )
        "#,
            [],
        )
        .unwrap();

        // 插入测试数据
        conn.execute(
            "INSERT INTO decision_test VALUES ('V001', 'H032', 100)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO decision_test VALUES ('V001', 'H033', 200)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO decision_test VALUES ('V001', 'H034', 300)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO decision_test VALUES ('V002', 'H032', 400)",
            [],
        )
        .unwrap();
    }

    #[test]
    fn test_execute_delete_with_in_clause_single_machine() {
        let conn = Connection::open_in_memory().unwrap();
        setup_test_table(&conn);

        let machines = vec!["H032".to_string()];
        let rows = execute_delete_with_in_clause(
            &conn,
            "decision_test",
            "V001",
            "machine_code",
            &machines,
        )
        .unwrap();

        assert_eq!(rows, 1);

        // 验证删除结果
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM decision_test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 3); // 剩余 3 条记录
    }

    #[test]
    fn test_execute_delete_with_in_clause_multiple_machines() {
        let conn = Connection::open_in_memory().unwrap();
        setup_test_table(&conn);

        let machines = vec!["H032".to_string(), "H033".to_string()];
        let rows = execute_delete_with_in_clause(
            &conn,
            "decision_test",
            "V001",
            "machine_code",
            &machines,
        )
        .unwrap();

        assert_eq!(rows, 2);

        // 验证只删除了指定版本的记录
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM decision_test WHERE version_id = 'V001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1); // V001 还剩 H034
    }

    #[test]
    fn test_execute_delete_with_in_clause_empty_list() {
        let conn = Connection::open_in_memory().unwrap();
        setup_test_table(&conn);

        let machines: Vec<String> = vec![];
        let rows = execute_delete_with_in_clause(
            &conn,
            "decision_test",
            "V001",
            "machine_code",
            &machines,
        )
        .unwrap();

        assert_eq!(rows, 0);

        // 验证没有删除任何记录
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM decision_test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 4);
    }

    #[test]
    fn test_execute_delete_with_in_clause_version_isolation() {
        let conn = Connection::open_in_memory().unwrap();
        setup_test_table(&conn);

        // 删除 V001 的 H032
        let machines = vec!["H032".to_string()];
        execute_delete_with_in_clause(&conn, "decision_test", "V001", "machine_code", &machines)
            .unwrap();

        // 验证 V002 的 H032 未被删除
        let v002_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM decision_test WHERE version_id = 'V002' AND machine_code = 'H032'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(v002_count, 1);
    }

    // ==========================================
    // execute_delete_with_in_clause_and_date_range 测试
    // ==========================================

    fn setup_test_table_with_dates(conn: &Connection) {
        conn.execute(
            r#"
            CREATE TABLE decision_test_dates (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                value INTEGER
            )
        "#,
            [],
        )
        .unwrap();

        // 插入测试数据
        let data = vec![
            ("V001", "H032", "2026-01-24", 100),
            ("V001", "H032", "2026-01-25", 200),
            ("V001", "H033", "2026-01-24", 300),
            ("V001", "H033", "2026-01-26", 400),
        ];

        for (vid, machine, date, value) in data {
            conn.execute(
                "INSERT INTO decision_test_dates VALUES (?, ?, ?, ?)",
                rusqlite::params![vid, machine, date, value],
            )
            .unwrap();
        }
    }

    #[test]
    fn test_execute_delete_with_date_range() {
        let conn = Connection::open_in_memory().unwrap();
        setup_test_table_with_dates(&conn);

        let machines = vec!["H032".to_string()];
        let rows = execute_delete_with_in_clause_and_date_range(
            &conn,
            "decision_test_dates",
            "V001",
            "machine_code",
            &machines,
            "plan_date",
            "2026-01-24",
            "2026-01-26",
        )
        .unwrap();

        assert_eq!(rows, 2); // H032 的 2026-01-24 和 2026-01-25

        // 验证剩余记录
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM decision_test_dates",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2); // H033 的两条记录保留
    }

    #[test]
    fn test_execute_delete_with_date_range_multiple_machines() {
        let conn = Connection::open_in_memory().unwrap();
        setup_test_table_with_dates(&conn);

        let machines = vec!["H032".to_string(), "H033".to_string()];
        let rows = execute_delete_with_in_clause_and_date_range(
            &conn,
            "decision_test_dates",
            "V001",
            "machine_code",
            &machines,
            "plan_date",
            "2026-01-24",
            "2026-01-25",
        )
        .unwrap();

        assert_eq!(rows, 2); // H032 和 H033 的 2026-01-24

        // 验证 2026-01-25 和 2026-01-26 的记录保留
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM decision_test_dates",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_execute_delete_with_date_range_empty_machines() {
        let conn = Connection::open_in_memory().unwrap();
        setup_test_table_with_dates(&conn);

        let machines: Vec<String> = vec![];
        let rows = execute_delete_with_in_clause_and_date_range(
            &conn,
            "decision_test_dates",
            "V001",
            "machine_code",
            &machines,
            "plan_date",
            "2026-01-24",
            "2026-01-27",
        )
        .unwrap();

        assert_eq!(rows, 0);

        // 验证没有删除任何记录
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM decision_test_dates",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 4);
    }
}

// ==========================================
// 热轧精整排产系统 - SQL 构建工具模块
// ==========================================
// 职责: 提供 SQL 语句构建的公共函数
// 目标: 消除 6+ 处重复的动态 SQL 构建代码
// ==========================================

/// 构建带可选过滤条件的 SQL 语句
///
/// # 功能
/// - 根据可选参数动态添加 WHERE 条件
/// - 统一的 SQL 构建模式
/// - 自动添加 ORDER BY 子句
///
/// # 用途
/// 替换 6+ 处类似代码：
/// ```ignore
/// let base_sql = r#"SELECT ... FROM table WHERE version_id = ?"#;
/// let sql = if optional_filter.is_some() {
///     format!("{} AND filter_column = ? ORDER BY ...", base_sql)
/// } else {
///     format!("{} ORDER BY ...", base_sql)
/// };
/// ```
///
/// # 参数
/// - `base_query`: 基础 SELECT 语句（不包含 ORDER BY）
/// - `additional_filter`: 可选的额外过滤条件（例如: "machine_code = ?"）
/// - `order_by_clause`: ORDER BY 子句（例如: "slack_t DESC"）
///
/// # 返回
/// - 完整的 SQL 查询语句
///
/// # 示例
/// ```
/// use hot_rolling_aps::decision::common::sql_builder::build_optional_filter_sql;
///
/// let base = "SELECT * FROM decision_test WHERE version_id = ?";
/// let order = "created_at DESC";
///
/// // 无额外过滤条件
/// let sql1 = build_optional_filter_sql(base, None, order);
/// assert_eq!(sql1, "SELECT * FROM decision_test WHERE version_id = ? ORDER BY created_at DESC");
///
/// // 有额外过滤条件
/// let sql2 = build_optional_filter_sql(base, Some("machine_code = ?"), order);
/// assert_eq!(sql2, "SELECT * FROM decision_test WHERE version_id = ? AND machine_code = ? ORDER BY created_at DESC");
/// ```
pub fn build_optional_filter_sql(
    base_query: &str,
    additional_filter: Option<&str>,
    order_by_clause: &str,
) -> String {
    if let Some(filter) = additional_filter {
        format!("{} AND {} ORDER BY {}", base_query, filter, order_by_clause)
    } else {
        format!("{} ORDER BY {}", base_query, order_by_clause)
    }
}

/// 构建带可选 LIMIT 的 SQL 语句
///
/// # 功能
/// - 根据可选的 limit 参数添加 LIMIT 子句
/// - 用于 Top N 查询
///
/// # 参数
/// - `base_query`: 基础查询（包含 WHERE 和 ORDER BY）
/// - `limit`: 可选的限制数量
///
/// # 返回
/// - 完整的 SQL 查询语句
///
/// # 示例
/// ```
/// use hot_rolling_aps::decision::common::sql_builder::build_query_with_optional_limit;
///
/// let base = "SELECT * FROM decision_test WHERE version_id = ? ORDER BY score DESC";
///
/// // 无限制
/// let sql1 = build_query_with_optional_limit(base, None);
/// assert_eq!(sql1, base);
///
/// // 有限制
/// let sql2 = build_query_with_optional_limit(base, Some(10));
/// assert_eq!(sql2, "SELECT * FROM decision_test WHERE version_id = ? ORDER BY score DESC LIMIT 10");
/// ```
pub fn build_query_with_optional_limit(base_query: &str, limit: Option<usize>) -> String {
    if let Some(n) = limit {
        format!("{} LIMIT {}", base_query, n)
    } else {
        base_query.to_string()
    }
}

/// 构建带日期范围过滤的 SQL WHERE 子句
///
/// # 功能
/// - 生成日期范围过滤的 WHERE 条件
/// - 支持开始日期和结束日期
///
/// # 参数
/// - `date_column`: 日期列名
/// - `start_date`: 开始日期（包含）
/// - `end_date`: 结束日期（不包含）
///
/// # 返回
/// - WHERE 子句片段，例如: "plan_date >= ? AND plan_date < ?"
///
/// # 示例
/// ```
/// use hot_rolling_aps::decision::common::sql_builder::build_date_range_filter;
///
/// let filter = build_date_range_filter("plan_date", "2026-01-24", "2026-01-31");
/// assert_eq!(filter, "plan_date >= '2026-01-24' AND plan_date < '2026-01-31'");
/// ```
pub fn build_date_range_filter(date_column: &str, start_date: &str, end_date: &str) -> String {
    format!(
        "{} >= '{}' AND {} < '{}'",
        date_column, start_date, date_column, end_date
    )
}

/// SQL 查询构建器（流式 API）
///
/// # 功能
/// - 提供链式调用的 SQL 构建接口
/// - 支持动态添加条件、排序、限制
///
/// # 示例
/// ```
/// use hot_rolling_aps::decision::common::sql_builder::SqlQueryBuilder;
///
/// let sql = SqlQueryBuilder::new("SELECT * FROM decision_test")
///     .where_clause("version_id = ?")
///     .and_if(Some("machine_code = ?"))
///     .order_by("score DESC")
///     .limit(10)
///     .build();
///
/// assert!(sql.contains("WHERE version_id = ?"));
/// assert!(sql.contains("AND machine_code = ?"));
/// assert!(sql.contains("ORDER BY score DESC"));
/// assert!(sql.contains("LIMIT 10"));
/// ```
#[derive(Debug, Clone)]
pub struct SqlQueryBuilder {
    select_clause: String,
    where_clauses: Vec<String>,
    order_by_clause: Option<String>,
    limit_clause: Option<usize>,
}

impl SqlQueryBuilder {
    /// 创建新的 SQL 查询构建器
    pub fn new(select: &str) -> Self {
        Self {
            select_clause: select.to_string(),
            where_clauses: Vec::new(),
            order_by_clause: None,
            limit_clause: None,
        }
    }

    /// 添加 WHERE 条件
    pub fn where_clause(mut self, condition: &str) -> Self {
        self.where_clauses.push(condition.to_string());
        self
    }

    /// 条件添加 AND 子句
    pub fn and_if(mut self, condition: Option<&str>) -> Self {
        if let Some(cond) = condition {
            self.where_clauses.push(cond.to_string());
        }
        self
    }

    /// 添加 ORDER BY 子句
    pub fn order_by(mut self, order: &str) -> Self {
        self.order_by_clause = Some(order.to_string());
        self
    }

    /// 添加 LIMIT 子句
    pub fn limit(mut self, n: usize) -> Self {
        self.limit_clause = Some(n);
        self
    }

    /// 构建最终的 SQL 语句
    pub fn build(&self) -> String {
        let mut sql = self.select_clause.clone();

        // 添加 WHERE 条件
        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.where_clauses.join(" AND "));
        }

        // 添加 ORDER BY
        if let Some(order) = &self.order_by_clause {
            sql.push_str(" ORDER BY ");
            sql.push_str(order);
        }

        // 添加 LIMIT
        if let Some(limit) = self.limit_clause {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        sql
    }
}

// ==========================================
// 单元测试
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================
    // build_optional_filter_sql 测试
    // ==========================================

    #[test]
    fn test_build_optional_filter_sql_without_filter() {
        let base = "SELECT * FROM decision_test WHERE version_id = ?";
        let order = "created_at DESC";
        let sql = build_optional_filter_sql(base, None, order);

        assert_eq!(
            sql,
            "SELECT * FROM decision_test WHERE version_id = ? ORDER BY created_at DESC"
        );
    }

    #[test]
    fn test_build_optional_filter_sql_with_filter() {
        let base = "SELECT * FROM decision_test WHERE version_id = ?";
        let order = "score DESC";
        let sql = build_optional_filter_sql(base, Some("machine_code = ?"), order);

        assert_eq!(
            sql,
            "SELECT * FROM decision_test WHERE version_id = ? AND machine_code = ? ORDER BY score DESC"
        );
    }

    #[test]
    fn test_build_optional_filter_sql_complex_filter() {
        let base = "SELECT * FROM decision_test WHERE version_id = ?";
        let order = "plan_date ASC, machine_code ASC";
        let sql = build_optional_filter_sql(
            base,
            Some("machine_code IN (?, ?) AND plan_date >= ?"),
            order,
        );

        assert!(sql.contains("AND machine_code IN (?, ?)"));
        assert!(sql.contains("ORDER BY plan_date ASC, machine_code ASC"));
    }

    // ==========================================
    // build_query_with_optional_limit 测试
    // ==========================================

    #[test]
    fn test_build_query_with_optional_limit_no_limit() {
        let base = "SELECT * FROM decision_test ORDER BY score DESC";
        let sql = build_query_with_optional_limit(base, None);

        assert_eq!(sql, base);
    }

    #[test]
    fn test_build_query_with_optional_limit_with_limit() {
        let base = "SELECT * FROM decision_test ORDER BY score DESC";
        let sql = build_query_with_optional_limit(base, Some(10));

        assert_eq!(
            sql,
            "SELECT * FROM decision_test ORDER BY score DESC LIMIT 10"
        );
    }

    #[test]
    fn test_build_query_with_optional_limit_zero() {
        let base = "SELECT * FROM decision_test";
        let sql = build_query_with_optional_limit(base, Some(0));

        assert_eq!(sql, "SELECT * FROM decision_test LIMIT 0");
    }

    // ==========================================
    // build_date_range_filter 测试
    // ==========================================

    #[test]
    fn test_build_date_range_filter() {
        let filter = build_date_range_filter("plan_date", "2026-01-24", "2026-01-31");
        assert_eq!(
            filter,
            "plan_date >= '2026-01-24' AND plan_date < '2026-01-31'"
        );
    }

    #[test]
    fn test_build_date_range_filter_different_column() {
        let filter = build_date_range_filter("created_at", "2026-01-01", "2026-02-01");
        assert_eq!(
            filter,
            "created_at >= '2026-01-01' AND created_at < '2026-02-01'"
        );
    }

    // ==========================================
    // SqlQueryBuilder 测试
    // ==========================================

    #[test]
    fn test_sql_builder_basic() {
        let sql = SqlQueryBuilder::new("SELECT * FROM decision_test")
            .where_clause("version_id = ?")
            .build();

        assert_eq!(sql, "SELECT * FROM decision_test WHERE version_id = ?");
    }

    #[test]
    fn test_sql_builder_with_order_by() {
        let sql = SqlQueryBuilder::new("SELECT * FROM decision_test")
            .where_clause("version_id = ?")
            .order_by("score DESC")
            .build();

        assert_eq!(
            sql,
            "SELECT * FROM decision_test WHERE version_id = ? ORDER BY score DESC"
        );
    }

    #[test]
    fn test_sql_builder_with_limit() {
        let sql = SqlQueryBuilder::new("SELECT * FROM decision_test")
            .where_clause("version_id = ?")
            .order_by("score DESC")
            .limit(10)
            .build();

        assert_eq!(
            sql,
            "SELECT * FROM decision_test WHERE version_id = ? ORDER BY score DESC LIMIT 10"
        );
    }

    #[test]
    fn test_sql_builder_multiple_where_clauses() {
        let sql = SqlQueryBuilder::new("SELECT * FROM decision_test")
            .where_clause("version_id = ?")
            .where_clause("machine_code = ?")
            .where_clause("plan_date >= ?")
            .build();

        assert!(sql.contains("WHERE version_id = ?"));
        assert!(sql.contains("AND machine_code = ?"));
        assert!(sql.contains("AND plan_date >= ?"));
    }

    #[test]
    fn test_sql_builder_and_if_with_some() {
        let sql = SqlQueryBuilder::new("SELECT * FROM decision_test")
            .where_clause("version_id = ?")
            .and_if(Some("machine_code = ?"))
            .order_by("score DESC")
            .build();

        assert!(sql.contains("AND machine_code = ?"));
    }

    #[test]
    fn test_sql_builder_and_if_with_none() {
        let sql = SqlQueryBuilder::new("SELECT * FROM decision_test")
            .where_clause("version_id = ?")
            .and_if(None)
            .order_by("score DESC")
            .build();

        assert!(!sql.contains("machine_code"));
        assert_eq!(
            sql,
            "SELECT * FROM decision_test WHERE version_id = ? ORDER BY score DESC"
        );
    }

    #[test]
    fn test_sql_builder_complex_query() {
        let machine_filter = Some("machine_code IN (?, ?)");

        let sql = SqlQueryBuilder::new("SELECT * FROM decision_capacity_opportunity")
            .where_clause("version_id = ?")
            .and_if(machine_filter)
            .where_clause("plan_date >= ?")
            .where_clause("plan_date < ?")
            .order_by("slack_t DESC")
            .limit(10)
            .build();

        assert!(sql.contains("WHERE version_id = ?"));
        assert!(sql.contains("AND machine_code IN (?, ?)"));
        assert!(sql.contains("AND plan_date >= ?"));
        assert!(sql.contains("ORDER BY slack_t DESC"));
        assert!(sql.contains("LIMIT 10"));
    }

    #[test]
    fn test_sql_builder_no_where_clause() {
        let sql = SqlQueryBuilder::new("SELECT COUNT(*) FROM decision_test")
            .order_by("created_at DESC")
            .limit(5)
            .build();

        assert_eq!(
            sql,
            "SELECT COUNT(*) FROM decision_test ORDER BY created_at DESC LIMIT 5"
        );
    }

    #[test]
    fn test_sql_builder_chainable() {
        // 测试链式调用的流畅性
        let builder = SqlQueryBuilder::new("SELECT *")
            .where_clause("a = ?")
            .where_clause("b = ?")
            .and_if(Some("c = ?"))
            .order_by("d DESC")
            .limit(100);

        let sql = builder.build();
        assert!(sql.contains("WHERE a = ? AND b = ? AND c = ?"));
        assert!(sql.contains("ORDER BY d DESC"));
        assert!(sql.contains("LIMIT 100"));
    }
}

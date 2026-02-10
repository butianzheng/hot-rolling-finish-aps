// ==========================================
// 热轧精整排产系统 - Decision 层公共工具模块
// ==========================================
// 职责: 提供仓储层公共工具函数
// 目标: 减少代码重复，提升可维护性
// ==========================================

/// JSON 序列化/反序列化工具
pub mod json_utils;

/// 数据库操作工具
pub mod db_utils;

/// SQL 构建工具
pub mod sql_builder;

// 重新导出常用函数
pub use db_utils::{build_in_clause, execute_delete_with_in_clause};
pub use json_utils::{
    deserialize_json_array, deserialize_json_array_optional, deserialize_json_optional,
    serialize_json_optional, serialize_json_vec,
};
pub use sql_builder::build_optional_filter_sql;

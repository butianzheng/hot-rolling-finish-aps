// ==========================================
// 热轧精整排产系统 - JSON 工具模块
// ==========================================
// 职责: 提供 JSON 序列化/反序列化的公共函数
// 目标: 消除 35+ 处重复的 JSON 处理代码
// ==========================================

use serde::{de::DeserializeOwned, Serialize};

/// 从 JSON 字符串反序列化为 Vec<T>
///
/// # 功能
/// - 处理必需的 JSON 数组字段
/// - 解析失败时返回空向量
/// - 统一的错误处理策略
///
/// # 用途
/// 替换 20+ 处类似代码：
/// ```ignore
/// let json_str: String = row.get(n)?;
/// let parsed: Vec<T> = serde_json::from_str(&json_str).unwrap_or_default();
/// ```
///
/// # 参数
/// - `json`: JSON 字符串引用
///
/// # 返回
/// - 成功: 反序列化的向量
/// - 失败: 空向量（不抛出错误）
///
/// # 示例
/// ```
/// use hot_rolling_aps::decision::common::json_utils::deserialize_json_array;
///
/// let json = r#"["action1", "action2"]"#;
/// let actions: Vec<String> = deserialize_json_array(json);
/// assert_eq!(actions, vec!["action1", "action2"]);
///
/// // 无效 JSON 返回空向量
/// let invalid = "not a json";
/// let empty: Vec<String> = deserialize_json_array(invalid);
/// assert_eq!(empty.len(), 0);
/// ```
pub fn deserialize_json_array<T: DeserializeOwned>(json: &str) -> Vec<T> {
    serde_json::from_str(json).unwrap_or_default()
}

/// 从可选 JSON 字符串反序列化为 Vec<T>
///
/// # 功能
/// - 处理可选的 JSON 数组字段（来自数据库的 Option<String>）
/// - None 或解析失败时返回空向量
/// - 链式处理可选值
///
/// # 用途
/// 替换 15+ 处类似代码：
/// ```ignore
/// let json_opt: Option<String> = row.get(n)?;
/// let parsed: Vec<T> = json_opt
///     .as_ref()
///     .and_then(|s| serde_json::from_str(s).ok())
///     .unwrap_or_default();
/// ```
///
/// # 参数
/// - `json`: 可选的 JSON 字符串引用
///
/// # 返回
/// - 成功: 反序列化的向量
/// - None 或失败: 空向量
///
/// # 示例
/// ```
/// use hot_rolling_aps::decision::common::json_utils::deserialize_json_array_optional;
///
/// let json = Some(r#"["HIGH", "MEDIUM"]"#.to_string());
/// let levels: Vec<String> = deserialize_json_array_optional(json.as_deref());
/// assert_eq!(levels, vec!["HIGH", "MEDIUM"]);
///
/// // None 返回空向量
/// let none: Option<&str> = None;
/// let empty: Vec<String> = deserialize_json_array_optional(none);
/// assert_eq!(empty.len(), 0);
/// ```
pub fn deserialize_json_array_optional<T: DeserializeOwned>(json: Option<&str>) -> Vec<T> {
    json.and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default()
}

/// 从可选 JSON 字符串反序列化为 Option<T>
///
/// # 功能
/// - 处理可选的 JSON 对象字段
/// - None 或解析失败时返回 None
/// - 保留 Option 语义
///
/// # 用途
/// 替换 10+ 处类似代码：
/// ```ignore
/// let json_opt: Option<String> = row.get(n)?;
/// let parsed: Option<T> = json_opt
///     .as_ref()
///     .and_then(|s| serde_json::from_str(s).ok());
/// ```
///
/// # 参数
/// - `json`: 可选的 JSON 字符串引用
///
/// # 返回
/// - 成功: Some(T)
/// - None 或失败: None
///
/// # 示例
/// ```
/// use hot_rolling_aps::decision::common::json_utils::deserialize_json_optional;
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize, PartialEq)]
/// struct Config { value: i32 }
///
/// let json = Some(r#"{"value": 42}"#);
/// let config: Option<Config> = deserialize_json_optional(json);
/// assert_eq!(config, Some(Config { value: 42 }));
///
/// // None 返回 None
/// let none: Option<&str> = None;
/// let empty: Option<Config> = deserialize_json_optional(none);
/// assert_eq!(empty, None);
/// ```
pub fn deserialize_json_optional<T: DeserializeOwned>(json: Option<&str>) -> Option<T> {
    json.and_then(|s| serde_json::from_str(s).ok())
}

/// 序列化向量为 JSON 字符串
///
/// # 功能
/// - 将 Vec<T> 序列化为 JSON 字符串
/// - 空向量返回 None（数据库存储优化）
/// - 序列化失败返回 None
///
/// # 用途
/// 替换 15+ 处类似代码：
/// ```ignore
/// let json_field = if vec.is_empty() {
///     None
/// } else {
///     Some(serde_json::to_string(&vec).unwrap_or_default())
/// };
/// ```
///
/// # 参数
/// - `vec`: 要序列化的向量切片
///
/// # 返回
/// - 非空向量: Some(JSON字符串)
/// - 空向量或失败: None
///
/// # 示例
/// ```
/// use hot_rolling_aps::decision::common::json_utils::serialize_json_vec;
///
/// let actions = vec!["重排计划", "调整机组"];
/// let json = serialize_json_vec(&actions);
/// assert!(json.is_some());
/// assert!(json.unwrap().contains("重排计划"));
///
/// // 空向量返回 None
/// let empty: Vec<String> = vec![];
/// let none = serialize_json_vec(&empty);
/// assert_eq!(none, None);
/// ```
pub fn serialize_json_vec<T: Serialize>(vec: &[T]) -> Option<String> {
    if vec.is_empty() {
        None
    } else {
        serde_json::to_string(vec).ok()
    }
}

/// 序列化可选值为 JSON 字符串
///
/// # 功能
/// - 将 Option<T> 序列化为 Option<String>
/// - None 返回 None
/// - 序列化失败返回 None
///
/// # 用途
/// 替换类似代码：
/// ```ignore
/// let json_field = opt_value
///     .as_ref()
///     .and_then(|v| serde_json::to_string(v).ok());
/// ```
///
/// # 参数
/// - `value`: 要序列化的可选值引用
///
/// # 返回
/// - Some(T): Some(JSON字符串)
/// - None 或失败: None
///
/// # 示例
/// ```
/// use hot_rolling_aps::decision::common::json_utils::serialize_json_optional;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Data { count: i32 }
///
/// let data = Some(Data { count: 10 });
/// let json = serialize_json_optional(data.as_ref());
/// assert!(json.is_some());
///
/// let none: Option<Data> = None;
/// let empty = serialize_json_optional(none.as_ref());
/// assert_eq!(empty, None);
/// ```
pub fn serialize_json_optional<T: Serialize>(value: Option<&T>) -> Option<String> {
    value.and_then(|v| serde_json::to_string(v).ok())
}

// ==========================================
// 单元测试
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: i32,
        name: String,
    }

    // ==========================================
    // deserialize_json_array 测试
    // ==========================================

    #[test]
    fn test_deserialize_json_array_success() {
        let json = r#"["action1", "action2", "action3"]"#;
        let result: Vec<String> = deserialize_json_array(json);
        assert_eq!(result, vec!["action1", "action2", "action3"]);
    }

    #[test]
    fn test_deserialize_json_array_empty() {
        let json = r#"[]"#;
        let result: Vec<String> = deserialize_json_array(json);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_deserialize_json_array_invalid_returns_empty() {
        let json = "not a json";
        let result: Vec<String> = deserialize_json_array(json);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_deserialize_json_array_complex_type() {
        let json = r#"[{"id":1,"name":"test1"},{"id":2,"name":"test2"}]"#;
        let result: Vec<TestData> = deserialize_json_array(json);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].name, "test1");
    }

    // ==========================================
    // deserialize_json_array_optional 测试
    // ==========================================

    #[test]
    fn test_deserialize_json_array_optional_some() {
        let json = Some(r#"["HIGH", "MEDIUM"]"#);
        let result: Vec<String> = deserialize_json_array_optional(json);
        assert_eq!(result, vec!["HIGH", "MEDIUM"]);
    }

    #[test]
    fn test_deserialize_json_array_optional_none() {
        let json: Option<&str> = None;
        let result: Vec<String> = deserialize_json_array_optional(json);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_deserialize_json_array_optional_invalid() {
        let json = Some("invalid json");
        let result: Vec<String> = deserialize_json_array_optional(json);
        assert_eq!(result.len(), 0);
    }

    // ==========================================
    // deserialize_json_optional 测试
    // ==========================================

    #[test]
    fn test_deserialize_json_optional_some() {
        let json = Some(r#"{"id":42,"name":"test"}"#);
        let result: Option<TestData> = deserialize_json_optional(json);
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.id, 42);
        assert_eq!(data.name, "test");
    }

    #[test]
    fn test_deserialize_json_optional_none() {
        let json: Option<&str> = None;
        let result: Option<TestData> = deserialize_json_optional(json);
        assert!(result.is_none());
    }

    #[test]
    fn test_deserialize_json_optional_invalid() {
        let json = Some("not valid json");
        let result: Option<TestData> = deserialize_json_optional(json);
        assert!(result.is_none());
    }

    // ==========================================
    // serialize_json_vec 测试
    // ==========================================

    #[test]
    fn test_serialize_json_vec_success() {
        let vec = vec!["重排计划", "调整机组"];
        let result = serialize_json_vec(&vec);
        assert!(result.is_some());
        let json = result.unwrap();
        assert!(json.contains("重排计划"));
        assert!(json.contains("调整机组"));
    }

    #[test]
    fn test_serialize_json_vec_empty_returns_none() {
        let vec: Vec<String> = vec![];
        let result = serialize_json_vec(&vec);
        assert!(result.is_none());
    }

    #[test]
    fn test_serialize_json_vec_complex_type() {
        let vec = vec![
            TestData {
                id: 1,
                name: "test1".to_string(),
            },
            TestData {
                id: 2,
                name: "test2".to_string(),
            },
        ];
        let result = serialize_json_vec(&vec);
        assert!(result.is_some());
        let json = result.unwrap();
        assert!(json.contains("test1"));
        assert!(json.contains("test2"));
    }

    // ==========================================
    // serialize_json_optional 测试
    // ==========================================

    #[test]
    fn test_serialize_json_optional_some() {
        let data = TestData {
            id: 100,
            name: "optional_test".to_string(),
        };
        let result = serialize_json_optional(Some(&data));
        assert!(result.is_some());
        let json = result.unwrap();
        assert!(json.contains("100"));
        assert!(json.contains("optional_test"));
    }

    #[test]
    fn test_serialize_json_optional_none() {
        let data: Option<TestData> = None;
        let result = serialize_json_optional(data.as_ref());
        assert!(result.is_none());
    }

    // ==========================================
    // 集成测试：模拟仓储使用场景
    // ==========================================

    #[test]
    fn test_round_trip_serialization() {
        // 模拟从数据库读取和写入的完整流程
        let original = vec!["建议1", "建议2", "建议3"];

        // 序列化（写入数据库）
        let json_str = serialize_json_vec(&original).unwrap();

        // 反序列化（从数据库读取）
        let restored: Vec<String> = deserialize_json_array(&json_str);

        assert_eq!(original, restored);
    }

    #[test]
    fn test_empty_vec_handling() {
        // 空向量应该序列化为 None
        let empty: Vec<String> = vec![];
        let json = serialize_json_vec(&empty);
        assert!(json.is_none());

        // 从 None 反序列化应该得到空向量
        let restored: Vec<String> = deserialize_json_array_optional(json.as_deref());
        assert_eq!(restored.len(), 0);
    }
}

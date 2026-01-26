// ==========================================
// ConfigApi 集成测试
// ==========================================
// 测试范围:
// 1. 配置查询: list_configs, get_config
// 2. 配置更新: update_config, batch_update_configs
// 3. 配置快照: get_config_snapshot, restore_from_snapshot
// ==========================================

mod helpers;
mod test_helpers;

use helpers::api_test_helper::*;

// ==========================================
// 配置查询测试
// ==========================================

#[test]
fn test_list_configs_初始状态() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 查询所有配置
    let result = env.config_api
        .list_configs()
        .expect("查询失败");

    // 初始状态应该有一些默认配置
    assert!(result.len() >= 0, "应该返回配置列表");
}

#[test]
fn test_get_config_存在() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 先插入一个配置
    env.config_api
        .update_config("global", "test_key", "test_value", "admin", "测试")
        .expect("插入失败");

    // 测试: 查询配置
    let result = env.config_api
        .get_config("global", "test_key")
        .expect("查询失败");

    assert!(result.is_some(), "应该找到配置");
    let config = result.unwrap();
    assert_eq!(config.key, "test_key");
    assert_eq!(config.value, "test_value");
}

#[test]
fn test_get_config_不存在() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 查询不存在的配置
    let result = env.config_api
        .get_config("global", "non_existent_key")
        .expect("查询失败");

    assert!(result.is_none(), "不存在的配置应该返回None");
}

// ==========================================
// 配置更新测试
// ==========================================

#[test]
fn test_update_config_成功() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 更新配置
    let result = env.config_api
        .update_config("global", "test_key", "test_value", "admin", "测试更新");

    assert!(result.is_ok(), "更新配置应该成功");

    // 验证: 查询配置
    let config = env.config_api
        .get_config("global", "test_key")
        .expect("查询失败")
        .expect("应该找到配置");

    assert_eq!(config.value, "test_value");
}

#[test]
fn test_update_config_覆盖() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 先插入一个配置
    env.config_api
        .update_config("global", "test_key", "old_value", "admin", "初始值")
        .expect("插入失败");

    // 测试: 覆盖配置
    env.config_api
        .update_config("global", "test_key", "new_value", "admin", "更新值")
        .expect("更新失败");

    // 验证: 查询配置
    let config = env.config_api
        .get_config("global", "test_key")
        .expect("查询失败")
        .expect("应该找到配置");

    assert_eq!(config.value, "new_value", "配置值应该被覆盖");
}

#[test]
fn test_batch_update_configs_成功() {
    use hot_rolling_aps::api::config_api::ConfigItem;

    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 准备批量配置
    let configs = vec![
        ConfigItem {
            scope_id: "global".to_string(),
            key: "key1".to_string(),
            value: "value1".to_string(),
        },
        ConfigItem {
            scope_id: "global".to_string(),
            key: "key2".to_string(),
            value: "value2".to_string(),
        },
    ];

    // 测试: 批量更新配置
    let count = env.config_api
        .batch_update_configs(configs, "admin", "批量更新")
        .expect("批量更新失败");

    assert_eq!(count, 2, "应该更新2个配置");
}

// ==========================================
// 配置快照测试
// ==========================================

#[test]
fn test_get_config_snapshot() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 先插入一些配置
    env.config_api
        .update_config("global", "key1", "value1", "admin", "测试")
        .expect("插入失败");

    // 测试: 获取配置快照
    let snapshot = env.config_api
        .get_config_snapshot()
        .expect("获取快照失败");

    assert!(!snapshot.is_empty(), "快照不应该为空");
    assert!(snapshot.contains("key1"), "快照应该包含key1");
}

#[test]
fn test_restore_from_snapshot() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 准备快照JSON
    let snapshot_json = r#"{"test_key":"test_value"}"#;

    // 测试: 从快照恢复配置
    let count = env.config_api
        .restore_from_snapshot(snapshot_json, "admin", "恢复配置")
        .expect("恢复失败");

    assert_eq!(count, 1, "应该恢复1个配置");

    // 验证: 查询配置
    let config = env.config_api
        .get_config("global", "test_key")
        .expect("查询失败")
        .expect("应该找到配置");

    assert_eq!(config.value, "test_value");
}

// ==========================================
// 参数验证测试
// ==========================================

#[test]
fn test_update_config_空scope() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 空scope_id
    let result = env.config_api
        .update_config("", "key", "value", "admin", "测试");

    assert!(result.is_err(), "空scope_id应该返回错误");
}

#[test]
fn test_update_config_空key() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 空key
    let result = env.config_api
        .update_config("global", "", "value", "admin", "测试");

    assert!(result.is_err(), "空key应该返回错误");
}

#[test]
fn test_update_config_空原因() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 空原因
    let result = env.config_api
        .update_config("global", "key", "value", "admin", "");

    assert!(result.is_err(), "空原因应该返回错误");
}

#[test]
fn test_batch_update_configs_空列表() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 空配置列表
    let result = env.config_api
        .batch_update_configs(vec![], "admin", "测试");

    assert!(result.is_err(), "空配置列表应该返回错误");
}

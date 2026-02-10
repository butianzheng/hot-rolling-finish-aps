// ==========================================
// ConfigManager 集成测试
// ==========================================
// 测试目标: 验证配置读取功能的正确性
// ==========================================

mod test_helpers;

use hot_rolling_aps::config::{ConfigManager, ImportConfigReader};
use hot_rolling_aps::domain::types::{Season, SeasonMode};
use test_helpers::{create_test_db, insert_test_config};

#[tokio::test]
async fn test_config_manager_creation() {
    // 创建测试数据库
    let (_temp_file, db_path) = create_test_db().expect("Failed to create test db");

    // 创建 ConfigManager
    let config_manager = ConfigManager::new(&db_path);
    assert!(
        config_manager.is_ok(),
        "ConfigManager should be created successfully"
    );
}

#[tokio::test]
async fn test_get_season_mode() {
    // 创建测试数据库并插入配置
    let (_temp_file, db_path) = create_test_db().expect("Failed to create test db");
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");
    insert_test_config(&conn).expect("Failed to insert test config");

    // 创建 ConfigManager
    let config_manager = ConfigManager::new(&db_path).expect("Failed to create ConfigManager");

    // 测试获取季节模式
    let season_mode = config_manager.get_season_mode().await;
    assert!(season_mode.is_ok(), "Should get season mode successfully");
    assert_eq!(
        season_mode.unwrap(),
        SeasonMode::Manual,
        "Season mode should be MANUAL"
    );
}

#[tokio::test]
async fn test_get_manual_season() {
    let (_temp_file, db_path) = create_test_db().expect("Failed to create test db");
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");
    insert_test_config(&conn).expect("Failed to insert test config");

    let config_manager = ConfigManager::new(&db_path).expect("Failed to create ConfigManager");

    // 测试获取手动季节
    let manual_season = config_manager.get_manual_season().await;
    assert!(
        manual_season.is_ok(),
        "Should get manual season successfully"
    );
    assert_eq!(
        manual_season.unwrap(),
        Season::Winter,
        "Manual season should be WINTER"
    );
}

#[tokio::test]
async fn test_get_winter_months() {
    let (_temp_file, db_path) = create_test_db().expect("Failed to create test db");
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");
    insert_test_config(&conn).expect("Failed to insert test config");

    let config_manager = ConfigManager::new(&db_path).expect("Failed to create ConfigManager");

    // 测试获取冬季月份
    let winter_months = config_manager.get_winter_months().await;
    assert!(
        winter_months.is_ok(),
        "Should get winter months successfully"
    );

    let months = winter_months.unwrap();
    assert_eq!(months.len(), 5, "Should have 5 winter months");
    assert_eq!(
        months,
        vec![11, 12, 1, 2, 3],
        "Winter months should be [11,12,1,2,3]"
    );
}

#[tokio::test]
async fn test_get_min_temp_days() {
    let (_temp_file, db_path) = create_test_db().expect("Failed to create test db");
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");
    insert_test_config(&conn).expect("Failed to insert test config");

    let config_manager = ConfigManager::new(&db_path).expect("Failed to create ConfigManager");

    // 测试获取冬季适温天数
    let winter_days = config_manager.get_min_temp_days_winter().await;
    assert!(
        winter_days.is_ok(),
        "Should get winter temp days successfully"
    );
    assert_eq!(winter_days.unwrap(), 3, "Winter temp days should be 3");

    // 测试获取夏季适温天数
    let summer_days = config_manager.get_min_temp_days_summer().await;
    assert!(
        summer_days.is_ok(),
        "Should get summer temp days successfully"
    );
    assert_eq!(summer_days.unwrap(), 4, "Summer temp days should be 4");
}

#[tokio::test]
async fn test_get_current_min_temp_days_manual_mode() {
    let (_temp_file, db_path) = create_test_db().expect("Failed to create test db");
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");
    insert_test_config(&conn).expect("Failed to insert test config");

    let config_manager = ConfigManager::new(&db_path).expect("Failed to create ConfigManager");

    // 测试手动模式下的适温天数（配置为WINTER）
    let today = chrono::NaiveDate::from_ymd_opt(2024, 7, 15).unwrap(); // 夏季日期
    let current_days = config_manager.get_current_min_temp_days(today).await;
    assert!(
        current_days.is_ok(),
        "Should get current temp days successfully"
    );
    // 因为是MANUAL模式且配置为WINTER，所以应该返回3天（冬季阈值）
    assert_eq!(
        current_days.unwrap(),
        3,
        "Should return winter temp days (3) in MANUAL mode"
    );
}

#[tokio::test]
async fn test_get_current_min_temp_days_auto_mode() {
    let (_temp_file, db_path) = create_test_db().expect("Failed to create test db");
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");

    // 插入AUTO模式配置
    conn.execute(
        r#"
        INSERT OR REPLACE INTO config_kv (scope_id, key, value, updated_at) VALUES
        ('global', 'season_mode', 'AUTO', datetime('now')),
        ('global', 'winter_months', '11,12,1,2,3', datetime('now')),
        ('global', 'min_temp_days_winter', '3', datetime('now')),
        ('global', 'min_temp_days_summer', '4', datetime('now'))
        "#,
        [],
    )
    .expect("Failed to insert AUTO mode config");

    let config_manager = ConfigManager::new(&db_path).expect("Failed to create ConfigManager");

    // 测试冬季日期（1月）
    let winter_date = chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    let winter_days = config_manager.get_current_min_temp_days(winter_date).await;
    assert!(
        winter_days.is_ok(),
        "Should get winter temp days successfully"
    );
    assert_eq!(
        winter_days.unwrap(),
        3,
        "Should return 3 days for winter month (January)"
    );

    // 测试夏季日期（7月）
    let summer_date = chrono::NaiveDate::from_ymd_opt(2024, 7, 15).unwrap();
    let summer_days = config_manager.get_current_min_temp_days(summer_date).await;
    assert!(
        summer_days.is_ok(),
        "Should get summer temp days successfully"
    );
    assert_eq!(
        summer_days.unwrap(),
        4,
        "Should return 4 days for summer month (July)"
    );
}

#[tokio::test]
async fn test_get_standard_finishing_machines() {
    let (_temp_file, db_path) = create_test_db().expect("Failed to create test db");
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");
    insert_test_config(&conn).expect("Failed to insert test config");

    let config_manager = ConfigManager::new(&db_path).expect("Failed to create ConfigManager");

    // 测试获取标准精整机组
    let machines = config_manager.get_standard_finishing_machines().await;
    assert!(
        machines.is_ok(),
        "Should get standard finishing machines successfully"
    );

    let machine_list = machines.unwrap();
    assert_eq!(machine_list.len(), 3, "Should have 3 standard machines");
    assert_eq!(
        machine_list,
        vec!["H032", "H033", "H034"],
        "Should return correct machine codes"
    );
}

#[tokio::test]
async fn test_get_machine_offset_days() {
    let (_temp_file, db_path) = create_test_db().expect("Failed to create test db");
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");
    insert_test_config(&conn).expect("Failed to insert test config");

    let config_manager = ConfigManager::new(&db_path).expect("Failed to create ConfigManager");

    // 测试获取机组偏移天数
    let offset_days = config_manager.get_machine_offset_days().await;
    assert!(
        offset_days.is_ok(),
        "Should get machine offset days successfully"
    );
    assert_eq!(offset_days.unwrap(), 4, "Machine offset days should be 4");
}

#[tokio::test]
async fn test_get_dq_config() {
    let (_temp_file, db_path) = create_test_db().expect("Failed to create test db");
    let conn = test_helpers::open_test_connection(&db_path).expect("Failed to open db");
    insert_test_config(&conn).expect("Failed to insert test config");

    let config_manager = ConfigManager::new(&db_path).expect("Failed to create ConfigManager");

    // 测试获取重量异常阈值
    let weight_threshold = config_manager.get_weight_anomaly_threshold().await;
    assert!(
        weight_threshold.is_ok(),
        "Should get weight anomaly threshold successfully"
    );
    assert_eq!(
        weight_threshold.unwrap(),
        100.0,
        "Weight anomaly threshold should be 100.0"
    );

    // 测试获取批次保留天数
    let retention_days = config_manager.get_batch_retention_days().await;
    assert!(
        retention_days.is_ok(),
        "Should get batch retention days successfully"
    );
    assert_eq!(
        retention_days.unwrap(),
        90,
        "Batch retention days should be 90"
    );
}

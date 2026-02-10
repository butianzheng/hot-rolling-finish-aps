// ==========================================
// RollerApi 集成测试
// ==========================================
// 测试范围:
// 1. 换辊窗口查询: list_campaigns, get_active_campaign, list_needs_roll_change
// 2. 换辊窗口管理: create_campaign, close_campaign
// ==========================================

mod helpers;
mod test_helpers;

use chrono::NaiveDate;
use helpers::api_test_helper::*;

// ==========================================
// 换辊窗口查询测试
// ==========================================

#[test]
fn test_list_campaigns_空结果() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案和版本
    let plan_id = env
        .plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    let version_id = env
        .plan_api
        .create_version(
            plan_id,
            30,
            None,
            Some("测试版本".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    // 测试: 查询换辊窗口（没有创建，应该为空）
    let campaigns = env
        .roller_api
        .list_campaigns(&version_id)
        .expect("查询失败");

    assert_eq!(campaigns.len(), 0, "未创建换辊窗口的版本应该返回空列表");
}

#[test]
fn test_get_active_campaign_不存在() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案和版本
    let plan_id = env
        .plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    let version_id = env
        .plan_api
        .create_version(
            plan_id,
            30,
            None,
            Some("测试版本".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    // 测试: 查询进行中的换辊窗口（不存在）
    let campaign = env
        .roller_api
        .get_active_campaign(&version_id, "H032")
        .expect("查询失败");

    assert!(campaign.is_none(), "不存在的换辊窗口应该返回None");
}

#[test]
fn test_list_needs_roll_change_空结果() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案和版本
    let plan_id = env
        .plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    let version_id = env
        .plan_api
        .create_version(
            plan_id,
            30,
            None,
            Some("测试版本".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    // 测试: 查询需要换辊的机组（没有创建，应该为空）
    let campaigns = env
        .roller_api
        .list_needs_roll_change(&version_id)
        .expect("查询失败");

    assert_eq!(campaigns.len(), 0, "没有换辊窗口时应该返回空列表");
}

// ==========================================
// 换辊窗口管理测试
// ==========================================

#[test]
fn test_create_campaign_成功() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案和版本
    let plan_id = env
        .plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    let version_id = env
        .plan_api
        .create_version(
            plan_id,
            30,
            None,
            Some("测试版本".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    // 测试: 创建换辊窗口
    let result = env.roller_api.create_campaign(
        &version_id,
        "H032",
        1,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        Some(1500.0),
        Some(2500.0),
        "admin",
        "创建换辊窗口",
    );

    assert!(result.is_ok(), "创建换辊窗口应该成功");

    // 验证: 查询换辊窗口
    let campaigns = env
        .roller_api
        .list_campaigns(&version_id)
        .expect("查询失败");

    assert_eq!(campaigns.len(), 1, "应该有1个换辊窗口");
    assert_eq!(campaigns[0].machine_code, "H032");
    assert_eq!(campaigns[0].campaign_no, 1);
}

#[test]
fn test_close_campaign_成功() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案和版本
    let plan_id = env
        .plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    let version_id = env
        .plan_api
        .create_version(
            plan_id,
            30,
            None,
            Some("测试版本".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    // 先创建换辊窗口
    env.roller_api
        .create_campaign(
            &version_id,
            "H032",
            1,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            Some(1500.0),
            Some(2500.0),
            "admin",
            "创建换辊窗口",
        )
        .expect("创建失败");

    // 测试: 结束换辊窗口
    let result = env.roller_api.close_campaign(
        &version_id,
        "H032",
        1,
        NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(),
        "admin",
        "结束换辊窗口",
    );

    assert!(result.is_ok(), "结束换辊窗口应该成功");

    // 验证: 查询进行中的换辊窗口（应该为空）
    let campaign = env
        .roller_api
        .get_active_campaign(&version_id, "H032")
        .expect("查询失败");

    assert!(campaign.is_none(), "结束后应该没有进行中的换辊窗口");
}

// ==========================================
// 参数验证测试
// ==========================================

#[test]
fn test_create_campaign_空版本ID() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 空版本ID
    let result = env.roller_api.create_campaign(
        "",
        "H032",
        1,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
        None,
        "admin",
        "测试",
    );

    assert!(result.is_err(), "空版本ID应该返回错误");
}

#[test]
fn test_create_campaign_空机组代码() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 空机组代码
    let result = env.roller_api.create_campaign(
        "version_id",
        "",
        1,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
        None,
        "admin",
        "测试",
    );

    assert!(result.is_err(), "空机组代码应该返回错误");
}

#[test]
fn test_create_campaign_空原因() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 空原因
    let result = env.roller_api.create_campaign(
        "version_id",
        "H032",
        1,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
        None,
        "admin",
        "",
    );

    assert!(result.is_err(), "空原因应该返回错误");
}

#[test]
fn test_list_campaigns_空版本ID() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 空版本ID
    let result = env.roller_api.list_campaigns("");

    assert!(result.is_err(), "空版本ID应该返回错误");
}

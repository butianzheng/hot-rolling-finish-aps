// ==========================================
// PlanApi 集成测试
// ==========================================
// 测试范围:
// 1. 方案管理: create_plan, list_plans, get_plan_detail
// 2. 版本管理: create_version, list_versions, activate_version
// 3. 排产计算: recalc_full
// 4. 查询: list_plan_items, list_items_by_date
// 5. 版本对比: compare_versions
// 6. 红线合规性: 冻结区保护、参数验证
// ==========================================

mod helpers;
mod test_helpers;

use chrono::NaiveDate;
use helpers::api_test_helper::*;
use helpers::test_data_builder::{CapacityPoolBuilder, MaterialBuilder, MaterialStateBuilder, PlanItemBuilder};
use hot_rolling_aps::api::ApiError;
use hot_rolling_aps::domain::types::SchedState;

// ==========================================
// 方案管理测试
// ==========================================

#[test]
fn test_create_plan_正常创建() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 创建方案
    let plan_id = env.plan_api
        .create_plan("2026年1月排产方案".to_string(), "admin".to_string())
        .expect("创建方案失败");

    assert!(!plan_id.is_empty(), "应该返回方案ID");

    // 验证: 方案已创建
    let plan = env.plan_repo
        .find_by_id(&plan_id)
        .expect("查询失败")
        .expect("方案不存在");

    assert_eq!(plan.plan_name, "2026年1月排产方案");
    assert_eq!(plan.created_by, "admin");
}

#[test]
fn test_list_plans_查询方案列表() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建2个方案
    env.plan_api
        .create_plan("方案A".to_string(), "admin".to_string())
        .expect("创建失败");

    env.plan_api
        .create_plan("方案B".to_string(), "admin".to_string())
        .expect("创建失败");

    // 测试: 查询方案列表
    let plans = env.plan_api
        .list_plans()
        .expect("查询失败");

    assert_eq!(plans.len(), 2, "应该返回2个方案");
}

#[test]
fn test_get_plan_detail_查询方案详情() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案
    let plan_id = env.plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    // 测试: 查询方案详情
    let plan = env.plan_api
        .get_plan_detail(&plan_id)
        .expect("查询失败")
        .expect("方案不存在");

    assert_eq!(plan.plan_id, plan_id);
    assert_eq!(plan.plan_name, "测试方案");
}

#[test]
fn test_get_plan_detail_不存在的方案() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 查询不存在的方案
    let result = env.plan_api
        .get_plan_detail("NOT_EXIST")
        .expect("查询失败");

    assert!(result.is_none(), "不应该找到方案");
}

// ==========================================
// 版本管理测试
// ==========================================

#[test]
fn test_create_version_正常创建() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案
    let plan_id = env.plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    // 测试: 创建版本
    let version_id = env.plan_api
        .create_version(
            plan_id.clone(),
            30,
            Some(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()),
            Some("初始版本".to_string()),
            "admin".to_string(),
        )
        .expect("创建版本失败");

    assert!(!version_id.is_empty(), "应该返回版本ID");

    // 验证: 版本已创建
    let version = env.plan_version_repo
        .find_by_id(&version_id)
        .expect("查询失败")
        .expect("版本不存在");

    assert_eq!(version.plan_id, plan_id);
    assert_eq!(version.created_by, Some("admin".to_string()));
}

#[test]
fn test_list_versions_查询版本列表() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案
    let plan_id = env.plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    // 创建2个版本
    env.plan_api
        .create_version(
            plan_id.clone(),
            30,
            None,
            Some("版本1".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    env.plan_api
        .create_version(
            plan_id.clone(),
            30,
            None,
            Some("版本2".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    // 测试: 查询版本列表
    let versions = env.plan_api
        .list_versions(&plan_id)
        .expect("查询失败");

    assert_eq!(versions.len(), 2, "应该返回2个版本");
}

#[test]
fn test_activate_version_激活版本() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案和版本
    let plan_id = env.plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    let version_id = env.plan_api
        .create_version(
            plan_id,
            30,
            None,
            Some("测试版本".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    // 测试: 激活版本
    env.plan_api
        .activate_version(&version_id, "admin")
        .expect("激活失败");

    // 验证: 版本状态已更新
    let version = env.plan_version_repo
        .find_by_id(&version_id)
        .expect("查询失败")
        .expect("版本不存在");

    assert_eq!(version.status, "ACTIVE");
}

// ==========================================
// 排产计算测试
// ==========================================

#[test]
fn test_recalc_full_基本场景() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 准备测试数据
    let materials = vec![
        MaterialBuilder::new("M001")
            .machine("M1")
            .weight(100.0)
            .build(),
        MaterialBuilder::new("M002")
            .machine("M1")
            .weight(150.0)
            .build(),
    ];

    let states = vec![
        MaterialStateBuilder::new("M001")
            .sched_state(SchedState::Ready)
            .build(),
        MaterialStateBuilder::new("M002")
            .sched_state(SchedState::Ready)
            .build(),
    ];

    env.prepare_materials(materials, states).unwrap();

    // 准备产能池
    let base_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let pools = vec![
        CapacityPoolBuilder::new("M1", base_date)
            .target(800.0)
            .limit(900.0)
            .build(),
        CapacityPoolBuilder::new("M1", base_date + chrono::Duration::days(1))
            .target(800.0)
            .limit(900.0)
            .build(),
    ];

    env.prepare_capacity_pools(pools).unwrap();

    // 创建方案和版本
    let plan_id = env.plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    let version_id = env.plan_api
        .create_version(
            plan_id,
            7, // 窗口7天
            None,
            Some("测试版本".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    // 测试: 一键重算
    let result = env.plan_api
        .recalc_full(
            &version_id,
            base_date,
            None,
            "admin",
        )
        .expect("重算失败");

    // 验证: 返回结果包含统计信息
    assert!(result.plan_items_count >= 0, "应该返回排产材料数量");
    assert!(result.success, "重算应该成功");

    // 验证: ActionLog已记录
    assert_action_logged(&env, "RECALC_FULL", 1).unwrap();
}

// ==========================================
// 查询测试
// ==========================================

#[test]
fn test_list_plan_items_查询排产明细() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案和版本
    let plan_id = env.plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    let version_id = env.plan_api
        .create_version(
            plan_id,
            30,
            None,
            Some("测试版本".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    // 测试: 查询排产明细（空结果）
    let items = env.plan_api
        .list_plan_items(&version_id)
        .expect("查询失败");

    assert_eq!(items.len(), 0, "新版本应该没有排产明细");
}

#[test]
fn test_list_items_by_date_按日期查询() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案和版本
    let plan_id = env.plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    let version_id = env.plan_api
        .create_version(
            plan_id,
            30,
            None,
            Some("测试版本".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    // 测试: 按日期查询排产明细
    let date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
    let items = env.plan_api
        .list_items_by_date(&version_id, date)
        .expect("查询失败");

    assert_eq!(items.len(), 0, "指定日期应该没有排产明细");
}

// ==========================================
// 版本对比测试
// ==========================================

#[test]
fn test_compare_versions_空版本对比() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案和2个版本
    let plan_id = env.plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    let version_a = env.plan_api
        .create_version(
            plan_id.clone(),
            30,
            None,
            Some("版本A".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    let version_b = env.plan_api
        .create_version(
            plan_id,
            30,
            None,
            Some("版本B".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    // 测试: 版本对比（两个空版本）
    let result = env.plan_api
        .compare_versions(&version_a, &version_b)
        .expect("对比失败");

    // 验证: 无差异
    assert_eq!(result.added_count, 0);
    assert_eq!(result.removed_count, 0);
    assert_eq!(result.moved_count, 0);
}

#[test]
fn test_compare_versions_kpi_聚合统计() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案和2个版本
    let plan_id = env
        .plan_api
        .create_plan("KPI对比测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    let version_a = env
        .plan_api
        .create_version(
            plan_id.clone(),
            30,
            None,
            Some("版本A".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    let version_b = env
        .plan_api
        .create_version(
            plan_id,
            30,
            None,
            Some("版本B".to_string()),
            "admin".to_string(),
        )
        .expect("创建失败");

    // 准备 material_master（满足 plan_item 外键）
    env.material_master_repo
        .batch_insert_material_master(vec![
            MaterialBuilder::new("MAT_KPI_001").weight(10.0).machine("H032").build(),
            MaterialBuilder::new("MAT_KPI_002").weight(7.0).machine("H032").build(),
            MaterialBuilder::new("MAT_KPI_003").weight(5.0).machine("H032").build(),
        ])
        .expect("插入material_master失败");

    let date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    // 版本A：2项（其中 1 项将在版本B移动，1项将在版本B被移除）
    env.plan_item_repo
        .batch_insert(&[
            PlanItemBuilder::new(&version_a, "MAT_KPI_001", "H032", date)
                .seq_no(1)
                .weight(10.0)
                .build(),
            PlanItemBuilder::new(&version_a, "MAT_KPI_003", "H032", date)
                .seq_no(2)
                .weight(5.0)
                .build(),
        ])
        .expect("插入plan_item(A)失败");

    // 版本B：2项（移动 MAT_KPI_001，新增 MAT_KPI_002）
    env.plan_item_repo
        .batch_insert(&[
            PlanItemBuilder::new(&version_b, "MAT_KPI_001", "H033", date)
                .seq_no(1)
                .weight(10.0)
                .build(),
            PlanItemBuilder::new(&version_b, "MAT_KPI_002", "H032", date)
                .seq_no(2)
                .weight(7.0)
                .build(),
        ])
        .expect("插入plan_item(B)失败");

    let result = env
        .plan_api
        .compare_versions_kpi(&version_a, &version_b)
        .expect("KPI对比失败");

    // diff counts（口径：仅比较机组/日期）
    assert_eq!(result.diff_counts.moved_count, 1);
    assert_eq!(result.diff_counts.added_count, 1);
    assert_eq!(result.diff_counts.removed_count, 1);
    assert_eq!(result.diff_counts.squeezed_out_count, 1);

    // plan_item 聚合
    assert_eq!(result.kpi_a.plan_items_count, 2);
    assert_eq!(result.kpi_b.plan_items_count, 2);
    assert!((result.kpi_a.total_weight_t - 15.0).abs() < 1e-9);
    assert!((result.kpi_b.total_weight_t - 17.0).abs() < 1e-9);

    // risk_snapshot 在该测试中未写入，相关指标应返回 None
    assert!(result.kpi_a.overflow_days.is_none());
    assert!(result.kpi_b.overflow_days.is_none());
}

#[test]
fn test_rollback_version_切换激活版本并恢复配置快照() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 1) 准备配置与方案
    env.config_api
        .update_config("global", "maturity_days_winter", "3", "admin", "init")
        .expect("写入配置失败");

    let plan_id = env
        .plan_api
        .create_plan("回滚测试方案".to_string(), "admin".to_string())
        .expect("创建方案失败");

    // 2) 创建版本A（快照应包含 maturity_days_winter=3）
    let version_a = env
        .plan_api
        .create_version(
            plan_id.clone(),
            30,
            None,
            Some("版本A".to_string()),
            "admin".to_string(),
        )
        .expect("创建版本A失败");

    // 3) 修改配置并创建版本B（快照应包含 maturity_days_winter=5）
    env.config_api
        .update_config("global", "maturity_days_winter", "5", "admin", "update")
        .expect("写入配置失败");

    let version_b = env
        .plan_api
        .create_version(
            plan_id.clone(),
            30,
            None,
            Some("版本B".to_string()),
            "admin".to_string(),
        )
        .expect("创建版本B失败");

    // 4) 先激活版本B（模拟“当前运行版本”）
    env.plan_api
        .activate_version(&version_b, "admin")
        .expect("激活版本B失败");

    // 5) 模拟运行中配置被改动（与版本B快照产生漂移）
    env.config_api
        .update_config("global", "maturity_days_winter", "7", "admin", "drift")
        .expect("写入配置失败");

    // 6) 回滚到版本A：应恢复 maturity_days_winter=3 + 激活版本A
    let resp = env
        .plan_api
        .rollback_version(&plan_id, &version_a, "admin", "回滚到A做基准")
        .expect("回滚失败");

    assert_eq!(resp.plan_id, plan_id);
    assert_eq!(resp.to_version_id, version_a);
    assert!(resp.message.contains("回滚"));

    let active = env
        .plan_version_repo
        .find_active_version(&plan_id)
        .expect("查询激活版本失败")
        .expect("应存在激活版本");
    assert_eq!(active.version_id, version_a);

    let cfg = env
        .config_api
        .get_config("global", "maturity_days_winter")
        .expect("读取配置失败")
        .expect("配置应存在");
    assert_eq!(cfg.value, "3");

    // 7) 审计日志应写入
    let logs = env
        .action_log_repo
        .find_by_action_type("ROLLBACK_VERSION", 10)
        .expect("查询 action_log 失败");
    assert!(!logs.is_empty(), "应写入 ROLLBACK_VERSION 审计日志");
}

// ==========================================
// 红线合规性测试
// ==========================================

#[test]
fn test_create_plan_空名称不允许() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 创建空名称方案
    let result = env.plan_api
        .create_plan("".to_string(), "admin".to_string());

    // 验证: 应该返回InvalidInput错误
    assert_invalid_input(result);
}

#[test]
fn test_create_version_无效窗口天数() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 创建方案
    let plan_id = env.plan_api
        .create_plan("测试方案".to_string(), "admin".to_string())
        .expect("创建失败");

    // 测试: 创建版本（窗口天数 = 0）
    let result = env.plan_api
        .create_version(
            plan_id.clone(),
            0, // 无效窗口天数
            None,
            Some("测试版本".to_string()),
            "admin".to_string(),
        );

    // 验证: 应该返回InvalidInput错误
    assert_invalid_input(result);

    // 测试: 创建版本（窗口天数 > 60）
    let result = env.plan_api
        .create_version(
            plan_id,
            100, // 过大的窗口天数
            None,
            Some("测试版本".to_string()),
            "admin".to_string(),
        );

    // 验证: 应该返回InvalidInput错误
    assert_invalid_input(result);
}

#[test]
fn test_activate_version_不存在的版本() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 激活不存在的版本
    let result = env.plan_api
        .activate_version("NOT_EXIST", "admin");

    // 验证: 应该返回错误
    assert!(result.is_err(), "激活不存在的版本应该失败");
}

// ==========================================
// 边界条件测试
// ==========================================

#[test]
fn test_list_versions_不存在的方案() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 查询不存在方案的版本列表
    let versions = env.plan_api
        .list_versions("NOT_EXIST")
        .expect("查询失败");

    assert_eq!(versions.len(), 0, "不存在的方案应该返回空列表");
}

#[test]
fn test_compare_versions_不存在的版本() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    // 测试: 对比不存在的版本（新实现会检查版本是否存在）
    let result = env.plan_api
        .compare_versions("NOT_EXIST_A", "NOT_EXIST_B");

    // 验证: 应该返回错误
    assert!(result.is_err(), "对比不存在的版本应该返回错误");

    // 验证错误类型
    match result {
        Err(ApiError::NotFound(_)) => {
            // 预期的错误类型
        }
        _ => panic!("应该返回NotFound错误"),
    }
}

#[test]
fn test_recalc_full_不存在的版本() {
    let env = ApiTestEnv::new().expect("无法创建测试环境");

    let base_date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();

    // 测试: 重算不存在的版本
    let result = env.plan_api
        .recalc_full(
            "NOT_EXIST",
            base_date,
            None,
            "admin",
        );

    // 验证: 应该返回错误
    assert!(result.is_err(), "重算不存在的版本应该失败");
}

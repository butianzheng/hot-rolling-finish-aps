// ==========================================
// 热轧精整排产系统 - Tauri 主入口
// ==========================================
// 依据: Claude_Dev_Master_Spec.md
// 技术栈: Tauri + Rust + SQLite
// 系统定位: 决策支持系统
// ==========================================

// 禁止控制台窗口 (Windows)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hot_rolling_aps::app::{AppState, get_default_db_path};

#[cfg(feature = "tauri-app")]
fn main() {
    use hot_rolling_aps::app::tauri_commands::*;

    // 初始化日志系统
    tracing_subscriber::fmt::init();

    tracing::info!("==================================================");
    tracing::info!("热轧精整排产系统 - 决策支持系统");
    tracing::info!("系统版本: {}", hot_rolling_aps::VERSION);
    tracing::info!("==================================================");

    // 获取数据库路径
    let db_path = get_default_db_path();
    tracing::info!("使用数据库: {}", db_path);

    // 创建AppState
    tracing::info!("正在初始化AppState...");
    let app_state = AppState::new(db_path)
        .expect("无法初始化AppState");

    tracing::info!("AppState初始化成功");
    tracing::info!("启动Tauri应用...");

    // 启动Tauri应用
    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // ==========================================
            // 材料导入相关命令 (5个)
            // ==========================================
            import_materials,
            list_import_conflicts,
            resolve_import_conflict,
            batch_resolve_import_conflicts,
            cancel_import_batch,

            // ==========================================
            // 材料相关命令 (8个)
            // ==========================================
            list_materials,
            get_material_pool_summary,
            get_material_detail,
            list_ready_materials,
            batch_lock_materials,
            batch_force_release,
            batch_set_urgent,
            list_materials_by_urgent_level,

            // ==========================================
            // 排产方案相关命令 (19个)
            // ==========================================
            create_plan,
            list_plans,
            get_plan_detail,
            get_latest_active_version_id,
            delete_plan,
            delete_version,
            create_version,
            list_versions,
            activate_version,
            rollback_version,
            simulate_recalc,
            recalc_full,
            get_strategy_presets,
            generate_strategy_drafts,
            apply_strategy_draft,
            get_strategy_draft_detail,
            list_strategy_drafts,
            cleanup_expired_strategy_drafts,
            get_plan_item_date_bounds,
            list_plan_items,
            list_items_by_date,
            compare_versions,
            compare_versions_kpi,
            move_items,

            // ==========================================
            // 驾驶舱相关命令 (9个)
            // ==========================================
            list_risk_snapshots,
            get_risk_snapshot,
            get_most_risky_date,
            get_unsatisfied_urgent_materials,
            get_cold_stock_materials,
            get_most_congested_machine,
            get_refresh_status,
            manual_refresh_decision,
            list_action_logs,
            list_action_logs_by_material,
            list_action_logs_by_version,
            get_recent_actions,

            // ==========================================
            // 配置管理相关命令 (8个)
            // ==========================================
            list_configs,
            get_config,
            update_config,
            batch_update_configs,
            get_config_snapshot,
            restore_config_from_snapshot,
            save_custom_strategy,
            list_custom_strategies,

            // ==========================================
            // 宽厚路径规则相关命令 (v0.6)
            // ==========================================
            get_path_rule_config,
            update_path_rule_config,
            list_path_override_pending,
            list_path_override_pending_summary,
            confirm_path_override,
            batch_confirm_path_override,
            batch_confirm_path_override_by_range,
            get_roll_cycle_anchor,
            reset_roll_cycle,

            // ==========================================
            // 换辊管理相关命令 (7个)
            // ==========================================
            list_roll_campaigns,
            list_roll_campaign_plans,
            get_active_roll_campaign,
            list_needs_roll_change,
            create_roll_campaign,
            close_roll_campaign,
            upsert_roll_campaign_plan,

            // ==========================================
            // 每日生产节奏管理相关命令 (7个)
            // ==========================================
            list_rhythm_presets,
            upsert_rhythm_preset,
            set_rhythm_preset_active,
            list_rhythm_targets,
            upsert_rhythm_target,
            apply_rhythm_preset,
            get_daily_rhythm_profile,

            // ==========================================
            // 决策支持相关命令 (6个)
            // ==========================================
            get_decision_day_summary,           // D1: 哪天最危险
            list_order_failure_set,             // D2: 哪些紧急单无法完成
            get_cold_stock_profile,             // D3: 哪些冷料压库
            get_machine_bottleneck_profile,     // D4: 哪个机组最堵
            get_roll_campaign_alert,            // D5: 换辊是否异常
            get_capacity_opportunity,           // D6: 是否存在产能优化空间

            // ==========================================
            // 产能池管理相关命令 (3个)
            // ==========================================
            get_capacity_pools,
            update_capacity_pool,
            batch_update_capacity_pools,

            // ==========================================
            // 前端遥测/错误上报 (1个)
            // ==========================================
            report_frontend_event,
        ])
        .run(tauri::generate_context!())
        .expect("启动Tauri应用失败");

    tracing::info!("Tauri应用已退出");
}

#[cfg(not(feature = "tauri-app"))]
fn main() {
    println!("==================================================");
    println!("热轧精整排产系统 - 决策支持系统");
    println!("系统版本: {}", hot_rolling_aps::VERSION);
    println!("==================================================");
    println!();
    println!("此可执行文件需要启用 tauri-app 特性");
    println!("使用: cargo run --features tauri-app");
    println!();
    println!("或者使用库模式:");
    println!("use hot_rolling_aps::app::AppState;");
    println!();
    println!("详细信息请参考: docs/TAURI_INTEGRATION_GUIDE.md");
}

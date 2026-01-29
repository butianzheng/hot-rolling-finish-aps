// ==========================================
// 热轧精整排产系统 - 配置层
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 11. 配置项全集
// ==========================================
// 职责: 系统配置管理,支持多级覆写
// 存储: config_kv 表
// ==========================================

pub mod config_manager;
pub mod import_config_trait;
pub mod strategy_profile;

// 重导出核心配置管理器
pub use config_manager::{config_keys, ConfigManager, ConfigScope};
pub use import_config_trait::ImportConfigReader;
pub use strategy_profile::{CustomStrategyParameters, CustomStrategyProfile};

// TODO: 添加配置验证器
// TODO: 添加配置变更监听器
// TODO: 添加配置导入/导出工具

// ==========================================
// 热轧精整排产系统 - API 层
// ==========================================
// 职责: 提供业务 API 接口,供 Tauri 命令调用
// ==========================================

pub mod config_api;
pub mod dashboard_api;
pub mod error;
pub mod import_api;
pub mod machine_config_api;
pub mod material_api;
pub mod path_rule_api;
pub mod plan_api;
pub mod rhythm_api;
pub mod roller_api;
pub mod validator;

// 重导出核心类型
pub use config_api::ConfigApi;
pub use dashboard_api::DashboardApi;
pub use error::{ApiError, ApiResult, ValidationViolation};
pub use import_api::{ImportApi, ImportApiResponse};
pub use machine_config_api::MachineConfigApi;
pub use material_api::MaterialApi;
pub use path_rule_api::PathRuleApi;
pub use plan_api::PlanApi;
pub use rhythm_api::RhythmApi;
pub use roller_api::RollerApi;
pub use validator::{ManualOperationValidator, ValidationMode};

// TODO: 添加请求日志记录

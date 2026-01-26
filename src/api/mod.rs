// ==========================================
// 热轧精整排产系统 - API 层
// ==========================================
// 职责: 提供业务 API 接口,供 Tauri 命令调用
// ==========================================

pub mod error;
pub mod dashboard_api;
pub mod material_api;
pub mod plan_api;
pub mod config_api;
pub mod roller_api;
pub mod validator;
pub mod import_api;

// 重导出核心类型
pub use error::{ApiError, ApiResult, ValidationViolation};
pub use dashboard_api::DashboardApi;
pub use material_api::MaterialApi;
pub use plan_api::PlanApi;
pub use config_api::ConfigApi;
pub use roller_api::RollerApi;
pub use validator::{ValidationMode, ManualOperationValidator};
pub use import_api::{ImportApi, ImportApiResponse};

// TODO: 添加请求日志记录

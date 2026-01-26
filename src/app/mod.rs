// ==========================================
// 热轧精整排产系统 - 应用层
// ==========================================
// 职责: Tauri 集成,连接前端与后端
// ==========================================

pub mod state;
pub mod tauri_commands;

// 重导出
pub use state::{AppState, get_default_db_path};

#[cfg(feature = "tauri-app")]
pub use tauri_commands::*;

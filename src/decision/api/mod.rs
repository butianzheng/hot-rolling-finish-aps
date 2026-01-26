// ==========================================
// 热轧精整排产系统 - DecisionApi 模块
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md
// 职责: 决策层 API 接口定义
// ==========================================

// DTO 定义
pub mod dto;

// DecisionApi trait 定义
pub mod decision_api;

// DecisionApi 实现
pub mod decision_api_impl;

// 重导出 DTO
pub use dto::*;

// 重导出 API
pub use decision_api::DecisionApi;
pub use decision_api_impl::DecisionApiImpl;

// ==========================================
// 热轧精整排产系统 - 操作日志数据仓储
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART A3 审计增强
// 依据: schema_v0.1.sql action_log 表
// 红线: 所有写入必须记录
// ==========================================


mod core;
mod queries;

#[cfg(test)]
mod tests;

pub use core::ActionLogRepository;

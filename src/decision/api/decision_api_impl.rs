// ==========================================
// 热轧精整排产系统 - DecisionApi 实现
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md
// 职责: DecisionApi trait 的具体实现
// ==========================================


mod conversions;
mod core;

#[cfg(test)]
mod tests;

pub use core::DecisionApiImpl;

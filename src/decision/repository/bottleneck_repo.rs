// ==========================================
// 热轧精整排产系统 - D4 机组堵塞仓储
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md - D4 用例
// 职责: 查询机组堵塞概况数据
// ==========================================
// P2 阶段: 优先从 decision_machine_bottleneck 读模型表读取
//         如果读模型表为空，回退到 capacity_pool/plan_item 实时计算
// ==========================================

mod core;
mod gap;
mod read_model;
mod realtime;

#[cfg(test)]
mod tests;

pub use core::BottleneckRepository;

// ==========================================
// 热轧精整排产系统 - 结构控制引擎
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - Structure Corrector
// 依据: Claude_Dev_Master_Spec.md - PART B3 产能与换辊
// 红线: 结构目标为软约束
// 红线: 锁定材料不可跳过（即使违反结构目标）
// ==========================================
// 职责: 结构软控制与违规标记
// 输入: 产能池 + 排产明细 + 材料主数据
// 输出: 结构违规标记 + 调整建议
// ==========================================
// 注: MVP 以提示为主,不强制调整
// ==========================================

mod core;
mod report;

#[cfg(test)]
mod tests;

pub use core::StructureCorrector;
pub use report::StructureViolationReport;


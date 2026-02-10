// ==========================================
// 热轧精整排产系统 - 材料导入 Repository 实现
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART E 工程结构
// 职责: 实现导入相关数据访问（使用 rusqlite）
// 红线: Repository 不含业务规则，只做数据 CRUD
// ==========================================

mod core;
mod impls;

pub use core::MaterialImportRepositoryImpl;

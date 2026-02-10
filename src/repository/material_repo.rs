// ==========================================
// 热轧精整排产系统 - 材料数据仓储
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART D 引擎铁律
// 红线: Repository 不含业务逻辑
// ==========================================

mod master;
mod state;

pub use master::MaterialMasterRepository;
pub use state::{
    MaterialStateRepository, MaterialStateSnapshotLite, PathOverrideRejectionSummary,
    UserConfirmedMaterialSummary,
};

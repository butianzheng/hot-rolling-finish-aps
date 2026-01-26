// ==========================================
// 热轧精整排产系统 - 数据仓储层
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART D 引擎铁律
// 红线: Repository 不含业务逻辑
// ==========================================
// 职责: 提供数据访问接口,屏蔽数据库细节
// 约束: 所有查询使用参数化,防止 SQL 注入
// ==========================================

pub mod action_log_repo;
pub mod capacity_repo;
pub mod error;
pub mod material_import_repo;
pub mod material_import_repo_impl;
pub mod material_repo;
pub mod plan_repo;
pub mod risk_repo;
pub mod roller_repo;

// 重导出核心仓储
pub use action_log_repo::ActionLogRepository;
pub use capacity_repo::CapacityPoolRepository;
pub use error::{RepositoryError, RepositoryResult};
pub use material_import_repo::MaterialImportRepository;
pub use material_import_repo_impl::MaterialImportRepositoryImpl;
pub use material_repo::{MaterialMasterRepository, MaterialStateRepository};
pub use plan_repo::{PlanItemRepository, PlanRepository, PlanVersionRepository};
pub use risk_repo::RiskSnapshotRepository;
pub use roller_repo::RollerCampaignRepository;

// TODO: 添加数据库连接池管理模块
// TODO: 添加事务管理模块
// TODO: 添加查询构建器 (Query Builder)
// TODO: 添加数据库迁移工具集成

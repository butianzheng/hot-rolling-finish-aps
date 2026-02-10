// ==========================================
// 热轧精整排产系统 - 引擎层仓储聚合
// ==========================================
// 职责: 聚合排产引擎所需的所有 Repository
// 目标: 减少 RecalcEngine 的构造函数参数数量，提升可维护性
// ==========================================

use std::sync::Arc;

use crate::repository::{
    ActionLogRepository, CapacityPoolRepository, MaterialMasterRepository, MaterialStateRepository,
    PlanItemRepository, PlanVersionRepository,
};

/// 排产引擎仓储集合
///
/// 聚合排产引擎所需的所有 Repository，简化依赖注入。
///
/// # 设计理念
/// - 将 6 个 Repository 参数合并为 1 个结构体参数
/// - 减少构造函数参数数量，提升代码可读性
/// - 便于单元测试时 mock 整个仓储层
///
/// # 包含的仓储
/// - `version_repo`: 版本管理
/// - `item_repo`: 排产项管理
/// - `material_state_repo`: 材料状态管理
/// - `material_master_repo`: 材料主数据管理
/// - `capacity_repo`: 产能池管理
/// - `action_log_repo`: 操作日志管理
#[derive(Clone)]
pub struct ScheduleRepositories {
    /// 版本仓储
    pub version_repo: Arc<PlanVersionRepository>,
    /// 排产项仓储
    pub item_repo: Arc<PlanItemRepository>,
    /// 材料状态仓储
    pub material_state_repo: Arc<MaterialStateRepository>,
    /// 材料主数据仓储
    pub material_master_repo: Arc<MaterialMasterRepository>,
    /// 产能池仓储
    pub capacity_repo: Arc<CapacityPoolRepository>,
    /// 操作日志仓储
    pub action_log_repo: Arc<ActionLogRepository>,
}

impl ScheduleRepositories {
    /// 创建新的仓储集合
    pub fn new(
        version_repo: Arc<PlanVersionRepository>,
        item_repo: Arc<PlanItemRepository>,
        material_state_repo: Arc<MaterialStateRepository>,
        material_master_repo: Arc<MaterialMasterRepository>,
        capacity_repo: Arc<CapacityPoolRepository>,
        action_log_repo: Arc<ActionLogRepository>,
    ) -> Self {
        Self {
            version_repo,
            item_repo,
            material_state_repo,
            material_master_repo,
            capacity_repo,
            action_log_repo,
        }
    }

    /// 获取版本仓储
    pub fn version_repo(&self) -> &Arc<PlanVersionRepository> {
        &self.version_repo
    }

    /// 获取排产项仓储
    pub fn item_repo(&self) -> &Arc<PlanItemRepository> {
        &self.item_repo
    }

    /// 获取材料状态仓储
    pub fn material_state_repo(&self) -> &Arc<MaterialStateRepository> {
        &self.material_state_repo
    }

    /// 获取材料主数据仓储
    pub fn material_master_repo(&self) -> &Arc<MaterialMasterRepository> {
        &self.material_master_repo
    }

    /// 获取产能池仓储
    pub fn capacity_repo(&self) -> &Arc<CapacityPoolRepository> {
        &self.capacity_repo
    }

    /// 获取操作日志仓储
    pub fn action_log_repo(&self) -> &Arc<ActionLogRepository> {
        &self.action_log_repo
    }
}

// 注: 单元测试需要在集成测试环境中运行，因为各个 Repository
// 的构造函数需要数据库连接，且每个仓储的初始化方式不同。
// ScheduleRepositories 作为简单的聚合结构体，其正确性由
// 集成测试和 RecalcEngine 的测试来验证。

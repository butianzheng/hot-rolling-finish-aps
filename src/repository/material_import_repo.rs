// ==========================================
// 热轧精整排产系统 - 材料导入 Repository Trait
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART E 工程结构
// 职责: 定义导入相关数据访问接口（不包含业务逻辑）
// 红线: Repository 不含业务规则，只做数据 CRUD
// ==========================================

use crate::domain::material::{
    ImportBatch, ImportConflict, MaterialMaster, MaterialState,
};
use async_trait::async_trait;
use std::error::Error;

// ==========================================
// MaterialImportRepository Trait
// ==========================================
// 用途: 材料导入相关数据访问
// 实现者: MaterialImportRepoImpl（使用 rusqlite）
#[async_trait]
pub trait MaterialImportRepository: Send + Sync {
    // ===== 批量写入（事务化）=====

    /// 批量插入 MaterialMaster（INSERT OR REPLACE 策略）
    ///
    /// # 参数
    /// - materials: 材料主数据列表
    ///
    /// # 返回
    /// - Ok(usize): 成功插入的记录数
    /// - Err: 数据库错误（整个事务回滚）
    async fn batch_insert_material_master(
        &self,
        materials: Vec<MaterialMaster>,
    ) -> Result<usize, Box<dyn Error>>;

    /// 批量插入 MaterialState（INSERT OR REPLACE 策略）
    ///
    /// # 参数
    /// - states: 材料状态列表
    ///
    /// # 返回
    /// - Ok(usize): 成功插入的记录数
    /// - Err: 数据库错误（整个事务回滚）
    async fn batch_insert_material_state(
        &self,
        states: Vec<MaterialState>,
    ) -> Result<usize, Box<dyn Error>>;

    // ===== 冲突队列管理 =====

    /// 插入冲突记录到 import_conflict 表
    ///
    /// # 参数
    /// - conflict: 冲突记录
    async fn insert_conflict(
        &self,
        conflict: ImportConflict,
    ) -> Result<(), Box<dyn Error>>;

    /// 批量插入冲突记录
    ///
    /// # 参数
    /// - conflicts: 冲突记录列表
    async fn batch_insert_conflicts(
        &self,
        conflicts: Vec<ImportConflict>,
    ) -> Result<usize, Box<dyn Error>>;

    /// 查询指定批次的冲突记录
    ///
    /// # 参数
    /// - batch_id: 批次 ID
    async fn get_conflicts_by_batch(
        &self,
        batch_id: &str,
    ) -> Result<Vec<ImportConflict>, Box<dyn Error>>;

    /// 查询指定材料号的冲突记录
    ///
    /// # 参数
    /// - material_id: 材料号
    async fn get_conflicts_by_material_id(
        &self,
        material_id: &str,
    ) -> Result<Vec<ImportConflict>, Box<dyn Error>>;

    /// 标记冲突为已解决
    ///
    /// # 参数
    /// - conflict_id: 冲突记录 ID
    async fn mark_conflict_resolved(
        &self,
        conflict_id: &str,
    ) -> Result<(), Box<dyn Error>>;

    /// 带过滤和分页的冲突列表查询
    ///
    /// # 参数
    /// - status: 冲突状态过滤 (OPEN/RESOLVED/IGNORED/None表示全部)
    /// - limit: 每页记录数
    /// - offset: 分页偏移
    async fn list_conflicts_with_filter(
        &self,
        status: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ImportConflict>, Box<dyn Error>>;

    /// 统计指定状态的冲突数量
    ///
    /// # 参数
    /// - status: 冲突状态 (OPEN/RESOLVED/IGNORED/None表示全部)
    async fn count_conflicts_by_status(
        &self,
        status: Option<&str>,
    ) -> Result<i64, Box<dyn Error>>;

    /// 按批次统计冲突数量（支持状态过滤）
    ///
    /// # 参数
    /// - batch_id: 批次ID
    /// - status: 冲突状态过滤 (OPEN/RESOLVED/IGNORED/None表示全部)
    ///
    /// # 返回
    /// - Ok(i64): 冲突数量
    async fn count_conflicts_by_batch(
        &self,
        batch_id: &str,
        status: Option<&str>,
    ) -> Result<i64, Box<dyn Error>>;

    /// 根据ID获取单个冲突记录
    ///
    /// # 参数
    /// - conflict_id: 冲突记录ID
    ///
    /// # 返回
    /// - Ok(Some(conflict)): 找到冲突记录
    /// - Ok(None): 未找到
    async fn get_conflict_by_id(
        &self,
        conflict_id: &str,
    ) -> Result<Option<ImportConflict>, Box<dyn Error>>;

    /// 解决冲突并记录解决方式
    ///
    /// # 参数
    /// - conflict_id: 冲突记录ID
    /// - action: 解决动作 (KEEP_EXISTING/OVERWRITE/MERGE)
    /// - note: 解决备注
    async fn resolve_conflict(
        &self,
        conflict_id: &str,
        action: &str,
        note: Option<&str>,
    ) -> Result<(), Box<dyn Error>>;

    // ===== 批次管理 =====

    /// 插入导入批次记录
    ///
    /// # 参数
    /// - batch: 批次信息
    async fn insert_batch(
        &self,
        batch: ImportBatch,
    ) -> Result<(), Box<dyn Error>>;

    /// 查询最近的导入批次
    ///
    /// # 参数
    /// - limit: 返回记录数限制
    async fn get_recent_batches(
        &self,
        limit: usize,
    ) -> Result<Vec<ImportBatch>, Box<dyn Error>>;

    // ===== 查询与校验 =====

    /// 检查材料号是否已存在
    ///
    /// # 参数
    /// - material_id: 材料号
    ///
    /// # 返回
    /// - Ok(true): 材料已存在
    /// - Ok(false): 材料不存在
    async fn exists_material(
        &self,
        material_id: &str,
    ) -> Result<bool, Box<dyn Error>>;

    /// 批量检查材料号是否存在
    ///
    /// # 参数
    /// - material_ids: 材料号列表
    ///
    /// # 返回
    /// - Ok(Vec<String>): 已存在的材料号列表
    async fn batch_check_exists(
        &self,
        material_ids: Vec<String>,
    ) -> Result<Vec<String>, Box<dyn Error>>;

    /// 统计 material_master 表记录数
    async fn count_materials(&self) -> Result<usize, Box<dyn Error>>;

    /// 统计 material_state 表记录数
    async fn count_states(&self) -> Result<usize, Box<dyn Error>>;
}

// ==========================================
// 热轧精整排产系统 - 排产方案领域模型
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART A2 红线
// 依据: Engine_Specs_v0.3_Integrated.md - plan/plan_version/plan_item
// ==========================================

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use crate::domain::types::PlanVersionStatus;

// ==========================================
// Plan - 排产方案
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub plan_id: String,              // 方案ID
    pub plan_name: String,            // 方案名称
    pub plan_type: String,            // 方案类型 (BASELINE/SCENARIO/SANDBOX)
    pub base_plan_id: Option<String>, // 基准方案ID (派生方案的源)
    pub created_by: String,           // 创建人
    pub created_at: NaiveDateTime,    // 创建时间
    pub updated_at: NaiveDateTime,    // 更新时间
}

// ==========================================
// PlanVersion - 方案版本
// ==========================================
// 用途: 沙盘模拟,历史回溯
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanVersion {
    pub version_id: String,                  // 版本ID
    pub plan_id: String,                     // 关联方案
    pub version_no: i32,                     // 版本号
    pub status: PlanVersionStatus,           // 状态 (类型安全的枚举)
    pub frozen_from_date: Option<NaiveDate>, // 冻结区起始日期
    pub recalc_window_days: Option<i32>,     // 重算窗口天数
    pub config_snapshot_json: Option<String>,// 配置快照 (JSON)
    pub created_by: Option<String>,          // 创建人
    pub created_at: NaiveDateTime,           // 创建时间
    pub revision: i32,                       // 乐观锁：版本修订号
}

impl PlanVersion {
    /// 判断是否为草稿状态
    pub fn is_draft(&self) -> bool {
        self.status == PlanVersionStatus::Draft
    }

    /// 判断是否为激活状态
    pub fn is_active(&self) -> bool {
        self.status == PlanVersionStatus::Active
    }

    /// 判断是否为归档状态
    pub fn is_archived(&self) -> bool {
        self.status == PlanVersionStatus::Archived
    }
}

// ==========================================
// PlanItem - 排产明细
// ==========================================
// 红线: 只是方案快照,不可反向污染 material_state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanItem {
    // ===== 主键字段 (复合主键: version_id + material_id) =====
    pub version_id: String,        // 关联版本
    pub material_id: String,       // 材料ID

    // ===== 排产信息 =====
    pub machine_code: String,      // 机组代码
    pub plan_date: NaiveDate,      // 排产日期
    pub seq_no: i32,               // 序号 (对齐schema: seq_no)
    pub weight_t: f64,             // 吨位

    // ===== 来源与标志 (对齐schema) =====
    pub source_type: String,       // 来源类型 (CALC/FROZEN/MANUAL)
    pub locked_in_plan: bool,      // 计划中锁定 (对齐schema)
    pub force_release_in_plan: bool, // 计划中强制放行 (对齐schema)
    pub violation_flags: Option<String>, // 违规标志 (JSON字符串, 对齐schema)

    // ===== 快照字段 (业务逻辑需要，但不存储在schema中) =====
    // 注: 这些字段由 API 层从 material_state / material_master 动态补充
    pub urgent_level: Option<String>,  // 紧急等级快照 (可选，用于可解释性)
    pub sched_state: Option<String>,   // 状态快照 (可选，用于可解释性)
    pub assign_reason: Option<String>, // 落位原因 (可选，用于可解释性)
    pub steel_grade: Option<String>,   // 钢种/出钢记号 (来自 material_master.steel_mark)
    pub width_mm: Option<f64>,         // 宽度快照 (来自 material_master.width_mm)
    pub thickness_mm: Option<f64>,     // 厚度快照 (来自 material_master.thickness_mm)
}

// 辅助类型别名 (兼容旧代码)
impl PlanItem {
    /// 获取序号 (兼容旧代码中的sequence_no)
    pub fn sequence_no(&self) -> i32 {
        self.seq_no
    }

    /// 判断是否冻结 (兼容旧代码中的is_frozen)
    pub fn is_frozen(&self) -> bool {
        self.locked_in_plan || self.source_type == "FROZEN"
    }

    /// 设置为冻结状态
    pub fn set_frozen(&mut self, frozen: bool) {
        self.locked_in_plan = frozen;
        if frozen {
            self.source_type = "FROZEN".to_string();
        }
    }
}

// ==========================================
// Trait: PlanVersionManagement
// ==========================================
// 用途: Recalc Engine 版本管理接口
pub trait PlanVersionManagement {
    // TODO: 创建新版本
    fn create_new_version(&self, window_days: i32) -> PlanVersion;

    // TODO: 激活版本
    fn activate_version(&mut self);

    // TODO: 判断是否可编辑
    fn is_editable(&self) -> bool;
}

// TODO: 实现 PlanVersionManagement trait
// TODO: 实现数据库映射 (sqlx derive)

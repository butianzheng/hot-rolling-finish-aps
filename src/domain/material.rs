// ==========================================
// 热轧精整排产系统 - 材料领域模型
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART C 数据与状态体系
// 依据: Engine_Specs_v0.3_Integrated.md - material_master/material_state
// 依据: Field_Mapping_Spec_v0.3_Integrated.md - 字段映射规范
// 依据: data_dictionary_v0.1.md - 数据字典
// ==========================================

use crate::domain::types::{RushLevel, SchedState, UrgentLevel};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

// ==========================================
// MaterialMaster - 材料主数据
// ==========================================
// 红线: 合同字段作为影子列,不独立建表
// 用途: 导入层写入,引擎层只读
// 对齐: schema_v0.1.sql material_master 表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialMaster {
    // ===== 主键 =====
    pub material_id: String, // 材料唯一标识（材料号）

    // ===== 基础信息 =====
    pub manufacturing_order_id: Option<String>, // 制造命令号
    pub material_status_code_src: Option<String>, // 材料状态码（源字段）
    pub steel_mark: Option<String>,            // 出钢记号（钢种影子字段）
    pub slab_id: Option<String>,               // 板坯号

    // ===== 机组信息 =====
    pub next_machine_code: Option<String>,     // 下道机组代码（源字段）
    pub rework_machine_code: Option<String>,   // 精整返修机组（源字段）
    pub current_machine_code: Option<String>,  // 当前机组代码（派生：COALESCE(rework, next)）

    // ===== 工艺维度 =====
    pub width_mm: Option<f64>,                 // 材料实际宽度（mm）
    pub thickness_mm: Option<f64>,             // 材料实际厚度（mm）
    pub length_m: Option<f64>,                 // 材料实际长度（m）
    pub weight_t: Option<f64>,                 // 材料实际重量（吨，3位小数）
    pub available_width_mm: Option<f64>,       // 可利用宽度（mm）

    // ===== 时间信息 =====
    pub due_date: Option<NaiveDate>,           // 合同交货期（ISO DATE）
    pub stock_age_days: Option<i32>,           // 状态时间（天）- 库存压力主口径
    pub output_age_days_raw: Option<i32>,      // 产出时间（天）- 适温反推基础
    pub status_updated_at: Option<DateTime<Utc>>, // 物料状态修改时间

    // ===== 合同影子字段（用于催料计算）=====
    pub contract_no: Option<String>,           // 合同号
    pub contract_nature: Option<String>,       // 合同性质代码（催料规则字段①）
    pub weekly_delivery_flag: Option<String>,  // 按周交货标志（催料规则字段②）
    pub export_flag: Option<String>,           // 出口标记（催料规则字段③，统一为 '1'/'0'）

    // ===== 审计字段 =====
    pub created_at: DateTime<Utc>,             // 记录创建时间
    pub updated_at: DateTime<Utc>,             // 记录更新时间
}

// ==========================================
// MaterialState - 材料系统状态
// ==========================================
// 红线: 唯一事实层,不可被 plan_item 反向污染
// 用途: 引擎写入,排产依据
// 对齐: schema_v0.1.sql material_state 表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialState {
    // ===== 主键与关联 =====
    pub material_id: String, // 关联 material_master（FK）

    // ===== 排产状态（Eligibility Engine 输出）=====
    pub sched_state: SchedState, // 排产状态（PENDING_MATURE/READY/SCHEDULED/LOCKED/FORCE_RELEASE/BLOCKED）
    pub lock_flag: bool,         // 锁定标记（冻结区材料）
    pub force_release_flag: bool, // 强制放行标记（绕过适温）

    // ===== 紧急等级（Urgency Engine 输出）=====
    pub urgent_level: UrgentLevel, // 最终紧急等级（L0-L3，等级制非评分）
    pub urgent_reason: Option<String>, // 紧急原因（JSON 格式，可解释性）
    pub rush_level: RushLevel,     // 催料组合规则结果（L0/L1/L2，中间变量）

    // ===== 适温派生字段（导入层派生）=====
    pub rolling_output_age_days: i32, // 等效轧制产出天数（按机组偏移规则派生）
    pub ready_in_days: i32,        // 距离适温还需天数（0=已适温）
    pub earliest_sched_date: Option<NaiveDate>, // 最早可排日期（today + ready_in_days）

    // ===== 库存压力 =====
    pub stock_age_days: i32, // 库存天数（状态时间）

    // ===== 排产落位（由 Capacity Filler 写入）=====
    pub scheduled_date: Option<NaiveDate>,    // 已排日期（NULL=未排）
    pub scheduled_machine_code: Option<String>, // 已排机组
    pub seq_no: Option<i32>,                  // 日内顺序号

    // ===== 人工干预标志 =====
    pub manual_urgent_flag: bool, // 人工红线标志

    // ===== 冻结区标志 =====
    pub in_frozen_zone: bool, // 是否在冻结区

    // ===== 审计字段 =====
    pub last_calc_version_id: Option<String>, // 最后计算版本ID（关联 plan_version）
    pub updated_at: DateTime<Utc>,    // 最后更新时间
    pub updated_by: Option<String>,   // 操作人/系统标识
}

// ==========================================
// RawMaterialRecord - 导入中间结构体
// ==========================================
// 用途: 导入管道中间产物（文件解析 → 字段映射 → 此结构）
// 生命周期: 仅在导入流程内
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawMaterialRecord {
    // 源字段（已类型转换）
    pub material_id: Option<String>,
    pub manufacturing_order_id: Option<String>,
    pub material_status_code_src: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub next_machine_code: Option<String>,
    pub rework_machine_code: Option<String>,
    pub width_mm: Option<f64>,
    pub thickness_mm: Option<f64>,
    pub length_m: Option<f64>,
    pub weight_t: Option<f64>,
    pub available_width_mm: Option<f64>,
    pub steel_mark: Option<String>,
    pub slab_id: Option<String>,
    pub stock_age_days: Option<i32>,
    pub output_age_days_raw: Option<i32>,
    pub status_updated_at: Option<DateTime<Utc>>,
    pub contract_no: Option<String>,
    pub contract_nature: Option<String>,
    pub weekly_delivery_flag: Option<String>,
    pub export_flag: Option<String>,

    // 元信息
    pub row_number: usize, // 原始文件行号（用于 DQ 报告）
}

// ==========================================
// ImportBatch - 导入批次
// ==========================================
// 用途: 记录导入批次元信息
// 对齐: v0.2_importer_schema.sql import_batch 表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportBatch {
    pub batch_id: String,                      // 批次 ID（UUID）
    pub file_name: Option<String>,             // 源文件名
    pub file_path: Option<String>,             // 源文件路径
    pub total_rows: i32,                       // 总行数
    pub success_rows: i32,                     // 成功导入行数
    pub blocked_rows: i32,                     // 阻断行数（DQ ERROR）
    pub warning_rows: i32,                     // 警告行数（DQ WARNING）
    pub conflict_rows: i32,                    // 冲突行数
    pub imported_at: Option<DateTime<Utc>>,    // 导入时间
    pub imported_by: Option<String>,           // 导入人
    pub elapsed_ms: Option<i32>,               // 导入耗时（毫秒）
    pub dq_report_json: Option<String>,        // DQ 报告 JSON
}

// ==========================================
// ImportConflict - 导入冲突记录
// ==========================================
// 用途: 记录主键重复/字段冲突等，进入人工队列
// 对齐: schema_v0.1.sql import_conflict 表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConflict {
    pub conflict_id: String,           // 冲突记录 ID（UUID）
    pub batch_id: String,              // 关联批次 ID
    pub row_number: usize,             // 原始文件行号
    pub material_id: Option<String>,   // 材料号（如果可解析）
    pub conflict_type: ConflictType,   // 冲突类型
    pub raw_data: String,              // 原始行数据（JSON）
    pub reason: String,                // 冲突原因
    pub resolved: bool,                // 是否已处理
    pub created_at: DateTime<Utc>,     // 创建时间
}

// ==========================================
// ConflictType - 冲突类型枚举
// ==========================================
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    PrimaryKeyMissing,  // 主键缺失
    PrimaryKeyDuplicate, // 主键重复
    ForeignKeyViolation, // 外键违反（如机组代码不存在）
    DataTypeError,      // 数据类型错误
}

// ==========================================
// DqViolation - 数据质量违规记录
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DqViolation {
    pub row_number: usize,             // 原始文件行号
    pub material_id: Option<String>,   // 材料号（如果可解析）
    pub level: DqLevel,                // 违规级别
    pub field: String,                 // 违规字段
    pub message: String,               // 违规描述
}

// ==========================================
// DqLevel - 数据质量级别
// ==========================================
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DqLevel {
    Error,   // 错误（阻断导入）
    Warning, // 警告（允许导入）
    Info,    // 提示（仅记录）
    Conflict, // 冲突（进入冲突队列）
}

// ==========================================
// DqReport - 数据质量报告
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DqReport {
    pub batch_id: String,              // 批次 ID
    pub summary: DqSummary,            // 汇总统计
    pub violations: Vec<DqViolation>,  // 违规明细
}

// ==========================================
// DqSummary - 数据质量汇总
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DqSummary {
    pub total_rows: usize,    // 总行数
    pub success: usize,       // 成功导入
    pub blocked: usize,       // 阻断（ERROR）
    pub warning: usize,       // 警告（WARNING）
    pub conflict: usize,      // 冲突（CONFLICT）
}

// ==========================================
// ImportResult - 导入结果
// ==========================================
// 用途: 导入接口返回值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub batch: ImportBatch,    // 批次信息
    pub summary: DqSummary,    // 汇总统计
    pub violations: Vec<DqViolation>, // 违规明细
    pub elapsed_time: std::time::Duration, // 导入耗时
}

// ==========================================
// Trait: MaterialEligibility
// ==========================================
// 用途: Eligibility Engine 判定逻辑接口
pub trait MaterialEligibility {
    /// 判断是否适温
    fn is_temperature_ready(&self, min_temp_days: i32) -> bool;

    /// 判断是否可排
    fn is_schedulable(&self) -> bool;

    /// 计算最早可排日期
    fn calculate_earliest_date(&self, today: NaiveDate, min_temp_days: i32) -> NaiveDate;
}

// ==========================================
// Trait: MaterialUrgency
// ==========================================
// 用途: Urgency Engine 判定逻辑接口
pub trait MaterialUrgency {
    /// 计算催料等级
    fn calculate_rush_level(&self) -> RushLevel;

    /// 判定最终紧急等级
    fn calculate_urgent_level(
        &self,
        today: NaiveDate,
        n1_days: i32,
        n2_days: i32,
    ) -> (UrgentLevel, String);
}

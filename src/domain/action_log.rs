// ==========================================
// 热轧精整排产系统 - 操作日志领域模型
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART A3 审计增强
// 依据: Engine_Specs_v0.3_Integrated.md - 9. Impact Summary Engine
// ==========================================

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ==========================================
// ActionLog - 操作日志
// ==========================================
// 红线: 所有写入必须记录
// 用途: 审计追踪,影响分析
// 对齐: schema_v0.1.sql action_log 表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLog {
    // ===== 主键 (对齐schema) =====
    pub action_id: String,         // 日志ID (对齐schema字段名)
    pub version_id: Option<String>, // 关联版本 (可选，配置更新等系统操作可为None)
    pub action_type: String,       // 操作类型 (存储为字符串)
    pub action_ts: NaiveDateTime,  // 操作时间戳 (对齐schema)
    pub actor: String,             // 操作人 (对齐schema字段名)

    // ===== 操作负载 =====
    pub payload_json: Option<JsonValue>, // 操作参数 (JSON)

    // ===== 影响摘要 =====
    pub impact_summary_json: Option<JsonValue>, // 影响摘要 (JSON)

    // ===== 扩展字段 (业务用) =====
    pub machine_code: Option<String>, // 机组代码
    pub date_range_start: Option<chrono::NaiveDate>, // 影响开始日期
    pub date_range_end: Option<chrono::NaiveDate>,   // 影响结束日期
    pub detail: Option<String>,    // 详细描述
}

// ==========================================
// ActionType - 操作类型
// ==========================================
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    Import,            // 导入材料
    Recalc,            // 一键重算
    LocalAdjust,       // 局部调整
    Lock,              // 锁定材料
    ForceRelease,      // 强制放行
    CreateVersion,     // 创建版本
    ActivateVersion,   // 激活版本
    RollChange,        // 换辊
    // ===== v0.4+ 路径规则相关 =====
    PathOverrideConfirm, // 路径突破人工确认
    PathOverrideReject,  // 路径突破人工拒绝
    RollCycleReset,      // 换辊周期重置（含锚点重置）
}

// ==========================================
// ImpactSummary - 影响摘要结构
// ==========================================
// 用途: Impact Summary Engine 输出格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactSummary {
    // ===== 材料影响 (汇总) =====
    pub moved_count: i32,          // 移动材料数量
    pub squeezed_out_count: i32,   // 挤出材料数量
    pub added_count: i32,          // 新增材料数量
    pub material_changes: Vec<MaterialChange>, // 详细材料变更列表

    // ===== 产能影响 =====
    pub capacity_delta_t: f64,     // 产能变化 (吨)
    pub overflow_delta_t: f64,     // 超限变化 (吨)
    pub capacity_changes: Vec<CapacityChange>, // 按日期/机组的产能变化

    // ===== 风险影响 =====
    pub risk_level_before: String, // 操作前风险等级
    pub risk_level_after: String,  // 操作后风险等级
    pub risk_changes: Vec<RiskChange>, // 按日期的风险变化

    // ===== 换辊影响 =====
    pub roll_campaign_affected: bool, // 是否影响换辊窗口
    pub roll_tonnage_delta_t: Option<f64>, // 换辊累计吨位变化

    // ===== 紧急单影响 =====
    pub urgent_material_affected: i32, // 受影响的紧急材料数量 (L2+L3)
    pub l3_critical_count: i32,        // 受影响的L3红线材料数量

    // ===== 冲突提示 =====
    pub locked_conflicts: Vec<String>, // 锁定冲突材料列表
    pub frozen_conflicts: Vec<String>, // 冻结冲突材料列表
    pub structure_suggestions: Vec<String>, // 结构补偿建议
}

// ==========================================
// MaterialChange - 材料变更记录
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialChange {
    pub material_no: String,       // 材料编号
    pub change_type: String,       // 变更类型: "added", "moved", "squeezed_out", "removed"
    pub from_date: Option<chrono::NaiveDate>, // 原计划日期
    pub to_date: Option<chrono::NaiveDate>,   // 新计划日期
    pub reason: String,            // 变更原因
}

// ==========================================
// CapacityChange - 产能变更记录
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityChange {
    pub date: chrono::NaiveDate,   // 日期
    pub machine_code: String,      // 机组
    pub used_capacity_before_t: f64, // 操作前已用产能
    pub used_capacity_after_t: f64,  // 操作后已用产能
    pub delta_t: f64,              // 变化量
}

// ==========================================
// RiskChange - 风险变更记录
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskChange {
    pub date: chrono::NaiveDate,   // 日期
    pub machine_code: String,      // 机组
    pub risk_before: String,       // 操作前风险等级
    pub risk_after: String,        // 操作后风险等级
    pub reason: String,            // 风险变化原因
}

// ==========================================
// ActionType 辅助方法
// ==========================================
impl ActionType {
    /// 转换为字符串 (用于数据库存储)
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::Import => "Import",
            ActionType::Recalc => "Recalc",
            ActionType::LocalAdjust => "LocalAdjust",
            ActionType::Lock => "Lock",
            ActionType::ForceRelease => "ForceRelease",
            ActionType::CreateVersion => "CreateVersion",
            ActionType::ActivateVersion => "ActivateVersion",
            ActionType::RollChange => "RollChange",
            // v0.4+ 路径规则相关
            ActionType::PathOverrideConfirm => "PathOverrideConfirm",
            ActionType::PathOverrideReject => "PathOverrideReject",
            ActionType::RollCycleReset => "RollCycleReset",
        }
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Import" => Some(ActionType::Import),
            "Recalc" => Some(ActionType::Recalc),
            "LocalAdjust" => Some(ActionType::LocalAdjust),
            "Lock" => Some(ActionType::Lock),
            "ForceRelease" => Some(ActionType::ForceRelease),
            "CreateVersion" => Some(ActionType::CreateVersion),
            "ActivateVersion" => Some(ActionType::ActivateVersion),
            "RollChange" => Some(ActionType::RollChange),
            // v0.4+ 路径规则相关
            "PathOverrideConfirm" => Some(ActionType::PathOverrideConfirm),
            "PathOverrideReject" => Some(ActionType::PathOverrideReject),
            "RollCycleReset" => Some(ActionType::RollCycleReset),
            _ => None,
        }
    }
}

// ==========================================
// ActionLog 辅助方法
// ==========================================
impl ActionLog {
    /// 创建新的操作日志
    ///
    /// # 参数
    /// - `action_id`: 日志ID (通常使用UUID)
    /// - `version_id`: 关联版本ID (可选)
    /// - `action_type`: 操作类型
    /// - `actor`: 操作人
    ///
    /// # 返回
    /// 新的 ActionLog 实例
    pub fn new(
        action_id: String,
        version_id: Option<String>,
        action_type: &str,
        actor: String,
    ) -> Self {
        Self {
            action_id,
            version_id,
            action_type: action_type.to_string(),
            action_ts: chrono::Utc::now().naive_utc(),
            actor,
            payload_json: None,
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: None,
        }
    }

    /// 设置影响摘要 (转换为JSON)
    pub fn with_impact_summary(mut self, summary: &ImpactSummary) -> Self {
        self.impact_summary_json = serde_json::to_value(summary).ok();
        self
    }

    /// 设置操作负载 (转换为JSON)
    pub fn with_payload<T: Serialize>(mut self, payload: &T) -> Self {
        self.payload_json = serde_json::to_value(payload).ok();
        self
    }

    /// 设置日期范围
    pub fn with_date_range(
        mut self,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
    ) -> Self {
        self.date_range_start = Some(start);
        self.date_range_end = Some(end);
        self
    }

    /// 设置机组代码
    pub fn with_machine_code(mut self, machine_code: String) -> Self {
        self.machine_code = Some(machine_code);
        self
    }

    /// 生成唯一ID (用于显示)
    pub fn get_display_id(&self) -> String {
        let version_part = self.version_id.as_deref().unwrap_or("SYSTEM");
        format!("{}_{}", version_part, &self.action_id[..8])
    }
}

// ==========================================
// ImpactSummary 辅助方法
// ==========================================
impl ImpactSummary {
    /// 创建空的影响摘要
    pub fn empty() -> Self {
        Self {
            moved_count: 0,
            squeezed_out_count: 0,
            added_count: 0,
            material_changes: vec![],
            capacity_delta_t: 0.0,
            overflow_delta_t: 0.0,
            capacity_changes: vec![],
            risk_level_before: "UNKNOWN".to_string(),
            risk_level_after: "UNKNOWN".to_string(),
            risk_changes: vec![],
            roll_campaign_affected: false,
            roll_tonnage_delta_t: None,
            urgent_material_affected: 0,
            l3_critical_count: 0,
            locked_conflicts: vec![],
            frozen_conflicts: vec![],
            structure_suggestions: vec![],
        }
    }

    /// 判断是否有显著影响
    pub fn has_significant_impact(&self) -> bool {
        self.moved_count > 0
            || self.squeezed_out_count > 0
            || self.capacity_delta_t.abs() > 0.01
            || self.overflow_delta_t.abs() > 0.01
            || self.risk_level_before != self.risk_level_after
            || !self.locked_conflicts.is_empty()
            || !self.frozen_conflicts.is_empty()
    }

    /// 生成简短摘要文本
    pub fn generate_summary_text(&self) -> String {
        let mut parts = vec![];

        if self.moved_count > 0 {
            parts.push(format!("移动{}个材料", self.moved_count));
        }
        if self.squeezed_out_count > 0 {
            parts.push(format!("挤出{}个材料", self.squeezed_out_count));
        }
        if self.added_count > 0 {
            parts.push(format!("新增{}个材料", self.added_count));
        }
        if self.capacity_delta_t.abs() > 0.01 {
            parts.push(format!("产能变化{:.1}吨", self.capacity_delta_t));
        }
        if self.risk_level_before != self.risk_level_after {
            parts.push(format!(
                "风险{}→{}",
                self.risk_level_before, self.risk_level_after
            ));
        }

        if parts.is_empty() {
            "无显著影响".to_string()
        } else {
            parts.join(", ")
        }
    }
}

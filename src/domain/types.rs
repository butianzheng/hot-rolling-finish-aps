// ==========================================
// 热轧精整排产系统 - 领域类型定义
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART A2 红线
// 依据: Engine_Specs_v0.3_Integrated.md - 0.2 紧急等级体系
// ==========================================

use serde::{Deserialize, Serialize};
use std::fmt;

// ==========================================
// 紧急等级 (Urgency Level)
// ==========================================
// 红线: 等级制,不是评分制
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UrgentLevel {
    L0, // 正常
    L1, // 关注
    L2, // 紧急
    L3, // 红线
}

impl fmt::Display for UrgentLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UrgentLevel::L0 => write!(f, "L0"),
            UrgentLevel::L1 => write!(f, "L1"),
            UrgentLevel::L2 => write!(f, "L2"),
            UrgentLevel::L3 => write!(f, "L3"),
        }
    }
}

// ==========================================
// 催料等级 (Rush Level)
// ==========================================
// 由合同字段组合计算,作为紧急等级抬升因子
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RushLevel {
    L0, // 无催料
    L1, // 一般催料
    L2, // 强催料
}

impl fmt::Display for RushLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RushLevel::L0 => write!(f, "L0"),
            RushLevel::L1 => write!(f, "L1"),
            RushLevel::L2 => write!(f, "L2"),
        }
    }
}

// ==========================================
// 排产状态 (Schedule State)
// ==========================================
// 依据: Engine_Specs 2.2 Eligibility Engine 输出
// 序列化格式: SCREAMING_SNAKE_CASE (与数据库一致)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SchedState {
    PendingMature,  // 未成熟(冷料)
    Ready,          // 适温待排
    Locked,         // 人工锁定
    ForceRelease,   // 强制放行
    Blocked,        // 数据质量阻断
    Scheduled,      // 已排产
}

impl fmt::Display for SchedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchedState::PendingMature => write!(f, "PENDING_MATURE"),
            SchedState::Ready => write!(f, "READY"),
            SchedState::Locked => write!(f, "LOCKED"),
            SchedState::ForceRelease => write!(f, "FORCE_RELEASE"),
            SchedState::Blocked => write!(f, "BLOCKED"),
            SchedState::Scheduled => write!(f, "SCHEDULED"),
        }
    }
}

// ==========================================
// 季节模式 (Season Mode)
// ==========================================
// 依据: Engine_Specs 0.1 季节模式与适温阈值
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SeasonMode {
    Auto,   // 按月份自动判断
    Manual, // 人工指定
}

impl fmt::Display for SeasonMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SeasonMode::Auto => write!(f, "AUTO"),
            SeasonMode::Manual => write!(f, "MANUAL"),
        }
    }
}

// ==========================================
// 季节类型 (Season Type)
// ==========================================
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Season {
    Winter, // 冬季
    Summer, // 夏季
}

impl fmt::Display for Season {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Season::Winter => write!(f, "WINTER"),
            Season::Summer => write!(f, "SUMMER"),
        }
    }
}

// ==========================================
// 风险等级 (Risk Level)
// ==========================================
// 依据: Engine_Specs 8. Risk Engine
// 顺序: Green < Yellow < Orange < Red
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel {
    Green,  // 正常
    Yellow, // 关注
    Orange, // 紧张
    Red,    // 危险
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RiskLevel::Green => write!(f, "GREEN"),
            RiskLevel::Yellow => write!(f, "YELLOW"),
            RiskLevel::Orange => write!(f, "ORANGE"),
            RiskLevel::Red => write!(f, "RED"),
        }
    }
}

// ==========================================
// 换辊状态 (Roll Campaign Status)
// ==========================================
// 依据: Engine_Specs 7. Roll Campaign Engine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RollStatus {
    Normal,    // 正常
    Suggest,   // 建议换辊
    HardStop,  // 强制换辊
}

impl fmt::Display for RollStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RollStatus::Normal => write!(f, "NORMAL"),
            RollStatus::Suggest => write!(f, "SUGGEST"),
            RollStatus::HardStop => write!(f, "HARD_STOP"),
        }
    }
}

// ==========================================
// 方案版本状态 (Plan Version Status)
// ==========================================
// 依据: Engine_Specs - plan_version 表
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlanVersionStatus {
    Draft,    // 草稿
    Active,   // 激活
    Archived, // 归档
}

impl fmt::Display for PlanVersionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanVersionStatus::Draft => write!(f, "DRAFT"),
            PlanVersionStatus::Active => write!(f, "ACTIVE"),
            PlanVersionStatus::Archived => write!(f, "ARCHIVED"),
        }
    }
}

impl PlanVersionStatus {
    /// 从字符串解析状态
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "DRAFT" => PlanVersionStatus::Draft,
            "ACTIVE" => PlanVersionStatus::Active,
            "ARCHIVED" => PlanVersionStatus::Archived,
            _ => PlanVersionStatus::Draft, // 默认值
        }
    }

    /// 转换为数据库存储的字符串
    pub fn to_db_str(&self) -> &'static str {
        match self {
            PlanVersionStatus::Draft => "DRAFT",
            PlanVersionStatus::Active => "ACTIVE",
            PlanVersionStatus::Archived => "ARCHIVED",
        }
    }
}

// TODO(P3-TD001): 实现 FromStr trait 便于配置解析
// TODO(P3-TD002): 实现数据库映射 (sqlx derive)

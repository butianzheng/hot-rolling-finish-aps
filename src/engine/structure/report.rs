use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==========================================
// StructureViolationReport - 结构违规报告
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureViolationReport {
    /// 机组代码
    pub machine_code: String,

    /// 排产日期
    pub plan_date: NaiveDate,

    /// 是否违规
    pub is_violated: bool,

    /// 违规描述
    pub violation_desc: Option<String>,

    /// 调整建议列表
    pub suggestions: Vec<String>,

    /// 偏差比例（最大偏差）
    pub deviation_ratio: f64,

    /// 实际配比 (steel_mark -> 占比)
    pub actual_ratio: HashMap<String, f64>,

    /// 目标配比 (steel_mark -> 占比)
    pub target_ratio: HashMap<String, f64>,
}

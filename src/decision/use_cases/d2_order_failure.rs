// ==========================================
// 热轧精整排产系统 - D2 用例：哪些紧急单无法完成
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 5.2 节
// 职责: 回答"哪些紧急单无法完成"，返回失败集合与原因
// ==========================================

use serde::{Deserialize, Serialize};

/// D2 用例：哪些紧急单无法完成
///
/// 输入: version_id, fail_type (可选)
/// 输出: Vec<OrderFailure> 按 urgency_level 降序
/// 刷新触发: plan_item_changed, material_state_changed
pub trait OrderFailureUseCase {
    /// 查询订单失败集合
    fn list_order_failures(
        &self,
        version_id: &str,
        fail_type: Option<&str>,
    ) -> Result<Vec<OrderFailure>, String>;

    /// 查询特定合同的失败情况
    fn get_contract_failure(
        &self,
        version_id: &str,
        contract_no: &str,
    ) -> Result<Option<OrderFailure>, String>;

    /// 统计失败订单数量
    fn count_failures(&self, version_id: &str) -> Result<FailureStats, String>;
}

/// 订单失败记录
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderFailure {
    /// 合同号
    pub contract_no: String,

    /// 版本 ID
    pub version_id: String,

    /// 交货期 (YYYY-MM-DD)
    pub due_date: String,

    /// 紧急等级 (L0/L1/L2/L3)
    pub urgency_level: String,

    /// 失败类型
    pub fail_type: FailureType,

    /// 总材料数
    pub total_materials: i32,

    /// 未排产数量
    pub unscheduled_count: i32,

    /// 未排产重量 (吨)
    pub unscheduled_weight_t: f64,

    /// 完成率 (0.0-1.0)
    pub completion_rate: f64,

    /// 距离交货期天数 (负数表示超期)
    pub days_to_due: i32,

    /// 失败原因列表
    pub failure_reasons: Vec<String>,

    /// 阻塞因素
    pub blocking_factors: Vec<BlockingFactor>,

    /// 建议措施
    pub suggested_actions: Vec<String>,
}

/// 失败类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureType {
    /// 超期未完成
    Overdue,
    /// 临期无法完成
    NearDueImpossible,
    /// 产能不足
    CapacityShortage,
    /// 结构冲突
    StructureConflict,
    /// 冷料未适温
    ColdStockNotReady,
    /// 其他
    Other,
}

/// 阻塞因素
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockingFactor {
    /// 因素代码
    pub code: String,

    /// 因素描述
    pub description: String,

    /// 影响材料数量
    pub affected_count: i32,

    /// 影响重量 (吨)
    pub affected_weight_t: f64,

    /// 是否可解除
    pub is_removable: bool,
}

/// 失败统计
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailureStats {
    /// 版本 ID
    pub version_id: String,

    /// 失败合同总数
    pub total_failures: i32,

    /// 超期合同数
    pub overdue_count: i32,

    /// 临期无法完成数
    pub near_due_impossible_count: i32,

    /// 产能不足导致失败数
    pub capacity_shortage_count: i32,

    /// 结构冲突导致失败数
    pub structure_conflict_count: i32,

    /// 受影响材料总数
    pub total_affected_materials: i32,

    /// 受影响总重量 (吨)
    pub total_affected_weight_t: f64,
}

impl OrderFailure {
    /// 创建新的订单失败记录
    pub fn new(
        contract_no: String,
        version_id: String,
        due_date: String,
        urgency_level: String,
    ) -> Self {
        Self {
            contract_no,
            version_id,
            due_date,
            urgency_level,
            fail_type: FailureType::Other,
            total_materials: 0,
            unscheduled_count: 0,
            unscheduled_weight_t: 0.0,
            completion_rate: 0.0,
            days_to_due: 0,
            failure_reasons: Vec::new(),
            blocking_factors: Vec::new(),
            suggested_actions: Vec::new(),
        }
    }

    /// 设置失败类型
    pub fn set_fail_type(&mut self, fail_type: FailureType) {
        self.fail_type = fail_type;
    }

    /// 设置材料信息
    pub fn set_material_info(&mut self, total: i32, unscheduled: i32, unscheduled_weight: f64) {
        self.total_materials = total;
        self.unscheduled_count = unscheduled;
        self.unscheduled_weight_t = unscheduled_weight;
        self.completion_rate = if total > 0 {
            (total - unscheduled) as f64 / total as f64
        } else {
            0.0
        };
    }

    /// 设置交货期信息
    pub fn set_due_date_info(&mut self, days_to_due: i32) {
        self.days_to_due = days_to_due;

        // 自动推断失败类型
        if days_to_due < 0 {
            self.fail_type = FailureType::Overdue;
        } else if days_to_due <= 3 && self.completion_rate < 0.8 {
            self.fail_type = FailureType::NearDueImpossible;
        }
    }

    /// 添加失败原因
    pub fn add_failure_reason(&mut self, reason: String) {
        self.failure_reasons.push(reason);
    }

    /// 添加阻塞因素
    pub fn add_blocking_factor(
        &mut self,
        code: String,
        description: String,
        affected_count: i32,
        affected_weight_t: f64,
        is_removable: bool,
    ) {
        self.blocking_factors.push(BlockingFactor {
            code,
            description,
            affected_count,
            affected_weight_t,
            is_removable,
        });
    }

    /// 添加建议措施
    pub fn add_suggested_action(&mut self, action: String) {
        self.suggested_actions.push(action);
    }

    /// 判断是否为高紧急度
    pub fn is_high_urgency(&self) -> bool {
        matches!(self.urgency_level.as_str(), "L2" | "L3")
    }

    /// 判断是否超期
    pub fn is_overdue(&self) -> bool {
        self.days_to_due < 0
    }

    /// 判断是否存在可解除的阻塞
    pub fn has_removable_blocks(&self) -> bool {
        self.blocking_factors.iter().any(|f| f.is_removable)
    }

    /// 获取不可解除的阻塞数量
    pub fn unremovable_block_count(&self) -> usize {
        self.blocking_factors
            .iter()
            .filter(|f| !f.is_removable)
            .count()
    }
}

impl FailureStats {
    /// 创建新的失败统计
    pub fn new(version_id: String) -> Self {
        Self {
            version_id,
            total_failures: 0,
            overdue_count: 0,
            near_due_impossible_count: 0,
            capacity_shortage_count: 0,
            structure_conflict_count: 0,
            total_affected_materials: 0,
            total_affected_weight_t: 0.0,
        }
    }

    /// 添加失败记录到统计
    pub fn add_failure(&mut self, failure: &OrderFailure) {
        self.total_failures += 1;
        self.total_affected_materials += failure.unscheduled_count;
        self.total_affected_weight_t += failure.unscheduled_weight_t;

        match failure.fail_type {
            FailureType::Overdue => self.overdue_count += 1,
            FailureType::NearDueImpossible => self.near_due_impossible_count += 1,
            FailureType::CapacityShortage => self.capacity_shortage_count += 1,
            FailureType::StructureConflict => self.structure_conflict_count += 1,
            _ => {}
        }
    }
}

impl std::fmt::Display for OrderFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (due: {}, urgency: {}, completion: {:.1}%)",
            self.contract_no,
            self.due_date,
            self.urgency_level,
            self.completion_rate * 100.0
        )
    }
}

impl std::fmt::Display for FailureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureType::Overdue => write!(f, "超期未完成"),
            FailureType::NearDueImpossible => write!(f, "临期无法完成"),
            FailureType::CapacityShortage => write!(f, "产能不足"),
            FailureType::StructureConflict => write!(f, "结构冲突"),
            FailureType::ColdStockNotReady => write!(f, "冷料未适温"),
            FailureType::Other => write!(f, "其他"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_failure_creation() {
        let failure = OrderFailure::new(
            "C001".to_string(),
            "V001".to_string(),
            "2026-02-01".to_string(),
            "L2".to_string(),
        );

        assert_eq!(failure.contract_no, "C001");
        assert_eq!(failure.urgency_level, "L2");
        assert!(failure.is_high_urgency());
    }

    #[test]
    fn test_material_info() {
        let mut failure = OrderFailure::new(
            "C001".to_string(),
            "V001".to_string(),
            "2026-02-01".to_string(),
            "L2".to_string(),
        );

        failure.set_material_info(10, 3, 150.0);

        assert_eq!(failure.total_materials, 10);
        assert_eq!(failure.unscheduled_count, 3);
        assert_eq!(failure.completion_rate, 0.7);
    }

    #[test]
    fn test_due_date_info() {
        let mut failure = OrderFailure::new(
            "C001".to_string(),
            "V001".to_string(),
            "2026-02-01".to_string(),
            "L2".to_string(),
        );

        failure.set_due_date_info(-5);
        assert!(failure.is_overdue());
        assert_eq!(failure.fail_type, FailureType::Overdue);
    }

    #[test]
    fn test_blocking_factors() {
        let mut failure = OrderFailure::new(
            "C001".to_string(),
            "V001".to_string(),
            "2026-02-01".to_string(),
            "L2".to_string(),
        );

        failure.add_blocking_factor("CAP".to_string(), "产能不足".to_string(), 2, 100.0, false);
        failure.add_blocking_factor("STRUCT".to_string(), "结构冲突".to_string(), 1, 50.0, true);

        assert_eq!(failure.blocking_factors.len(), 2);
        assert!(failure.has_removable_blocks());
        assert_eq!(failure.unremovable_block_count(), 1);
    }

    #[test]
    fn test_failure_stats() {
        let mut stats = FailureStats::new("V001".to_string());

        let mut failure1 = OrderFailure::new(
            "C001".to_string(),
            "V001".to_string(),
            "2026-02-01".to_string(),
            "L2".to_string(),
        );
        failure1.set_fail_type(FailureType::Overdue);
        failure1.set_material_info(10, 3, 150.0);

        let mut failure2 = OrderFailure::new(
            "C002".to_string(),
            "V001".to_string(),
            "2026-02-05".to_string(),
            "L1".to_string(),
        );
        failure2.set_fail_type(FailureType::CapacityShortage);
        failure2.set_material_info(5, 2, 100.0);

        stats.add_failure(&failure1);
        stats.add_failure(&failure2);

        assert_eq!(stats.total_failures, 2);
        assert_eq!(stats.overdue_count, 1);
        assert_eq!(stats.capacity_shortage_count, 1);
        assert_eq!(stats.total_affected_materials, 5);
        assert_eq!(stats.total_affected_weight_t, 250.0);
    }
}

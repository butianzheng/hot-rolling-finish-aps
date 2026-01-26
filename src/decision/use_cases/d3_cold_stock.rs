// ==========================================
// 热轧精整排产系统 - D3 用例：哪些冷料压库
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 5.3 节
// 职责: 回答"哪些冷料压库"，返回冷料分桶与趋势
// ==========================================

use serde::{Deserialize, Serialize};

/// D3 用例：哪些冷料压库
///
/// 输入: version_id, machine_code (可选)
/// 输出: Vec<ColdStockProfile> 按 pressure_score 降序
/// 刷新触发: material_state_changed, plan_item_changed
pub trait ColdStockUseCase {
    /// 查询冷料压库概况
    fn get_cold_stock_profile(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
    ) -> Result<Vec<ColdStockProfile>, String>;

    /// 查询特定机组的冷料分桶
    fn get_machine_cold_stock(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> Result<Vec<ColdStockProfile>, String>;

    /// 统计冷料总量
    fn get_cold_stock_summary(&self, version_id: &str) -> Result<ColdStockSummary, String>;
}

/// 冷料压库概况
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColdStockProfile {
    /// 版本 ID
    pub version_id: String,

    /// 机组代码
    pub machine_code: String,

    /// 年龄分桶 (例如: "0-7", "8-14", "15-30", "30+")
    pub age_bin: String,

    /// 最小天数
    pub age_min_days: i32,

    /// 最大天数 (None 表示无上限)
    pub age_max_days: Option<i32>,

    /// 材料数量
    pub count: i32,

    /// 总重量 (吨)
    pub weight_t: f64,

    /// 压库分数 (0-100)
    pub pressure_score: f64,

    /// 压库等级 (LOW/MEDIUM/HIGH/CRITICAL)
    pub pressure_level: String,

    /// 压库原因列表
    pub reasons: Vec<String>,

    /// 结构缺口描述
    pub structure_gap: Option<String>,

    /// 预计适温日期 (YYYY-MM-DD)
    pub estimated_ready_date: Option<String>,

    /// 是否可强制释放
    pub can_force_release: bool,

    /// 建议措施
    pub suggested_actions: Vec<String>,
}

/// 冷料总量统计
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColdStockSummary {
    /// 版本 ID
    pub version_id: String,

    /// 冷料总数
    pub total_count: i32,

    /// 冷料总重量 (吨)
    pub total_weight_t: f64,

    /// 按机组分组统计
    pub by_machine: Vec<MachineStockStat>,

    /// 按年龄分组统计
    pub by_age: Vec<AgeStockStat>,

    /// 高压库机组数量
    pub high_pressure_machines: i32,

    /// 平均压库天数
    pub avg_age_days: f64,
}

/// 机组库存统计
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineStockStat {
    /// 机组代码
    pub machine_code: String,

    /// 材料数量
    pub count: i32,

    /// 总重量 (吨)
    pub weight_t: f64,

    /// 压库分数
    pub pressure_score: f64,
}

/// 年龄库存统计
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgeStockStat {
    /// 年龄分桶
    pub age_bin: String,

    /// 材料数量
    pub count: i32,

    /// 总重量 (吨)
    pub weight_t: f64,
}

impl ColdStockProfile {
    /// 创建新的冷料概况
    pub fn new(
        version_id: String,
        machine_code: String,
        age_bin: String,
        age_min_days: i32,
        age_max_days: Option<i32>,
    ) -> Self {
        Self {
            version_id,
            machine_code,
            age_bin,
            age_min_days,
            age_max_days,
            count: 0,
            weight_t: 0.0,
            pressure_score: 0.0,
            pressure_level: "LOW".to_string(),
            reasons: Vec::new(),
            structure_gap: None,
            estimated_ready_date: None,
            can_force_release: false,
            suggested_actions: Vec::new(),
        }
    }

    /// 添加材料到分桶
    pub fn add_material(&mut self, weight_t: f64) {
        self.count += 1;
        self.weight_t += weight_t;
        self.recalculate_pressure();
    }

    /// 重新计算压库分数
    fn recalculate_pressure(&mut self) {
        // 基于年龄和数量计算压库分数
        let age_factor = match self.age_min_days {
            0..=7 => 0.2,
            8..=14 => 0.4,
            15..=30 => 0.7,
            _ => 1.0,
        };

        let count_factor = match self.count {
            0..=5 => 0.3,
            6..=10 => 0.6,
            11..=20 => 0.8,
            _ => 1.0,
        };

        self.pressure_score = (age_factor * 0.6 + count_factor * 0.4) * 100.0;

        // 确定压库等级
        self.pressure_level = match self.pressure_score {
            s if s >= 80.0 => "CRITICAL".to_string(),
            s if s >= 60.0 => "HIGH".to_string(),
            s if s >= 40.0 => "MEDIUM".to_string(),
            _ => "LOW".to_string(),
        };
    }

    /// 设置压库原因
    pub fn add_reason(&mut self, reason: String) {
        self.reasons.push(reason);
    }

    /// 设置结构缺口
    pub fn set_structure_gap(&mut self, gap: String) {
        self.structure_gap = Some(gap);
    }

    /// 设置预计适温日期
    pub fn set_estimated_ready_date(&mut self, date: String) {
        self.estimated_ready_date = Some(date);
    }

    /// 设置是否可强制释放
    pub fn set_can_force_release(&mut self, can_release: bool) {
        self.can_force_release = can_release;
    }

    /// 添加建议措施
    pub fn add_suggested_action(&mut self, action: String) {
        self.suggested_actions.push(action);
    }

    /// 判断是否为高压库
    pub fn is_high_pressure(&self) -> bool {
        matches!(self.pressure_level.as_str(), "HIGH" | "CRITICAL")
    }

    /// 判断是否为长期库存
    pub fn is_long_term(&self) -> bool {
        self.age_min_days >= 30
    }

    /// 获取平均年龄
    pub fn avg_age(&self) -> f64 {
        if let Some(max) = self.age_max_days {
            (self.age_min_days + max) as f64 / 2.0
        } else {
            self.age_min_days as f64
        }
    }
}

impl ColdStockSummary {
    /// 创建新的冷料总量统计
    pub fn new(version_id: String) -> Self {
        Self {
            version_id,
            total_count: 0,
            total_weight_t: 0.0,
            by_machine: Vec::new(),
            by_age: Vec::new(),
            high_pressure_machines: 0,
            avg_age_days: 0.0,
        }
    }

    /// 添加机组统计
    pub fn add_machine_stat(&mut self, stat: MachineStockStat) {
        self.total_count += stat.count;
        self.total_weight_t += stat.weight_t;
        if stat.pressure_score >= 60.0 {
            self.high_pressure_machines += 1;
        }
        self.by_machine.push(stat);
    }

    /// 添加年龄统计
    pub fn add_age_stat(&mut self, stat: AgeStockStat) {
        self.by_age.push(stat);
    }

    /// 计算平均年龄
    pub fn calculate_avg_age(&mut self, total_age_days: f64) {
        if self.total_count > 0 {
            self.avg_age_days = total_age_days / self.total_count as f64;
        }
    }

    /// 判断是否存在高压库
    pub fn has_high_pressure(&self) -> bool {
        self.high_pressure_machines > 0
    }
}

impl std::fmt::Display for ColdStockProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{} (age: {}, count: {}, pressure: {})",
            self.machine_code, self.age_bin, self.age_min_days, self.count, self.pressure_level
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cold_stock_profile_creation() {
        let profile = ColdStockProfile::new(
            "V001".to_string(),
            "H032".to_string(),
            "15-30".to_string(),
            15,
            Some(30),
        );

        assert_eq!(profile.machine_code, "H032");
        assert_eq!(profile.age_bin, "15-30");
        assert_eq!(profile.count, 0);
    }

    #[test]
    fn test_add_material() {
        let mut profile = ColdStockProfile::new(
            "V001".to_string(),
            "H032".to_string(),
            "15-30".to_string(),
            15,
            Some(30),
        );

        profile.add_material(100.0);
        profile.add_material(50.0);

        assert_eq!(profile.count, 2);
        assert_eq!(profile.weight_t, 150.0);
        assert!(profile.pressure_score > 0.0);
    }

    #[test]
    fn test_pressure_calculation() {
        let mut profile = ColdStockProfile::new(
            "V001".to_string(),
            "H032".to_string(),
            "30+".to_string(),
            30,
            None,
        );

        for _ in 0..25 {
            profile.add_material(50.0);
        }

        assert!(profile.is_high_pressure());
        assert!(profile.is_long_term());
    }

    #[test]
    fn test_avg_age() {
        let profile = ColdStockProfile::new(
            "V001".to_string(),
            "H032".to_string(),
            "15-30".to_string(),
            15,
            Some(30),
        );

        assert_eq!(profile.avg_age(), 22.5);
    }

    #[test]
    fn test_cold_stock_summary() {
        let mut summary = ColdStockSummary::new("V001".to_string());

        summary.add_machine_stat(MachineStockStat {
            machine_code: "H032".to_string(),
            count: 10,
            weight_t: 500.0,
            pressure_score: 70.0,
        });

        summary.add_machine_stat(MachineStockStat {
            machine_code: "H033".to_string(),
            count: 5,
            weight_t: 250.0,
            pressure_score: 40.0,
        });

        assert_eq!(summary.total_count, 15);
        assert_eq!(summary.total_weight_t, 750.0);
        assert_eq!(summary.high_pressure_machines, 1);
        assert!(summary.has_high_pressure());
    }

    #[test]
    fn test_structure_gap() {
        let mut profile = ColdStockProfile::new(
            "V001".to_string(),
            "H032".to_string(),
            "15-30".to_string(),
            15,
            Some(30),
        );

        profile.set_structure_gap("缺少 1250mm 宽度".to_string());
        assert!(profile.structure_gap.is_some());
    }
}

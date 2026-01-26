// ==========================================
// 热轧精整排产系统 - 决策对象：冷料桶
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 6 节
// 职责: 定义冷料桶决策对象，按库存天数分桶统计冷料压库情况
// ==========================================

use serde::{Deserialize, Serialize};

/// 冷料桶 (ColdStockBucket)
///
/// 按 stock_age_days/状态时间等口径分桶，输出压库/结构风险。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColdStockBucket {
    /// 所属版本 ID
    pub version_id: String,

    /// 机组代码
    pub machine_code: String,

    /// 年龄区间 (例如: "0-7", "7-14", "14-30", "30+")
    pub age_bin: String,

    /// 区间下限 (天)
    pub age_min_days: i32,

    /// 区间上限 (天，None 表示无上限)
    pub age_max_days: Option<i32>,

    /// 材料数量
    pub count: i32,

    /// 总重量 (吨)
    pub weight_t: f64,

    /// 压库压力分数 (0-100)
    pub pressure_score: f64,

    /// 压库原因列表
    pub reasons: Vec<String>,

    /// 结构缺口描述
    pub structure_gap: Option<String>,
}

impl ColdStockBucket {
    /// 创建新的冷料桶
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
            reasons: Vec::new(),
            structure_gap: None,
        }
    }

    /// 创建标准年龄区间的冷料桶
    pub fn from_age_range(
        version_id: String,
        machine_code: String,
        age_min_days: i32,
        age_max_days: Option<i32>,
    ) -> Self {
        let age_bin = if let Some(max) = age_max_days {
            format!("{}-{}", age_min_days, max)
        } else {
            format!("{}+", age_min_days)
        };

        Self::new(version_id, machine_code, age_bin, age_min_days, age_max_days)
    }

    /// 添加材料到桶中
    pub fn add_material(&mut self, weight_t: f64) {
        self.count += 1;
        self.weight_t += weight_t;
        self.recalculate_pressure();
    }

    /// 批量添加材料
    pub fn add_materials(&mut self, count: i32, total_weight_t: f64) {
        self.count += count;
        self.weight_t += total_weight_t;
        self.recalculate_pressure();
    }

    /// 重新计算压库压力分数
    fn recalculate_pressure(&mut self) {
        // 压力分数计算逻辑：
        // 1. 基础分数：库存天数越长，分数越高
        // 2. 重量因子：重量越大，分数越高
        // 3. 数量因子：数量越多，分数越高

        let age_factor = if let Some(max) = self.age_max_days {
            (self.age_min_days + max) as f64 / 2.0
        } else {
            self.age_min_days as f64 * 1.5
        };

        // 年龄因子：30+ 天的冷料基础分 30-45 分
        let age_score = (age_factor / 1.5).min(30.0);
        // 重量因子：每 500 吨加 10 分，最多 40 分
        let weight_score = (self.weight_t / 500.0 * 10.0).min(40.0);
        // 数量因子：每 10 个加 3 分，最多 30 分
        let count_score = (self.count as f64 / 10.0 * 3.0).min(30.0);

        self.pressure_score = (age_score + weight_score + count_score).min(100.0);
    }

    /// 添加压库原因
    pub fn add_reason(&mut self, reason: String) {
        self.reasons.push(reason);
    }

    /// 设置结构缺口
    pub fn set_structure_gap(&mut self, gap: String) {
        self.structure_gap = Some(gap);
    }

    /// 判断是否为高压力桶 (压力分数 > 阈值)
    pub fn is_high_pressure(&self, threshold: f64) -> bool {
        self.pressure_score > threshold
    }

    /// 判断是否为空桶
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// 获取平均单件重量
    pub fn avg_weight_per_piece(&self) -> f64 {
        if self.count > 0 {
            self.weight_t / self.count as f64
        } else {
            0.0
        }
    }

    /// 获取压力等级描述
    pub fn pressure_level(&self) -> &str {
        match self.pressure_score {
            s if s >= 80.0 => "严重",
            s if s >= 60.0 => "高",
            s if s >= 40.0 => "中",
            s if s >= 20.0 => "低",
            _ => "正常",
        }
    }
}

impl std::fmt::Display for ColdStockBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{} (count: {}, weight: {:.1}t, pressure: {:.1})",
            self.machine_code,
            self.age_bin,
            self.count,
            self.weight_t,
            self.pressure_score
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cold_stock_bucket_creation() {
        let bucket = ColdStockBucket::from_age_range(
            "V001".to_string(),
            "H032".to_string(),
            7,
            Some(14),
        );

        assert_eq!(bucket.age_bin, "7-14");
        assert_eq!(bucket.age_min_days, 7);
        assert_eq!(bucket.age_max_days, Some(14));
        assert_eq!(bucket.count, 0);
        assert_eq!(bucket.weight_t, 0.0);
    }

    #[test]
    fn test_add_material() {
        let mut bucket = ColdStockBucket::from_age_range(
            "V001".to_string(),
            "H032".to_string(),
            7,
            Some(14),
        );

        bucket.add_material(100.5);
        assert_eq!(bucket.count, 1);
        assert_eq!(bucket.weight_t, 100.5);

        bucket.add_material(50.3);
        assert_eq!(bucket.count, 2);
        assert_eq!(bucket.weight_t, 150.8);
    }

    #[test]
    fn test_pressure_calculation() {
        let mut bucket = ColdStockBucket::from_age_range(
            "V001".to_string(),
            "H032".to_string(),
            30,
            None,
        );

        // 添加大量冷料
        bucket.add_materials(50, 5000.0);

        // 压力分数应该较高
        assert!(bucket.pressure_score > 50.0);
        assert!(bucket.is_high_pressure(50.0));
    }

    #[test]
    fn test_pressure_level() {
        let mut bucket = ColdStockBucket::from_age_range(
            "V001".to_string(),
            "H032".to_string(),
            30,
            None,
        );

        bucket.pressure_score = 85.0;
        assert_eq!(bucket.pressure_level(), "严重");

        bucket.pressure_score = 65.0;
        assert_eq!(bucket.pressure_level(), "高");

        bucket.pressure_score = 45.0;
        assert_eq!(bucket.pressure_level(), "中");

        bucket.pressure_score = 25.0;
        assert_eq!(bucket.pressure_level(), "低");

        bucket.pressure_score = 10.0;
        assert_eq!(bucket.pressure_level(), "正常");
    }

    #[test]
    fn test_avg_weight() {
        let mut bucket = ColdStockBucket::from_age_range(
            "V001".to_string(),
            "H032".to_string(),
            7,
            Some(14),
        );

        bucket.add_materials(10, 1000.0);
        assert_eq!(bucket.avg_weight_per_piece(), 100.0);
    }

    #[test]
    fn test_reasons_and_gap() {
        let mut bucket = ColdStockBucket::from_age_range(
            "V001".to_string(),
            "H032".to_string(),
            30,
            None,
        );

        bucket.add_reason("产能不足".to_string());
        bucket.add_reason("结构不匹配".to_string());
        bucket.set_structure_gap("缺少宽度 1500mm 的产能".to_string());

        assert_eq!(bucket.reasons.len(), 2);
        assert!(bucket.structure_gap.is_some());
    }
}

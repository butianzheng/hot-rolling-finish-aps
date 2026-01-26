// ==========================================
// 热轧精整排产系统 - D4 用例：哪个机组最堵
// ==========================================
// 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 5.4 节
// 职责: 回答"哪个机组最堵"，返回机组-日热力图与明细
// ==========================================

use serde::{Deserialize, Serialize};

/// D4 用例：哪个机组最堵
///
/// 输入: version_id, machine_code (可选), date_range
/// 输出: Vec<MachineBottleneckProfile> 按 bottleneck_score 降序
/// 刷新触发: plan_item_changed, capacity_pool_changed
pub trait MachineBottleneckUseCase {
    /// 查询机组堵塞概况
    fn get_machine_bottleneck_profile(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<MachineBottleneckProfile>, String>;

    /// 查询最堵塞的机组-日组合
    fn get_top_bottlenecks(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        top_n: usize,
    ) -> Result<Vec<MachineBottleneckProfile>, String>;

    /// 获取机组堵塞热力图数据
    fn get_bottleneck_heatmap(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<BottleneckHeatmap, String>;
}

/// 机组堵塞概况
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineBottleneckProfile {
    /// 版本 ID
    pub version_id: String,

    /// 机组代码
    pub machine_code: String,

    /// 排产日期 (YYYY-MM-DD)
    pub plan_date: String,

    /// 堵塞分数 (0-100)
    pub bottleneck_score: f64,

    /// 堵塞等级 (NONE/LOW/MEDIUM/HIGH/CRITICAL)
    pub bottleneck_level: String,

    /// 堵塞类型
    pub bottleneck_types: Vec<BottleneckType>,

    /// 堵塞原因列表
    pub reasons: Vec<BottleneckReason>,

    /// 剩余产能 (吨)
    pub remaining_capacity_t: f64,

    /// 产能利用率 (0.0-1.0)
    pub capacity_utilization: f64,

    /// 是否需要换辊
    pub needs_roll_change: bool,

    /// 结构违规数量
    pub structure_violations: i32,

    /// 待排材料数量
    pub pending_materials: i32,

    /// 建议措施
    pub suggested_actions: Vec<String>,
}

/// 堵塞类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottleneckType {
    /// 产能堵塞
    Capacity,
    /// 结构堵塞
    Structure,
    /// 换辊堵塞
    RollChange,
    /// 冷料堵塞
    ColdStock,
    /// 复合堵塞
    Mixed,
}

/// 堵塞原因
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BottleneckReason {
    /// 原因代码
    pub code: String,

    /// 原因描述
    pub description: String,

    /// 严重程度 (0.0-1.0)
    pub severity: f64,

    /// 影响材料数量
    pub affected_materials: i32,
}

/// 堵塞热力图
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BottleneckHeatmap {
    /// 版本 ID
    pub version_id: String,

    /// 日期范围
    pub date_range: (String, String),

    /// 机组列表
    pub machines: Vec<String>,

    /// 热力图数据 (machine_code -> date -> score)
    pub data: Vec<HeatmapCell>,

    /// 最大堵塞分数
    pub max_score: f64,

    /// 平均堵塞分数
    pub avg_score: f64,
}

/// 热力图单元格
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatmapCell {
    /// 机组代码
    pub machine_code: String,

    /// 日期
    pub date: String,

    /// 堵塞分数
    pub score: f64,

    /// 堵塞等级
    pub level: String,
}

impl MachineBottleneckProfile {
    /// 创建新的机组堵塞概况
    pub fn new(version_id: String, machine_code: String, plan_date: String) -> Self {
        Self {
            version_id,
            machine_code,
            plan_date,
            bottleneck_score: 0.0,
            bottleneck_level: "NONE".to_string(),
            bottleneck_types: Vec::new(),
            reasons: Vec::new(),
            remaining_capacity_t: 0.0,
            capacity_utilization: 0.0,
            needs_roll_change: false,
            structure_violations: 0,
            pending_materials: 0,
            suggested_actions: Vec::new(),
        }
    }

    /// 设置产能信息
    pub fn set_capacity_info(&mut self, remaining_t: f64, utilization: f64) {
        self.remaining_capacity_t = remaining_t;
        self.capacity_utilization = utilization;

        // 判断是否为产能堵塞
        if utilization >= 0.95 {
            self.add_bottleneck_type(BottleneckType::Capacity);
        }
    }

    /// 设置结构信息
    pub fn set_structure_info(&mut self, violations: i32) {
        self.structure_violations = violations;

        // 判断是否为结构堵塞
        if violations > 0 {
            self.add_bottleneck_type(BottleneckType::Structure);
        }
    }

    /// 设置换辊信息
    pub fn set_roll_change_info(&mut self, needs_change: bool) {
        self.needs_roll_change = needs_change;

        // 判断是否为换辊堵塞
        if needs_change {
            self.add_bottleneck_type(BottleneckType::RollChange);
        }
    }

    /// 添加堵塞类型
    fn add_bottleneck_type(&mut self, bottleneck_type: BottleneckType) {
        if !self.bottleneck_types.contains(&bottleneck_type) {
            self.bottleneck_types.push(bottleneck_type);
        }

        // 更新堵塞等级
        if self.bottleneck_types.len() > 1 {
            self.bottleneck_types.clear();
            self.bottleneck_types.push(BottleneckType::Mixed);
        }
    }

    /// 添加堵塞原因
    pub fn add_reason(
        &mut self,
        code: String,
        description: String,
        severity: f64,
        affected_materials: i32,
    ) {
        self.reasons.push(BottleneckReason {
            code,
            description,
            severity,
            affected_materials,
        });
        self.recalculate_score();
    }

    /// 重新计算堵塞分数
    fn recalculate_score(&mut self) {
        if self.reasons.is_empty() {
            self.bottleneck_score = 0.0;
            self.bottleneck_level = "NONE".to_string();
            return;
        }

        // 计算加权堵塞分数
        let max_severity = self
            .reasons
            .iter()
            .map(|r| r.severity)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);

        self.bottleneck_score = (max_severity * 100.0).min(100.0);

        // 确定堵塞等级
        self.bottleneck_level = match self.bottleneck_score {
            s if s >= 90.0 => "CRITICAL".to_string(),
            s if s >= 70.0 => "HIGH".to_string(),
            s if s >= 50.0 => "MEDIUM".to_string(),
            s if s > 0.0 => "LOW".to_string(),
            _ => "NONE".to_string(),
        };
    }

    /// 添加建议措施
    pub fn add_suggested_action(&mut self, action: String) {
        self.suggested_actions.push(action);
    }

    /// 判断是否为严重堵塞
    pub fn is_severe(&self) -> bool {
        matches!(self.bottleneck_level.as_str(), "HIGH" | "CRITICAL")
    }

    /// 判断是否存在堵塞
    pub fn has_bottleneck(&self) -> bool {
        self.bottleneck_score > 0.0
    }

    /// 获取主要堵塞原因
    pub fn primary_reason(&self) -> Option<&BottleneckReason> {
        self.reasons
            .iter()
            .max_by(|a, b| a.severity.partial_cmp(&b.severity).unwrap_or(std::cmp::Ordering::Equal))
    }
}

impl BottleneckHeatmap {
    /// 创建新的堵塞热力图
    pub fn new(version_id: String, start_date: String, end_date: String) -> Self {
        Self {
            version_id,
            date_range: (start_date, end_date),
            machines: Vec::new(),
            data: Vec::new(),
            max_score: 0.0,
            avg_score: 0.0,
        }
    }

    /// 添加热力图单元格
    pub fn add_cell(&mut self, machine_code: String, date: String, score: f64, level: String) {
        self.data.push(HeatmapCell {
            machine_code: machine_code.clone(),
            date,
            score,
            level,
        });

        // 更新机组列表
        if !self.machines.contains(&machine_code) {
            self.machines.push(machine_code);
        }

        // 更新统计信息
        self.max_score = self.max_score.max(score);
        self.recalculate_avg();
    }

    /// 重新计算平均分数
    fn recalculate_avg(&mut self) {
        if !self.data.is_empty() {
            let sum: f64 = self.data.iter().map(|c| c.score).sum();
            self.avg_score = sum / self.data.len() as f64;
        }
    }

    /// 获取特定机组-日的分数
    pub fn get_score(&self, machine_code: &str, date: &str) -> Option<f64> {
        self.data
            .iter()
            .find(|c| c.machine_code == machine_code && c.date == date)
            .map(|c| c.score)
    }
}

impl std::fmt::Display for MachineBottleneckProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{} (score: {:.1}, level: {}, util: {:.1}%)",
            self.machine_code,
            self.plan_date,
            self.bottleneck_score,
            self.bottleneck_level,
            self.capacity_utilization * 100.0
        )
    }
}

impl std::fmt::Display for BottleneckType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BottleneckType::Capacity => write!(f, "产能堵塞"),
            BottleneckType::Structure => write!(f, "结构堵塞"),
            BottleneckType::RollChange => write!(f, "换辊堵塞"),
            BottleneckType::ColdStock => write!(f, "冷料堵塞"),
            BottleneckType::Mixed => write!(f, "复合堵塞"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_machine_bottleneck_profile_creation() {
        let profile = MachineBottleneckProfile::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-23".to_string(),
        );

        assert_eq!(profile.machine_code, "H032");
        assert_eq!(profile.bottleneck_level, "NONE");
        assert!(!profile.has_bottleneck());
    }

    #[test]
    fn test_capacity_info() {
        let mut profile = MachineBottleneckProfile::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-23".to_string(),
        );

        profile.set_capacity_info(50.0, 0.96);

        assert_eq!(profile.remaining_capacity_t, 50.0);
        assert_eq!(profile.capacity_utilization, 0.96);
        assert!(profile.bottleneck_types.contains(&BottleneckType::Capacity));
    }

    #[test]
    fn test_add_reason() {
        let mut profile = MachineBottleneckProfile::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-23".to_string(),
        );

        profile.add_reason("CAP_FULL".to_string(), "产能已满".to_string(), 0.95, 10);

        assert_eq!(profile.reasons.len(), 1);
        assert!(profile.is_severe());
        assert!(profile.has_bottleneck());
    }

    #[test]
    fn test_mixed_bottleneck() {
        let mut profile = MachineBottleneckProfile::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-23".to_string(),
        );

        profile.set_capacity_info(50.0, 0.96);
        profile.set_structure_info(3);

        assert!(profile.bottleneck_types.contains(&BottleneckType::Mixed));
    }

    #[test]
    fn test_primary_reason() {
        let mut profile = MachineBottleneckProfile::new(
            "V001".to_string(),
            "H032".to_string(),
            "2026-01-23".to_string(),
        );

        profile.add_reason("R1".to_string(), "原因1".to_string(), 0.7, 5);
        profile.add_reason("R2".to_string(), "原因2".to_string(), 0.9, 8);

        let primary = profile.primary_reason();
        assert!(primary.is_some());
        assert_eq!(primary.unwrap().code, "R2");
    }

    #[test]
    fn test_bottleneck_heatmap() {
        let mut heatmap = BottleneckHeatmap::new(
            "V001".to_string(),
            "2026-01-23".to_string(),
            "2026-01-25".to_string(),
        );

        heatmap.add_cell("H032".to_string(), "2026-01-23".to_string(), 85.0, "HIGH".to_string());
        heatmap.add_cell("H032".to_string(), "2026-01-24".to_string(), 60.0, "MEDIUM".to_string());
        heatmap.add_cell("H033".to_string(), "2026-01-23".to_string(), 40.0, "LOW".to_string());

        assert_eq!(heatmap.machines.len(), 2);
        assert_eq!(heatmap.data.len(), 3);
        assert_eq!(heatmap.max_score, 85.0);
        assert!(heatmap.avg_score > 0.0);

        let score = heatmap.get_score("H032", "2026-01-23");
        assert_eq!(score, Some(85.0));
    }
}

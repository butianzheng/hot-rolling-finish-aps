// ==========================================
// 热轧精整排产系统 - 换辊领域模型
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART B3 产能与换辊
// 依据: Engine_Specs_v0.3_Integrated.md - 7. Roll Campaign Engine
// 依据: Engine_Specs_v0.3_Integrated.md - 14. RollCycle State Model [v0.4+]
// ==========================================

use crate::domain::types::{AnchorSource, RollStatus};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

// ==========================================
// RollerCampaign - 换辊窗口
// ==========================================
// 红线: 换辊硬停止优先于材料优先级
// 对齐: schema_v0.1.sql roller_campaign 表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollerCampaign {
    // ===== 主键 (对齐schema) =====
    pub version_id: String,   // 关联排产版本
    pub machine_code: String, // 机组代码
    pub campaign_no: i32,     // 换辊批次号

    // ===== 时间范围 =====
    pub start_date: NaiveDate,       // 开始日期
    pub end_date: Option<NaiveDate>, // 结束日期 (null表示进行中)

    // ===== 吨位统计 (对齐schema字段名) =====
    pub cum_weight_t: f64,        // 累计吨位 (cumulative weight)
    pub suggest_threshold_t: f64, // 建议换辊阈值
    pub hard_limit_t: f64,        // 强制换辊阈值

    // ===== 状态 =====
    pub status: RollStatus, // 换辊状态 (存储为字符串)

    // ===== 路径锚点 [v0.4+] =====
    // 依据: Engine_Specs 14.2 RollCycleState
    pub path_anchor_material_id: Option<String>, // 路径锚点材料ID
    pub path_anchor_width_mm: Option<f64>,       // 锚点宽度 (mm)
    pub path_anchor_thickness_mm: Option<f64>,   // 锚点厚度 (mm)
    pub anchor_source: Option<AnchorSource>,     // 锚点来源类型
}

// ==========================================
// Trait: RollerCampaignMonitor
// ==========================================
// 用途: Roll Campaign Engine 监控逻辑接口
pub trait RollerCampaignMonitor {
    /// 判断是否需要换辊 (建议)
    ///
    /// # 返回
    /// - `true`: 累计吨位 ≥ 建议阈值
    /// - `false`: 累计吨位 < 建议阈值
    fn should_change_roll(&self) -> bool;

    /// 判断是否强制换辊 (硬停止)
    ///
    /// # 返回
    /// - `true`: 累计吨位 ≥ 硬停止阈值
    /// - `false`: 累计吨位 < 硬停止阈值
    fn is_hard_stop(&self) -> bool;

    /// 计算剩余可用吨位
    ///
    /// # 返回
    /// 剩余可用吨位 (hard_limit_t - cum_weight_t)，最小为 0
    fn remaining_tonnage_t(&self) -> f64;

    /// 更新累计吨位
    ///
    /// # 参数
    /// - `weight_t`: 增加的吨位
    fn add_tonnage(&mut self, weight_t: f64);

    /// 计算辊使用率
    ///
    /// # 返回
    /// 辊使用率 (0.0 - 1.0+)，相对于硬停止阈值
    fn utilization_ratio(&self) -> f64;
}

// ==========================================
// RollerCampaignMonitor trait 实现
// ==========================================
impl RollerCampaignMonitor for RollerCampaign {
    /// 判断是否需要换辊 (建议)
    fn should_change_roll(&self) -> bool {
        self.cum_weight_t >= self.suggest_threshold_t
    }

    /// 判断是否强制换辊 (硬停止)
    fn is_hard_stop(&self) -> bool {
        self.cum_weight_t >= self.hard_limit_t
    }

    /// 计算剩余可用吨位
    fn remaining_tonnage_t(&self) -> f64 {
        (self.hard_limit_t - self.cum_weight_t).max(0.0)
    }

    /// 更新累计吨位
    fn add_tonnage(&mut self, weight_t: f64) {
        self.cum_weight_t += weight_t;
    }

    /// 计算辊使用率
    fn utilization_ratio(&self) -> f64 {
        if self.hard_limit_t <= 0.0 {
            return 0.0;
        }
        self.cum_weight_t / self.hard_limit_t
    }
}

// ==========================================
// 辅助方法
// ==========================================
impl RollerCampaign {
    /// 创建新的换辊窗口
    ///
    /// # 参数
    /// - `version_id`: 排产版本ID
    /// - `machine_code`: 机组代码
    /// - `campaign_no`: 换辊批次号
    /// - `start_date`: 开始日期
    /// - `suggest_threshold_t`: 建议换辊阈值 (默认1500.0)
    /// - `hard_limit_t`: 强制换辊阈值 (默认2500.0)
    ///
    /// # 返回
    /// 新的 RollerCampaign 实例
    pub fn new(
        version_id: String,
        machine_code: String,
        campaign_no: i32,
        start_date: NaiveDate,
        suggest_threshold_t: Option<f64>,
        hard_limit_t: Option<f64>,
    ) -> Self {
        Self {
            version_id,
            machine_code,
            campaign_no,
            start_date,
            end_date: None,
            cum_weight_t: 0.0,
            suggest_threshold_t: suggest_threshold_t.unwrap_or(1500.0),
            hard_limit_t: hard_limit_t.unwrap_or(2500.0),
            status: RollStatus::Normal,
            // 锚点字段初始化为 None [v0.4+]
            path_anchor_material_id: None,
            path_anchor_width_mm: None,
            path_anchor_thickness_mm: None,
            anchor_source: None,
        }
    }

    /// 结束换辊窗口
    ///
    /// # 参数
    /// - `end_date`: 结束日期
    pub fn close(&mut self, end_date: NaiveDate) {
        self.end_date = Some(end_date);
    }

    /// 判断是否进行中
    ///
    /// # 返回
    /// - `true`: end_date 为 None
    /// - `false`: end_date 不为 None
    pub fn is_active(&self) -> bool {
        self.end_date.is_none()
    }

    /// 生成唯一ID (用于显示)
    ///
    /// # 返回
    /// 格式: "{version_id}_{machine_code}_{campaign_no}"
    pub fn get_id(&self) -> String {
        format!(
            "{}_{}_C{}",
            self.version_id, self.machine_code, self.campaign_no
        )
    }

    // ==========================================
    // 路径锚点相关方法 [v0.4+]
    // ==========================================
    // 依据: Engine_Specs 14. RollCycle State Model

    /// 更新路径锚点
    ///
    /// # 参数
    /// - `material_id`: 锚点材料ID（S2 种子策略时可为 None）
    /// - `width_mm`: 锚点宽度 (mm)
    /// - `thickness_mm`: 锚点厚度 (mm)
    /// - `source`: 锚点来源类型
    pub fn update_anchor(
        &mut self,
        material_id: Option<String>,
        width_mm: f64,
        thickness_mm: f64,
        source: AnchorSource,
    ) {
        self.path_anchor_material_id = material_id;
        self.path_anchor_width_mm = Some(width_mm);
        self.path_anchor_thickness_mm = Some(thickness_mm);
        self.anchor_source = Some(source);
    }

    /// 重置路径锚点（换辊时调用）
    ///
    /// 依据: Engine_Specs 14.4 周期切换行为
    pub fn reset_anchor(&mut self) {
        self.path_anchor_material_id = None;
        self.path_anchor_width_mm = None;
        self.path_anchor_thickness_mm = None;
        self.anchor_source = Some(AnchorSource::None);
    }

    /// 判断是否有有效锚点
    ///
    /// # 返回
    /// - `true`: 有有效锚点（宽度和厚度均不为空）
    /// - `false`: 无有效锚点
    pub fn has_valid_anchor(&self) -> bool {
        self.path_anchor_width_mm.is_some() && self.path_anchor_thickness_mm.is_some()
    }

    /// 获取锚点宽度（带默认值）
    ///
    /// # 参数
    /// - `default`: 默认值（无锚点时返回）
    ///
    /// # 返回
    /// 锚点宽度或默认值
    pub fn anchor_width_or(&self, default: f64) -> f64 {
        self.path_anchor_width_mm.unwrap_or(default)
    }

    /// 获取锚点厚度（带默认值）
    ///
    /// # 参数
    /// - `default`: 默认值（无锚点时返回）
    ///
    /// # 返回
    /// 锚点厚度或默认值
    pub fn anchor_thickness_or(&self, default: f64) -> f64 {
        self.path_anchor_thickness_mm.unwrap_or(default)
    }
}

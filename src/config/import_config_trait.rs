// ==========================================
// 热轧精整排产系统 - 导入配置读取 Trait
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART E 工程结构
// 依据: Engine_Specs_v0.3_Integrated.md - 0.1 季节模式与适温阈值
// 职责: 定义导入模块所需的配置读取接口（不包含实现）
// 红线: 不包含配置写入、不包含业务逻辑
// ==========================================

use crate::domain::types::{Season, SeasonMode};
use async_trait::async_trait;
use std::error::Error;

// ==========================================
// ImportConfigReader Trait
// ==========================================
// 用途: 导入模块所需的配置读取接口
// 实现者: ConfigManagerImpl（从 config_kv 表读取）
#[async_trait]
pub trait ImportConfigReader: Send + Sync {
    // ===== 季节与适温配置 =====

    /// 获取季节模式
    ///
    /// # 返回
    /// - SeasonMode::Auto: 按月份自动判断
    /// - SeasonMode::Manual: 人工指定
    ///
    /// # 默认值
    /// - AUTO
    async fn get_season_mode(&self) -> Result<SeasonMode, Box<dyn Error>>;

    /// 获取当前季节（仅当 season_mode = MANUAL 时有效）
    ///
    /// # 返回
    /// - Season::Winter 或 Season::Summer
    ///
    /// # 默认值
    /// - WINTER
    async fn get_manual_season(&self) -> Result<Season, Box<dyn Error>>;

    /// 获取冬季月份列表（用于 AUTO 模式判断）
    ///
    /// # 返回
    /// - Vec<u32>: 月份数字列表（如 [11, 12, 1, 2, 3]）
    ///
    /// # 默认值
    /// - [11, 12, 1, 2, 3]
    async fn get_winter_months(&self) -> Result<Vec<u32>, Box<dyn Error>>;

    /// 获取冬季适温天数阈值
    ///
    /// # 返回
    /// - i32: 最小适温天数
    ///
    /// # 默认值
    /// - 3
    async fn get_min_temp_days_winter(&self) -> Result<i32, Box<dyn Error>>;

    /// 获取夏季适温天数阈值
    ///
    /// # 返回
    /// - i32: 最小适温天数
    ///
    /// # 默认值
    /// - 4
    async fn get_min_temp_days_summer(&self) -> Result<i32, Box<dyn Error>>;

    /// 获取当前适温天数阈值（自动判断季节）
    ///
    /// # 参数
    /// - today: 当前日期
    ///
    /// # 返回
    /// - i32: 根据季节模式和当前日期返回对应阈值
    ///
    /// # 逻辑
    /// 1. 读取 season_mode
    /// 2. 若 AUTO：根据 today 月份和 winter_months 判断季节
    /// 3. 若 MANUAL：使用 manual_season
    /// 4. 返回对应季节的 min_temp_days
    async fn get_current_min_temp_days(
        &self,
        today: chrono::NaiveDate,
    ) -> Result<i32, Box<dyn Error>>;

    // ===== 机组代码配置 =====

    /// 获取标准精整机组代码列表（不加偏移的机组）
    ///
    /// # 返回
    /// - Vec<String>: 机组代码列表
    ///
    /// # 默认值
    /// - ["H032", "H033", "H034"]
    ///
    /// # 用途
    /// - 用于判断 rolling_output_age_days 是否需要 +4 偏移
    async fn get_standard_finishing_machines(&self) -> Result<Vec<String>, Box<dyn Error>>;

    /// 获取非标准机组的天数偏移量
    ///
    /// # 返回
    /// - i32: 偏移天数
    ///
    /// # 默认值
    /// - 4
    ///
    /// # 用途
    /// - 用于计算 rolling_output_age_days = output_age_days_raw + offset
    async fn get_machine_offset_days(&self) -> Result<i32, Box<dyn Error>>;

    // ===== 紧急等级阈值配置 =====

    /// 获取 N1 阈值（天数）
    ///
    /// # 返回
    /// - i32: N1 阈值天数
    ///
    /// # 默认值
    /// - 3
    ///
    /// # 用途
    /// - 用于判定紧急等级 L2（临期 N1）
    async fn get_n1_threshold_days(&self) -> Result<i32, Box<dyn Error>>;

    /// 获取 N2 阈值（天数）
    ///
    /// # 返回
    /// - i32: N2 阈值天数
    ///
    /// # 默认值
    /// - 7
    ///
    /// # 用途
    /// - 用于判定紧急等级 L1（临期 N2）
    async fn get_n2_threshold_days(&self) -> Result<i32, Box<dyn Error>>;

    // ===== 数据质量配置 =====

    /// 获取重量异常上限（吨）
    ///
    /// # 返回
    /// - f64: 重量上限（超过则 DQ 警告）
    ///
    /// # 默认值
    /// - 100.0
    ///
    /// # 用途
    /// - 用于检测可能的单位错误（如原始数据单位为 kg）
    async fn get_weight_anomaly_threshold(&self) -> Result<f64, Box<dyn Error>>;

    /// 获取导入批次保留天数
    ///
    /// # 返回
    /// - i32: 保留天数（超期批次可清理）
    ///
    /// # 默认值
    /// - 90
    async fn get_batch_retention_days(&self) -> Result<i32, Box<dyn Error>>;
}

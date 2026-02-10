// ==========================================
// Mock 配置实现 - 用于集成测试
// ==========================================

use async_trait::async_trait;
use chrono::Datelike;
use hot_rolling_aps::config::ImportConfigReader;
use hot_rolling_aps::domain::types::{Season, SeasonMode};
use std::error::Error;

/// Mock 配置结构
#[derive(Debug, Clone)]
pub struct MockConfig {
    pub season_mode: SeasonMode,
    pub winter_months: Vec<u32>,
    pub min_temp_days_winter: i32,
    pub min_temp_days_summer: i32,
    pub n1_threshold_days: i32,
    pub n2_threshold_days: i32,
    pub standard_machines: Vec<String>,
    pub offset_days: i32,
    pub manual_season: Season,
}

impl MockConfig {
    /// 创建默认配置
    pub fn default() -> Self {
        Self {
            season_mode: SeasonMode::Auto,
            winter_months: vec![11, 12, 1, 2, 3],
            min_temp_days_winter: 3,
            min_temp_days_summer: 4,
            n1_threshold_days: 3,
            n2_threshold_days: 7,
            standard_machines: vec!["H032".to_string(), "H033".to_string(), "H034".to_string()],
            offset_days: 4,
            manual_season: Season::Winter,
        }
    }

    /// 创建自定义配置
    pub fn with_n1_n2(n1: i32, n2: i32) -> Self {
        let mut config = Self::default();
        config.n1_threshold_days = n1;
        config.n2_threshold_days = n2;
        config
    }

    /// 创建冬季配置
    pub fn winter() -> Self {
        let mut config = Self::default();
        config.season_mode = SeasonMode::Manual;
        config.manual_season = Season::Winter;
        config.min_temp_days_winter = 3;
        config
    }

    /// 创建夏季配置
    pub fn summer() -> Self {
        let mut config = Self::default();
        config.season_mode = SeasonMode::Manual;
        config.manual_season = Season::Summer;
        config.min_temp_days_summer = 4;
        config
    }
}

#[async_trait]
impl ImportConfigReader for MockConfig {
    async fn get_season_mode(&self) -> Result<SeasonMode, Box<dyn Error>> {
        Ok(self.season_mode)
    }

    async fn get_winter_months(&self) -> Result<Vec<u32>, Box<dyn Error>> {
        Ok(self.winter_months.clone())
    }

    async fn get_manual_season(&self) -> Result<Season, Box<dyn Error>> {
        Ok(self.manual_season)
    }

    async fn get_min_temp_days_winter(&self) -> Result<i32, Box<dyn Error>> {
        Ok(self.min_temp_days_winter)
    }

    async fn get_min_temp_days_summer(&self) -> Result<i32, Box<dyn Error>> {
        Ok(self.min_temp_days_summer)
    }

    async fn get_n1_threshold_days(&self) -> Result<i32, Box<dyn Error>> {
        Ok(self.n1_threshold_days)
    }

    async fn get_n2_threshold_days(&self) -> Result<i32, Box<dyn Error>> {
        Ok(self.n2_threshold_days)
    }

    async fn get_standard_finishing_machines(&self) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(self.standard_machines.clone())
    }

    async fn get_machine_offset_days(&self) -> Result<i32, Box<dyn Error>> {
        Ok(self.offset_days)
    }

    async fn get_current_min_temp_days(
        &self,
        today: chrono::NaiveDate,
    ) -> Result<i32, Box<dyn Error>> {
        // 简化实现：根据季节模式返回对应阈值
        match self.season_mode {
            SeasonMode::Manual => match self.manual_season {
                Season::Winter => Ok(self.min_temp_days_winter),
                Season::Summer => Ok(self.min_temp_days_summer),
            },
            SeasonMode::Auto => {
                let month = today.month();
                if self.winter_months.contains(&month) {
                    Ok(self.min_temp_days_winter)
                } else {
                    Ok(self.min_temp_days_summer)
                }
            }
        }
    }

    async fn get_weight_anomaly_threshold(&self) -> Result<f64, Box<dyn Error>> {
        Ok(100.0) // 默认值
    }

    async fn get_batch_retention_days(&self) -> Result<i32, Box<dyn Error>> {
        Ok(90) // 默认值
    }
}

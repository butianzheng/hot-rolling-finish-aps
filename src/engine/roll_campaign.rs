// ==========================================
// 热轧精整排产系统 - 换辊窗口引擎
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 7. Roll Campaign Engine
// 依据: Claude_Dev_Master_Spec.md - PART B3 产能与换辊
// 红线: 换辊硬停止优先于材料优先级
// ==========================================
// 职责: 换辊窗口计算与硬停止判定
// 输入: 累计吨位 + 换辊阈值
// 输出: 换辊状态 + 换辊原因
// ==========================================

use crate::domain::roller::RollerCampaign;
use crate::domain::types::RollStatus;
use serde_json::json;

// ==========================================
// RollCampaignEngine - 换辊窗口引擎
// ==========================================
/// 换辊窗口引擎
/// 职责: 判定换辊状态、计算剩余吨位、生成换辊原因
/// 红线: 换辊硬停止优先于材料优先级
pub struct RollCampaignEngine {
    // 无状态引擎,不需要注入依赖
    // Repository 操作由调用方处理
}

impl RollCampaignEngine {
    /// 构造函数
    ///
    /// # 返回
    /// 新的 RollCampaignEngine 实例
    pub fn new() -> Self {
        Self {}
    }

    // ==========================================
    // 核心方法
    // ==========================================

    /// 检查换辊状态
    ///
    /// # 参数
    /// - `campaign`: 换辊窗口
    ///
    /// # 返回
    /// (RollStatus, reason)
    ///
    /// # 规则
    /// - accumulated_tonnage_t ≥ hard_limit_t → HARD_STOP
    /// - accumulated_tonnage_t ≥ suggest_threshold_t → SUGGEST
    /// - 否则 → NORMAL
    pub fn check_roll_status(&self, campaign: &RollerCampaign) -> (RollStatus, String) {
        // 1. 判断硬停止
        if campaign.cum_weight_t >= campaign.hard_limit_t {
            let reason = self.generate_hard_stop_reason(campaign);
            return (RollStatus::HardStop, reason);
        }

        // 2. 判断建议换辊
        if campaign.cum_weight_t >= campaign.suggest_threshold_t {
            let reason = self.generate_suggest_reason(campaign);
            return (RollStatus::Suggest, reason);
        }

        // 3. 正常状态
        let reason = self.generate_normal_reason(campaign);
        (RollStatus::Normal, reason)
    }

    // ==========================================
    // 换辊判定辅助方法
    // ==========================================

    /// 判断是否建议换辊
    ///
    /// # 参数
    /// - `accumulated_tonnage_t`: 累计吨位
    /// - `suggest_threshold_t`: 建议阈值
    ///
    /// # 返回
    /// - `true`: 需要建议换辊
    /// - `false`: 不需要建议换辊
    pub fn should_suggest_roll_change(
        &self,
        accumulated_tonnage_t: f64,
        suggest_threshold_t: f64,
    ) -> bool {
        accumulated_tonnage_t >= suggest_threshold_t
    }

    /// 判断是否强制换辊 (硬停止)
    ///
    /// # 参数
    /// - `accumulated_tonnage_t`: 累计吨位
    /// - `hard_limit_t`: 硬停止阈值
    ///
    /// # 返回
    /// - `true`: 强制换辊
    /// - `false`: 不需要强制换辊
    pub fn should_hard_stop(&self, accumulated_tonnage_t: f64, hard_limit_t: f64) -> bool {
        accumulated_tonnage_t >= hard_limit_t
    }

    /// 计算剩余可用吨位
    ///
    /// # 参数
    /// - `campaign`: 换辊窗口
    ///
    /// # 返回
    /// 剩余可用吨位 (hard_limit_t - cum_weight_t)，最小为 0
    pub fn calculate_remaining_tonnage(&self, campaign: &RollerCampaign) -> f64 {
        (campaign.hard_limit_t - campaign.cum_weight_t).max(0.0)
    }

    /// 计算辊使用率
    ///
    /// # 参数
    /// - `campaign`: 换辊窗口
    ///
    /// # 返回
    /// 辊使用率 (0.0 - 1.0+)
    pub fn get_utilization_ratio(&self, campaign: &RollerCampaign) -> f64 {
        if campaign.hard_limit_t <= 0.0 {
            return 0.0;
        }
        campaign.cum_weight_t / campaign.hard_limit_t
    }

    // ==========================================
    // 可解释性方法
    // ==========================================

    /// 生成换辊原因 (可解释性)
    ///
    /// # 参数
    /// - `campaign`: 换辊窗口
    /// - `status`: 换辊状态
    ///
    /// # 返回
    /// JSON 格式的原因说明
    pub fn generate_roll_reason(&self, campaign: &RollerCampaign, status: RollStatus) -> String {
        match status {
            RollStatus::HardStop => self.generate_hard_stop_reason(campaign),
            RollStatus::Suggest => self.generate_suggest_reason(campaign),
            RollStatus::Normal => self.generate_normal_reason(campaign),
        }
    }

    /// 生成硬停止原因
    fn generate_hard_stop_reason(&self, campaign: &RollerCampaign) -> String {
        let utilization = self.get_utilization_ratio(campaign);
        let over_limit = campaign.cum_weight_t - campaign.hard_limit_t;

        json!({
            "status": "HARD_STOP",
            "reason": "累计吨位已达到硬停止阈值，必须立即换辊",
            "cum_weight_t": campaign.cum_weight_t,
            "hard_limit_t": campaign.hard_limit_t,
            "over_limit_t": over_limit,
            "utilization": format!("{:.1}%", utilization * 100.0),
            "machine_code": campaign.machine_code,
            "campaign_no": campaign.campaign_no,
        })
        .to_string()
    }

    /// 生成建议换辊原因
    fn generate_suggest_reason(&self, campaign: &RollerCampaign) -> String {
        let utilization = self.get_utilization_ratio(campaign);
        let remaining = self.calculate_remaining_tonnage(campaign);

        json!({
            "status": "SUGGEST",
            "reason": "累计吨位已达到建议换辊阈值，建议安排换辊",
            "cum_weight_t": campaign.cum_weight_t,
            "suggest_threshold_t": campaign.suggest_threshold_t,
            "hard_limit_t": campaign.hard_limit_t,
            "remaining_t": remaining,
            "utilization": format!("{:.1}%", utilization * 100.0),
            "machine_code": campaign.machine_code,
            "campaign_no": campaign.campaign_no,
        })
        .to_string()
    }

    /// 生成正常状态原因
    fn generate_normal_reason(&self, campaign: &RollerCampaign) -> String {
        let utilization = self.get_utilization_ratio(campaign);
        let remaining = self.calculate_remaining_tonnage(campaign);

        json!({
            "status": "NORMAL",
            "reason": "累计吨位正常，可继续排产",
            "cum_weight_t": campaign.cum_weight_t,
            "suggest_threshold_t": campaign.suggest_threshold_t,
            "hard_limit_t": campaign.hard_limit_t,
            "remaining_t": remaining,
            "utilization": format!("{:.1}%", utilization * 100.0),
            "machine_code": campaign.machine_code,
            "campaign_no": campaign.campaign_no,
        })
        .to_string()
    }

    // ==========================================
    // 配置方法
    // ==========================================

    /// 获取默认换辊阈值
    ///
    /// # 返回
    /// (suggest_threshold_t, hard_limit_t)
    ///
    /// # 默认值
    /// - suggest_threshold_t: 1500.0 吨
    /// - hard_limit_t: 2500.0 吨
    pub fn get_default_thresholds(&self) -> (f64, f64) {
        (1500.0, 2500.0)
    }

    /// 获取换辊阈值 (按机组)
    ///
    /// # 参数
    /// - `_machine_code`: 机组代码 (预留参数，用于未来按机组配置)
    ///
    /// # 返回
    /// (suggest_threshold_t, hard_limit_t)
    ///
    /// # 说明
    /// 当前返回默认值，未来可从配置读取
    pub fn get_roll_thresholds(&self, _machine_code: &str) -> (f64, f64) {
        self.get_default_thresholds()
    }

    // ==========================================
    // 辅助方法
    // ==========================================

    /// 判断是否可以添加材料
    ///
    /// # 参数
    /// - `campaign`: 换辊窗口
    /// - `weight_t`: 材料重量
    ///
    /// # 返回
    /// - `true`: 可以添加（不会超过硬停止阈值）
    /// - `false`: 不可添加（会超过硬停止阈值）
    pub fn can_add_material(&self, campaign: &RollerCampaign, weight_t: f64) -> bool {
        campaign.cum_weight_t + weight_t <= campaign.hard_limit_t
    }

    /// 预测添加材料后的状态
    ///
    /// # 参数
    /// - `campaign`: 换辊窗口
    /// - `weight_t`: 材料重量
    ///
    /// # 返回
    /// 添加后的换辊状态
    pub fn predict_status_after_add(&self, campaign: &RollerCampaign, weight_t: f64) -> RollStatus {
        let new_weight = campaign.cum_weight_t + weight_t;

        if new_weight >= campaign.hard_limit_t {
            RollStatus::HardStop
        } else if new_weight >= campaign.suggest_threshold_t {
            RollStatus::Suggest
        } else {
            RollStatus::Normal
        }
    }
}

// ==========================================
// Default trait 实现
// ==========================================
impl Default for RollCampaignEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ==========================================
// 单元测试
// ==========================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::roller::RollerCampaignMonitor;
    use chrono::NaiveDate;

    /// 创建测试用的换辊窗口
    fn create_test_campaign(cum_weight_t: f64) -> RollerCampaign {
        RollerCampaign::new(
            "v1".to_string(),
            "H032".to_string(),
            1,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            Some(1500.0),
            Some(2500.0),
        )
        .tap_mut(|c| c.cum_weight_t = cum_weight_t)
    }

    #[test]
    fn test_new() {
        let engine = RollCampaignEngine::new();
        assert_eq!(engine.get_default_thresholds(), (1500.0, 2500.0));
    }

    #[test]
    fn test_check_roll_status_normal() {
        let engine = RollCampaignEngine::new();
        let campaign = create_test_campaign(1000.0);

        let (status, reason) = engine.check_roll_status(&campaign);

        assert_eq!(status, RollStatus::Normal);
        assert!(reason.contains("NORMAL"));
        assert!(reason.contains("1000"));
    }

    #[test]
    fn test_check_roll_status_suggest() {
        let engine = RollCampaignEngine::new();
        let campaign = create_test_campaign(1500.0); // 等于建议阈值

        let (status, reason) = engine.check_roll_status(&campaign);

        assert_eq!(status, RollStatus::Suggest);
        assert!(reason.contains("SUGGEST"));
        assert!(reason.contains("1500"));
    }

    #[test]
    fn test_check_roll_status_suggest_over_threshold() {
        let engine = RollCampaignEngine::new();
        let campaign = create_test_campaign(2000.0); // 超过建议阈值但未达硬停止

        let (status, reason) = engine.check_roll_status(&campaign);

        assert_eq!(status, RollStatus::Suggest);
        assert!(reason.contains("SUGGEST"));
    }

    #[test]
    fn test_check_roll_status_hard_stop() {
        let engine = RollCampaignEngine::new();
        let campaign = create_test_campaign(2500.0); // 等于硬停止阈值

        let (status, reason) = engine.check_roll_status(&campaign);

        assert_eq!(status, RollStatus::HardStop);
        assert!(reason.contains("HARD_STOP"));
        assert!(reason.contains("2500"));
    }

    #[test]
    fn test_check_roll_status_hard_stop_over_limit() {
        let engine = RollCampaignEngine::new();
        let campaign = create_test_campaign(3000.0); // 超过硬停止阈值

        let (status, reason) = engine.check_roll_status(&campaign);

        assert_eq!(status, RollStatus::HardStop);
        assert!(reason.contains("HARD_STOP"));
        assert!(reason.contains("over_limit_t"));
    }

    #[test]
    fn test_should_suggest_roll_change() {
        let engine = RollCampaignEngine::new();

        assert!(!engine.should_suggest_roll_change(1000.0, 1500.0));
        assert!(engine.should_suggest_roll_change(1500.0, 1500.0));
        assert!(engine.should_suggest_roll_change(2000.0, 1500.0));
    }

    #[test]
    fn test_should_hard_stop() {
        let engine = RollCampaignEngine::new();

        assert!(!engine.should_hard_stop(2000.0, 2500.0));
        assert!(engine.should_hard_stop(2500.0, 2500.0));
        assert!(engine.should_hard_stop(3000.0, 2500.0));
    }

    #[test]
    fn test_calculate_remaining_tonnage() {
        let engine = RollCampaignEngine::new();
        let campaign = create_test_campaign(1500.0);

        let remaining = engine.calculate_remaining_tonnage(&campaign);

        assert_eq!(remaining, 1000.0); // 2500 - 1500
    }

    #[test]
    fn test_calculate_remaining_tonnage_over_limit() {
        let engine = RollCampaignEngine::new();
        let campaign = create_test_campaign(3000.0);

        let remaining = engine.calculate_remaining_tonnage(&campaign);

        assert_eq!(remaining, 0.0); // 已超限，最小为0
    }

    #[test]
    fn test_get_utilization_ratio() {
        let engine = RollCampaignEngine::new();
        let campaign = create_test_campaign(1250.0);

        let ratio = engine.get_utilization_ratio(&campaign);

        assert_eq!(ratio, 0.5); // 1250 / 2500 = 0.5 (50%)
    }

    #[test]
    fn test_get_utilization_ratio_over_limit() {
        let engine = RollCampaignEngine::new();
        let campaign = create_test_campaign(3000.0);

        let ratio = engine.get_utilization_ratio(&campaign);

        assert_eq!(ratio, 1.2); // 3000 / 2500 = 1.2 (120%)
    }

    #[test]
    fn test_generate_roll_reason() {
        let engine = RollCampaignEngine::new();
        let campaign = create_test_campaign(2000.0);

        let reason = engine.generate_roll_reason(&campaign, RollStatus::Suggest);

        assert!(reason.contains("SUGGEST"));
        assert!(reason.contains("machine_code"));
        assert!(reason.contains("H032"));
    }

    #[test]
    fn test_get_default_thresholds() {
        let engine = RollCampaignEngine::new();
        let (suggest, hard) = engine.get_default_thresholds();

        assert_eq!(suggest, 1500.0);
        assert_eq!(hard, 2500.0);
    }

    #[test]
    fn test_get_roll_thresholds() {
        let engine = RollCampaignEngine::new();
        let (suggest, hard) = engine.get_roll_thresholds("H032");

        assert_eq!(suggest, 1500.0);
        assert_eq!(hard, 2500.0);
    }

    #[test]
    fn test_can_add_material() {
        let engine = RollCampaignEngine::new();
        let campaign = create_test_campaign(2000.0);

        assert!(engine.can_add_material(&campaign, 400.0)); // 2000 + 400 = 2400 < 2500
        assert!(engine.can_add_material(&campaign, 500.0)); // 2000 + 500 = 2500 = 2500
        assert!(!engine.can_add_material(&campaign, 600.0)); // 2000 + 600 = 2600 > 2500
    }

    #[test]
    fn test_predict_status_after_add() {
        let engine = RollCampaignEngine::new();
        let campaign = create_test_campaign(1000.0);

        assert_eq!(
            engine.predict_status_after_add(&campaign, 400.0),
            RollStatus::Normal
        ); // 1000 + 400 = 1400 < 1500
        assert_eq!(
            engine.predict_status_after_add(&campaign, 500.0),
            RollStatus::Suggest
        ); // 1000 + 500 = 1500 = 1500
        assert_eq!(
            engine.predict_status_after_add(&campaign, 1000.0),
            RollStatus::Suggest
        ); // 1000 + 1000 = 2000 < 2500
        assert_eq!(
            engine.predict_status_after_add(&campaign, 1500.0),
            RollStatus::HardStop
        ); // 1000 + 1500 = 2500 = 2500
    }

    // ==========================================
    // 领域模型测试 (RollerCampaignMonitor trait)
    // ==========================================

    #[test]
    fn test_roller_campaign_monitor_should_change_roll() {
        let campaign = create_test_campaign(1500.0);
        assert!(campaign.should_change_roll());

        let campaign = create_test_campaign(1000.0);
        assert!(!campaign.should_change_roll());
    }

    #[test]
    fn test_roller_campaign_monitor_is_hard_stop() {
        let campaign = create_test_campaign(2500.0);
        assert!(campaign.is_hard_stop());

        let campaign = create_test_campaign(2000.0);
        assert!(!campaign.is_hard_stop());
    }

    #[test]
    fn test_roller_campaign_monitor_remaining_tonnage() {
        let campaign = create_test_campaign(1500.0);
        assert_eq!(campaign.remaining_tonnage_t(), 1000.0);

        let campaign = create_test_campaign(3000.0);
        assert_eq!(campaign.remaining_tonnage_t(), 0.0);
    }

    #[test]
    fn test_roller_campaign_monitor_add_tonnage() {
        let mut campaign = create_test_campaign(1000.0);
        campaign.add_tonnage(500.0);
        assert_eq!(campaign.cum_weight_t, 1500.0);
    }

    #[test]
    fn test_roller_campaign_monitor_utilization_ratio() {
        let campaign = create_test_campaign(1250.0);
        assert_eq!(campaign.utilization_ratio(), 0.5);
    }

    #[test]
    fn test_roller_campaign_new() {
        let campaign = RollerCampaign::new(
            "v1".to_string(),
            "H032".to_string(),
            1,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            None,
            None,
        );

        assert_eq!(campaign.version_id, "v1");
        assert_eq!(campaign.machine_code, "H032");
        assert_eq!(campaign.campaign_no, 1);
        assert_eq!(campaign.cum_weight_t, 0.0);
        assert_eq!(campaign.suggest_threshold_t, 1500.0);
        assert_eq!(campaign.hard_limit_t, 2500.0);
        assert_eq!(campaign.status, RollStatus::Normal);
        assert!(campaign.is_active());
    }

    #[test]
    fn test_roller_campaign_close() {
        let mut campaign = create_test_campaign(2000.0);
        assert!(campaign.is_active());

        campaign.close(NaiveDate::from_ymd_opt(2025, 1, 10).unwrap());
        assert!(!campaign.is_active());
        assert_eq!(
            campaign.end_date,
            Some(NaiveDate::from_ymd_opt(2025, 1, 10).unwrap())
        );
    }

    #[test]
    fn test_roller_campaign_get_id() {
        let campaign = create_test_campaign(1000.0);
        assert_eq!(campaign.get_id(), "v1_H032_C1");
    }
}

// ==========================================
// tap_mut 辅助trait (仅用于测试)
// ==========================================
#[cfg(test)]
trait TapMut: Sized {
    fn tap_mut<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut Self),
    {
        f(&mut self);
        self
    }
}

#[cfg(test)]
impl<T> TapMut for T {}

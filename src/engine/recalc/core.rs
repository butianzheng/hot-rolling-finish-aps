use super::{RecalcConfig, RecalcEngine, ResolvedStrategyProfile};
use crate::config::strategy_profile::{CustomStrategyParameters, CustomStrategyProfile};
use crate::config::ConfigManager;
use crate::engine::events::{OptionalEventPublisher, ScheduleEventPublisher};
use crate::engine::strategy::ScheduleStrategy;
use crate::engine::RiskEngine;
use crate::engine::{CapacityFiller, EligibilityEngine, PrioritySorter, UrgencyEngine};
use crate::repository::{
    ActionLogRepository, CapacityPoolRepository, MaterialMasterRepository, MaterialStateRepository,
    PathOverridePendingRepository, PlanItemRepository, PlanVersionRepository,
    RiskSnapshotRepository, RollerCampaignRepository,
};
use std::error::Error;
use std::sync::Arc;

const CUSTOM_STRATEGY_PREFIX: &str = "custom:";

fn is_params_empty(params: &CustomStrategyParameters) -> bool {
    params.urgent_weight.is_none()
        && params.capacity_weight.is_none()
        && params.cold_stock_weight.is_none()
        && params.due_date_weight.is_none()
        && params.rolling_output_age_weight.is_none()
        && params.cold_stock_age_threshold_days.is_none()
        && params.overflow_tolerance_pct.is_none()
}

impl RecalcEngine {
    /// 创建新的RecalcEngine实例
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version_repo: Arc<PlanVersionRepository>,
        item_repo: Arc<PlanItemRepository>,
        material_state_repo: Arc<MaterialStateRepository>,
        material_master_repo: Arc<MaterialMasterRepository>,
        capacity_repo: Arc<CapacityPoolRepository>,
        action_log_repo: Arc<ActionLogRepository>,
        risk_snapshot_repo: Arc<RiskSnapshotRepository>,
        roller_campaign_repo: Arc<RollerCampaignRepository>,
        path_override_pending_repo: Arc<PathOverridePendingRepository>,
        eligibility_engine: Arc<EligibilityEngine<ConfigManager>>,
        urgency_engine: Arc<UrgencyEngine>,
        priority_sorter: Arc<PrioritySorter>,
        capacity_filler: Arc<CapacityFiller>,
        risk_engine: Arc<RiskEngine>,
        config: RecalcConfig,
        config_manager: Arc<ConfigManager>,
        event_publisher: Option<Arc<dyn ScheduleEventPublisher>>,
    ) -> Self {
        let event_publisher = match event_publisher {
            Some(p) => OptionalEventPublisher::with_publisher(p),
            None => OptionalEventPublisher::none(),
        };

        Self {
            version_repo,
            item_repo,
            material_state_repo,
            material_master_repo,
            capacity_repo,
            action_log_repo,
            risk_snapshot_repo,
            roller_campaign_repo,
            path_override_pending_repo,
            eligibility_engine,
            urgency_engine,
            priority_sorter,
            capacity_filler,
            risk_engine,
            event_publisher,
            config,
            config_manager,
        }
    }

    /// 创建带默认配置的RecalcEngine实例
    #[allow(clippy::too_many_arguments)]
    pub fn with_default_config(
        version_repo: Arc<PlanVersionRepository>,
        item_repo: Arc<PlanItemRepository>,
        material_state_repo: Arc<MaterialStateRepository>,
        material_master_repo: Arc<MaterialMasterRepository>,
        capacity_repo: Arc<CapacityPoolRepository>,
        action_log_repo: Arc<ActionLogRepository>,
        risk_snapshot_repo: Arc<RiskSnapshotRepository>,
        roller_campaign_repo: Arc<RollerCampaignRepository>,
        path_override_pending_repo: Arc<PathOverridePendingRepository>,
        eligibility_engine: Arc<EligibilityEngine<ConfigManager>>,
        urgency_engine: Arc<UrgencyEngine>,
        priority_sorter: Arc<PrioritySorter>,
        capacity_filler: Arc<CapacityFiller>,
        risk_engine: Arc<RiskEngine>,
        config_manager: Arc<ConfigManager>,
        event_publisher: Option<Arc<dyn ScheduleEventPublisher>>,
    ) -> Self {
        Self::new(
            version_repo,
            item_repo,
            material_state_repo,
            material_master_repo,
            capacity_repo,
            action_log_repo,
            risk_snapshot_repo,
            roller_campaign_repo,
            path_override_pending_repo,
            eligibility_engine,
            urgency_engine,
            priority_sorter,
            capacity_filler,
            risk_engine,
            RecalcConfig::default(),
            config_manager,
            event_publisher,
        )
    }

    /// 从仓储集合创建 RecalcEngine 实例
    ///
    /// 这是推荐的构造方式，使用 `ScheduleRepositories` 聚合仓储依赖，
    /// 减少构造函数参数数量，提升代码可读性。
    ///
    /// # 参数
    /// - `repos`: 仓储集合
    /// - `eligibility_engine`: 资格判定引擎
    /// - `urgency_engine`: 紧急度引擎
    /// - `priority_sorter`: 优先级排序器
    /// - `capacity_filler`: 产能填充器
    /// - `config`: 重算配置
    /// - `config_manager`: 配置管理器
    /// - `event_publisher`: 事件发布器（可选）
    ///
    /// # 示例
    /// ```ignore
    /// use crate::engine::{RecalcEngine, ScheduleRepositories};
    ///
    /// let repos = ScheduleRepositories::new(...);
    /// let engine = RecalcEngine::from_repositories(
    ///     repos,
    ///     eligibility_engine,
    ///     urgency_engine,
    ///     priority_sorter,
    ///     capacity_filler,
    ///     RecalcConfig::default(),
    ///     config_manager,
    ///     None,
    /// );
    /// ```
    pub fn from_repositories(
        repos: crate::engine::repositories::ScheduleRepositories,
        risk_snapshot_repo: Arc<RiskSnapshotRepository>,
        roller_campaign_repo: Arc<RollerCampaignRepository>,
        path_override_pending_repo: Arc<PathOverridePendingRepository>,
        eligibility_engine: Arc<EligibilityEngine<ConfigManager>>,
        urgency_engine: Arc<UrgencyEngine>,
        priority_sorter: Arc<PrioritySorter>,
        capacity_filler: Arc<CapacityFiller>,
        risk_engine: Arc<RiskEngine>,
        config: RecalcConfig,
        config_manager: Arc<ConfigManager>,
        event_publisher: Option<Arc<dyn ScheduleEventPublisher>>,
    ) -> Self {
        Self::new(
            repos.version_repo,
            repos.item_repo,
            repos.material_state_repo,
            repos.material_master_repo,
            repos.capacity_repo,
            repos.action_log_repo,
            risk_snapshot_repo,
            roller_campaign_repo,
            path_override_pending_repo,
            eligibility_engine,
            urgency_engine,
            priority_sorter,
            capacity_filler,
            risk_engine,
            config,
            config_manager,
            event_publisher,
        )
    }

    pub fn resolve_strategy_profile(
        &self,
        strategy_key: &str,
    ) -> Result<ResolvedStrategyProfile, Box<dyn Error>> {
        let raw = strategy_key.trim();
        if raw.is_empty() {
            return Err("策略不能为空".into());
        }

        if raw.starts_with(CUSTOM_STRATEGY_PREFIX) {
            let id = raw[CUSTOM_STRATEGY_PREFIX.len()..].trim();
            if id.is_empty() {
                return Err("自定义策略ID不能为空".into());
            }

            let profile: CustomStrategyProfile = self
                .config_manager
                .get_custom_strategy_profile(id)?
                .ok_or_else(|| format!("自定义策略不存在: {}", id))?;

            let base_strategy = profile.base_strategy.parse::<ScheduleStrategy>()?;

            let params = profile.parameters;
            let params = if is_params_empty(&params) {
                None
            } else {
                Some(params)
            };

            return Ok(ResolvedStrategyProfile {
                strategy_key: raw.to_string(),
                base_strategy,
                title_cn: profile.title,
                parameters: params,
            });
        }

        let base_strategy = raw.parse::<ScheduleStrategy>()?;
        Ok(ResolvedStrategyProfile {
            strategy_key: base_strategy.as_str().to_string(),
            base_strategy,
            title_cn: base_strategy.title_cn().to_string(),
            parameters: None,
        })
    }

    /// 获取默认联动窗口天数
    pub fn get_default_cascade_days(&self) -> i32 {
        self.config.default_cascade_days
    }

    /// 获取默认计算窗口天数
    pub fn get_default_window_days(&self) -> i32 {
        self.config.default_window_days
    }

    /// 获取冻结区天数
    pub fn get_frozen_days_before_today(&self) -> i32 {
        self.config.frozen_days_before_today
    }

    /// 创建默认产能池
    ///
    /// 当数据库中不存在指定版本/机组/日期的产能池时，使用此方法创建默认值。
    /// 默认值：target=1800t, limit=2000t, 其余字段为 0 或 None。
    pub(super) fn create_default_capacity_pool(
        version_id: &str,
        machine_code: &str,
        plan_date: chrono::NaiveDate,
    ) -> crate::domain::CapacityPool {
        crate::domain::CapacityPool {
            version_id: version_id.to_string(),
            machine_code: machine_code.to_string(),
            plan_date,
            target_capacity_t: 1800.0,
            limit_capacity_t: 2000.0,
            used_capacity_t: 0.0,
            overflow_t: 0.0,
            frozen_capacity_t: 0.0,
            accumulated_tonnage_t: 0.0,
            roll_campaign_id: None,
        }
    }
}

// ==========================================
// 热轧精整排产系统 - 适温准入引擎
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 2. Eligibility Engine
// 红线: 非适温材料不得进入当日产能池
// ==========================================
// 职责: 锁定过滤 + 适温准入判定
// 输入: material_master + material_state
// 输出: 更新 material_state (ready_in_days, earliest_sched_date, sched_state)
// ==========================================

use crate::config::ImportConfigReader;
use crate::domain::material::{MaterialMaster, MaterialState};
use crate::domain::types::{SchedState, Season, SeasonMode};
use crate::engine::EligibilityCore;
use chrono::{NaiveDate, Utc};
use std::error::Error;
use std::sync::Arc;
use tracing::instrument;

// ==========================================
// EligibilityEngine - 适温准入引擎
// ==========================================
// 红线: 不直接写库,只计算和返回更新后的状态
pub struct EligibilityEngine<C>
where
    C: ImportConfigReader,
{
    config: Arc<C>,
}

impl<C> EligibilityEngine<C>
where
    C: ImportConfigReader,
{
    /// 创建新的 EligibilityEngine 实例
    ///
    /// # 参数
    /// - config: 配置读取器
    pub fn new(config: Arc<C>) -> Self {
        Self { config }
    }

    /// 评估单个材料的适温状态
    ///
    /// # 参数
    /// - material: 材料主数据
    /// - state: 当前材料状态
    /// - today: 当前日期
    ///
    /// # 返回
    /// - (MaterialState, Vec<String>): 更新后的状态和决策原因
    #[instrument(skip(self, material, state), fields(material_id = %state.material_id))]
    pub async fn evaluate_single(
        &self,
        material: &MaterialMaster,
        state: &MaterialState,
        today: NaiveDate,
    ) -> Result<(MaterialState, Vec<String>), Box<dyn Error>> {
        let mut reasons = Vec::new();
        let mut updated_state = state.clone();

        // === 步骤 1: 数据质量检查 ===
        let output_age_raw = match material.output_age_days_raw {
            Some(days) if days >= 0 => days,
            _ => {
                reasons.push("ERROR: output_age_days_raw missing or invalid".to_string());
                updated_state.sched_state = SchedState::Blocked;
                updated_state.updated_at = Utc::now();
                return Ok((updated_state, reasons));
            }
        };

        let current_machine = match &material.current_machine_code {
            Some(code) if !code.is_empty() => code.as_str(),
            _ => {
                reasons.push("ERROR: current_machine_code missing".to_string());
                updated_state.sched_state = SchedState::Blocked;
                updated_state.updated_at = Utc::now();
                return Ok((updated_state, reasons));
            }
        };

        // === 步骤 2: 获取配置 ===
        let standard_machines = self.config.get_standard_finishing_machines().await?;
        let offset_days = self.config.get_machine_offset_days().await?;

        // === 步骤 3: 计算静态快照值（用于 fallback，向后兼容）===
        let rolling_output_age_days_static = EligibilityCore::calculate_rolling_output_age_days(
            output_age_raw,
            current_machine,
            &standard_machines,
            offset_days,
        );

        // === 步骤 3.5: 计算实际产出天数（动态版本，v0.7）===
        // 若 material.rolling_output_date 存在，则动态计算 (today - rolling_output_date)
        // 否则使用静态快照值（历史数据 fallback）
        let actual_output_age_days = EligibilityCore::calculate_actual_output_age_days(
            material.rolling_output_date,
            today,
            rolling_output_age_days_static,
        );

        // === 步骤 4: 判定季节 ===
        let season_mode = self.config.get_season_mode().await?;
        let winter_months = self.config.get_winter_months().await?;
        let manual_season = if matches!(season_mode, SeasonMode::Manual) {
            self.config.get_manual_season().await?
        } else {
            Season::Winter // 默认值,在 Auto 模式下会被覆盖
        };
        let season = EligibilityCore::determine_season(today, season_mode, manual_season, &winter_months);

        // === 步骤 5: 获取适温阈值 ===
        let min_temp_days = match season {
            Season::Winter => self.config.get_min_temp_days_winter().await?,
            Season::Summer => self.config.get_min_temp_days_summer().await?,
        };

        // === 步骤 6: 计算适温状态 ===
        let ready_in_days = EligibilityCore::calculate_ready_in_days(
            actual_output_age_days,
            min_temp_days,
        );
        let earliest_sched_date = EligibilityCore::calculate_earliest_sched_date(
            today,
            ready_in_days,
        );

        // === 步骤 7: 判定 sched_state (考虑锁定和强制放行) ===
        let (sched_state, state_reasons) = EligibilityCore::determine_sched_state(
            state.lock_flag,
            state.force_release_flag,
            Some(output_age_raw),
            Some(current_machine),
            ready_in_days,
        );
        reasons.extend(state_reasons);

        // === 步骤 8: 计算 rush_level ===
        let rush_level = EligibilityCore::calculate_rush_level(
            material.contract_nature.as_deref(),
            material.weekly_delivery_flag.as_deref(),
            material.export_flag.as_deref(),
        );

        // === 步骤 9: 计算 urgent_level ===
        // 说明：N1/N2 为紧急等级阈值配置，不应复用适温阈值（min_temp_days_*）。
        let n1_days = self.config.get_n1_threshold_days().await?;
        let n2_days = self.config.get_n2_threshold_days().await?;

        let (urgent_level, urgent_reasons) = EligibilityCore::calculate_urgent_level(
            material.due_date,
            today,
            n1_days,
            n2_days,
            rush_level,
            Some(earliest_sched_date),
            state.manual_urgent_flag,
            state.in_frozen_zone,
        );
        reasons.extend(urgent_reasons);

        // === 步骤 10: 更新 MaterialState ===
        updated_state.sched_state = sched_state;
        updated_state.urgent_level = urgent_level;
        updated_state.rush_level = rush_level;
        updated_state.rolling_output_age_days = actual_output_age_days;
        updated_state.ready_in_days = ready_in_days;
        updated_state.earliest_sched_date = Some(earliest_sched_date);
        updated_state.updated_at = Utc::now();

        Ok((updated_state, reasons))
    }

    /// 批量评估材料的适温状态
    ///
    /// # 参数
    /// - materials: 材料主数据和状态的列表
    /// - today: 当前日期
    ///
    /// # 返回
    /// - Vec<(MaterialState, Vec<String>)>: 更新后的状态和决策原因列表
    pub async fn evaluate_batch(
        &self,
        materials: Vec<(&MaterialMaster, &MaterialState)>,
        today: NaiveDate,
    ) -> Result<Vec<(MaterialState, Vec<String>)>, Box<dyn Error>> {
        let mut results = Vec::new();

        for (material, state) in materials {
            let result = self.evaluate_single(material, state, today).await?;
            results.push(result);
        }

        Ok(results)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::domain::types::{UrgentLevel, RushLevel};

    // ==========================================
    // Mock ConfigReader
    // ==========================================
    struct MockConfigReader;

    #[async_trait]
    impl ImportConfigReader for MockConfigReader {
        async fn get_season_mode(&self) -> Result<SeasonMode, Box<dyn std::error::Error>> {
            Ok(SeasonMode::Manual)
        }

        async fn get_manual_season(&self) -> Result<Season, Box<dyn std::error::Error>> {
            Ok(Season::Winter)
        }

        async fn get_winter_months(&self) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
            Ok(vec![11, 12, 1, 2, 3])
        }

        async fn get_min_temp_days_winter(&self) -> Result<i32, Box<dyn std::error::Error>> {
            Ok(3)
        }

        async fn get_min_temp_days_summer(&self) -> Result<i32, Box<dyn std::error::Error>> {
            Ok(4)
        }

        async fn get_current_min_temp_days(
            &self,
            _today: NaiveDate,
        ) -> Result<i32, Box<dyn std::error::Error>> {
            Ok(3)
        }

        async fn get_standard_finishing_machines(
            &self,
        ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
            Ok(vec![
                "H032".to_string(),
                "H033".to_string(),
                "H034".to_string(),
            ])
        }

        async fn get_machine_offset_days(&self) -> Result<i32, Box<dyn std::error::Error>> {
            Ok(4)
        }

        async fn get_weight_anomaly_threshold(&self) -> Result<f64, Box<dyn std::error::Error>> {
            Ok(100.0)
        }

        async fn get_batch_retention_days(&self) -> Result<i32, Box<dyn std::error::Error>> {
            Ok(90)
        }

        async fn get_n1_threshold_days(&self) -> Result<i32, Box<dyn std::error::Error>> {
            Ok(7)
        }

        async fn get_n2_threshold_days(&self) -> Result<i32, Box<dyn std::error::Error>> {
            Ok(3)
        }
    }

    // ==========================================
    // 测试辅助函数
    // ==========================================
    fn create_test_material(material_id: &str) -> MaterialMaster {
        MaterialMaster {
            material_id: material_id.to_string(),
            manufacturing_order_id: None,
            material_status_code_src: None,
            steel_mark: None,
            slab_id: None,
            next_machine_code: Some("H032".to_string()),
            rework_machine_code: None,
            current_machine_code: Some("H032".to_string()),
            width_mm: Some(1250.0),
            thickness_mm: Some(2.5),
            length_m: Some(100.0),
            weight_t: Some(2.450),
            available_width_mm: None,
            due_date: None,
            stock_age_days: Some(10),
            output_age_days_raw: Some(5),
            rolling_output_date: None,  // v0.7
            status_updated_at: None,
            contract_no: None,
            contract_nature: None,
            weekly_delivery_flag: None,
            export_flag: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_state(material_id: &str) -> MaterialState {
        MaterialState {
            material_id: material_id.to_string(),
            sched_state: SchedState::Ready,
            lock_flag: false,
            force_release_flag: false,
            urgent_level: UrgentLevel::L0,
            urgent_reason: None,
            rush_level: RushLevel::L0,
            rolling_output_age_days: 5,
            ready_in_days: 0,
            earliest_sched_date: None,
            stock_age_days: 10,
            scheduled_date: None,
            scheduled_machine_code: None,
            seq_no: None,
            manual_urgent_flag: false,
            user_confirmed: false,
            user_confirmed_at: None,
            user_confirmed_by: None,
            user_confirmed_reason: None,
            in_frozen_zone: false,
            last_calc_version_id: None,
            updated_at: Utc::now(),
            updated_by: Some("system".to_string()),
        }
    }

    // ==========================================
    // 测试用例
    // ==========================================

    #[tokio::test]
    async fn test_evaluate_single_basic() {
        let config = Arc::new(MockConfigReader);
        let engine = EligibilityEngine::new(config);
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        let material = create_test_material("MAT001");
        let state = create_test_state("MAT001");

        let (updated_state, reasons) = engine
            .evaluate_single(&material, &state, today)
            .await
            .unwrap();

        // 验证状态更新
        assert_eq!(updated_state.material_id, "MAT001");
        assert_eq!(updated_state.rolling_output_age_days, 5); // H032 不加偏移
        assert_eq!(updated_state.ready_in_days, 0); // 5 >= 3 (已适温)
        assert_eq!(updated_state.sched_state, SchedState::Ready);
        assert_eq!(updated_state.rush_level, RushLevel::L0);
        assert!(!reasons.is_empty()); // 应该有决策原因
    }

    #[tokio::test]
    async fn test_evaluate_single_locked() {
        let config = Arc::new(MockConfigReader);
        let engine = EligibilityEngine::new(config);
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        let material = create_test_material("MAT_LOCKED");
        let mut state = create_test_state("MAT_LOCKED");
        state.lock_flag = true; // 设置锁定标记

        let (updated_state, reasons) = engine
            .evaluate_single(&material, &state, today)
            .await
            .unwrap();

        // 验证锁定状态
        assert_eq!(updated_state.sched_state, SchedState::Locked);
        assert!(reasons.iter().any(|r| r.contains("LOCKED")));
    }

    #[tokio::test]
    async fn test_evaluate_single_force_release() {
        let config = Arc::new(MockConfigReader);
        let engine = EligibilityEngine::new(config);
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        let mut material = create_test_material("MAT_FORCE");
        material.output_age_days_raw = Some(1); // 未成熟(需要3天)

        let mut state = create_test_state("MAT_FORCE");
        state.force_release_flag = true; // 设置强制放行标记

        let (updated_state, reasons) = engine
            .evaluate_single(&material, &state, today)
            .await
            .unwrap();

        // 验证强制放行状态
        assert_eq!(updated_state.sched_state, SchedState::ForceRelease);
        assert!(reasons.iter().any(|r| r.contains("FORCE_RELEASE")));
    }

    #[tokio::test]
    async fn test_evaluate_single_blocked_missing_output_age() {
        let config = Arc::new(MockConfigReader);
        let engine = EligibilityEngine::new(config);
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        let mut material = create_test_material("MAT_BLOCKED");
        material.output_age_days_raw = None; // 缺失必填字段

        let state = create_test_state("MAT_BLOCKED");

        let (updated_state, reasons) = engine
            .evaluate_single(&material, &state, today)
            .await
            .unwrap();

        // 验证阻断状态
        assert_eq!(updated_state.sched_state, SchedState::Blocked);
        assert!(reasons.iter().any(|r| r.contains("output_age_days_raw missing")));
    }

    #[tokio::test]
    async fn test_evaluate_batch() {
        let config = Arc::new(MockConfigReader);
        let engine = EligibilityEngine::new(config);
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        // 准备3个材料
        let material1 = create_test_material("MAT001");
        let state1 = create_test_state("MAT001");

        let material2 = create_test_material("MAT002");
        let mut state2 = create_test_state("MAT002");
        state2.lock_flag = true; // 锁定

        let material3 = create_test_material("MAT003");
        let mut state3 = create_test_state("MAT003");
        state3.force_release_flag = true; // 强制放行

        let materials = vec![
            (&material1, &state1),
            (&material2, &state2),
            (&material3, &state3),
        ];

        let results = engine.evaluate_batch(materials, today).await.unwrap();

        // 验证批量处理结果
        assert_eq!(results.len(), 3);

        // 验证第一个材料(正常)
        assert_eq!(results[0].0.material_id, "MAT001");
        assert_eq!(results[0].0.sched_state, SchedState::Ready);

        // 验证第二个材料(锁定)
        assert_eq!(results[1].0.material_id, "MAT002");
        assert_eq!(results[1].0.sched_state, SchedState::Locked);

        // 验证第三个材料(强制放行)
        assert_eq!(results[2].0.material_id, "MAT003");
        assert_eq!(results[2].0.sched_state, SchedState::ForceRelease);
    }
}

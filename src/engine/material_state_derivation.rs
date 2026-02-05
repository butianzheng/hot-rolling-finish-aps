// ==========================================
// 热轧精整排产系统 - MaterialState 派生引擎
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - Eligibility Engine + Urgency Engine
// 依据: Field_Mapping_Spec_v0.3_Integrated.md - 适温反推规则
// 职责: 从 MaterialMaster 派生 MaterialState（事实层）
// 红线: 不生成 plan_item，不访问 UI，通过 repository 操作数据库
// ==========================================

use crate::config::ImportConfigReader;
use crate::domain::material::{MaterialMaster, MaterialState};
use crate::domain::types::{RushLevel, SchedState, Season, SeasonMode, UrgentLevel};
use crate::engine::EligibilityCore;
use chrono::{NaiveDate, Utc};
use std::error::Error;

// ==========================================
// MaterialStateDerivationService
// ==========================================
pub struct MaterialStateDerivationService;

impl MaterialStateDerivationService {
    /// 创建新的 MaterialStateDerivationService 实例
    pub fn new() -> Self {
        Self
    }

    /// 派生 material_state（主入口）
    ///
    /// # 参数
    /// - material: 材料主数据
    /// - config: 配置读取器
    /// - today: 当前日期（用于适温/交期判定）
    ///
    /// # 返回
    /// - MaterialState: 派生后的状态（可直接写库）
    pub async fn derive(
        &self,
        material: &MaterialMaster,
        config: &dyn ImportConfigReader,
        today: NaiveDate,
    ) -> Result<MaterialState, Box<dyn Error>> {
        // === 步骤 1: 校验必填字段 ===
        let output_age_raw = match material.output_age_days_raw {
            Some(days) if days >= 0 => days,
            _ => {
                // 缺失或非法 → BLOCKED 状态
                return Ok(self.create_blocked_state(
                    material,
                    vec!["ERROR: output_age_days_raw missing or invalid".to_string()],
                ));
            }
        };

        let current_machine = match &material.current_machine_code {
            Some(code) if !code.is_empty() => code.as_str(),
            _ => {
                return Ok(self.create_blocked_state(
                    material,
                    vec!["ERROR: current_machine_code missing".to_string()],
                ));
            }
        };

        // === 步骤 2: 派生 rolling_output_age_days（静态快照值）===
        let standard_machines = config.get_standard_finishing_machines().await?;
        let offset_days = config.get_machine_offset_days().await?;
        let rolling_output_age_days_static = EligibilityCore::calculate_rolling_output_age_days(
            output_age_raw,
            current_machine,
            &standard_machines,
            offset_days,
        );

        // === 步骤 2.5: 计算实际产出天数（动态版本，v0.7）===
        let actual_output_age_days = EligibilityCore::calculate_actual_output_age_days(
            material.rolling_output_date,
            today,
            rolling_output_age_days_static,
        );

        // === 步骤 3: 判定季节 ===
        let season_mode = config.get_season_mode().await?;
        let winter_months = config.get_winter_months().await?;
        let manual_season = if matches!(season_mode, SeasonMode::Manual) {
            config.get_manual_season().await?
        } else {
            Season::Winter // 默认值,在 Auto 模式下会被覆盖
        };
        let season = EligibilityCore::determine_season(today, season_mode, manual_season, &winter_months);

        // === 步骤 4: 获取适温阈值 ===
        let min_temp_days = match season {
            Season::Winter => config.get_min_temp_days_winter().await?,
            Season::Summer => config.get_min_temp_days_summer().await?,
        };

        // === 步骤 5: 计算适温状态 ===
        let ready_in_days = EligibilityCore::calculate_ready_in_days(
            actual_output_age_days,
            min_temp_days,
        );
        let earliest_sched_date = EligibilityCore::calculate_earliest_sched_date(
            today,
            ready_in_days,
        );

        // 在导入阶段,lock_flag 和 force_release_flag 都是 false
        let (sched_state, _reasons) = EligibilityCore::determine_sched_state(
            false, // lock_flag
            false, // force_release_flag
            Some(output_age_raw),
            Some(current_machine),
            ready_in_days,
        );

        // === 步骤 6: 计算 rush_level ===
        let rush_level = EligibilityCore::calculate_rush_level(
            material.contract_nature.as_deref(),
            material.weekly_delivery_flag.as_deref(),
            material.export_flag.as_deref(),
        );

        // === 步骤 7: 获取紧急判定阈值 ===
        // 说明：N1/N2 为紧急等级阈值配置，不应复用适温阈值（min_temp_days_*）。
        let n1_days = config.get_n1_threshold_days().await?;
        let n2_days = config.get_n2_threshold_days().await?;

        // === 步骤 8: 计算 urgent_level（7层判定）===
        let (urgent_level, urgent_reasons) = EligibilityCore::calculate_urgent_level(
            material.due_date,
            today,
            n1_days,
            n2_days,
            rush_level,
            Some(earliest_sched_date), // 包装为 Option
            false, // manual_urgent_flag (导入阶段为 false)
            false, // in_frozen_zone (导入阶段为 false)
        );

        // === 步骤 9: 构建 MaterialState ===
        Ok(MaterialState {
            material_id: material.material_id.clone(),
            sched_state,
            lock_flag: false,
            force_release_flag: false,
            urgent_level,
            urgent_reason: Some(serde_json::to_string(&urgent_reasons)?),
            rush_level,
            rolling_output_age_days: actual_output_age_days,
            ready_in_days,
            earliest_sched_date: Some(earliest_sched_date),
            stock_age_days: material.stock_age_days.unwrap_or(0),
            scheduled_date: None,
            scheduled_machine_code: None,
            seq_no: None,
            manual_urgent_flag: false,
            user_confirmed: false,
            user_confirmed_at: None,
            user_confirmed_by: None,
            user_confirmed_reason: None,
            in_frozen_zone: false,
            last_calc_version_id: None, // 导入阶段未关联计算版本
            updated_at: Utc::now(),
            updated_by: Some("system".to_string()),
        })
    }

    /// 创建 BLOCKED 状态（数据质量问题）
    fn create_blocked_state(
        &self,
        material: &MaterialMaster,
        reasons: Vec<String>,
    ) -> MaterialState {
        MaterialState {
            material_id: material.material_id.clone(),
            sched_state: SchedState::Blocked,
            lock_flag: false,
            force_release_flag: false,
            urgent_level: UrgentLevel::L0,
            urgent_reason: Some(serde_json::to_string(&reasons).unwrap_or_default()),
            rush_level: RushLevel::L0,
            rolling_output_age_days: 0,
            ready_in_days: 0,
            earliest_sched_date: None,
            stock_age_days: material.stock_age_days.unwrap_or(0),
            scheduled_date: None,
            scheduled_machine_code: None,
            seq_no: None,
            manual_urgent_flag: false,
            user_confirmed: false,
            user_confirmed_at: None,
            user_confirmed_by: None,
            user_confirmed_reason: None,
            in_frozen_zone: false,
            last_calc_version_id: None, // BLOCKED 状态未关联计算版本
            updated_at: Utc::now(),
            updated_by: Some("system".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    // Mock ConfigReader
    struct MockConfigReader;

    #[async_trait]
    impl ImportConfigReader for MockConfigReader {
        async fn get_season_mode(&self) -> Result<SeasonMode, Box<dyn Error>> {
            Ok(SeasonMode::Manual)
        }

        async fn get_manual_season(&self) -> Result<Season, Box<dyn Error>> {
            Ok(Season::Winter)
        }

        async fn get_winter_months(&self) -> Result<Vec<u32>, Box<dyn Error>> {
            Ok(vec![11, 12, 1, 2, 3])
        }

        async fn get_min_temp_days_winter(&self) -> Result<i32, Box<dyn Error>> {
            Ok(3)
        }

        async fn get_min_temp_days_summer(&self) -> Result<i32, Box<dyn Error>> {
            Ok(4)
        }

        async fn get_current_min_temp_days(
            &self,
            _today: NaiveDate,
        ) -> Result<i32, Box<dyn Error>> {
            Ok(3)
        }

        async fn get_standard_finishing_machines(&self) -> Result<Vec<String>, Box<dyn Error>> {
            Ok(vec![
                "H032".to_string(),
                "H033".to_string(),
                "H034".to_string(),
            ])
        }

        async fn get_machine_offset_days(&self) -> Result<i32, Box<dyn Error>> {
            Ok(4)
        }

        async fn get_weight_anomaly_threshold(&self) -> Result<f64, Box<dyn Error>> {
            Ok(100.0)
        }

        async fn get_batch_retention_days(&self) -> Result<i32, Box<dyn Error>> {
            Ok(90)
        }

        async fn get_n1_threshold_days(&self) -> Result<i32, Box<dyn Error>> {
            Ok(7)
        }

        async fn get_n2_threshold_days(&self) -> Result<i32, Box<dyn Error>> {
            Ok(3)
        }
    }

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

    #[tokio::test]
    async fn test_basic_derivation() {
        let service = MaterialStateDerivationService;
        let config = MockConfigReader;
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        let material = create_test_material("MAT001");

        let state = service.derive(&material, &config, today).await.unwrap();

        assert_eq!(state.material_id, "MAT001");
        assert_eq!(state.rolling_output_age_days, 5); // H032 不加偏移
        assert_eq!(state.ready_in_days, 0);           // 5 >= 3（已适温）
        assert_eq!(state.sched_state, SchedState::Ready);
        assert_eq!(state.rush_level, RushLevel::L0);
    }

    #[tokio::test]
    async fn test_pending_mature() {
        let service = MaterialStateDerivationService;
        let config = MockConfigReader;
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        let mut material = create_test_material("MAT_COLD");
        material.output_age_days_raw = Some(1); // 未成熟

        let state = service.derive(&material, &config, today).await.unwrap();

        assert_eq!(state.rolling_output_age_days, 1);
        assert_eq!(state.ready_in_days, 2); // 3 - 1 = 2
        assert_eq!(
            state.earliest_sched_date,
            Some(NaiveDate::from_ymd_opt(2025, 1, 16).unwrap())
        );
        assert_eq!(state.sched_state, SchedState::PendingMature);
    }

    #[tokio::test]
    async fn test_machine_offset() {
        let service = MaterialStateDerivationService;
        let config = MockConfigReader;
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        let mut material = create_test_material("MAT_OFFSET");
        material.current_machine_code = Some("H030".to_string()); // 非标准机组
        material.output_age_days_raw = Some(5);

        let state = service.derive(&material, &config, today).await.unwrap();

        assert_eq!(state.rolling_output_age_days, 9); // 5 + 4
        assert_eq!(state.ready_in_days, 0);           // 9 >= 3
    }

    #[tokio::test]
    async fn test_rush_level_l2() {
        let service = MaterialStateDerivationService;
        let config = MockConfigReader;
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        let mut material = create_test_material("MAT_RUSH_L2");
        material.contract_nature = Some("A".to_string());
        material.weekly_delivery_flag = Some("D".to_string());
        material.export_flag = Some("0".to_string());
        material.due_date = Some(NaiveDate::from_ymd_opt(2025, 2, 14).unwrap());

        let state = service.derive(&material, &config, today).await.unwrap();

        assert_eq!(state.rush_level, RushLevel::L2);
        assert_eq!(state.urgent_level, UrgentLevel::L2);
        assert!(state.urgent_reason.unwrap().contains("RUSH_L2"));
    }

    #[tokio::test]
    async fn test_temp_block_red_line() {
        let service = MaterialStateDerivationService;
        let config = MockConfigReader;
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        let mut material = create_test_material("MAT_BLOCKED");
        material.output_age_days_raw = Some(0); // 需等 3 天
        material.due_date = Some(NaiveDate::from_ymd_opt(2025, 1, 16).unwrap()); // today+2

        let state = service.derive(&material, &config, today).await.unwrap();

        assert_eq!(state.ready_in_days, 3);
        assert_eq!(
            state.earliest_sched_date,
            Some(NaiveDate::from_ymd_opt(2025, 1, 17).unwrap())
        ); // today+3

        // 适温阻断红线：due=1/16 < earliest=1/17 → L3
        assert_eq!(state.urgent_level, UrgentLevel::L3);
        assert!(state.urgent_reason.unwrap().contains("TEMP_BLOCK"));
    }

    #[tokio::test]
    async fn test_blocked_state_missing_output_age() {
        let service = MaterialStateDerivationService;
        let config = MockConfigReader;
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        let mut material = create_test_material("MAT_NO_OUTPUT");
        material.output_age_days_raw = None; // 缺失

        let state = service.derive(&material, &config, today).await.unwrap();

        assert_eq!(state.sched_state, SchedState::Blocked);
        assert!(state
            .urgent_reason
            .unwrap()
            .contains("output_age_days_raw missing"));
    }
}

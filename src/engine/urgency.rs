// ==========================================
// 热轧精整排产系统 - 紧急等级判定引擎
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 3. Urgency Engine
// 红线: 紧急等级是"等级制",不是评分制
// ==========================================
// 职责: 计算催料等级 + 判定最终紧急等级
// 输入: material_master + material_state
// 输出: 更新 material_state (urgent_level, rush_level, urgent_reason)
// ==========================================

use crate::domain::material::{MaterialMaster, MaterialState};
use crate::domain::types::{RushLevel, UrgentLevel};
use chrono::{Duration, NaiveDate};
use serde_json::json;
use tracing::instrument;


// ==========================================
// UrgencyEngine - 紧急等级判定引擎
// ==========================================
pub struct UrgencyEngine {
    // TODO: 注入 MaterialStateRepository
    // TODO: 注入 ConfigManager
}

impl UrgencyEngine {
    /// 创建新的紧急等级判定引擎
    pub fn new() -> Self {
        Self {}
    }

    // ==========================================
    // 核心方法
    // ==========================================

    /// 批量判定紧急等级（推荐使用）
    ///
    /// 批量处理可以优化性能，避免重复计算
    ///
    /// 返回更新后的 MaterialState 列表
    #[instrument(skip(self, materials), fields(count = materials.len()))]
    pub fn evaluate_batch(
        &self,
        materials: Vec<(MaterialMaster, MaterialState)>,
        today: NaiveDate,
        n1_days: i32,
        n2_days: i32,
    ) -> Vec<MaterialState> {
        materials
            .into_iter()
            .map(|(master, mut state)| {
                let (urgent_level, rush_level, urgent_reason) =
                    self.evaluate_single(&master, &state, today, n1_days, n2_days);

                // 更新状态
                state.urgent_level = urgent_level;
                state.rush_level = rush_level;
                state.urgent_reason = Some(urgent_reason);

                state
            })
            .collect()
    }

    /// 单个材料判定（内部使用）
    ///
    /// 返回: (UrgentLevel, RushLevel, urgent_reason_json)
    pub fn evaluate_single(
        &self,
        master: &MaterialMaster,
        state: &MaterialState,
        today: NaiveDate,
        n1_days: i32,
        n2_days: i32,
    ) -> (UrgentLevel, RushLevel, String) {
        // 1. 计算催料等级
        let (rush_level, _rush_reason) = self.calculate_rush_level(
            master.contract_nature.as_deref(),
            master.weekly_delivery_flag.as_deref(),
            master.export_flag.as_deref(),
        );

        // 2. 判定最终紧急等级
        let (urgent_level, urgent_reason) =
            self.determine_urgent_level(state, master, rush_level, today, n1_days, n2_days);

        (urgent_level, rush_level, urgent_reason)
    }

    // ==========================================
    // 催料等级计算 (依据 Engine_Specs 0.3)
    // ==========================================

    /// 计算催料等级
    ///
    /// 规则（顺序执行，命中即返回）:
    /// 1) contract_nature 非空 且 首字母 ∉ {Y,X} 且 weekly_delivery_flag='D' → L2
    /// 2) contract_nature 非空 且 首字母 ∉ {Y,X} 且 weekly_delivery_flag='A' 且 export_flag='1' → L1
    /// 3) 其他 → L0
    ///
    /// 边界处理:
    /// - contract_nature 为 None 或空字符串 → L0
    /// - weekly_delivery_flag 为 None → L0
    /// - export_flag 为 None → 视为 '0'
    pub fn calculate_rush_level(
        &self,
        contract_nature: Option<&str>,
        weekly_delivery_flag: Option<&str>,
        export_flag: Option<&str>,
    ) -> (RushLevel, String) {
        // 边界处理：contract_nature 缺失或为空
        let nature = match contract_nature {
            Some(n) if !n.is_empty() => n,
            _ => return (RushLevel::L0, "RUSH_RULE_L0: contract_nature missing or empty".to_string()),
        };

        // 检查首字母是否为 Y 或 X（研发/试验合同，不参与催料）
        let first_char = nature.chars().next().unwrap().to_uppercase().next().unwrap();
        if first_char == 'Y' || first_char == 'X' {
            return (
                RushLevel::L0,
                format!("RUSH_RULE_L0: contract_nature={} (research/test contract)", nature),
            );
        }

        // 规则1: L2 强催料
        // 条件: contract_nature 非空 且 首字母 ∉ {Y, X} 且 weekly_delivery_flag = 'D'
        if let Some("D") = weekly_delivery_flag {
            return (
                RushLevel::L2,
                format!("RUSH_RULE_L2: contract_nature={}, weekly_delivery=D", nature),
            );
        }

        // 规则2: L1 一般催料
        // 条件: contract_nature 非空 且 首字母 ∉ {Y, X} 且 weekly_delivery_flag = 'A' 且 export_flag = '1'
        if let Some("A") = weekly_delivery_flag {
            let export = export_flag.unwrap_or("0");
            if export == "1" {
                return (
                    RushLevel::L1,
                    format!("RUSH_RULE_L1: contract_nature={}, weekly_delivery=A, export=1", nature),
                );
            }
        }

        // 规则3: L0 无催料（默认）
        (
            RushLevel::L0,
            "RUSH_RULE_L0: no rush conditions met".to_string(),
        )
    }

    // ==========================================
    // 最终紧急等级判定 (依据 Engine_Specs 3.2)
    // ==========================================

    /// 判定最终紧急等级
    ///
    /// 顺序（优先级递减）:
    /// 1) manual_urgent_flag=true → L3 (人工红线)
    /// 2) in_frozen_zone=true → 至少 L2 (冻结区)
    /// 3) due_date < today → L3 (超期)
    /// 4) due_date ≤ today+N1 且 earliest_sched_date > due_date → L3 (适温阻断红线)
    /// 5) due_date ≤ today+N1 → L2 (临近交期)
    /// 6) due_date ≤ today+N2 → L1 (临近交期)
    /// 7) urgent_level = max(urgent_level, rush_level) (催料抬升)
    /// 8) 默认 → L0
    ///
    /// 返回: (UrgentLevel, urgent_reason_json)
    pub fn determine_urgent_level(
        &self,
        state: &MaterialState,
        master: &MaterialMaster,
        rush_level: RushLevel,
        today: NaiveDate,
        n1_days: i32,
        n2_days: i32,
    ) -> (UrgentLevel, String) {
        let mut factors = Vec::new();
        let mut current_level = UrgentLevel::L0;

        // 规则1: 人工红线（最高优先级）
        if state.manual_urgent_flag {
            let reason = json!({
                "level": "L3",
                "primary_reason": "MANUAL_URGENT",
                "factors": ["manual_urgent_flag=true"],
                "details": {
                    "today": today.to_string(),
                }
            });
            return (UrgentLevel::L3, reason.to_string());
        }

        // 规则2: 冻结区保护（工业红线）
        if state.in_frozen_zone {
            factors.push("in_frozen_zone=true".to_string());
            current_level = UrgentLevel::L2; // 至少 L2
        }

        // 规则3: 超期红线
        if let Some(due_date) = master.due_date {
            if due_date < today {
                factors.push(format!("overdue: due_date={} < today={}", due_date, today));
                let reason = json!({
                    "level": "L3",
                    "primary_reason": "OVERDUE",
                    "factors": factors,
                    "details": {
                        "due_date": due_date.to_string(),
                        "today": today.to_string(),
                        "overdue_days": (today - due_date).num_days(),
                    }
                });
                return (UrgentLevel::L3, reason.to_string());
            }

            // 规则4: 适温阻断红线
            let n1_threshold = today + Duration::days(n1_days as i64);
            if due_date <= n1_threshold {
                if let Some(earliest_sched) = state.earliest_sched_date {
                    if earliest_sched > due_date {
                        factors.push(format!(
                            "temp_blocked: due_date={}, earliest_sched_date={}, cannot meet deadline",
                            due_date, earliest_sched
                        ));
                        let reason = json!({
                            "level": "L3",
                            "primary_reason": "TEMP_BLOCKED",
                            "factors": factors,
                            "details": {
                                "due_date": due_date.to_string(),
                                "earliest_sched_date": earliest_sched.to_string(),
                                "today": today.to_string(),
                                "n1_days": n1_days,
                            }
                        });
                        return (UrgentLevel::L3, reason.to_string());
                    }
                }

                // 规则5: 临期N1（紧急）
                factors.push(format!("near_due_n1: due_date={}, within {} days", due_date, n1_days));
                current_level = Self::max_level(current_level, UrgentLevel::L2);
            } else {
                // 规则6: 临期N2（关注）
                let n2_threshold = today + Duration::days(n2_days as i64);
                if due_date <= n2_threshold {
                    factors.push(format!("near_due_n2: due_date={}, within {} days", due_date, n2_days));
                    current_level = Self::max_level(current_level, UrgentLevel::L1);
                }
            }
        }

        // 规则7: 催料抬升
        let rush_urgent_level = Self::rush_to_urgent(rush_level);
        if rush_urgent_level > UrgentLevel::L0 {
            let original_level = current_level;
            current_level = Self::max_level(current_level, rush_urgent_level);
            if current_level != original_level {
                factors.push(format!(
                    "rush_elevated: rush_level={:?}, elevated from {:?} to {:?}",
                    rush_level, original_level, current_level
                ));
            }
        }

        // 规则8: 默认正常
        if factors.is_empty() {
            factors.push("no urgent conditions".to_string());
        }

        let primary_reason = if state.in_frozen_zone {
            "FROZEN_ZONE"
        } else if factors.iter().any(|f| f.starts_with("near_due_n1")) {
            "NEAR_DUE_N1"
        } else if factors.iter().any(|f| f.starts_with("near_due_n2")) {
            "NEAR_DUE_N2"
        } else if factors.iter().any(|f| f.starts_with("rush_elevated")) {
            "RUSH_ELEVATED"
        } else {
            "NORMAL"
        };

        let reason = json!({
            "level": format!("{:?}", current_level),
            "primary_reason": primary_reason,
            "factors": factors,
            "details": {
                "today": today.to_string(),
                "n1_days": n1_days,
                "n2_days": n2_days,
                "rush_level": format!("{:?}", rush_level),
            }
        });

        (current_level, reason.to_string())
    }

    /// 将 RushLevel 转换为对应的 UrgentLevel
    fn rush_to_urgent(rush: RushLevel) -> UrgentLevel {
        match rush {
            RushLevel::L0 => UrgentLevel::L0,
            RushLevel::L1 => UrgentLevel::L1,
            RushLevel::L2 => UrgentLevel::L2,
        }
    }

    /// 返回两个等级中较高的一个
    fn max_level(a: UrgentLevel, b: UrgentLevel) -> UrgentLevel {
        use UrgentLevel::*;
        match (a, b) {
            (L3, _) | (_, L3) => L3,
            (L2, _) | (_, L2) => L2,
            (L1, _) | (_, L1) => L1,
            _ => L0,
        }
    }
}

// ==========================================
// 单元测试
// ==========================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::SchedState;
    use chrono::Utc;

    // ==========================================
    // 测试数据准备
    // ==========================================

    /// 基准日期: 2026-01-17
    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, 17).unwrap()
    }

    /// 临期阈值
    const N1_DAYS: i32 = 3; // 临期阈值N1
    const N2_DAYS: i32 = 7; // 临期阈值N2

    /// 创建基础材料主数据模板
    fn base_master() -> MaterialMaster {
        MaterialMaster {
            material_id: "TEST_MAT_001".to_string(),
            manufacturing_order_id: None,
            material_status_code_src: None,
            steel_mark: None,
            slab_id: None,
            next_machine_code: None,
            rework_machine_code: None,
            current_machine_code: None,
            width_mm: Some(1500.0),
            thickness_mm: Some(10.0),
            length_m: Some(100.0),
            weight_t: Some(10.5),
            available_width_mm: Some(1500.0),
            due_date: Some(NaiveDate::from_ymd_opt(2026, 1, 25).unwrap()),
            stock_age_days: Some(10),
            output_age_days_raw: Some(5),
            rolling_output_date: None,  // v0.7
            status_updated_at: Some(Utc::now()),
            contract_no: Some("C001".to_string()),
            contract_nature: Some("A".to_string()),
            weekly_delivery_flag: Some("N".to_string()),
            export_flag: Some("0".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 创建基础材料状态模板
    fn base_state() -> MaterialState {
        MaterialState {
            material_id: "TEST_MAT_001".to_string(),
            sched_state: SchedState::Ready,
            lock_flag: false,
            force_release_flag: false,
            urgent_level: UrgentLevel::L0,
            urgent_reason: None,
            rush_level: RushLevel::L0,
            rolling_output_age_days: 5,
            ready_in_days: 0,
            earliest_sched_date: Some(today()),
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
            updated_by: None,
        }
    }

    // ==========================================
    // 第一部分：正常案例（Normal Cases）
    // ==========================================

    #[test]
    fn test_scenario_1_manual_urgent() {
        // 场景1: 人工红线（规则1）
        let engine = UrgencyEngine::new();

        let mut state = base_state();
        state.manual_urgent_flag = true;

        let master = base_master();

        let (urgent_level, rush_level, urgent_reason) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言
        assert_eq!(urgent_level, UrgentLevel::L3, "人工红线应为L3");
        assert_eq!(rush_level, RushLevel::L0, "无催料条件");
        assert!(urgent_reason.contains("MANUAL_URGENT"), "原因应包含 MANUAL_URGENT");
        assert!(urgent_reason.contains("\"level\":\"L3\""), "JSON中level应为L3");
    }

    #[test]
    fn test_scenario_2_frozen_zone() {
        // 场景2: 冻结区保护（规则2）
        let engine = UrgencyEngine::new();

        let mut state = base_state();
        state.in_frozen_zone = true;

        let master = base_master();

        let (urgent_level, rush_level, urgent_reason) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言
        assert!(urgent_level >= UrgentLevel::L2, "冻结区材料至少L2");
        assert_eq!(rush_level, RushLevel::L0, "无催料条件");
        assert!(urgent_reason.contains("FROZEN_ZONE") || urgent_reason.contains("in_frozen_zone"),
                "原因应包含冻结区信息");
    }

    #[test]
    fn test_scenario_3_overdue() {
        // 场景3: 超期红线（规则3）
        let engine = UrgencyEngine::new();

        let state = base_state();

        let mut master = base_master();
        master.due_date = Some(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()); // 2天前

        let (urgent_level, rush_level, urgent_reason) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言
        assert_eq!(urgent_level, UrgentLevel::L3, "超期材料应为L3");
        assert_eq!(rush_level, RushLevel::L0, "无催料条件");
        assert!(urgent_reason.contains("OVERDUE"), "原因应包含 OVERDUE");
        assert!(urgent_reason.contains("overdue_days"), "应包含超期天数");
    }

    #[test]
    fn test_scenario_4_temp_blocked() {
        // 场景4: 适温阻断红线（规则4）
        let engine = UrgencyEngine::new();

        let mut state = base_state();
        state.earliest_sched_date = Some(NaiveDate::from_ymd_opt(2026, 1, 22).unwrap()); // 5天后才适温

        let mut master = base_master();
        master.due_date = Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()); // 3天后交期（在N1内）

        let (urgent_level, rush_level, urgent_reason) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言
        assert_eq!(urgent_level, UrgentLevel::L3, "适温阻断应为L3");
        assert_eq!(rush_level, RushLevel::L0, "无催料条件");
        assert!(urgent_reason.contains("TEMP_BLOCKED"), "原因应包含 TEMP_BLOCKED");
    }

    #[test]
    fn test_scenario_5_near_due_n1() {
        // 场景5: 临期N1（规则5）
        let engine = UrgencyEngine::new();

        let state = base_state();

        let mut master = base_master();
        master.due_date = Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()); // 3天后（正好N1边界）

        let (urgent_level, rush_level, urgent_reason) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言
        assert_eq!(urgent_level, UrgentLevel::L2, "临期N1应为L2");
        assert_eq!(rush_level, RushLevel::L0, "无催料条件");
        assert!(urgent_reason.contains("NEAR_DUE_N1") || urgent_reason.contains("near_due_n1"),
                "原因应包含 NEAR_DUE_N1");
    }

    #[test]
    fn test_scenario_6_near_due_n2() {
        // 场景6: 临期N2（规则6）
        let engine = UrgencyEngine::new();

        let state = base_state();

        let mut master = base_master();
        master.due_date = Some(NaiveDate::from_ymd_opt(2026, 1, 24).unwrap()); // 7天后（正好N2边界）

        let (urgent_level, rush_level, urgent_reason) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言
        assert_eq!(urgent_level, UrgentLevel::L1, "临期N2应为L1");
        assert_eq!(rush_level, RushLevel::L0, "无催料条件");
        assert!(urgent_reason.contains("NEAR_DUE_N2") || urgent_reason.contains("near_due_n2"),
                "原因应包含 NEAR_DUE_N2");
    }

    #[test]
    fn test_scenario_7_rush_elevated_l2() {
        // 场景7: 催料抬升L2（规则7）
        let engine = UrgencyEngine::new();

        let state = base_state();

        let mut master = base_master();
        master.contract_nature = Some("B".to_string()); // 非Y/X
        master.weekly_delivery_flag = Some("D".to_string()); // 按周交货
        master.due_date = Some(NaiveDate::from_ymd_opt(2026, 2, 10).unwrap()); // 远期交货

        let (urgent_level, rush_level, urgent_reason) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言
        assert_eq!(rush_level, RushLevel::L2, "应判定为L2强催料");
        assert_eq!(urgent_level, UrgentLevel::L2, "催料应抬升至L2");
        assert!(urgent_reason.contains("rush_elevated") || urgent_reason.contains("RUSH"),
                "原因应包含催料抬升信息");
    }

    #[test]
    fn test_scenario_8_normal() {
        // 场景8: 默认正常（规则8）
        let engine = UrgencyEngine::new();

        let state = base_state();

        let mut master = base_master();
        master.due_date = Some(NaiveDate::from_ymd_opt(2026, 2, 10).unwrap()); // 远期交货

        let (urgent_level, rush_level, urgent_reason) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言
        assert_eq!(urgent_level, UrgentLevel::L0, "无紧急条件应为L0");
        assert_eq!(rush_level, RushLevel::L0, "无催料条件");
        assert!(urgent_reason.contains("NORMAL") || urgent_reason.contains("no urgent conditions"),
                "原因应包含正常信息");
    }

    // ==========================================
    // 第二部分：边界案例（Boundary Cases）
    // ==========================================

    #[test]
    fn test_scenario_9_n1_boundary() {
        // 场景9: N1临界值（today + N1）
        let engine = UrgencyEngine::new();

        let state = base_state();

        let mut master = base_master();
        master.due_date = Some(today() + Duration::days(N1_DAYS as i64)); // 正好N1天后

        let (urgent_level, _, _) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言：边界值应该包含在内（<=）
        assert_eq!(urgent_level, UrgentLevel::L2, "N1边界应判定为L2");
    }

    #[test]
    fn test_scenario_10_n2_boundary() {
        // 场景10: N2临界值（today + N2）
        let engine = UrgencyEngine::new();

        let state = base_state();

        let mut master = base_master();
        master.due_date = Some(today() + Duration::days(N2_DAYS as i64)); // 正好N2天后

        let (urgent_level, _, _) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言：边界值应该包含在内（<=）
        assert_eq!(urgent_level, UrgentLevel::L1, "N2边界应判定为L1");
    }

    #[test]
    fn test_scenario_11_due_date_missing() {
        // 场景11: due_date 缺失
        let engine = UrgencyEngine::new();

        let state = base_state();

        let mut master = base_master();
        master.due_date = None;

        let (urgent_level, rush_level, urgent_reason) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言：跳过所有交期相关规则
        assert_eq!(urgent_level, UrgentLevel::L0, "缺失交期应为L0");
        assert_eq!(rush_level, RushLevel::L0, "无催料条件");
        assert!(urgent_reason.contains("NORMAL") || urgent_reason.contains("no urgent"),
                "原因应包含正常信息");
    }

    #[test]
    fn test_scenario_12_contract_nature_missing() {
        // 场景12: contract_nature 缺失
        let engine = UrgencyEngine::new();

        let state = base_state();

        let mut master = base_master();
        master.contract_nature = None;
        master.weekly_delivery_flag = Some("D".to_string());

        let (_, rush_level, _) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言：缺失合同性质时催料等级为L0
        assert_eq!(rush_level, RushLevel::L0, "缺失contract_nature应为L0");
    }

    #[test]
    fn test_scenario_13_earliest_sched_date_missing() {
        // 场景13: earliest_sched_date 缺失
        let engine = UrgencyEngine::new();

        let mut state = base_state();
        state.earliest_sched_date = None;

        let mut master = base_master();
        master.due_date = Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap());

        let (urgent_level, _, _) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言：跳过规则4（适温阻断），但规则5（临期N1）仍生效
        assert_eq!(urgent_level, UrgentLevel::L2, "临期N1应为L2");
    }

    #[test]
    fn test_scenario_14_config_error_n1_gt_n2() {
        // 场景14: 配置异常（n1 > n2）
        // 注意：在当前实现中，引擎不会自动交换参数，
        // 这个测试只是验证不会panic
        let engine = UrgencyEngine::new();

        let state = base_state();
        let master = base_master();

        let n1_days = 10; // 错误：n1 > n2
        let n2_days = 7;

        // 应该不会panic
        let (_, _, _) = engine.evaluate_single(&master, &state, today(), n1_days, n2_days);
    }

    // ==========================================
    // 第三部分：工业边缘案例（Industrial Edge Cases）
    // ==========================================

    #[test]
    fn test_scenario_15_frozen_zone_and_manual_urgent() {
        // 场景15: 冻结区 + 人工红线（多规则组合）
        let engine = UrgencyEngine::new();

        let mut state = base_state();
        state.manual_urgent_flag = true;
        state.in_frozen_zone = true;

        let master = base_master();

        let (urgent_level, _, urgent_reason) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言：人工红线优先
        assert_eq!(urgent_level, UrgentLevel::L3, "人工红线优先，应为L3");
        assert!(urgent_reason.contains("MANUAL_URGENT"), "原因应包含 MANUAL_URGENT");
    }

    #[test]
    fn test_scenario_16_frozen_zone_elevates_rush_l1() {
        // 场景16: 冻结区 + 催料L1（抬升测试）
        let engine = UrgencyEngine::new();

        let mut state = base_state();
        state.in_frozen_zone = true;

        let mut master = base_master();
        master.contract_nature = Some("A".to_string());
        master.weekly_delivery_flag = Some("A".to_string());
        master.export_flag = Some("1".to_string()); // 触发L1催料
        master.due_date = Some(NaiveDate::from_ymd_opt(2026, 2, 15).unwrap()); // 远期

        let (urgent_level, rush_level, urgent_reason) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言：冻结区规则抬升到L2
        assert_eq!(rush_level, RushLevel::L1, "应判定为L1催料");
        assert_eq!(urgent_level, UrgentLevel::L2, "冻结区应抬升至L2");
        assert!(urgent_reason.contains("FROZEN_ZONE") || urgent_reason.contains("in_frozen_zone"),
                "原因应包含冻结区信息");
    }

    #[test]
    fn test_scenario_17_temp_blocked_priority() {
        // 场景17: 适温阻断 + 临期N1（物理冲突）
        let engine = UrgencyEngine::new();

        let mut state = base_state();
        state.earliest_sched_date = Some(NaiveDate::from_ymd_opt(2026, 1, 22).unwrap());

        let mut master = base_master();
        master.due_date = Some(NaiveDate::from_ymd_opt(2026, 1, 19).unwrap()); // 2天后（N1内）

        let (urgent_level, _, urgent_reason) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言：适温阻断优先
        assert_eq!(urgent_level, UrgentLevel::L3, "适温阻断应为L3");
        assert!(urgent_reason.contains("TEMP_BLOCKED"), "原因应包含 TEMP_BLOCKED");
    }

    #[test]
    fn test_scenario_18_rush_l2_elevates_near_due_n2() {
        // 场景18: 催料L2 + 临期N2（抬升测试）
        let engine = UrgencyEngine::new();

        let state = base_state();

        let mut master = base_master();
        master.due_date = Some(NaiveDate::from_ymd_opt(2026, 1, 24).unwrap()); // 7天后（N2边界）
        master.contract_nature = Some("A".to_string());
        master.weekly_delivery_flag = Some("D".to_string()); // 触发L2催料

        let (urgent_level, rush_level, _) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言：max(L1临期, L2催料) = L2
        assert_eq!(rush_level, RushLevel::L2, "应判定为L2催料");
        assert_eq!(urgent_level, UrgentLevel::L2, "催料应抬升至L2");
    }

    #[test]
    fn test_scenario_19_extreme_combination() {
        // 场景19: 超期 + 冻结区 + 人工红线（极端组合）
        // 注意：人工红线会提前返回，所以即使有超期和冻结区，也是L3+MANUAL_URGENT
        let engine = UrgencyEngine::new();

        let mut state = base_state();
        state.manual_urgent_flag = true;
        state.in_frozen_zone = true;

        let mut master = base_master();
        master.due_date = Some(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()); // 超期

        let (urgent_level, _, urgent_reason) =
            engine.evaluate_single(&master, &state, today(), N1_DAYS, N2_DAYS);

        // 断言：人工红线优先
        assert_eq!(urgent_level, UrgentLevel::L3, "人工红线优先，应为L3");
        assert!(urgent_reason.contains("MANUAL_URGENT"), "原因应包含 MANUAL_URGENT");
    }

    // ==========================================
    // 第四部分：催料等级专项测试（Rush Level Tests）
    // ==========================================

    #[test]
    fn test_scenario_20_rush_l2_weekly_delivery_d() {
        // 场景20: 催料L2 - 按周交货D
        let engine = UrgencyEngine::new();

        let (rush_level, reason) = engine.calculate_rush_level(
            Some("B"), // 非Y/X
            Some("D"), // 按周交货
            None,
        );

        assert_eq!(rush_level, RushLevel::L2, "按周交货D应为L2");
        assert!(reason.contains("RUSH_RULE_L2"), "原因应包含 RUSH_RULE_L2");
        assert!(reason.contains("weekly_delivery=D"), "原因应包含 weekly_delivery=D");
    }

    #[test]
    fn test_scenario_21_rush_l1_export() {
        // 场景21: 催料L1 - 按周交货A + 出口
        let engine = UrgencyEngine::new();

        let (rush_level, reason) = engine.calculate_rush_level(
            Some("C"), // 非Y/X
            Some("A"), // 按周交货A
            Some("1"), // 出口
        );

        assert_eq!(rush_level, RushLevel::L1, "按周交货A+出口应为L1");
        assert!(reason.contains("RUSH_RULE_L1"), "原因应包含 RUSH_RULE_L1");
        assert!(reason.contains("export=1"), "原因应包含 export=1");
    }

    #[test]
    fn test_scenario_22_rush_l0_research_contract() {
        // 场景22: 催料L0 - 研发合同（Y开头）
        let engine = UrgencyEngine::new();

        let (rush_level, reason) = engine.calculate_rush_level(
            Some("Y123"), // 研发合同
            Some("D"),    // 即使是D也不触发
            Some("1"),
        );

        assert_eq!(rush_level, RushLevel::L0, "研发合同应为L0");
        assert!(reason.contains("RUSH_RULE_L0"), "原因应包含 RUSH_RULE_L0");
        assert!(reason.contains("research") || reason.contains("test"),
                "原因应说明研发/试验合同");
    }

    #[test]
    fn test_scenario_23_rush_l0_export_without_weekly_a() {
        // 场景23: 催料L0 - 出口但非按周交货A
        let engine = UrgencyEngine::new();

        let (rush_level, reason) = engine.calculate_rush_level(
            Some("D"),  // 非Y/X
            Some("N"),  // 非A/D
            Some("1"),  // 出口但不满足L1条件
        );

        assert_eq!(rush_level, RushLevel::L0, "不满足催料条件应为L0");
        assert!(reason.contains("RUSH_RULE_L0"), "原因应包含 RUSH_RULE_L0");
    }

    #[test]
    fn test_scenario_24_evaluate_batch() {
        // 场景24: 批量判定测试
        let engine = UrgencyEngine::new();

        let master1 = base_master();
        let mut state1 = base_state();
        state1.material_id = "MAT_001".to_string();
        state1.manual_urgent_flag = true; // L3

        let mut master2 = base_master();
        master2.material_id = "MAT_002".to_string();
        master2.due_date = Some(NaiveDate::from_ymd_opt(2026, 1, 20).unwrap());
        let mut state2 = base_state();
        state2.material_id = "MAT_002".to_string(); // L2 (临期N1)

        let materials = vec![(master1, state1), (master2, state2)];

        let results = engine.evaluate_batch(materials, today(), N1_DAYS, N2_DAYS);

        // 断言
        assert_eq!(results.len(), 2, "应返回2个结果");
        assert_eq!(results[0].urgent_level, UrgentLevel::L3, "第一个材料应为L3");
        assert_eq!(results[1].urgent_level, UrgentLevel::L2, "第二个材料应为L2");
    }
}



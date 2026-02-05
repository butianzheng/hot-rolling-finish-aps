// ==========================================
// 热轧精整排产系统 - Eligibility Core 纯函数库
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 2. Eligibility Engine
// 职责: 提供适温判定、状态判定、紧急等级计算的纯逻辑
// 红线: 无状态、无副作用、无 I/O 操作
// ==========================================

use crate::domain::types::{RushLevel, SchedState, Season, SeasonMode, UrgentLevel};
use chrono::{Datelike, Duration, NaiveDate};

// ==========================================
// EligibilityCore - 纯函数工具类
// ==========================================
pub struct EligibilityCore;

impl EligibilityCore {
    /// 计算等效轧制产出天数
    ///
    /// # 规则 (Engine_Specs 0.4)
    /// - 若 current_machine_code ∉ {H032,H033,H034} → output_age_days_raw + offset_days
    /// - 否则 → output_age_days_raw
    ///
    /// # 参数
    /// - output_age_days_raw: 原始产出天数
    /// - current_machine_code: 当前机组代码
    /// - standard_machines: 标准机组列表
    /// - offset_days: 偏移天数(默认4天)
    pub fn calculate_rolling_output_age_days(
        output_age_days_raw: i32,
        current_machine_code: &str,
        standard_machines: &[String],
        offset_days: i32,
    ) -> i32 {
        if standard_machines.contains(&current_machine_code.to_string()) {
            output_age_days_raw
        } else {
            output_age_days_raw + offset_days
        }
    }

    /// 计算实际产出天数（动态版本，v0.7 新增）
    ///
    /// # 规则
    /// - 若 rolling_output_date 存在 → (today - rolling_output_date).num_days()
    /// - 否则 → 使用 rolling_output_age_days_fallback（静态快照值）
    ///
    /// # 参数
    /// - rolling_output_date: 轧制产出日期（固定基准，v0.7 新增）
    /// - today: 当前日期
    /// - rolling_output_age_days_fallback: 静态快照值（向后兼容）
    ///
    /// # 返回
    /// - i32: 实际产出天数
    ///
    /// # 示例
    /// ```
    /// // 产出日期 2025-01-13，今天 2025-01-20 → 7天
    /// let output_date = chrono::NaiveDate::from_ymd_opt(2025, 1, 13).unwrap();
    /// let today = chrono::NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();
    /// let age_days = EligibilityCore::calculate_actual_output_age_days(
    ///     Some(output_date),
    ///     today,
    ///     1,  // fallback 值（不会使用）
    /// );
    /// assert_eq!(age_days, 7);
    /// ```
    pub fn calculate_actual_output_age_days(
        rolling_output_date: Option<NaiveDate>,
        today: NaiveDate,
        rolling_output_age_days_fallback: i32,
    ) -> i32 {
        match rolling_output_date {
            Some(output_date) => {
                // 新逻辑：动态计算
                let duration = today.signed_duration_since(output_date);
                duration.num_days() as i32
            }
            None => {
                // Fallback：使用静态快照值（向后兼容历史数据）
                rolling_output_age_days_fallback
            }
        }
    }

    /// 计算距离适温还需天数
    ///
    /// # 规则
    /// - ready_in_days = max(0, min_temp_days - rolling_output_age_days)
    ///
    /// # 参数
    /// - rolling_output_age_days: 等效轧制产出天数
    /// - min_temp_days: 最小适温天数(冬季3天/夏季4天)
    pub fn calculate_ready_in_days(
        rolling_output_age_days: i32,
        min_temp_days: i32,
    ) -> i32 {
        (min_temp_days - rolling_output_age_days).max(0)
    }

    /// 计算最早可排日期
    ///
    /// # 规则
    /// - earliest_sched_date = today + ready_in_days
    ///
    /// # 参数
    /// - today: 当前日期
    /// - ready_in_days: 距离适温还需天数
    pub fn calculate_earliest_sched_date(
        today: NaiveDate,
        ready_in_days: i32,
    ) -> NaiveDate {
        today + Duration::days(ready_in_days as i64)
    }

    /// 判定排产状态
    ///
    /// # 规则 (Engine_Specs 2.3)
    /// 1. lock_flag=1 → LOCKED
    /// 2. force_release_flag=1 → FORCE_RELEASE
    /// 3. output_age_days_raw 缺失或 <0 → BLOCKED
    /// 4. current_machine_code 缺失 → BLOCKED
    /// 5. ready_in_days > 0 → PENDING_MATURE
    /// 6. 否则 → READY
    ///
    /// # 参数
    /// - lock_flag: 锁定标记
    /// - force_release_flag: 强制放行标记
    /// - output_age_days_raw: 原始产出天数(可能缺失)
    /// - current_machine_code: 当前机组代码(可能缺失)
    /// - ready_in_days: 距离适温还需天数
    ///
    /// # 返回
    /// - (SchedState, Vec<String>): 状态 + 决策原因
    pub fn determine_sched_state(
        lock_flag: bool,
        force_release_flag: bool,
        output_age_days_raw: Option<i32>,
        current_machine_code: Option<&str>,
        ready_in_days: i32,
    ) -> (SchedState, Vec<String>) {
        let mut reasons = Vec::new();

        // 规则 1: 锁定优先
        if lock_flag {
            reasons.push("LOCKED: lock_flag=1".to_string());
            return (SchedState::Locked, reasons);
        }

        // 规则 2: 强制放行
        if force_release_flag {
            reasons.push("FORCE_RELEASE: force_release_flag=1".to_string());
            return (SchedState::ForceRelease, reasons);
        }

        // 规则 3: 数据质量检查 - output_age_days_raw
        if output_age_days_raw.is_none() {
            reasons.push("BLOCKED: output_age_days_raw missing".to_string());
            return (SchedState::Blocked, reasons);
        }
        if let Some(days) = output_age_days_raw {
            if days < 0 {
                reasons.push(format!("BLOCKED: output_age_days_raw invalid ({})", days));
                return (SchedState::Blocked, reasons);
            }
        }

        // 规则 4: 数据质量检查 - current_machine_code
        if current_machine_code.is_none() {
            reasons.push("BLOCKED: current_machine_code missing".to_string());
            return (SchedState::Blocked, reasons);
        }

        // 规则 5: 适温判定
        if ready_in_days > 0 {
            reasons.push(format!("PENDING_MATURE: ready_in_days={}", ready_in_days));
            return (SchedState::PendingMature, reasons);
        }

        // 规则 6: 默认 READY
        reasons.push("READY: ready_in_days=0".to_string());
        (SchedState::Ready, reasons)
    }

    /// 判定季节
    ///
    /// # 规则 (Engine_Specs 0.1)
    /// - MANUAL 模式: 使用人工指定的季节(覆盖 AUTO)
    /// - AUTO 模式: 按月份判断(默认冬季:11,12,1,2,3)
    ///
    /// # 参数
    /// - today: 当前日期
    /// - season_mode: 季节模式(AUTO/MANUAL)
    /// - manual_season: 人工指定的季节
    /// - winter_months: 冬季月份列表
    pub fn determine_season(
        today: NaiveDate,
        season_mode: SeasonMode,
        manual_season: Season,
        winter_months: &[u32],
    ) -> Season {
        match season_mode {
            SeasonMode::Manual => manual_season,
            SeasonMode::Auto => {
                let current_month = today.month();
                if winter_months.contains(&current_month) {
                    Season::Winter
                } else {
                    Season::Summer
                }
            }
        }
    }

    /// 计算催料等级
    ///
    /// # 规则 (Engine_Specs 0.3)
    /// 1. contract_nature 非空 且 首字母 ∉ {Y,X} 且 weekly_delivery_flag = 'D' → L2
    /// 2. contract_nature 非空 且 首字母 ∉ {Y,X} 且 weekly_delivery_flag = 'A' 且 export_flag = '1' → L1
    /// 3. 其他 → L0
    ///
    /// # 参数
    /// - contract_nature: 合同性质代码
    /// - weekly_delivery_flag: 按周交货标志
    /// - export_flag: 出口标记
    pub fn calculate_rush_level(
        contract_nature: Option<&str>,
        weekly_delivery_flag: Option<&str>,
        export_flag: Option<&str>,
    ) -> RushLevel {
        // 检查 contract_nature 是否非空且首字母不是 Y 或 X
        let is_valid_contract = contract_nature
            .and_then(|s| s.chars().next())
            .map(|first_char| first_char != 'Y' && first_char != 'X')
            .unwrap_or(false);

        if !is_valid_contract {
            return RushLevel::L0;
        }

        // 规则 1: weekly_delivery_flag = 'D' → L2
        if weekly_delivery_flag == Some("D") {
            return RushLevel::L2;
        }

        // 规则 2: weekly_delivery_flag = 'A' 且 export_flag = '1' → L1
        if weekly_delivery_flag == Some("A") && export_flag == Some("1") {
            return RushLevel::L1;
        }

        // 规则 3: 其他 → L0
        RushLevel::L0
    }

    /// 计算紧急等级
    ///
    /// # 规则 (Engine_Specs 3.2) - 7层判定顺序
    /// 1. 人工红线 → L3
    /// 2. 冻结区材料 → 至少 L2
    /// 3. due_date < today → L3
    /// 4. 适温阻断红线:due_date ≤ today+N1 且 earliest_sched_date > due_date → L3
    /// 5. 临近交期:due_date ≤ today+N1 → L2; due_date ≤ today+N2 → L1
    /// 6. 业务抬升:urgent_level = max(urgent_level, rush_level)
    /// 7. 默认 → L0
    ///
    /// # 参数
    /// - due_date: 合同交货期
    /// - today: 当前日期
    /// - n1_days: N1阈值(2/3/5天)
    /// - n2_days: N2阈值(7/10/14天)
    /// - rush_level: 催料等级
    /// - earliest_sched_date: 最早可排日期
    /// - manual_urgent_flag: 人工红线标志
    /// - in_frozen_zone: 是否在冻结区
    ///
    /// # 返回
    /// - (UrgentLevel, Vec<String>): 紧急等级 + 决策原因
    pub fn calculate_urgent_level(
        due_date: Option<NaiveDate>,
        today: NaiveDate,
        n1_days: i32,
        n2_days: i32,
        rush_level: RushLevel,
        earliest_sched_date: Option<NaiveDate>,
        manual_urgent_flag: bool,
        in_frozen_zone: bool,
    ) -> (UrgentLevel, Vec<String>) {
        let mut reasons = Vec::new();

        // 规则 1: 人工红线 → L3
        if manual_urgent_flag {
            reasons.push("MANUAL_RED_LINE".to_string());
            return (UrgentLevel::L3, reasons);
        }

        // 规则 2: 冻结区材料 → 至少 L2
        if in_frozen_zone {
            reasons.push("FREEZE".to_string());
            return (UrgentLevel::L2, reasons);
        }

        // 如果没有交货期,跳过交期相关判定
        let Some(due) = due_date else {
            // 规则 6: 业务抬升(无交期时)
            return match rush_level {
                RushLevel::L2 => {
                    reasons.push("RUSH_L2".to_string());
                    (UrgentLevel::L2, reasons)
                }
                RushLevel::L1 => {
                    reasons.push("RUSH_L1".to_string());
                    (UrgentLevel::L1, reasons)
                }
                RushLevel::L0 => {
                    reasons.push("DEFAULT_L0".to_string());
                    (UrgentLevel::L0, reasons)
                }
            };
        };

        // 规则 3: 超期 → L3
        if due < today {
            reasons.push(format!("LATE: due_date={} < today={}", due, today));
            return (UrgentLevel::L3, reasons);
        }

        // 计算距离交期的天数
        let days_to_due = (due - today).num_days() as i32;

        // 规则 4: 适温阻断红线
        if days_to_due <= n1_days {
            if let Some(earliest) = earliest_sched_date {
                if earliest > due {
                    reasons.push(format!(
                        "TEMP_BLOCK: due_date={} <= today+N1={}, earliest_sched_date={} > due_date",
                        due,
                        today + Duration::days(n1_days as i64),
                        earliest
                    ));
                    return (UrgentLevel::L3, reasons);
                }
            }
        }

        // 规则 5: 临近交期
        let mut urgent_level = if days_to_due <= n1_days {
            reasons.push(format!("N1: days_to_due={} <= N1={}", days_to_due, n1_days));
            UrgentLevel::L2
        } else if days_to_due <= n2_days {
            reasons.push(format!("N2: days_to_due={} <= N2={}", days_to_due, n2_days));
            UrgentLevel::L1
        } else {
            UrgentLevel::L0
        };

        // 规则 6: 业务抬升
        let rush_urgent = match rush_level {
            RushLevel::L2 => UrgentLevel::L2,
            RushLevel::L1 => UrgentLevel::L1,
            RushLevel::L0 => UrgentLevel::L0,
        };

        if rush_urgent > urgent_level {
            reasons.push(format!("RUSH_{:?}", rush_level));
            urgent_level = rush_urgent;
        }

        // 规则 7: 默认
        if urgent_level == UrgentLevel::L0 && reasons.is_empty() {
            reasons.push("DEFAULT_L0".to_string());
        }

        (urgent_level, reasons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================
    // 测试 1: 机组偏移规则
    // ==========================================

    #[test]
    fn test_calculate_rolling_output_age_days_standard_machine() {
        let standard_machines = vec!["H032".to_string(), "H033".to_string(), "H034".to_string()];
        let result = EligibilityCore::calculate_rolling_output_age_days(
            5,
            "H032",
            &standard_machines,
            4,
        );
        assert_eq!(result, 5); // 标准机组不加偏移
    }

    #[test]
    fn test_calculate_rolling_output_age_days_non_standard_machine() {
        let standard_machines = vec!["H032".to_string(), "H033".to_string(), "H034".to_string()];
        let result = EligibilityCore::calculate_rolling_output_age_days(
            5,
            "H030",
            &standard_machines,
            4,
        );
        assert_eq!(result, 9); // 非标准机组 +4
    }

    #[test]
    fn test_calculate_rolling_output_age_days_zero_offset() {
        let standard_machines = vec!["H032".to_string()];
        let result = EligibilityCore::calculate_rolling_output_age_days(
            10,
            "H030",
            &standard_machines,
            0,
        );
        assert_eq!(result, 10); // 偏移为0
    }

    // ==========================================
    // 测试 1.5: 实际产出天数计算（动态版本，v0.7）
    // ==========================================

    #[test]
    fn test_calculate_actual_output_age_days_with_date() {
        // 正常情况：有 rolling_output_date，动态计算
        let output_date = NaiveDate::from_ymd_opt(2025, 1, 13).unwrap();
        let today = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();

        let result = EligibilityCore::calculate_actual_output_age_days(
            Some(output_date),
            today,
            999,  // fallback 值（不应被使用）
        );

        assert_eq!(result, 7);  // 7天产出 (2025-01-20 - 2025-01-13)
    }

    #[test]
    fn test_calculate_actual_output_age_days_same_day() {
        // 边界情况：当天产出
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        let result = EligibilityCore::calculate_actual_output_age_days(
            Some(today),
            today,
            999,
        );

        assert_eq!(result, 0);  // 0天产出
    }

    #[test]
    fn test_calculate_actual_output_age_days_long_duration() {
        // 长时间跨度：50天前产出
        let output_date = NaiveDate::from_ymd_opt(2024, 11, 25).unwrap();
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();

        let result = EligibilityCore::calculate_actual_output_age_days(
            Some(output_date),
            today,
            999,
        );

        assert_eq!(result, 50);  // 50天产出
    }

    #[test]
    fn test_calculate_actual_output_age_days_fallback() {
        // Fallback 情况：无 rolling_output_date（历史数据）
        let today = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();

        let result = EligibilityCore::calculate_actual_output_age_days(
            None,
            today,
            5,  // fallback 值（静态快照）
        );

        assert_eq!(result, 5);  // 使用 fallback 值
    }

    // ==========================================
    // 测试 2: 适温天数计算
    // ==========================================

    #[test]
    fn test_calculate_ready_in_days_mature() {
        let result = EligibilityCore::calculate_ready_in_days(5, 3);
        assert_eq!(result, 0); // 5 >= 3,已适温
    }

    #[test]
    fn test_calculate_ready_in_days_immature() {
        let result = EligibilityCore::calculate_ready_in_days(1, 3);
        assert_eq!(result, 2); // 3 - 1 = 2,需等2天
    }

    #[test]
    fn test_calculate_ready_in_days_exact() {
        let result = EligibilityCore::calculate_ready_in_days(3, 3);
        assert_eq!(result, 0); // 刚好适温
    }

    #[test]
    fn test_calculate_ready_in_days_negative_input() {
        let result = EligibilityCore::calculate_ready_in_days(-1, 3);
        assert_eq!(result, 4); // max(0, 3 - (-1)) = 4
    }

    // ==========================================
    // 测试 3: 最早可排日期
    // ==========================================

    #[test]
    fn test_calculate_earliest_sched_date_immediate() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();
        let result = EligibilityCore::calculate_earliest_sched_date(today, 0);
        assert_eq!(result, today); // ready_in_days=0,今天就可排
    }

    #[test]
    fn test_calculate_earliest_sched_date_future() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();
        let result = EligibilityCore::calculate_earliest_sched_date(today, 3);
        assert_eq!(
            result,
            NaiveDate::from_ymd_opt(2025, 1, 17).unwrap()
        ); // 3天后
    }

    // ==========================================
    // 测试 4: 排产状态判定
    // ==========================================

    #[test]
    fn test_determine_sched_state_locked() {
        let (state, reasons) = EligibilityCore::determine_sched_state(
            true,  // lock_flag
            false, // force_release_flag
            Some(5),
            Some("H032"),
            0,
        );
        assert_eq!(state, SchedState::Locked);
        assert!(reasons.contains(&"LOCKED: lock_flag=1".to_string()));
    }

    #[test]
    fn test_determine_sched_state_force_release() {
        let (state, reasons) = EligibilityCore::determine_sched_state(
            false, // lock_flag
            true,  // force_release_flag
            Some(5),
            Some("H032"),
            2,
        );
        assert_eq!(state, SchedState::ForceRelease);
        assert!(reasons.contains(&"FORCE_RELEASE: force_release_flag=1".to_string()));
    }

    #[test]
    fn test_determine_sched_state_blocked_missing_output_age() {
        let (state, reasons) = EligibilityCore::determine_sched_state(
            false, // lock_flag
            false, // force_release_flag
            None,  // output_age_days_raw missing
            Some("H032"),
            0,
        );
        assert_eq!(state, SchedState::Blocked);
        assert!(reasons.contains(&"BLOCKED: output_age_days_raw missing".to_string()));
    }

    #[test]
    fn test_determine_sched_state_blocked_invalid_output_age() {
        let (state, reasons) = EligibilityCore::determine_sched_state(
            false,
            false,
            Some(-1), // invalid
            Some("H032"),
            0,
        );
        assert_eq!(state, SchedState::Blocked);
        assert!(reasons.iter().any(|r| r.contains("invalid")));
    }

    #[test]
    fn test_determine_sched_state_blocked_missing_machine() {
        let (state, reasons) = EligibilityCore::determine_sched_state(
            false,
            false,
            Some(5),
            None, // current_machine_code missing
            0,
        );
        assert_eq!(state, SchedState::Blocked);
        assert!(reasons.contains(&"BLOCKED: current_machine_code missing".to_string()));
    }

    #[test]
    fn test_determine_sched_state_pending_mature() {
        let (state, reasons) = EligibilityCore::determine_sched_state(
            false,
            false,
            Some(1),
            Some("H032"),
            2, // ready_in_days > 0
        );
        assert_eq!(state, SchedState::PendingMature);
        assert!(reasons.iter().any(|r| r.contains("PENDING_MATURE")));
    }

    #[test]
    fn test_determine_sched_state_ready() {
        let (state, reasons) = EligibilityCore::determine_sched_state(
            false,
            false,
            Some(5),
            Some("H032"),
            0, // ready_in_days = 0
        );
        assert_eq!(state, SchedState::Ready);
        assert!(reasons.contains(&"READY: ready_in_days=0".to_string()));
    }

    // ==========================================
    // 测试 5: 季节判定
    // ==========================================

    #[test]
    fn test_determine_season_manual_winter() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap(); // 夏季月份
        let result = EligibilityCore::determine_season(
            today,
            SeasonMode::Manual,
            Season::Winter,
            &[11, 12, 1, 2, 3],
        );
        assert_eq!(result, Season::Winter); // 人工指定优先
    }

    #[test]
    fn test_determine_season_auto_winter() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let result = EligibilityCore::determine_season(
            today,
            SeasonMode::Auto,
            Season::Summer,
            &[11, 12, 1, 2, 3],
        );
        assert_eq!(result, Season::Winter); // 1月在冬季月份列表
    }

    #[test]
    fn test_determine_season_auto_summer() {
        let today = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let result = EligibilityCore::determine_season(
            today,
            SeasonMode::Auto,
            Season::Winter,
            &[11, 12, 1, 2, 3],
        );
        assert_eq!(result, Season::Summer); // 6月不在冬季月份列表
    }

    // ==========================================
    // 测试 6: 催料等级计算
    // ==========================================

    #[test]
    fn test_calculate_rush_level_l2() {
        let result = EligibilityCore::calculate_rush_level(Some("A"), Some("D"), Some("0"));
        assert_eq!(result, RushLevel::L2); // 非Y/X + D → L2
    }

    #[test]
    fn test_calculate_rush_level_l1() {
        let result = EligibilityCore::calculate_rush_level(Some("A"), Some("A"), Some("1"));
        assert_eq!(result, RushLevel::L1); // 非Y/X + A + export=1 → L1
    }

    #[test]
    fn test_calculate_rush_level_l0_missing_fields() {
        let result = EligibilityCore::calculate_rush_level(None, Some("D"), Some("0"));
        assert_eq!(result, RushLevel::L0); // 缺失字段 → L0
    }

    #[test]
    fn test_calculate_rush_level_l0_y_contract() {
        let result = EligibilityCore::calculate_rush_level(Some("Y123"), Some("D"), Some("0"));
        assert_eq!(result, RushLevel::L0); // Y开头 → L0
    }

    #[test]
    fn test_calculate_rush_level_l0_x_contract() {
        let result = EligibilityCore::calculate_rush_level(Some("X456"), Some("D"), Some("0"));
        assert_eq!(result, RushLevel::L0); // X开头 → L0
    }

    // ==========================================
    // 测试 7: 紧急等级计算(7层判定)
    // ==========================================

    #[test]
    fn test_calculate_urgent_level_manual_red_line() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();
        let (level, reasons) = EligibilityCore::calculate_urgent_level(
            None,
            today,
            2,
            7,
            RushLevel::L0,
            None,
            true,  // manual_urgent_flag
            false,
        );
        assert_eq!(level, UrgentLevel::L3);
        assert!(reasons.contains(&"MANUAL_RED_LINE".to_string()));
    }

    #[test]
    fn test_calculate_urgent_level_frozen_zone() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();
        let (level, reasons) = EligibilityCore::calculate_urgent_level(
            None,
            today,
            2,
            7,
            RushLevel::L0,
            None,
            false,
            true, // in_frozen_zone
        );
        assert_eq!(level, UrgentLevel::L2);
        assert!(reasons.contains(&"FREEZE".to_string()));
    }

    #[test]
    fn test_calculate_urgent_level_late() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();
        let due = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(); // 超期
        let (level, reasons) = EligibilityCore::calculate_urgent_level(
            Some(due),
            today,
            2,
            7,
            RushLevel::L0,
            None,
            false,
            false,
        );
        assert_eq!(level, UrgentLevel::L3);
        assert!(reasons.iter().any(|r| r.contains("LATE")));
    }

    #[test]
    fn test_calculate_urgent_level_temp_block() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();
        let due = NaiveDate::from_ymd_opt(2025, 1, 16).unwrap(); // today+2 (= N1)
        let earliest = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap(); // 适温阻断
        let (level, reasons) = EligibilityCore::calculate_urgent_level(
            Some(due),
            today,
            2,
            7,
            RushLevel::L0,
            Some(earliest),
            false,
            false,
        );
        assert_eq!(level, UrgentLevel::L3);
        assert!(reasons.iter().any(|r| r.contains("TEMP_BLOCK")));
    }

    #[test]
    fn test_calculate_urgent_level_n1() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();
        let due = NaiveDate::from_ymd_opt(2025, 1, 16).unwrap(); // today+2 (= N1)
        let earliest = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(); // 不阻断
        let (level, reasons) = EligibilityCore::calculate_urgent_level(
            Some(due),
            today,
            2,
            7,
            RushLevel::L0,
            Some(earliest),
            false,
            false,
        );
        assert_eq!(level, UrgentLevel::L2);
        assert!(reasons.iter().any(|r| r.contains("N1")));
    }

    #[test]
    fn test_calculate_urgent_level_n2() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();
        let due = NaiveDate::from_ymd_opt(2025, 1, 21).unwrap(); // today+7 (= N2)
        let (level, reasons) = EligibilityCore::calculate_urgent_level(
            Some(due),
            today,
            2,
            7,
            RushLevel::L0,
            None,
            false,
            false,
        );
        assert_eq!(level, UrgentLevel::L1);
        assert!(reasons.iter().any(|r| r.contains("N2")));
    }

    #[test]
    fn test_calculate_urgent_level_rush_uplift() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();
        let due = NaiveDate::from_ymd_opt(2025, 2, 14).unwrap(); // 远期
        let (level, reasons) = EligibilityCore::calculate_urgent_level(
            Some(due),
            today,
            2,
            7,
            RushLevel::L2,
            None,
            false,
            false,
        );
        assert_eq!(level, UrgentLevel::L2);
        assert!(reasons.iter().any(|r| r.contains("RUSH")));
    }

    #[test]
    fn test_calculate_urgent_level_default_l0() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();
        let due = NaiveDate::from_ymd_opt(2025, 2, 14).unwrap(); // 远期
        let (level, reasons) = EligibilityCore::calculate_urgent_level(
            Some(due),
            today,
            2,
            7,
            RushLevel::L0,
            None,
            false,
            false,
        );
        assert_eq!(level, UrgentLevel::L0);
        assert!(reasons.contains(&"DEFAULT_L0".to_string()));
    }

    #[test]
    fn test_calculate_urgent_level_no_due_date_with_rush() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();
        let (level, reasons) = EligibilityCore::calculate_urgent_level(
            None, // 无交货期
            today,
            2,
            7,
            RushLevel::L1,
            None,
            false,
            false,
        );
        assert_eq!(level, UrgentLevel::L1);
        assert!(reasons.contains(&"RUSH_L1".to_string()));
    }
}

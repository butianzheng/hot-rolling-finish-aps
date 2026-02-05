// ==========================================
// 热轧精整排产系统 - 字段派生服务实现
// ==========================================
// 依据: Field_Mapping_Spec_v0.3_Integrated.md - 3. 关键派生字段口径
// 依据: Engine_Specs_v0.3_Integrated.md - 0.3 催料等级 + 0.4 轧制产出时间反推
// 职责: current_machine_code / rolling_output_age_days / rush_level 派生
// ==========================================

use crate::domain::types::RushLevel;
use crate::importer::material_importer_trait::DerivationService as DerivationServiceTrait;

pub struct DerivationService;

impl DerivationServiceTrait for DerivationService {
    /// 派生 current_machine_code（阶段 3）
    ///
    /// # 规则
    /// - COALESCE(rework_machine_code, next_machine_code)
    fn derive_current_machine_code(
        &self,
        rework: Option<String>,
        next: Option<String>,
    ) -> Option<String> {
        rework.or(next)
    }

    /// 派生 rolling_output_age_days（阶段 4）
    ///
    /// # 规则
    /// - 若 current_machine_code ∉ {H032, H033, H034}
    ///   → rolling_output_age_days = output_age_days_raw + 4
    /// - 否则 → rolling_output_age_days = output_age_days_raw
    fn derive_rolling_output_age_days(
        &self,
        output_age_raw: i32,
        machine_code: &str,
    ) -> i32 {
        const STANDARD_MACHINES: &[&str] = &["H032", "H033", "H034"];

        if STANDARD_MACHINES.contains(&machine_code) {
            output_age_raw
        } else {
            output_age_raw + 4
        }
    }

    /// 派生 rolling_output_date（阶段 3.5 - v0.7 新增）
    ///
    /// # 规则
    /// - rolling_output_date = import_date - output_age_days_raw
    /// - 若 output_age_days_raw 缺失或非法（<0），则返回 None
    ///
    /// # 示例
    /// ```
    /// // 2025-01-14 导入，产出天数 = 1
    /// // → rolling_output_date = 2025-01-14 - 1天 = 2025-01-13
    /// ```
    fn derive_rolling_output_date(
        &self,
        import_date: chrono::NaiveDate,
        output_age_days_raw: Option<i32>,
    ) -> Option<chrono::NaiveDate> {
        match output_age_days_raw {
            Some(days) if days >= 0 => {
                Some(import_date - chrono::Duration::days(days as i64))
            }
            _ => None,
        }
    }

    /// 派生 rush_level（阶段 5）
    ///
    /// # 规则（3层判定）
    /// 1. contract_nature 非空 且首字符∉{Y,X} 且 weekly='D' → L2
    /// 2. contract_nature 非空 且首字符∉{Y,X} 且 weekly='A' 且 export='1' → L1
    /// 3. 其他 → L0
    fn derive_rush_level(
        &self,
        contract_nature: Option<String>,
        weekly_flag: Option<String>,
        export_flag: Option<String>,
    ) -> RushLevel {
        // 缺失任一字段 → L0
        let nature = match contract_nature {
            Some(n) if !n.is_empty() => n,
            _ => return RushLevel::L0,
        };

        let weekly = match weekly_flag {
            Some(w) => w,
            None => return RushLevel::L0,
        };

        let export = export_flag.unwrap_or_else(|| "0".to_string());

        // 规则 1: contract_nature 首字符 ∉ {Y, X} 且 weekly='D' → L2
        // 防御性编程：确保首字符大写再比较
        let first_char = nature.to_uppercase().chars().next().unwrap_or('?');
        if first_char != 'Y' && first_char != 'X' && weekly == "D" {
            return RushLevel::L2;
        }

        // 规则 2: contract_nature 首字符 ∉ {Y, X} 且 weekly='A' 且 export='1' → L1
        if first_char != 'Y' && first_char != 'X' && weekly == "A" && export == "1" {
            return RushLevel::L1;
        }

        // 规则 3: 其他 → L0
        RushLevel::L0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_current_machine_code() {
        let service = DerivationService;

        // rework 优先
        assert_eq!(
            service.derive_current_machine_code(
                Some("H033".to_string()),
                Some("H032".to_string())
            ),
            Some("H033".to_string())
        );

        // rework 为空，使用 next
        assert_eq!(
            service.derive_current_machine_code(None, Some("H032".to_string())),
            Some("H032".to_string())
        );

        // 两者皆为空
        assert_eq!(service.derive_current_machine_code(None, None), None);
    }

    #[test]
    fn test_derive_rolling_output_age_days() {
        let service = DerivationService;

        // 标准机组（H032）：不加偏移
        assert_eq!(service.derive_rolling_output_age_days(5, "H032"), 5);
        assert_eq!(service.derive_rolling_output_age_days(5, "H033"), 5);
        assert_eq!(service.derive_rolling_output_age_days(5, "H034"), 5);

        // 非标准机组：+4 偏移
        assert_eq!(service.derive_rolling_output_age_days(5, "H030"), 9);
        assert_eq!(service.derive_rolling_output_age_days(5, "H035"), 9);
    }

    #[test]
    fn test_derive_rolling_output_date() {
        let service = DerivationService;

        // 正常情况：2025-01-14 导入，产出天数=1 → 2025-01-13
        let import_date = chrono::NaiveDate::from_ymd_opt(2025, 1, 14).unwrap();
        assert_eq!(
            service.derive_rolling_output_date(import_date, Some(1)),
            Some(chrono::NaiveDate::from_ymd_opt(2025, 1, 13).unwrap())
        );

        // 边界情况：当天产出（days=0）→ 产出日期 = 导入日期
        assert_eq!(
            service.derive_rolling_output_date(import_date, Some(0)),
            Some(import_date)
        );

        // 长时间跨度：50天前产出
        assert_eq!(
            service.derive_rolling_output_date(import_date, Some(50)),
            Some(chrono::NaiveDate::from_ymd_opt(2024, 11, 25).unwrap())
        );

        // 错误情况：output_age_days_raw 缺失 → None
        assert_eq!(
            service.derive_rolling_output_date(import_date, None),
            None
        );

        // 错误情况：负值（非法）→ None
        assert_eq!(
            service.derive_rolling_output_date(import_date, Some(-1)),
            None
        );
    }

    #[test]
    fn test_derive_rush_level_l2() {
        let service = DerivationService;

        // contract_nature='A' 且 weekly='D' → L2
        let rush_level = service.derive_rush_level(
            Some("A".to_string()),
            Some("D".to_string()),
            Some("0".to_string()),
        );
        assert_eq!(rush_level, RushLevel::L2);
    }

    #[test]
    fn test_derive_rush_level_l1() {
        let service = DerivationService;

        // contract_nature='B' 且 weekly='A' 且 export='1' → L1
        let rush_level = service.derive_rush_level(
            Some("B".to_string()),
            Some("A".to_string()),
            Some("1".to_string()),
        );
        assert_eq!(rush_level, RushLevel::L1);
    }

    #[test]
    fn test_derive_rush_level_l0_default() {
        let service = DerivationService;

        // contract_nature='A' 且 weekly='A' （不匹配任何规则）→ L0
        let rush_level = service.derive_rush_level(
            Some("A".to_string()),
            Some("A".to_string()),
            Some("0".to_string()),
        );
        assert_eq!(rush_level, RushLevel::L0);
    }

    #[test]
    fn test_derive_rush_level_l0_excluded_nature() {
        let service = DerivationService;

        // contract_nature='Y' （首字符排除）→ L0
        let rush_level = service.derive_rush_level(
            Some("Y".to_string()),
            Some("D".to_string()),
            Some("0".to_string()),
        );
        assert_eq!(rush_level, RushLevel::L0);

        // contract_nature='X' （首字符排除）→ L0
        let rush_level = service.derive_rush_level(
            Some("X".to_string()),
            Some("D".to_string()),
            Some("0".to_string()),
        );
        assert_eq!(rush_level, RushLevel::L0);
    }

    #[test]
    fn test_derive_rush_level_l0_missing_fields() {
        let service = DerivationService;

        // contract_nature 缺失 → L0
        let rush_level = service.derive_rush_level(None, Some("D".to_string()), Some("0".to_string()));
        assert_eq!(rush_level, RushLevel::L0);

        // weekly_flag 缺失 → L0
        let rush_level = service.derive_rush_level(Some("A".to_string()), None, Some("0".to_string()));
        assert_eq!(rush_level, RushLevel::L0);
    }
}

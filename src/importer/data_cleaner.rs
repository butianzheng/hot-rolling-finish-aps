// ==========================================
// 热轧精整排产系统 - 数据清洗器实现
// ==========================================
// 依据: Field_Mapping_Spec_v0.3_Integrated.md - 6. 数据质量规则
// 职责: TRIM / UPPER / NULL 标准化 / 数值范围校验
// ==========================================

use crate::importer::error::{ImportError, ImportResult};
use crate::importer::material_importer_trait::DataCleaner as DataCleanerTrait;
use chrono::NaiveDate;

pub struct DataCleaner;

impl DataCleanerTrait for DataCleaner {
    fn clean_text(&self, value: &str, uppercase: bool) -> String {
        let trimmed = value.trim();
        if uppercase {
            trimmed.to_uppercase()
        } else {
            trimmed.to_string()
        }
    }

    fn normalize_null(&self, value: Option<String>) -> Option<String> {
        value.and_then(|v| {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    }

    fn parse_date_yyyymmdd(&self, value: &str) -> Result<NaiveDate, Box<dyn std::error::Error>> {
        NaiveDate::parse_from_str(value, "%Y%m%d")
            .or_else(|_| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    fn validate_decimal(
        &self,
        value: f64,
        min: f64,
        max: f64,
    ) -> Result<f64, Box<dyn std::error::Error>> {
        if value < min || value > max {
            Err(Box::new(ImportError::ValueRangeError {
                row: 0, // 调用方需指定行号
                field: "".to_string(),
                value,
                min,
                max,
            }))
        } else {
            Ok(value)
        }
    }

    fn clean_export_flag(&self, value: Option<String>) -> Option<String> {
        self.normalize_null(value).map(|v| {
            let upper = v.to_uppercase();
            match upper.as_str() {
                "1" | "Y" | "是" | "TRUE" => "1".to_string(),
                "0" | "N" | "否" | "FALSE" | "" => "0".to_string(),
                _ => "0".to_string(), // 默认为否
            }
        })
    }
}

impl DataCleaner {
    /// 清洗并标准化催料字段（TRIM + UPPER）
    pub fn clean_contract_nature(&self, value: Option<String>) -> Option<String> {
        self.normalize_null(value.map(|v| self.clean_text(&v, true)))
    }

    /// 清洗按周交货标志（TRIM + UPPER）
    pub fn clean_weekly_delivery_flag(&self, value: Option<String>) -> Option<String> {
        self.normalize_null(value.map(|v| self.clean_text(&v, true)))
    }

    /// 清洗并标准化出口标记（统一为 '1' / '0'）
    pub fn clean_export_flag(&self, value: Option<String>) -> Option<String> {
        self.normalize_null(value).map(|v| {
            let upper = v.to_uppercase();
            match upper.as_str() {
                "1" | "Y" | "是" | "TRUE" => "1".to_string(),
                "0" | "N" | "否" | "FALSE" | "" => "0".to_string(),
                _ => "0".to_string(), // 默认为否
            }
        })
    }

    /// 校验重量范围（0 < weight <= threshold）
    pub fn validate_weight(
        &self,
        weight: Option<f64>,
        threshold: f64,
        row: usize,
    ) -> ImportResult<()> {
        if let Some(w) = weight {
            if w <= 0.0 {
                return Err(ImportError::ValueRangeError {
                    row,
                    field: "weight_t".to_string(),
                    value: w,
                    min: 0.0,
                    max: f64::MAX,
                });
            }
            if w > threshold {
                // 这是警告级别，不阻断导入
                // 调用方需记录 DQ Warning
            }
        }
        Ok(())
    }

    /// 校验宽度/厚度范围（> 0）
    pub fn validate_dimension(
        &self,
        dimension: Option<f64>,
        field_name: &str,
        row: usize,
    ) -> ImportResult<()> {
        if let Some(d) = dimension {
            if d <= 0.0 {
                return Err(ImportError::ValueRangeError {
                    row,
                    field: field_name.to_string(),
                    value: d,
                    min: 0.0,
                    max: f64::MAX,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text_basic() {
        let cleaner = DataCleaner;
        assert_eq!(cleaner.clean_text("  hello  ", false), "hello");
        assert_eq!(cleaner.clean_text("  hello  ", true), "HELLO");
    }

    #[test]
    fn test_normalize_null() {
        let cleaner = DataCleaner;
        assert_eq!(cleaner.normalize_null(Some("  ".to_string())), None);
        assert_eq!(cleaner.normalize_null(Some("".to_string())), None);
        assert_eq!(
            cleaner.normalize_null(Some("  value  ".to_string())),
            Some("value".to_string())
        );
        assert_eq!(cleaner.normalize_null(None), None);
    }

    #[test]
    fn test_clean_export_flag() {
        let cleaner = DataCleaner;
        assert_eq!(
            cleaner.clean_export_flag(Some("1".to_string())),
            Some("1".to_string())
        );
        assert_eq!(
            cleaner.clean_export_flag(Some("Y".to_string())),
            Some("1".to_string())
        );
        assert_eq!(
            cleaner.clean_export_flag(Some("是".to_string())),
            Some("1".to_string())
        );
        assert_eq!(
            cleaner.clean_export_flag(Some("0".to_string())),
            Some("0".to_string())
        );
        assert_eq!(
            cleaner.clean_export_flag(Some("N".to_string())),
            Some("0".to_string())
        );
        assert_eq!(
            cleaner.clean_export_flag(Some("invalid".to_string())),
            Some("0".to_string())
        );
    }

    #[test]
    fn test_parse_date_yyyymmdd() {
        let cleaner = DataCleaner;
        let date = cleaner.parse_date_yyyymmdd("20250120").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 1, 20).unwrap());

        // 兼容 YYYY-MM-DD
        let date = cleaner.parse_date_yyyymmdd("2025-01-20").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 1, 20).unwrap());
    }

    #[test]
    fn test_validate_decimal() {
        let cleaner = DataCleaner;
        assert!(cleaner.validate_decimal(5.0, 0.0, 10.0).is_ok());
        assert!(cleaner.validate_decimal(-1.0, 0.0, 10.0).is_err());
        assert!(cleaner.validate_decimal(11.0, 0.0, 10.0).is_err());
    }
}

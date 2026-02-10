// ==========================================
// 热轧精整排产系统 - 数据质量校验器实现
// ==========================================
// 依据: Field_Mapping_Spec_v0.3_Integrated.md - 6. 数据质量规则
// 职责: DQ Level 1/2/3 校验 + DQ 报告生成
// ==========================================

use crate::domain::material::{DqLevel, DqReport, DqSummary, DqViolation, RawMaterialRecord};
use crate::importer::material_importer_trait::DqValidator as DqValidatorTrait;
use std::collections::HashSet;

pub struct DqValidator {
    weight_anomaly_threshold: f64, // 重量异常阈值（吨）
}

impl DqValidator {
    pub fn new(weight_anomaly_threshold: f64) -> Self {
        Self {
            weight_anomaly_threshold,
        }
    }
}

impl DqValidatorTrait for DqValidator {
    /// 校验主键（material_id 非空且唯一）
    fn validate_primary_key(&self, records: &[RawMaterialRecord]) -> Vec<DqViolation> {
        let mut violations = Vec::new();
        let mut seen_ids = HashSet::new();

        for record in records {
            // DQ Level 1: 主键缺失
            if record.material_id.is_none() {
                violations.push(DqViolation {
                    row_number: record.row_number,
                    material_id: None,
                    level: DqLevel::Error,
                    field: "material_id".to_string(),
                    message: "主键缺失".to_string(),
                });
                continue;
            }

            let id = record.material_id.as_ref().unwrap();

            // DQ Level 1: 主键重复（同批次内）
            if !seen_ids.insert(id.clone()) {
                violations.push(DqViolation {
                    row_number: record.row_number,
                    material_id: Some(id.clone()),
                    level: DqLevel::Conflict,
                    field: "material_id".to_string(),
                    message: "重复材料号（同批次内）".to_string(),
                });
            }
        }

        violations
    }

    /// 校验必填字段
    fn validate_required_fields(&self, record: &RawMaterialRecord) -> Vec<DqViolation> {
        let mut violations = Vec::new();

        // 机组代码（必须通过派生计算得到）
        // 此处假设 current_machine_code 已在外部派生
        // 如果外部派生失败，应在此检查

        // 产出时间（天）- 适温反推基础
        if record.output_age_days_raw.is_none() {
            violations.push(DqViolation {
                row_number: record.row_number,
                material_id: record.material_id.clone(),
                level: DqLevel::Error,
                field: "output_age_days_raw".to_string(),
                message: "产出时间缺失，无法判定适温".to_string(),
            });
        } else if let Some(days) = record.output_age_days_raw {
            if days < 0 {
                violations.push(DqViolation {
                    row_number: record.row_number,
                    material_id: record.material_id.clone(),
                    level: DqLevel::Error,
                    field: "output_age_days_raw".to_string(),
                    message: format!("产出时间为负数: {}", days),
                });
            }
        }

        // 状态时间（天）- 库存压力口径
        if let Some(days) = record.stock_age_days {
            if days < 0 {
                violations.push(DqViolation {
                    row_number: record.row_number,
                    material_id: record.material_id.clone(),
                    level: DqLevel::Warning,
                    field: "stock_age_days".to_string(),
                    message: format!("状态时间为负数: {}", days),
                });
            }
        }

        // 催料字段完整性（INFO 级别）
        if record.contract_nature.is_none()
            || record.weekly_delivery_flag.is_none()
            || record.export_flag.is_none()
        {
            violations.push(DqViolation {
                row_number: record.row_number,
                material_id: record.material_id.clone(),
                level: DqLevel::Info,
                field: "contract_nature,weekly_delivery_flag,export_flag".to_string(),
                message: "催料字段不完整，rush_level 设为 L0".to_string(),
            });
        }

        violations
    }

    /// 校验数值范围
    fn validate_ranges(&self, record: &RawMaterialRecord) -> Vec<DqViolation> {
        let mut violations = Vec::new();

        // 重量范围校验
        if let Some(weight) = record.weight_t {
            if weight <= 0.0 {
                violations.push(DqViolation {
                    row_number: record.row_number,
                    material_id: record.material_id.clone(),
                    level: DqLevel::Warning,
                    field: "weight_t".to_string(),
                    message: format!("重量 <= 0: {:.3}", weight),
                });
            } else if weight > self.weight_anomaly_threshold {
                violations.push(DqViolation {
                    row_number: record.row_number,
                    material_id: record.material_id.clone(),
                    level: DqLevel::Warning,
                    field: "weight_t".to_string(),
                    message: format!(
                        "重量异常 ({:.3} > {:.3}t)，可能单位错误",
                        weight, self.weight_anomaly_threshold
                    ),
                });
            }
        }

        // 宽度范围校验
        if let Some(width) = record.width_mm {
            if width <= 0.0 {
                violations.push(DqViolation {
                    row_number: record.row_number,
                    material_id: record.material_id.clone(),
                    level: DqLevel::Warning,
                    field: "width_mm".to_string(),
                    message: format!("宽度 <= 0: {:.1}", width),
                });
            }
        }

        // 厚度范围校验
        if let Some(thickness) = record.thickness_mm {
            if thickness <= 0.0 {
                violations.push(DqViolation {
                    row_number: record.row_number,
                    material_id: record.material_id.clone(),
                    level: DqLevel::Warning,
                    field: "thickness_mm".to_string(),
                    message: format!("厚度 <= 0: {:.2}", thickness),
                });
            }
        }

        violations
    }

    /// 生成 DQ 报告
    fn generate_dq_report(&self, batch_id: String, violations: Vec<DqViolation>) -> DqReport {
        // 统计各级别数量
        let error_count = violations
            .iter()
            .filter(|v| matches!(v.level, DqLevel::Error))
            .count();
        let warning_count = violations
            .iter()
            .filter(|v| matches!(v.level, DqLevel::Warning))
            .count();
        let conflict_count = violations
            .iter()
            .filter(|v| matches!(v.level, DqLevel::Conflict))
            .count();

        DqReport {
            batch_id,
            summary: DqSummary {
                total_rows: 0, // 外部填充
                success: 0,    // 外部填充
                blocked: error_count,
                warning: warning_count,
                conflict: conflict_count,
            },
            violations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_record(material_id: Option<String>, row_number: usize) -> RawMaterialRecord {
        RawMaterialRecord {
            material_id,
            manufacturing_order_id: None,
            material_status_code_src: None,
            due_date: None,
            next_machine_code: Some("H032".to_string()),
            rework_machine_code: None,
            width_mm: Some(1250.0),
            thickness_mm: Some(2.5),
            length_m: Some(100.0),
            weight_t: Some(2.450),
            available_width_mm: None,
            steel_mark: None,
            slab_id: None,
            stock_age_days: Some(10),
            output_age_days_raw: Some(5),
            status_updated_at: None,
            contract_no: None,
            contract_nature: Some("A".to_string()),
            weekly_delivery_flag: Some("D".to_string()),
            export_flag: Some("0".to_string()),
            row_number,
        }
    }

    #[test]
    fn test_validate_primary_key_missing() {
        let validator = DqValidator::new(100.0);
        let records = vec![create_test_record(None, 1)];

        let violations = validator.validate_primary_key(&records);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].level, DqLevel::Error);
        assert_eq!(violations[0].field, "material_id");
    }

    #[test]
    fn test_validate_primary_key_duplicate() {
        let validator = DqValidator::new(100.0);
        let records = vec![
            create_test_record(Some("MAT001".to_string()), 1),
            create_test_record(Some("MAT001".to_string()), 2),
        ];

        let violations = validator.validate_primary_key(&records);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].level, DqLevel::Conflict);
        assert_eq!(violations[0].row_number, 2);
    }

    #[test]
    fn test_validate_required_fields_output_age_missing() {
        let validator = DqValidator::new(100.0);
        let mut record = create_test_record(Some("MAT001".to_string()), 1);
        record.output_age_days_raw = None;

        let violations = validator.validate_required_fields(&record);

        assert!(violations
            .iter()
            .any(|v| v.field == "output_age_days_raw" && matches!(v.level, DqLevel::Error)));
    }

    #[test]
    fn test_validate_ranges_weight_zero() {
        let validator = DqValidator::new(100.0);
        let mut record = create_test_record(Some("MAT001".to_string()), 1);
        record.weight_t = Some(0.0);

        let violations = validator.validate_ranges(&record);

        assert!(violations
            .iter()
            .any(|v| v.field == "weight_t" && matches!(v.level, DqLevel::Warning)));
    }

    #[test]
    fn test_validate_ranges_weight_anomaly() {
        let validator = DqValidator::new(100.0);
        let mut record = create_test_record(Some("MAT001".to_string()), 1);
        record.weight_t = Some(150.0); // 超过阈值

        let violations = validator.validate_ranges(&record);

        assert!(violations
            .iter()
            .any(|v| v.field == "weight_t" && v.message.contains("重量异常")));
    }
}

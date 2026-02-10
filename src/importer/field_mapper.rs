// ==========================================
// 热轧精整排产系统 - 字段映射器实现
// ==========================================
// 依据: Field_Mapping_Spec_v0.3_Integrated.md - 标准字段映射表
// 职责: 源字段 → 标准字段映射 + 类型转换
// ==========================================

use crate::domain::material::RawMaterialRecord;
use crate::importer::error::{ImportError, ImportResult};
use crate::importer::material_importer_trait::FieldMapper as FieldMapperTrait;
use chrono::{DateTime, NaiveDate, Utc};
use std::collections::HashMap;

pub struct FieldMapper;

impl FieldMapperTrait for FieldMapper {
    fn map_to_raw_material(
        &self,
        row: HashMap<String, String>,
        row_number: usize,
    ) -> Result<RawMaterialRecord, Box<dyn std::error::Error>> {
        Ok(RawMaterialRecord {
            // 主键
            material_id: self.get_string(&row, "材料号"),

            // 基础信息
            manufacturing_order_id: self.get_string(&row, "制造命令号"),
            material_status_code_src: self.get_string(&row, "材料状态码"),
            steel_mark: self.get_string(&row, "出钢记号"),
            slab_id: self.get_string(&row, "板坯号"),

            // 机组信息
            next_machine_code: self.get_string(&row, "下道机组代码"),
            rework_machine_code: self.get_string(&row, "精整返修机组"),

            // 工艺维度
            width_mm: self.parse_f64(&row, "材料实际宽度", row_number)?,
            thickness_mm: self.parse_f64(&row, "材料实际厚度", row_number)?,
            length_m: self.parse_f64(&row, "材料实际长度", row_number)?,
            weight_t: self.parse_f64(&row, "材料实际重量", row_number)?,
            available_width_mm: self.parse_f64(&row, "可利用宽度", row_number)?,

            // 时间信息
            due_date: self.parse_date(&row, "合同交货期", row_number)?,
            stock_age_days: self.parse_i32(&row, "状态时间(天)", row_number)?,
            output_age_days_raw: self.parse_i32(&row, "产出时间(天)", row_number)?,
            status_updated_at: self.parse_datetime(&row, "物料状态修改时间", row_number)?,

            // 合同影子字段
            contract_no: self.get_string(&row, "合同号"),
            contract_nature: self.get_string(&row, "合同性质代码"),
            weekly_delivery_flag: self.get_string(&row, "按周交货标志"),
            export_flag: self.get_string(&row, "出口标记"),

            // 元信息
            row_number,
        })
    }
}

impl FieldMapper {
    /// 提取字符串字段（返回 Option），支持多个可能的列名（别名）
    fn get_string(&self, row: &HashMap<String, String>, key: &str) -> Option<String> {
        // 定义列名别名映射
        let aliases: Vec<&str> = match key {
            "可利用宽度" => vec!["可利用宽度", "材料可用宽度"],
            "合同交货期" => vec!["合同交货期", "交货期"],
            "状态时间(天)" => vec!["状态时间(天)", "库存天数"],
            "产出时间(天)" => vec!["产出时间(天)", "出钢天数"],
            "物料状态修改时间" => vec!["物料状态修改时间", "状态更新时间"],
            "合同性质代码" => vec!["合同性质代码", "合同性质"],
            "按周交货标志" => vec!["按周交货标志", "周交期标记"],
            _ => vec![key],
        };

        // 尝试所有可能的列名
        for alias in aliases {
            if let Some(v) = row.get(alias) {
                let trimmed = v.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
        None
    }

    /// 解析浮点数
    fn parse_f64(
        &self,
        row: &HashMap<String, String>,
        key: &str,
        row_number: usize,
    ) -> ImportResult<Option<f64>> {
        match self.get_string(row, key) {
            None => Ok(None),
            Some(value) => {
                value
                    .parse::<f64>()
                    .map(Some)
                    .map_err(|_| ImportError::TypeConversionError {
                        row: row_number,
                        field: key.to_string(),
                        message: format!("无法解析为浮点数: {}", value),
                    })
            }
        }
    }

    /// 解析整数
    fn parse_i32(
        &self,
        row: &HashMap<String, String>,
        key: &str,
        row_number: usize,
    ) -> ImportResult<Option<i32>> {
        match self.get_string(row, key) {
            None => Ok(None),
            Some(value) => {
                value
                    .parse::<i32>()
                    .map(Some)
                    .map_err(|_| ImportError::TypeConversionError {
                        row: row_number,
                        field: key.to_string(),
                        message: format!("无法解析为整数: {}", value),
                    })
            }
        }
    }

    /// 解析日期（YYYYMMDD → NaiveDate）
    fn parse_date(
        &self,
        row: &HashMap<String, String>,
        key: &str,
        row_number: usize,
    ) -> ImportResult<Option<NaiveDate>> {
        match self.get_string(row, key) {
            None => Ok(None),
            Some(value) => {
                // 尝试解析 YYYYMMDD 格式
                NaiveDate::parse_from_str(&value, "%Y%m%d")
                    .map(Some)
                    .or_else(|_| {
                        // 尝试解析 YYYY-MM-DD 格式（兼容）
                        NaiveDate::parse_from_str(&value, "%Y-%m-%d").map(Some)
                    })
                    .map_err(|_| ImportError::DateFormatError {
                        row: row_number,
                        field: key.to_string(),
                        value: value.clone(),
                    })
            }
        }
    }

    /// 解析日期时间（YYYYMMDDHHMMSS → DateTime<Utc>）
    fn parse_datetime(
        &self,
        row: &HashMap<String, String>,
        key: &str,
        row_number: usize,
    ) -> ImportResult<Option<DateTime<Utc>>> {
        match self.get_string(row, key) {
            None => Ok(None),
            Some(value) => {
                // 尝试解析 YYYYMMDDHHMMSS 格式
                let naive_dt = chrono::NaiveDateTime::parse_from_str(&value, "%Y%m%d%H%M%S")
                    .or_else(|_| {
                        // 尝试 ISO 8601 格式（兼容）
                        chrono::NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S")
                    })
                    .map_err(|_| ImportError::TypeConversionError {
                        row: row_number,
                        field: key.to_string(),
                        message: format!("日期时间格式错误: {}", value),
                    })?;

                Ok(Some(DateTime::<Utc>::from_naive_utc_and_offset(
                    naive_dt, Utc,
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_mapper_basic() {
        let mut row = HashMap::new();
        row.insert("材料号".to_string(), "MAT001".to_string());
        row.insert("材料实际重量".to_string(), "2.450".to_string());
        row.insert("下道机组代码".to_string(), "H032".to_string());

        let mapper = FieldMapper;
        let record = mapper.map_to_raw_material(row, 1).unwrap();

        assert_eq!(record.material_id, Some("MAT001".to_string()));
        assert_eq!(record.weight_t, Some(2.450));
        assert_eq!(record.next_machine_code, Some("H032".to_string()));
    }

    #[test]
    fn test_field_mapper_trim_whitespace() {
        let mut row = HashMap::new();
        row.insert("材料号".to_string(), "  MAT001  ".to_string());

        let mapper = FieldMapper;
        let record = mapper.map_to_raw_material(row, 1).unwrap();

        assert_eq!(record.material_id, Some("MAT001".to_string()));
    }

    #[test]
    fn test_field_mapper_empty_as_none() {
        let mut row = HashMap::new();
        row.insert("材料号".to_string(), "MAT001".to_string());
        row.insert("合同号".to_string(), "".to_string());

        let mapper = FieldMapper;
        let record = mapper.map_to_raw_material(row, 1).unwrap();

        assert_eq!(record.material_id, Some("MAT001".to_string()));
        assert_eq!(record.contract_no, None);
    }

    #[test]
    fn test_field_mapper_date_yyyymmdd() {
        let mut row = HashMap::new();
        row.insert("材料号".to_string(), "MAT001".to_string());
        row.insert("合同交货期".to_string(), "20250120".to_string());

        let mapper = FieldMapper;
        let record = mapper.map_to_raw_material(row, 1).unwrap();

        assert_eq!(
            record.due_date,
            Some(NaiveDate::from_ymd_opt(2025, 1, 20).unwrap())
        );
    }

    #[test]
    fn test_field_mapper_invalid_number() {
        let mut row = HashMap::new();
        row.insert("材料号".to_string(), "MAT001".to_string());
        row.insert("材料实际重量".to_string(), "invalid".to_string());

        let mapper = FieldMapper;
        let result = mapper.map_to_raw_material(row, 1);

        assert!(result.is_err());
    }
}

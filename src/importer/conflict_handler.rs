// ==========================================
// 热轧精整排产系统 - 冲突处理器实现
// ==========================================
// 依据: data_dictionary_v0.1.md - 重复材料号导入策略 Option C
// 职责: 检测同批次内/跨批次重复 material_id
// ==========================================

use crate::domain::material::RawMaterialRecord;
use crate::importer::material_importer_trait::ConflictHandler as ConflictHandlerTrait;
use std::collections::HashMap;

pub struct ConflictHandler;

impl ConflictHandlerTrait for ConflictHandler {
    /// 检测同批次内重复材料号
    ///
    /// # 返回
    /// - Vec<(行号, material_id)>: 重复记录列表（不包括第一次出现）
    fn detect_duplicates(&self, records: &[RawMaterialRecord]) -> Vec<(usize, String)> {
        let mut first_occurrence: HashMap<String, usize> = HashMap::new();
        let mut duplicates = Vec::new();

        for record in records {
            if let Some(material_id) = &record.material_id {
                if let Some(_first_row) = first_occurrence.get(material_id) {
                    // 发现重复：记录当前行号
                    duplicates.push((record.row_number, material_id.clone()));
                } else {
                    // 首次出现：记录行号
                    first_occurrence.insert(material_id.clone(), record.row_number);
                }
            }
        }

        duplicates
    }

    /// 检测跨批次重复（需查询数据库）
    ///
    /// # 参数
    /// - records: 待导入记录列表
    /// - existing_ids: 数据库中已存在的 material_id 列表
    ///
    /// # 返回
    /// - Vec<(行号, material_id)>: 跨批次重复记录列表
    fn detect_cross_batch_duplicates(
        &self,
        records: &[RawMaterialRecord],
        existing_ids: &[String],
    ) -> Vec<(usize, String)> {
        let existing_set: std::collections::HashSet<_> = existing_ids.iter().collect();
        let mut duplicates = Vec::new();

        for record in records {
            if let Some(material_id) = &record.material_id {
                if existing_set.contains(material_id) {
                    duplicates.push((record.row_number, material_id.clone()));
                }
            }
        }

        duplicates
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
    fn test_detect_duplicates_none() {
        let handler = ConflictHandler;
        let records = vec![
            create_test_record(Some("MAT001".to_string()), 1),
            create_test_record(Some("MAT002".to_string()), 2),
        ];

        let duplicates = handler.detect_duplicates(&records);

        assert_eq!(duplicates.len(), 0);
    }

    #[test]
    fn test_detect_duplicates_found() {
        let handler = ConflictHandler;
        let records = vec![
            create_test_record(Some("MAT001".to_string()), 1),
            create_test_record(Some("MAT002".to_string()), 2),
            create_test_record(Some("MAT001".to_string()), 3), // 重复
        ];

        let duplicates = handler.detect_duplicates(&records);

        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].0, 3); // 行号
        assert_eq!(duplicates[0].1, "MAT001");
    }

    #[test]
    fn test_detect_duplicates_multiple() {
        let handler = ConflictHandler;
        let records = vec![
            create_test_record(Some("MAT001".to_string()), 1),
            create_test_record(Some("MAT001".to_string()), 2), // 重复
            create_test_record(Some("MAT001".to_string()), 3), // 再次重复
        ];

        let duplicates = handler.detect_duplicates(&records);

        assert_eq!(duplicates.len(), 2);
        assert_eq!(duplicates[0].0, 2);
        assert_eq!(duplicates[1].0, 3);
    }

    #[test]
    fn test_detect_cross_batch_duplicates() {
        let handler = ConflictHandler;
        let records = vec![
            create_test_record(Some("MAT001".to_string()), 1),
            create_test_record(Some("MAT002".to_string()), 2),
        ];

        let existing_ids = vec!["MAT001".to_string()];

        let duplicates = handler.detect_cross_batch_duplicates(&records, &existing_ids);

        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].0, 1);
        assert_eq!(duplicates[0].1, "MAT001");
    }
}

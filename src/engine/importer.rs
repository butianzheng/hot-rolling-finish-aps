// ==========================================
// 热轧精整排产系统 - 材料导入引擎
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 1. 总体引擎架构与数据流
// 依据: Field_Mapping_Spec_v0.3_Integrated.md - 字段映射与口径
// 依据: data_dictionary_v0.1.md - 数据字典
// ==========================================
// 职责: CSV/Excel 导入 + 字段映射 + DQ检查 + 状态派生 + 批次管理
// 红线: 不含UI逻辑,只负责数据处理和状态派生
// ==========================================

use crate::config::ImportConfigReader;
use crate::domain::material::{
    ConflictType, DqLevel, DqReport, DqSummary, DqViolation, ImportBatch, ImportConflict,
    ImportResult, MaterialMaster, MaterialState, RawMaterialRecord,
};
use crate::engine::MaterialStateDerivationService;
use crate::repository::material_import_repo::MaterialImportRepository;
use chrono::{NaiveDate, Utc};
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

// ==========================================
// MaterialImporter - 材料导入引擎
// ==========================================
/// 材料导入引擎
///
/// # 职责
/// 1. 解析CSV/Excel文件
/// 2. 字段映射与派生计算
/// 3. 数据质量检查
/// 4. 调用MaterialStateDerivationService派生状态
/// 5. 批量保存到数据库
/// 6. 记录批次信息和冲突记录
///
/// # 红线
/// - 不含UI逻辑
/// - 所有数据库操作通过Repository
/// - 不修改物理状态,只写入material_master和material_state
pub struct MaterialImporter<R:?Sized, C>
where
    R: MaterialImportRepository,
    C: ImportConfigReader,
{
    repo: Arc<R>,
    config: Arc<C>,
    state_service: MaterialStateDerivationService,
}

impl<R: ?Sized, C> MaterialImporter<R, C>
where
    R: MaterialImportRepository,
    C: ImportConfigReader,
{
    /// 创建新的 MaterialImporter 实例
    ///
    /// # 参数
    /// - repo: 材料导入仓储
    /// - config: 配置读取器
    pub fn new(repo: Arc<R>, config: Arc<C>) -> Self {
        Self {
            repo,
            config,
            state_service: MaterialStateDerivationService::new(),
        }
    }

    /// 从CSV文件导入材料数据(主入口)
    ///
    /// # 参数
    /// - file_path: CSV文件路径
    /// - operator: 操作人
    ///
    /// # 返回
    /// - ImportResult: 导入结果(批次信息 + DQ报告 + 统计)
    ///
    /// # 流程
    /// 1. 解析CSV文件 → Vec<RawMaterialRecord>
    /// 2. 数据质量检查 → 过滤ERROR级别违规
    /// 3. 字段映射与派生 → Vec<MaterialMaster>
    /// 4. 状态派生 → Vec<MaterialState>
    /// 5. 批量保存(事务化)
    /// 6. 记录批次和冲突
    pub async fn import_from_csv(
        &self,
        file_path: &str,
        operator: &str,
    ) -> Result<ImportResult, Box<dyn Error>> {
        let start_time = std::time::Instant::now();
        let batch_id = Uuid::new_v4().to_string();
        let today = chrono::Local::now().naive_local().date();

        // === 步骤 1: 解析CSV文件 ===
        let raw_records = self.parse_csv(file_path)?;
        let total_rows = raw_records.len();

        // === 步骤 2: 数据质量检查 ===
        let (valid_records, violations, conflicts) =
            self.validate_records(raw_records, &batch_id);

        // === 步骤 3: 字段映射与派生 ===
        let masters = self.map_to_material_master(valid_records.clone())?;

        // === 步骤 4: 状态派生 ===
        let states = self
            .derive_material_states(&masters, today)
            .await?;

        // === 步骤 5: 批量保存(事务化) ===
        let success_count = self.save_materials_and_states(masters, states).await?;

        // === 步骤 6: 记录批次和冲突 ===
        self.save_conflicts(conflicts).await?;

        let elapsed = start_time.elapsed();
        let summary = DqSummary {
            total_rows,
            success: success_count,
            blocked: violations
                .iter()
                .filter(|v| matches!(v.level, DqLevel::Error))
                .count(),
            warning: violations
                .iter()
                .filter(|v| matches!(v.level, DqLevel::Warning))
                .count(),
            conflict: violations
                .iter()
                .filter(|v| matches!(v.level, DqLevel::Conflict))
                .count(),
        };

        let batch = self
            .create_import_batch(
                batch_id.clone(),
                file_path,
                operator,
                &summary,
                elapsed.as_millis() as i32,
            )
            .await?;

        Ok(ImportResult {
            batch,
            summary,
            violations,
            elapsed_time: elapsed,
        })
    }

    /// 解析CSV文件
    ///
    /// # 参数
    /// - file_path: CSV文件路径
    ///
    /// # 返回
    /// - Vec<RawMaterialRecord>: 原始记录列表
    ///
    /// # 字段映射(依据 Field_Mapping_Spec_v0.3)
    /// - 材料号 → material_id
    /// - 制造命令号 → manufacturing_order_id
    /// - 合同交货期 → due_date
    /// - 下道机组代码 → next_machine_code
    /// - 精整返修机组 → rework_machine_code
    /// - 材料实际宽度 → width_mm
    /// - 材料实际厚度 → thickness_mm
    /// - 材料实际长度 → length_m
    /// - 材料实际重量 → weight_t
    /// - 状态时间(天) → stock_age_days
    /// - 产出时间(天) → output_age_days_raw
    /// - 合同性质代码 → contract_nature
    /// - 按周交货标志 → weekly_delivery_flag
    /// - 出口标记 → export_flag
    fn parse_csv(&self, file_path: &str) -> Result<Vec<RawMaterialRecord>, Box<dyn Error>> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(file_path)?;

        let mut records = Vec::new();
        for (row_idx, result) in reader.records().enumerate() {
            let record = result?;
            let row_number = row_idx + 2; // +2 因为行号从1开始,且跳过header

            // 解析每个字段(按照Field_Mapping_Spec_v0.3)
            let raw = RawMaterialRecord {
                material_id: Self::get_string_field(&record, 0), // 材料号
                manufacturing_order_id: Self::get_string_field(&record, 1), // 制造命令号
                material_status_code_src: Self::get_string_field(&record, 2), // 材料状态码
                steel_mark: Self::get_string_field(&record, 7), // 出钢记号
                slab_id: Self::get_string_field(&record, 8), // 板坯号
                next_machine_code: Self::get_string_field(&record, 9), // 下道机组代码
                rework_machine_code: Self::get_string_field(&record, 10), // 精整返修机组
                width_mm: Self::get_f64_field(&record, 11), // 材料实际宽度
                thickness_mm: Self::get_f64_field(&record, 12), // 材料实际厚度
                length_m: Self::get_f64_field(&record, 13), // 材料实际长度
                weight_t: Self::get_f64_field(&record, 14), // 材料实际重量
                available_width_mm: Self::get_f64_field(&record, 15), // 可利用宽度
                due_date: Self::get_date_field(&record, 3), // 合同交货期
                stock_age_days: Self::get_i32_field(&record, 4), // 状态时间(天)
                output_age_days_raw: Self::get_i32_field(&record, 5), // 产出时间(天)
                status_updated_at: Self::get_datetime_field(&record, 6), // 物料状态修改时间
                contract_no: Self::get_string_field(&record, 16), // 合同号
                contract_nature: Self::get_string_field(&record, 17), // 合同性质代码
                weekly_delivery_flag: Self::get_string_field(&record, 18), // 按周交货标志
                export_flag: Self::get_string_field(&record, 19), // 出口标记
                row_number,
            };

            records.push(raw);
        }

        Ok(records)
    }

    /// 数据质量检查(依据 Field_Mapping_Spec_v0.3 - DQ Rules)
    ///
    /// # 参数
    /// - records: 原始记录列表
    /// - batch_id: 批次ID
    ///
    /// # 返回
    /// - (valid_records, violations, conflicts)
    ///
    /// # DQ规则
    /// 1. 主键唯一性: material_id 必须非空且唯一
    /// 2. 日期字段: due_date 非法值视为缺失
    /// 3. 数值字段: width/thickness/weight 必须 >0
    /// 4. 适温反推字段: output_age_days_raw 必须 >=0
    /// 5. 催料字段: contract_nature/weekly_delivery_flag/export_flag 缺失→WARNING
    fn validate_records(
        &self,
        records: Vec<RawMaterialRecord>,
        batch_id: &str,
    ) -> (
        Vec<RawMaterialRecord>,
        Vec<DqViolation>,
        Vec<ImportConflict>,
    ) {
        let mut valid_records = Vec::new();
        let mut violations = Vec::new();
        let mut conflicts = Vec::new();

        // 记录已见过的material_id(用于检测重复)
        let mut seen_ids = std::collections::HashSet::new();

        for record in records {
            let mut has_error = false;

            // === DQ规则1: 主键检查 ===
            let is_empty_key = match &record.material_id {
                None => true,
                Some(s) => s.trim().is_empty(),
            };

            if is_empty_key {
                violations.push(DqViolation {
                    row_number: record.row_number,
                    material_id: None,
                    level: DqLevel::Error,
                    field: "material_id".to_string(),
                    message: "主键缺失".to_string(),
                });
                conflicts.push(ImportConflict {
                    conflict_id: Uuid::new_v4().to_string(),
                    batch_id: batch_id.to_string(),
                    row_number: record.row_number,
                    material_id: None,
                    conflict_type: ConflictType::PrimaryKeyMissing,
                    raw_data: format!("row_{}", record.row_number),
                    reason: "material_id缺失".to_string(),
                    resolved: false,
                    created_at: Utc::now(),
                });
                has_error = true;
            } else if let Some(id) = &record.material_id {
                if seen_ids.contains(id) {
                    violations.push(DqViolation {
                        row_number: record.row_number,
                        material_id: Some(id.clone()),
                        level: DqLevel::Conflict,
                        field: "material_id".to_string(),
                        message: format!("主键重复: {}", id),
                    });
                    conflicts.push(ImportConflict {
                        conflict_id: Uuid::new_v4().to_string(),
                        batch_id: batch_id.to_string(),
                        row_number: record.row_number,
                        material_id: Some(id.clone()),
                        conflict_type: ConflictType::PrimaryKeyDuplicate,
                        raw_data: format!("material_id={}", id),
                        reason: format!("material_id重复: {}", id),
                        resolved: false,
                        created_at: Utc::now(),
                    });
                    has_error = true;
                } else {
                    seen_ids.insert(id.clone());
                }
            }

            // === DQ规则3: 数值字段检查 ===
            if let Some(width) = record.width_mm {
                if width <= 0.0 {
                    violations.push(DqViolation {
                        row_number: record.row_number,
                        material_id: record.material_id.clone(),
                        level: DqLevel::Warning,
                        field: "width_mm".to_string(),
                        message: format!("宽度异常: {} <= 0", width),
                    });
                }
            }

            if let Some(thickness) = record.thickness_mm {
                if thickness <= 0.0 {
                    violations.push(DqViolation {
                        row_number: record.row_number,
                        material_id: record.material_id.clone(),
                        level: DqLevel::Warning,
                        field: "thickness_mm".to_string(),
                        message: format!("厚度异常: {} <= 0", thickness),
                    });
                }
            }

            if let Some(weight) = record.weight_t {
                if weight <= 0.0 {
                    violations.push(DqViolation {
                        row_number: record.row_number,
                        material_id: record.material_id.clone(),
                        level: DqLevel::Error,
                        field: "weight_t".to_string(),
                        message: format!("重量异常: {} <= 0", weight),
                    });
                    has_error = true;
                }
            }

            // === DQ规则4: 适温反推字段检查 ===
            match record.output_age_days_raw {
                None => {
                    violations.push(DqViolation {
                        row_number: record.row_number,
                        material_id: record.material_id.clone(),
                        level: DqLevel::Error,
                        field: "output_age_days_raw".to_string(),
                        message: "产出时间缺失".to_string(),
                    });
                    has_error = true;
                }
                Some(days) if days < 0 => {
                    violations.push(DqViolation {
                        row_number: record.row_number,
                        material_id: record.material_id.clone(),
                        level: DqLevel::Error,
                        field: "output_age_days_raw".to_string(),
                        message: format!("产出时间非法: {} < 0", days),
                    });
                    has_error = true;
                }
                _ => {}
            }

            // === DQ规则5: 催料字段检查(WARNING) ===
            if record.contract_nature.is_none() {
                violations.push(DqViolation {
                    row_number: record.row_number,
                    material_id: record.material_id.clone(),
                    level: DqLevel::Warning,
                    field: "contract_nature".to_string(),
                    message: "合同性质代码缺失".to_string(),
                });
            }

            if record.weekly_delivery_flag.is_none() {
                violations.push(DqViolation {
                    row_number: record.row_number,
                    material_id: record.material_id.clone(),
                    level: DqLevel::Warning,
                    field: "weekly_delivery_flag".to_string(),
                    message: "按周交货标志缺失".to_string(),
                });
            }

            if record.export_flag.is_none() {
                violations.push(DqViolation {
                    row_number: record.row_number,
                    material_id: record.material_id.clone(),
                    level: DqLevel::Warning,
                    field: "export_flag".to_string(),
                    message: "出口标记缺失".to_string(),
                });
            }

            // 如果没有ERROR级别违规,则加入有效记录列表
            if !has_error {
                valid_records.push(record);
            }
        }

        (valid_records, violations, conflicts)
    }

    /// 字段映射: RawMaterialRecord → MaterialMaster
    ///
    /// # 派生规则(依据 Field_Mapping_Spec_v0.3)
    /// - current_machine_code = COALESCE(rework_machine_code, next_machine_code)
    fn map_to_material_master(
        &self,
        records: Vec<RawMaterialRecord>,
    ) -> Result<Vec<MaterialMaster>, Box<dyn Error>> {
        let mut masters = Vec::new();
        let now = Utc::now();

        for record in records {
            // 派生 current_machine_code
            let current_machine_code = record
                .rework_machine_code
                .clone()
                .or(record.next_machine_code.clone());

            let master = MaterialMaster {
                material_id: record.material_id.unwrap_or_default(),
                manufacturing_order_id: record.manufacturing_order_id,
                material_status_code_src: record.material_status_code_src,
                steel_mark: record.steel_mark,
                slab_id: record.slab_id,
                next_machine_code: record.next_machine_code,
                rework_machine_code: record.rework_machine_code,
                current_machine_code,
                width_mm: record.width_mm,
                thickness_mm: record.thickness_mm,
                length_m: record.length_m,
                weight_t: record.weight_t,
                available_width_mm: record.available_width_mm,
                due_date: record.due_date,
                stock_age_days: record.stock_age_days,
                output_age_days_raw: record.output_age_days_raw,
                status_updated_at: record.status_updated_at,
                contract_no: record.contract_no,
                contract_nature: record.contract_nature,
                weekly_delivery_flag: record.weekly_delivery_flag,
                export_flag: record.export_flag,
                created_at: now,
                updated_at: now,
            };

            masters.push(master);
        }

        Ok(masters)
    }

    /// 状态派生: MaterialMaster → MaterialState
    ///
    /// # 依赖
    /// - MaterialStateDerivationService
    async fn derive_material_states(
        &self,
        masters: &[MaterialMaster],
        today: NaiveDate,
    ) -> Result<Vec<MaterialState>, Box<dyn Error>> {
        let mut states = Vec::new();

        for master in masters {
            let state = self
                .state_service
                .derive(master, self.config.as_ref(), today)
                .await?;
            states.push(state);
        }

        Ok(states)
    }

    /// 批量保存材料主数据和状态(事务化)
    async fn save_materials_and_states(
        &self,
        masters: Vec<MaterialMaster>,
        states: Vec<MaterialState>,
    ) -> Result<usize, Box<dyn Error>> {
        // 保存材料主数据
        let count = self.repo.batch_insert_material_master(masters).await?;

        // 保存材料状态
        self.repo.batch_insert_material_state(states).await?;

        Ok(count)
    }

    /// 保存冲突记录
    async fn save_conflicts(
        &self,
        conflicts: Vec<ImportConflict>,
    ) -> Result<(), Box<dyn Error>> {
        if !conflicts.is_empty() {
            self.repo.batch_insert_conflicts(conflicts).await?;
        }
        Ok(())
    }

    /// 创建导入批次记录
    async fn create_import_batch(
        &self,
        batch_id: String,
        file_path: &str,
        operator: &str,
        summary: &DqSummary,
        elapsed_ms: i32,
    ) -> Result<ImportBatch, Box<dyn Error>> {
        let dq_report = DqReport {
            batch_id: batch_id.clone(),
            summary: summary.clone(),
            violations: vec![], // violations已在ImportResult中返回
        };

        let batch = ImportBatch {
            batch_id: batch_id.clone(),
            file_name: Some(
                std::path::Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            ),
            file_path: Some(file_path.to_string()),
            total_rows: summary.total_rows as i32,
            success_rows: summary.success as i32,
            blocked_rows: summary.blocked as i32,
            warning_rows: summary.warning as i32,
            conflict_rows: summary.conflict as i32,
            imported_at: Some(Utc::now()),
            imported_by: Some(operator.to_string()),
            elapsed_ms: Some(elapsed_ms),
            dq_report_json: Some(serde_json::to_string(&dq_report)?),
        };

        self.repo.insert_batch(batch.clone()).await?;

        Ok(batch)
    }

    // ==========================================
    // 辅助方法: CSV字段解析
    // ==========================================

    fn get_string_field(record: &csv::StringRecord, index: usize) -> Option<String> {
        record
            .get(index)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn get_f64_field(record: &csv::StringRecord, index: usize) -> Option<f64> {
        record
            .get(index)
            .and_then(|s| s.trim().parse::<f64>().ok())
    }

    fn get_i32_field(record: &csv::StringRecord, index: usize) -> Option<i32> {
        record
            .get(index)
            .and_then(|s| s.trim().parse::<i32>().ok())
    }

    fn get_date_field(record: &csv::StringRecord, index: usize) -> Option<NaiveDate> {
        record.get(index).and_then(|s| {
            NaiveDate::parse_from_str(s.trim(), "%Y%m%d")
                .or_else(|_| NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d"))
                .ok()
        })
    }

    fn get_datetime_field(
        record: &csv::StringRecord,
        index: usize,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        record.get(index).and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s.trim())
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        })
    }
}

// ==========================================
// 测试辅助
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_field_helpers() {
        let record = csv::StringRecord::from(vec![
            "MAT001",
            "123.45",
            "100",
            "20260118",
            "2026-01-18T00:00:00Z",
            "",
        ]);

        assert_eq!(
            MaterialImporter::<DummyRepo, DummyConfig>::get_string_field(&record, 0),
            Some("MAT001".to_string())
        );
        assert_eq!(
            MaterialImporter::<DummyRepo, DummyConfig>::get_f64_field(&record, 1),
            Some(123.45)
        );
        assert_eq!(
            MaterialImporter::<DummyRepo, DummyConfig>::get_i32_field(&record, 2),
            Some(100)
        );
        assert_eq!(
            MaterialImporter::<DummyRepo, DummyConfig>::get_string_field(&record, 5),
            None
        ); // 空字段
    }

    // 测试用 Dummy 实现
    struct DummyRepo;
    #[async_trait::async_trait]
    impl MaterialImportRepository for DummyRepo {
        async fn batch_insert_material_master(
            &self,
            _materials: Vec<MaterialMaster>,
        ) -> Result<usize, Box<dyn Error>> {
            Ok(0)
        }
        async fn batch_insert_material_state(
            &self,
            _states: Vec<MaterialState>,
        ) -> Result<usize, Box<dyn Error>> {
            Ok(0)
        }
        async fn insert_conflict(
            &self,
            _conflict: ImportConflict,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
        async fn batch_insert_conflicts(
            &self,
            _conflicts: Vec<ImportConflict>,
        ) -> Result<usize, Box<dyn Error>> {
            Ok(0)
        }
        async fn get_conflicts_by_batch(
            &self,
            _batch_id: &str,
        ) -> Result<Vec<ImportConflict>, Box<dyn Error>> {
            Ok(vec![])
        }
        async fn get_conflicts_by_material_id(
            &self,
            _material_id: &str,
        ) -> Result<Vec<ImportConflict>, Box<dyn Error>> {
            Ok(vec![])
        }
        async fn mark_conflict_resolved(
            &self,
            _conflict_id: &str,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
        async fn insert_batch(&self, _batch: ImportBatch) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
        async fn get_recent_batches(
            &self,
            _limit: usize,
        ) -> Result<Vec<ImportBatch>, Box<dyn Error>> {
            Ok(vec![])
        }
        async fn exists_material(&self, _material_id: &str) -> Result<bool, Box<dyn Error>> {
            Ok(false)
        }
        async fn batch_check_exists(
            &self,
            _material_ids: Vec<String>,
        ) -> Result<Vec<String>, Box<dyn Error>> {
            Ok(vec![])
        }
        async fn count_materials(&self) -> Result<usize, Box<dyn Error>> {
            Ok(0)
        }
        async fn count_states(&self) -> Result<usize, Box<dyn Error>> {
            Ok(0)
        }
        async fn list_conflicts_with_filter(
            &self,
            _status: Option<&str>,
            _limit: i32,
            _offset: i32,
        ) -> Result<Vec<ImportConflict>, Box<dyn Error>> {
            Ok(vec![])
        }
        async fn count_conflicts_by_status(
            &self,
            _status: Option<&str>,
        ) -> Result<i64, Box<dyn Error>> {
            Ok(0)
        }
        async fn get_conflict_by_id(
            &self,
            _conflict_id: &str,
        ) -> Result<Option<ImportConflict>, Box<dyn Error>> {
            Ok(None)
        }
        async fn resolve_conflict(
            &self,
            _conflict_id: &str,
            _action: &str,
            _note: Option<&str>,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
        async fn count_conflicts_by_batch(
            &self,
            _batch_id: &str,
            _status: Option<&str>,
        ) -> Result<i64, Box<dyn Error>> {
            Ok(0)
        }
        async fn delete_materials_by_batch(
            &self,
            _batch_id: &str,
        ) -> Result<usize, Box<dyn Error>> {
            Ok(0)
        }
        async fn delete_conflicts_by_batch(
            &self,
            _batch_id: &str,
        ) -> Result<usize, Box<dyn Error>> {
            Ok(0)
        }
        async fn delete_batch(
            &self,
            _batch_id: &str,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    struct DummyConfig;
    #[async_trait::async_trait]
    impl ImportConfigReader for DummyConfig {
        async fn get_season_mode(&self) -> Result<crate::domain::types::SeasonMode, Box<dyn Error>> {
            Ok(crate::domain::types::SeasonMode::Auto)
        }
        async fn get_manual_season(&self) -> Result<crate::domain::types::Season, Box<dyn Error>> {
            Ok(crate::domain::types::Season::Winter)
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
        async fn get_standard_finishing_machines(&self) -> Result<Vec<String>, Box<dyn Error>> {
            Ok(vec!["H032".to_string(), "H033".to_string(), "H034".to_string()])
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
        async fn get_current_min_temp_days(&self, _today: chrono::NaiveDate) -> Result<i32, Box<dyn Error>> {
            Ok(3)
        }
        async fn get_n1_threshold_days(&self) -> Result<i32, Box<dyn Error>> {
            Ok(7)
        }
        async fn get_n2_threshold_days(&self) -> Result<i32, Box<dyn Error>> {
            Ok(3)
        }
    }
}

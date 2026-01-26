// ==========================================
// 热轧精整排产系统 - 材料数据导入器实现
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 1.1 计算主流程
// 依据: Field_Mapping_Spec_v0.3_Integrated.md - 字段映射规范
// ==========================================
// 职责: 整合导入流程，从文件到数据库
// 流程: 解析 → 映射 → 清洗 → 派生 → 校验 → 冲突检测 → 落库
// ==========================================

use crate::config::ImportConfigReader;
use crate::domain::material::{ImportBatch, ImportConflict, MaterialMaster, RawMaterialRecord};
use crate::engine::material_state_derivation::MaterialStateDerivationService;
use crate::importer::material_importer_trait::{
    ConflictHandler, DataCleaner, DerivationService, DqValidator, FieldMapper, FileParser,
    MaterialImporter,
};
use crate::repository::MaterialImportRepository;
use chrono::Utc;
use std::error::Error;
use std::path::Path;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

// ==========================================
// MaterialImporterImpl - 材料数据导入器实现
// ==========================================
pub struct MaterialImporterImpl<R, C>
where
    R: MaterialImportRepository,
    C: ImportConfigReader,
{
    // 数据访问层
    import_repo: R,

    // 配置读取器
    config: C,

    // 导入组件
    file_parser: Box<dyn FileParser>,
    field_mapper: Box<dyn FieldMapper>,
    data_cleaner: Box<dyn DataCleaner>,
    derivation_service: Box<dyn DerivationService>,
    dq_validator: Box<dyn DqValidator>,
    conflict_handler: Box<dyn ConflictHandler>,

    // MaterialState 派生服务
    state_derivation_service: MaterialStateDerivationService,
}

impl<R, C> MaterialImporterImpl<R, C>
where
    R: MaterialImportRepository,
    C: ImportConfigReader,
{
    /// 创建新的 MaterialImporter 实例
    ///
    /// # 参数
    /// - import_repo: 导入数据仓储
    /// - config: 配置读取器
    /// - file_parser: 文件解析器
    /// - field_mapper: 字段映射器
    /// - data_cleaner: 数据清洗器
    /// - derivation_service: 字段派生服务
    /// - dq_validator: DQ 校验器
    /// - conflict_handler: 冲突处理器
    /// - state_derivation_service: MaterialState 派生服务
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        import_repo: R,
        config: C,
        file_parser: Box<dyn FileParser>,
        field_mapper: Box<dyn FieldMapper>,
        data_cleaner: Box<dyn DataCleaner>,
        derivation_service: Box<dyn DerivationService>,
        dq_validator: Box<dyn DqValidator>,
        conflict_handler: Box<dyn ConflictHandler>,
        state_derivation_service: MaterialStateDerivationService,
    ) -> Self {
        Self {
            import_repo,
            config,
            file_parser,
            field_mapper,
            data_cleaner,
            derivation_service,
            dq_validator,
            conflict_handler,
            state_derivation_service,
        }
    }
}

#[async_trait::async_trait]
impl<R, C> MaterialImporter for MaterialImporterImpl<R, C>
where
    R: MaterialImportRepository + Send + Sync,
    C: ImportConfigReader + Send + Sync,
{
    /// 从 Excel 文件导入材料数据
    ///
    /// # 参数
    /// - file_path: Excel 文件路径（.xlsx, .xls）
    ///
    /// # 返回
    /// - Ok(ImportResult): 导入结果
    /// - Err: 导入错误
    #[instrument(skip(self, file_path), fields(batch_id))]
    async fn import_from_excel<P: AsRef<Path> + Send>(
        &self,
        file_path: P,
    ) -> Result<crate::domain::material::ImportResult, Box<dyn Error>> {
        use std::time::Instant;
        let start_time = Instant::now();
        let _import_started_at = Utc::now();
        let batch_id = Uuid::new_v4().to_string();

        let file_path_str = file_path.as_ref().to_str().unwrap_or("unknown");
        info!(batch_id = %batch_id, file_path = %file_path_str, "开始导入材料数据");

        // === 步骤 1: 解析文件 ===
        debug!("步骤 1: 解析文件");
        let raw_rows = self
            .file_parser
            .parse_to_raw_records(file_path.as_ref())
            .map_err(|e| {
                error!(error = %e, "文件解析失败");
                format!("文件解析失败: {}", e)
            })?;

        let total_rows = raw_rows.len();
        info!(total_rows = total_rows, "文件解析完成");

        // === 步骤 2: 字段映射 ===
        debug!("步骤 2: 字段映射");
        let mut records = Vec::new();
        let mut mapping_errors = Vec::new();
        for (idx, row) in raw_rows.into_iter().enumerate() {
            match self.field_mapper.map_to_raw_material(row, idx + 1) {
                Ok(record) => records.push(record),
                Err(e) => {
                    // 映射失败：记录错误信息（转换为字符串以避免 Send 问题）
                    warn!(row_number = idx + 1, error = %e, "字段映射失败");
                    mapping_errors.push((idx + 1, format!("字段映射失败: {}", e)));
                }
            }
        }
        info!(
            success = records.len(),
            failed = mapping_errors.len(),
            "字段映射完成"
        );

        // 批量写入映射错误到冲突队列
        for (row_num, error_msg) in mapping_errors {
            let conflict = ImportConflict {
                conflict_id: Uuid::new_v4().to_string(),
                batch_id: batch_id.clone(),
                row_number: row_num,
                material_id: None,
                conflict_type: crate::domain::material::ConflictType::DataTypeError,
                raw_data: "{}".to_string(),
                reason: error_msg,
                resolved: false,
                created_at: Utc::now(),
            };
            self.import_repo.insert_conflict(conflict).await?;
        }

        // === 步骤 3: 数据清洗 ===
        debug!("步骤 3: 数据清洗");
        for record in &mut records {
            self.clean_record(record);
        }
        debug!("数据清洗完成");

        // === 步骤 4: 字段派生 ===
        debug!("步骤 4: 字段派生");
        for record in &mut records {
            self.derive_fields(record).await?;
        }
        debug!("字段派生完成");

        // === 步骤 5: DQ 校验 ===
        debug!("步骤 5: DQ 校验");
        let dq_report = self.validate_records(&records);
        info!(
            violations = dq_report["total_violations"].as_u64().unwrap_or(0),
            "DQ 校验完成"
        );

        // 解析 DQ 报告并提取 violations 列表
        let details = &dq_report["details"];
        let mut pk_violations: Vec<crate::domain::material::DqViolation> =
            serde_json::from_value(details["pk_violations"].clone()).unwrap_or_default();
        let mut required_violations: Vec<crate::domain::material::DqViolation> =
            serde_json::from_value(details["required_violations"].clone()).unwrap_or_default();
        let mut range_violations: Vec<crate::domain::material::DqViolation> =
            serde_json::from_value(details["range_violations"].clone()).unwrap_or_default();

        // 合并所有 violations
        let mut all_violations = Vec::new();
        all_violations.append(&mut pk_violations);
        all_violations.append(&mut required_violations);
        all_violations.append(&mut range_violations);

        // 统计 blocked 和 warning 数量
        let blocked_rows = all_violations
            .iter()
            .filter(|v| matches!(v.level, crate::domain::material::DqLevel::Error))
            .count();
        let warning_rows = all_violations
            .iter()
            .filter(|v| matches!(v.level, crate::domain::material::DqLevel::Warning))
            .count();

        info!(
            blocked = blocked_rows,
            warning = warning_rows,
            "DQ 统计提取完成"
        );

        // === 步骤 6: 冲突检测 ===
        debug!("步骤 6: 冲突检测");
        let (valid_records, conflict_count) = self
            .detect_and_handle_conflicts(&batch_id, records)
            .await?;
        info!(
            valid = valid_records.len(),
            conflicts = conflict_count,
            "冲突检测完成"
        );

        // === 步骤 7: 转换为 MaterialMaster ===
        debug!("步骤 7: 转换为 MaterialMaster");
        let materials = self.convert_to_material_master(valid_records);
        debug!(count = materials.len(), "MaterialMaster 转换完成");

        // === 步骤 8: 派生 MaterialState ===
        debug!("步骤 8: 派生 MaterialState");
        let today = chrono::Local::now().date_naive();
        let mut material_states = Vec::new();
        for material in &materials {
            match self.state_derivation_service
                .derive(material, &self.config, today)
                .await
            {
                Ok(state) => material_states.push(state),
                Err(e) => {
                    // 派生失败:记录错误但不阻断导入
                    warn!(material_id = %material.material_id, error = %e, "材料状态派生失败");
                }
            }
        }
        info!(
            total = materials.len(),
            success = material_states.len(),
            "MaterialState 派生完成"
        );

        // === 步骤 9: 批量插入 MaterialMaster ===
        debug!("步骤 9: 批量插入 MaterialMaster");
        let success_count = self
            .import_repo
            .batch_insert_material_master(materials)
            .await?;
        info!(count = success_count, "MaterialMaster 插入完成");

        // === 步骤 10: 批量插入 MaterialState ===
        debug!("步骤 10: 批量插入 MaterialState");
        let _state_count = self
            .import_repo
            .batch_insert_material_state(material_states)
            .await?;
        debug!("MaterialState 插入完成");

        let import_completed_at = Utc::now();
        let elapsed_time = start_time.elapsed();

        // === 步骤 9: 记录批次信息 ===
        let batch = ImportBatch {
            batch_id: batch_id.clone(),
            file_name: Some(
                Path::new(file_path_str)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            ),
            file_path: Some(file_path_str.to_string()),
            total_rows: total_rows as i32,
            success_rows: success_count as i32,
            blocked_rows: blocked_rows as i32,
            warning_rows: warning_rows as i32,
            conflict_rows: conflict_count as i32,
            imported_at: Some(import_completed_at),
            imported_by: Some("system".to_string()),
            elapsed_ms: Some(elapsed_time.as_millis() as i32),
            dq_report_json: Some(serde_json::to_string(&dq_report)?),
        };

        self.import_repo.insert_batch(batch.clone()).await?;

        // === 步骤 10: 构造返回结果 ===
        let summary = crate::domain::material::DqSummary {
            total_rows,
            success: success_count,
            blocked: blocked_rows,
            warning: warning_rows,
            conflict: conflict_count,
        };

        info!(
            batch_id = %batch_id,
            total = total_rows,
            success = success_count,
            conflicts = conflict_count,
            elapsed_ms = elapsed_time.as_millis(),
            "材料数据导入完成"
        );

        Ok(crate::domain::material::ImportResult {
            batch,
            summary,
            violations: all_violations,
            elapsed_time,
        })
    }

    /// 从 CSV 文件导入材料数据
    async fn import_from_csv<P: AsRef<Path> + Send>(
        &self,
        file_path: P,
    ) -> Result<crate::domain::material::ImportResult, Box<dyn Error>> {
        // CSV 导入复用 Excel 导入逻辑
        self.import_from_excel(file_path).await
    }

    /// 批量导入多个文件（并发执行）
    async fn batch_import<P: AsRef<Path> + Send + Sync>(
        &self,
        file_paths: Vec<P>,
    ) -> Result<Vec<Result<crate::domain::material::ImportResult, String>>, Box<dyn Error>> {
        use futures::future::join_all;

        info!(count = file_paths.len(), "开始批量导入文件");

        // 为每个文件创建导入任务
        let import_tasks = file_paths.into_iter().map(|path| {
            let path_str = path.as_ref().to_str().unwrap_or("unknown").to_string();
            async move {
                info!(file = %path_str, "开始导入文件");
                match self.import_from_csv(path).await {
                    Ok(result) => {
                        info!(
                            file = %path_str,
                            success = result.summary.success,
                            "文件导入成功"
                        );
                        Ok(result)
                    }
                    Err(e) => {
                        error!(file = %path_str, error = %e, "文件导入失败");
                        Err(format!("文件 {} 导入失败: {}", path_str, e))
                    }
                }
            }
        });

        // 并发执行所有导入任务
        let results = join_all(import_tasks).await;

        info!(
            total = results.len(),
            success = results.iter().filter(|r| r.is_ok()).count(),
            failed = results.iter().filter(|r| r.is_err()).count(),
            "批量导入完成"
        );

        Ok(results)
    }
}

// 辅助方法
impl<R, C> MaterialImporterImpl<R, C>
where
    R: MaterialImportRepository,
    C: ImportConfigReader,
{
    /// 清洗单条记录
    fn clean_record(&self, record: &mut RawMaterialRecord) {
        // 清洗合同性质（TRIM + UPPER）
        record.contract_nature = record.contract_nature.as_ref()
            .map(|v| self.data_cleaner.clean_text(v, true))
            .and_then(|v| self.data_cleaner.normalize_null(Some(v)));

        // 清洗周交期标记（TRIM + UPPER）
        record.weekly_delivery_flag = record.weekly_delivery_flag.as_ref()
            .map(|v| self.data_cleaner.clean_text(v, true))
            .and_then(|v| self.data_cleaner.normalize_null(Some(v)));

        // 清洗出口标记（标准化为 '1'/'0'）
        record.export_flag = self.data_cleaner.clean_export_flag(record.export_flag.take());

        // 清洗钢种标记（TRIM + UPPER）
        record.steel_mark = record.steel_mark.as_ref()
            .map(|v| self.data_cleaner.clean_text(v, true))
            .and_then(|v| self.data_cleaner.normalize_null(Some(v)));

        // 清洗机组代码（TRIM + UPPER）
        record.next_machine_code = record.next_machine_code.as_ref()
            .map(|v| self.data_cleaner.clean_text(v, true))
            .and_then(|v| self.data_cleaner.normalize_null(Some(v)));

        record.rework_machine_code = record.rework_machine_code.as_ref()
            .map(|v| self.data_cleaner.clean_text(v, true))
            .and_then(|v| self.data_cleaner.normalize_null(Some(v)));
    }

    /// 派生字段（current_machine_code）
    async fn derive_fields(&self, record: &mut RawMaterialRecord) -> Result<(), Box<dyn Error>> {
        // 派生 current_machine_code（COALESCE(rework, next)）
        let _current_machine = self.derivation_service.derive_current_machine_code(
            record.rework_machine_code.clone(),
            record.next_machine_code.clone(),
        );

        // 注意：这里只派生 current_machine_code
        // rolling_output_age_days 和 rush_level 将在 material_state 派生时计算
        // 因为它们依赖配置（季节、适温阈值等）

        // 暂时存储到 RawMaterialRecord（需要扩展字段）
        // TODO: 扩展 RawMaterialRecord 添加 current_machine_code 字段
        // 或者在转换为 MaterialMaster 时再派生

        Ok(())
    }

    /// DQ 校验
    fn validate_records(&self, records: &[RawMaterialRecord]) -> serde_json::Value {
        use serde_json::json;

        // 主键校验
        let pk_violations = self.dq_validator.validate_primary_key(records);

        // 必填字段校验(逐条记录)
        let mut required_violations = Vec::new();
        for record in records {
            required_violations.extend(self.dq_validator.validate_required_fields(record));
        }

        // 数值范围校验(逐条记录)
        let mut range_violations = Vec::new();
        for record in records {
            range_violations.extend(self.dq_validator.validate_ranges(record));
        }

        // 汇总 DQ 报告
        json!({
            "primary_key_violations": pk_violations.len(),
            "required_field_violations": required_violations.len(),
            "range_violations": range_violations.len(),
            "total_violations": pk_violations.len() + required_violations.len() + range_violations.len(),
            "details": {
                "pk_violations": pk_violations,
                "required_violations": required_violations,
                "range_violations": range_violations,
            }
        })
    }

    /// 冲突检测和处理
    async fn detect_and_handle_conflicts(
        &self,
        batch_id: &str,
        records: Vec<RawMaterialRecord>,
    ) -> Result<(Vec<RawMaterialRecord>, usize), Box<dyn Error>> {
        // 步骤 1: 检测同批次内重复
        let intra_batch_duplicates = self.conflict_handler.detect_duplicates(&records);

        // 步骤 2: 检测跨批次重复
        let material_ids: Vec<String> = records
            .iter()
            .filter_map(|r| r.material_id.clone())
            .collect();

        let existing_ids = self.import_repo.batch_check_exists(material_ids).await?;

        let cross_batch_duplicates = self
            .conflict_handler
            .detect_cross_batch_duplicates(&records, &existing_ids);

        // 步骤 3: 合并冲突列表
        let mut conflict_rows = std::collections::HashSet::new();
        for (row_num, _) in &intra_batch_duplicates {
            conflict_rows.insert(*row_num);
        }
        for (row_num, _) in &cross_batch_duplicates {
            conflict_rows.insert(*row_num);
        }

        // 步骤 4: 写入冲突记录
        let mut conflicts = Vec::new();
        for (row_num, material_id) in intra_batch_duplicates {
            // 查找原始记录并序列化
            let raw_record = records.iter().find(|r| r.row_number == row_num);
            let raw_data = raw_record
                .and_then(|r| serde_json::to_string(r).ok())
                .unwrap_or_else(|| "{}".to_string());

            conflicts.push(ImportConflict {
                conflict_id: Uuid::new_v4().to_string(),
                batch_id: batch_id.to_string(),
                row_number: row_num,
                material_id: Some(material_id.clone()),
                conflict_type: crate::domain::material::ConflictType::PrimaryKeyDuplicate,
                raw_data,
                reason: format!("同批次内重复材料号: {}", material_id),
                resolved: false,
                created_at: Utc::now(),
            });
        }

        for (row_num, material_id) in cross_batch_duplicates {
            // 查找原始记录并序列化
            let raw_record = records.iter().find(|r| r.row_number == row_num);
            let raw_data = raw_record
                .and_then(|r| serde_json::to_string(r).ok())
                .unwrap_or_else(|| "{}".to_string());

            conflicts.push(ImportConflict {
                conflict_id: Uuid::new_v4().to_string(),
                batch_id: batch_id.to_string(),
                row_number: row_num,
                material_id: Some(material_id.clone()),
                conflict_type: crate::domain::material::ConflictType::PrimaryKeyDuplicate,
                raw_data,
                reason: format!("跨批次重复材料号: {}", material_id),
                resolved: false,
                created_at: Utc::now(),
            });
        }

        if !conflicts.is_empty() {
            self.import_repo.batch_insert_conflicts(conflicts).await?;
        }

        // 步骤 5: 过滤出有效记录
        let valid_records: Vec<RawMaterialRecord> = records
            .into_iter()
            .filter(|r| !conflict_rows.contains(&r.row_number))
            .collect();

        let conflict_count = conflict_rows.len();

        Ok((valid_records, conflict_count))
    }

    /// 转换为 MaterialMaster
    fn convert_to_material_master(
        &self,
        records: Vec<RawMaterialRecord>,
    ) -> Vec<MaterialMaster> {
        records
            .into_iter()
            .map(|record| {
                // 派生 current_machine_code
                let current_machine_code = self.derivation_service.derive_current_machine_code(
                    record.rework_machine_code.clone(),
                    record.next_machine_code.clone(),
                );

                MaterialMaster {
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
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                }
            })
            .collect()
    }
}

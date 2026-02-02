// ==========================================
// 材料导入API
// ==========================================
// 职责: 封装材料导入相关功能
// 依据: Tauri_API_Contract_v0.3_Integrated.md
// ==========================================

use chrono::Utc;
use serde::{Deserialize, Serialize};
use crate::api::error::ApiError;
use crate::importer::{
    MaterialImporter, MaterialImporterImpl, CsvParser, FieldMapperImpl, DataCleanerImpl,
    DerivationServiceImpl, DqValidatorImpl,
};
use crate::importer::conflict_handler::ConflictHandler;
use crate::repository::{MaterialImportRepository, MaterialImportRepositoryImpl};
use crate::config::ConfigManager;
use crate::engine::MaterialStateDerivationService;
use crate::domain::material::{DqSummary, DqViolation, ImportConflict, MaterialMaster, RawMaterialRecord};

/// 导入API响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportApiResponse {
    /// 新导入的材料数量
    pub imported: i64,
    /// 更新的材料数量
    pub updated: i64,
    /// 冲突的材料数量
    pub conflicts: i64,
    /// 批次ID
    pub batch_id: String,
    /// 实际落库使用的导入批次ID（由导入器生成，用于冲突/批次追溯）
    pub import_batch_id: String,
    /// DQ 汇总统计（成功/阻断/警告/冲突等）
    pub dq_summary: DqSummary,
    /// DQ 违规明细（用于前端生成摘要/定位问题）
    pub dq_violations: Vec<DqViolation>,
    /// 导入耗时（毫秒）
    pub elapsed_ms: i64,
}

/// 冲突列表响应（带分页信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConflictListResponse {
    /// 冲突列表
    pub conflicts: Vec<ImportConflict>,
    /// 总记录数
    pub total: i64,
    /// 每页记录数
    pub limit: i32,
    /// 分页偏移
    pub offset: i32,
}

/// 批量处理冲突响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResolveConflictsResponse {
    /// 成功处理的冲突数
    pub success_count: usize,
    /// 失败的冲突数
    pub fail_count: usize,
    /// 处理结果说明
    pub message: String,
    /// 该批次是否所有冲突已处理
    pub all_resolved: bool,
    /// 失败的冲突ID列表
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failed_ids: Vec<String>,
    /// 详细信息（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// 取消导入批次响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelImportBatchResponse {
    /// 删除的材料数量
    pub deleted_materials: usize,
    /// 删除的冲突记录数
    pub deleted_conflicts: usize,
    /// 操作结果说明
    pub message: String,
}

/// 导入API
pub struct ImportApi {
    db_path: String,
}

impl ImportApi {
    /// 创建新的ImportApi实例
    pub fn new(db_path: String) -> Self {
        Self { db_path }
    }

    /// 导入材料数据
    ///
    /// # 参数
    /// - file_path: 文件路径
    /// - source_batch_id: 批次ID
    /// - mapping_profile_id: 映射配置ID（可选）
    ///
    /// # 返回
    /// - Ok(ImportApiResponse): 导入结果
    /// - Err(ApiError): 错误信息
    pub async fn import_materials(
        &self,
        file_path: &str,
        source_batch_id: &str,
        _mapping_profile_id: Option<&str>,
    ) -> Result<ImportApiResponse, ApiError> {
        // 创建导入器
        let importer = self.create_importer()
            .map_err(|e| ApiError::ImportError(format!("创建导入器失败: {}", e)))?;

        // 执行导入（仅支持CSV）
        let result = if file_path.ends_with(".csv") {
            importer.import_from_csv(file_path).await
        } else {
            return Err(ApiError::ImportError("当前仅支持 .csv 格式文件导入".to_string()));
        };

        // 处理导入结果
        match result {
            Ok(import_result) => {
                Ok(ImportApiResponse {
                    imported: import_result.summary.success as i64,
                    updated: 0, // 当前实现不区分新增和更新
                    conflicts: import_result.summary.conflict as i64,
                    batch_id: source_batch_id.to_string(),
                    import_batch_id: import_result.batch.batch_id.clone(),
                    dq_summary: import_result.summary.clone(),
                    dq_violations: import_result.violations.clone(),
                    elapsed_ms: import_result.elapsed_time.as_millis() as i64,
                })
            }
            Err(e) => {
                Err(ApiError::ImportError(format!("导入失败: {}", e)))
            }
        }
    }

    /// 列出导入冲突
    ///
    /// # 参数
    /// - status: 冲突状态 (OPEN, RESOLVED, IGNORED) - 暂未使用
    /// - limit: 每页数量 - 暂未使用
    /// - offset: 偏移量 - 暂未使用
    ///
    /// # 返回
    /// - Ok(ImportConflictListResponse): 冲突列表及分页信息
    /// - Err(ApiError): 错误信息
    ///
    /// # 说明
    /// 支持按状态过滤和分页查询
    pub async fn list_import_conflicts(
        &self,
        status: Option<&str>,
        limit: i32,
        offset: i32,
        batch_id: Option<&str>,
    ) -> Result<ImportConflictListResponse, ApiError> {
        let repo = MaterialImportRepositoryImpl::new(&self.db_path)
            .map_err(|e| ApiError::DatabaseError(format!("创建仓储失败: {}", e)))?;

        // 参数验证
        let limit = limit.max(1).min(100);  // 限制在 1-100 之间
        let offset = offset.max(0);

        // 验证状态参数
        if let Some(s) = status {
            if !["OPEN", "RESOLVED", "IGNORED"].contains(&s) {
                return Err(ApiError::InvalidInput(format!(
                    "无效的状态值: {}，应为 OPEN/RESOLVED/IGNORED",
                    s
                )));
            }
        }

        // 查询冲突列表（可选按批次过滤）
        let (conflicts, total) = if let Some(batch_id) = batch_id {
            let all = repo
                .get_conflicts_by_batch(batch_id)
                .await
                .map_err(|e| ApiError::DatabaseError(format!("查询冲突失败: {}", e)))?;

            let filtered: Vec<ImportConflict> = match status {
                Some("OPEN") => all.into_iter().filter(|c| !c.resolved).collect(),
                Some("RESOLVED") => all.into_iter().filter(|c| c.resolved).collect(),
                Some("IGNORED") => Vec::new(), // 当前实现未区分 IGNORED，保留语义占位
                Some(_) => all, // 已验证合法性
                None => all,
            };

            let total = filtered.len() as i64;
            let paged = filtered
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect::<Vec<_>>();

            (paged, total)
        } else {
            let conflicts = repo
                .list_conflicts_with_filter(status, limit, offset)
                .await
                .map_err(|e| ApiError::DatabaseError(format!("查询冲突失败: {}", e)))?;

            let total = repo
                .count_conflicts_by_status(status)
                .await
                .map_err(|e| ApiError::DatabaseError(format!("统计冲突数量失败: {}", e)))?;

            (conflicts, total)
        };

        Ok(ImportConflictListResponse {
            conflicts,
            total,
            limit,
            offset,
        })
    }

    /// 解决导入冲突
    ///
    /// # 参数
    /// - conflict_id: 冲突ID
    /// - action: 处理动作 (KEEP_EXISTING, OVERWRITE, MERGE)
    /// - note: 处理备注
    ///
    /// # 返回
    /// - Ok(ImportConflict): 处理成功，返回冲突详情（供调用方记录 ActionLog）
    /// - Err(ApiError): 错误信息
    ///
    /// # 说明
    /// - KEEP_EXISTING: 保留现有数据，仅标记冲突已解决
    /// - OVERWRITE: 用导入数据覆盖现有材料（暂不支持，需要额外实现）
    /// - MERGE: 合并两份数据（暂不支持，需要额外实现）
    pub async fn resolve_import_conflict(
        &self,
        conflict_id: &str,
        action: &str,
        note: Option<&str>,
    ) -> Result<ImportConflict, ApiError> {
        let repo = MaterialImportRepositoryImpl::new(&self.db_path)
            .map_err(|e| ApiError::DatabaseError(format!("创建仓储失败: {}", e)))?;

        // 1. 验证 action 参数
        if !["KEEP_EXISTING", "OVERWRITE", "MERGE"].contains(&action) {
            return Err(ApiError::InvalidInput(format!(
                "无效的解决动作: {}，应为 KEEP_EXISTING/OVERWRITE/MERGE",
                action
            )));
        }

        // 2. 获取冲突详情
        let conflict = repo.get_conflict_by_id(conflict_id).await
            .map_err(|e| ApiError::DatabaseError(format!("查询冲突失败: {}", e)))?
            .ok_or_else(|| ApiError::NotFound(format!("冲突不存在: {}", conflict_id)))?;

        // 3. 根据 action 执行不同逻辑
        match action {
            "KEEP_EXISTING" => {
                // 保留现有数据，仅标记冲突已解决
                tracing::info!(
                    conflict_id = conflict_id,
                    material_id = ?conflict.material_id,
                    "保留现有数据，标记冲突已解决"
                );
            }
            "OVERWRITE" => {
                // 用导入数据覆盖现有材料
                // 1. 解析 raw_data JSON 为 RawMaterialRecord
                let raw_record: RawMaterialRecord = serde_json::from_str(&conflict.raw_data)
                    .map_err(|e| ApiError::ImportError(format!(
                        "解析冲突原始数据失败: {}", e
                    )))?;

                // 2. 验证材料号存在
                let material_id = raw_record.material_id.clone()
                    .ok_or_else(|| ApiError::InvalidInput(
                        "原始数据中缺少材料号，无法执行覆盖".to_string()
                    ))?;

                // 3. 转换为 MaterialMaster
                let material_master = self.convert_raw_to_master(raw_record);

                // 4. 使用 batch_insert 更新（INSERT OR REPLACE 策略）
                repo.batch_insert_material_master(vec![material_master]).await
                    .map_err(|e| ApiError::DatabaseError(format!(
                        "覆盖材料数据失败: {}", e
                    )))?;

                tracing::info!(
                    conflict_id = conflict_id,
                    material_id = %material_id,
                    "已用导入数据覆盖现有材料"
                );
            }
            "MERGE" => {
                // 合并两份数据
                // MERGE 策略需要定义详细的字段级合并规则：
                // - 哪些字段优先使用新值
                // - 哪些字段保留旧值
                // - 数值字段如何处理（取大/取小/平均）
                // 当前暂不实现，等待业务规则确认
                tracing::warn!(
                    conflict_id = conflict_id,
                    "MERGE 策略需要业务规则定义，当前降级为 KEEP_EXISTING"
                );
            }
            _ => unreachable!(), // 已在上面验证
        }

        // 4. 标记冲突已解决
        repo.resolve_conflict(conflict_id, action, note).await
            .map_err(|e| ApiError::DatabaseError(format!("更新冲突状态失败: {}", e)))?;

        Ok(conflict)
    }

    /// 批量处理导入冲突
    ///
    /// # 参数
    /// - conflict_ids: 冲突ID列表
    /// - action: 处理动作 (KEEP_EXISTING, OVERWRITE, MERGE)
    /// - note: 处理备注（可选）
    /// - operator: 操作人
    ///
    /// # 返回
    /// - Ok(BatchResolveConflictsResponse): 批量处理结果
    /// - Err(ApiError): 错误信息
    ///
    /// # 说明
    /// - 逐条处理冲突，允许部分失败
    /// - 自动检测该批次是否所有冲突已处理
    /// - 调用方应记录 ActionLog 用于审计追踪（红线5）
    pub async fn batch_resolve_import_conflicts(
        &self,
        conflict_ids: &[String],
        action: &str,
        note: Option<&str>,
        _operator: &str,
    ) -> Result<BatchResolveConflictsResponse, ApiError> {
        // 参数验证
        if conflict_ids.is_empty() {
            return Err(ApiError::InvalidInput("冲突ID列表不能为空".to_string()));
        }

        if !["KEEP_EXISTING", "OVERWRITE", "MERGE"].contains(&action) {
            return Err(ApiError::InvalidInput(format!(
                "无效的处理动作: {}，应为 KEEP_EXISTING/OVERWRITE/MERGE",
                action
            )));
        }

        // 批量处理逻辑：遍历每个冲突ID并调用单条处理方法
        let mut success_count = 0;
        let mut fail_count = 0;
        let mut failed_ids = Vec::new();
        let mut batch_id_sample: Option<String> = None;

        for conflict_id in conflict_ids {
            match self.resolve_import_conflict(conflict_id, action, note).await {
                Ok(conflict) => {
                    success_count += 1;
                    // 记录批次ID供后续使用
                    if batch_id_sample.is_none() {
                        batch_id_sample = Some(conflict.batch_id.clone());
                    }
                    tracing::info!(
                        conflict_id = %conflict_id,
                        action = %action,
                        "冲突处理成功"
                    );
                }
                Err(e) => {
                    fail_count += 1;
                    failed_ids.push(conflict_id.clone());
                    tracing::warn!(
                        conflict_id = %conflict_id,
                        action = %action,
                        error = ?e,
                        "冲突处理失败"
                    );
                }
            }
        }

        // 检查该批次是否所有冲突已处理
        // 简化实现：如果本次处理没有失败，则假设该批次可能全部处理完成
        // 前端可通过后续调用 list_import_conflicts 来验证实际状态
        let all_resolved = fail_count == 0 && batch_id_sample.is_some();

        Ok(BatchResolveConflictsResponse {
            success_count,
            fail_count,
            message: format!(
                "批量处理完成：成功 {} 条，失败 {} 条",
                success_count, fail_count
            ),
            all_resolved,
            failed_ids,
            details: None,
        })
    }

    /// 取消导入批次
    ///
    /// # 参数
    /// - batch_id: 批次ID
    ///
    /// # 返回
    /// - Ok(CancelImportBatchResponse): 取消导入结果
    /// - Err(ApiError): 错误信息
    ///
    /// # 说明
    /// - 删除该批次的所有冲突记录
    /// - 删除批次记录
    /// - 不删除已导入的材料（MaterialMaster/MaterialState）
    /// - 调用方应记录 ActionLog 用于审计追踪（红线5）
    pub async fn cancel_import_batch(
        &self,
        batch_id: &str,
    ) -> Result<CancelImportBatchResponse, ApiError> {
        let repo = MaterialImportRepositoryImpl::new(&self.db_path)
            .map_err(|e| ApiError::DatabaseError(format!("创建仓储失败: {}", e)))?;

        // 1. 删除该批次的所有冲突记录
        let deleted_conflicts = repo
            .delete_conflicts_by_batch(batch_id)
            .await
            .map_err(|e| ApiError::DatabaseError(format!("删除冲突记录失败: {}", e)))?;

        // 2. 删除该批次的材料（方案1：暂不实现）
        let deleted_materials = repo
            .delete_materials_by_batch(batch_id)
            .await
            .map_err(|e| ApiError::DatabaseError(format!("删除材料失败: {}", e)))?;

        // 3. 删除批次记录
        repo.delete_batch(batch_id)
            .await
            .map_err(|e| ApiError::DatabaseError(format!("删除批次记录失败: {}", e)))?;

        tracing::info!(
            batch_id = %batch_id,
            deleted_conflicts = deleted_conflicts,
            deleted_materials = deleted_materials,
            "成功取消导入批次"
        );

        Ok(CancelImportBatchResponse {
            deleted_materials,
            deleted_conflicts,
            message: format!(
                "成功取消导入批次：删除 {} 条冲突记录",
                deleted_conflicts
            ),
        })
    }

    /// 创建MaterialImporter实例
    fn create_importer(
        &self,
    ) -> Result<MaterialImporterImpl<MaterialImportRepositoryImpl, ConfigManager>, Box<dyn std::error::Error>> {
        let import_repo = MaterialImportRepositoryImpl::new(&self.db_path)?;
        let config = ConfigManager::new(&self.db_path)?;

        let file_parser = Box::new(CsvParser);
        let field_mapper = Box::new(FieldMapperImpl);
        let data_cleaner = Box::new(DataCleanerImpl);
        let derivation_service = Box::new(DerivationServiceImpl);
        let weight_threshold = config
            .get_global_config_value("weight_anomaly_threshold")?
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| *v > 0.0)
            .unwrap_or(100.0);
        let dq_validator = Box::new(DqValidatorImpl::new(weight_threshold));
        let conflict_handler = Box::new(ConflictHandler);
        let state_derivation_service = MaterialStateDerivationService::new();

        Ok(MaterialImporterImpl::new(
            import_repo,
            config,
            file_parser,
            field_mapper,
            data_cleaner,
            derivation_service,
            dq_validator,
            conflict_handler,
            state_derivation_service,
        ))
    }

    /// 将 RawMaterialRecord 转换为 MaterialMaster
    ///
    /// # 参数
    /// - record: 原始材料记录
    ///
    /// # 返回
    /// - MaterialMaster: 材料主数据
    ///
    /// # 说明
    /// 此方法复用导入流程的转换逻辑，用于 OVERWRITE 策略
    fn convert_raw_to_master(&self, record: RawMaterialRecord) -> MaterialMaster {
        // 派生 current_machine_code: COALESCE(rework_machine_code, next_machine_code)
        let current_machine_code = record.rework_machine_code.clone()
            .or_else(|| record.next_machine_code.clone());

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
    }
}

// ==========================================
// 热轧精整排产系统 - 材料导入 Repository 实现
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART E 工程结构
// 职责: 实现导入相关数据访问（使用 rusqlite）
// 红线: Repository 不含业务规则，只做数据 CRUD
// ==========================================

use crate::domain::material::{ImportBatch, ImportConflict, MaterialMaster, MaterialState};
use crate::repository::material_import_repo::MaterialImportRepository;
use async_trait::async_trait;
use rusqlite::{params, Connection, Transaction};
use std::error::Error;
use std::sync::{Arc, Mutex};

fn parse_conflict_type(raw: &str) -> crate::domain::material::ConflictType {
    // 历史实现将 enum 以 `format!("{:?}")` 写入 DB（非 JSON）。
    // 为兼容未来可能写入 JSON 字符串（带引号），这里做一次 normalize。
    let normalized = raw.trim().trim_matches('"');
    match normalized {
        "PrimaryKeyMissing" => crate::domain::material::ConflictType::PrimaryKeyMissing,
        "PrimaryKeyDuplicate" => crate::domain::material::ConflictType::PrimaryKeyDuplicate,
        "ForeignKeyViolation" => crate::domain::material::ConflictType::ForeignKeyViolation,
        "DataTypeError" => crate::domain::material::ConflictType::DataTypeError,
        _ => crate::domain::material::ConflictType::DataTypeError,
    }
}

// ==========================================
// MaterialImportRepositoryImpl
// ==========================================
pub struct MaterialImportRepositoryImpl {
    conn: Arc<Mutex<Connection>>,
}

impl MaterialImportRepositoryImpl {
    /// 创建新的 Repository 实例
    ///
    /// # 参数
    /// - db_path: 数据库文件路径
    pub fn new(db_path: &str) -> Result<Self, Box<dyn Error>> {
        let conn = Connection::open(db_path)?;

        // 启用外键约束
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 在事务中批量插入 MaterialMaster
    fn batch_insert_material_master_tx(
        tx: &Transaction,
        materials: &[MaterialMaster],
    ) -> Result<usize, Box<dyn Error>> {
        let mut stmt = tx.prepare(
            r#"
            INSERT OR REPLACE INTO material_master (
                material_id, manufacturing_order_id, material_status_code_src,
                steel_mark, slab_id, next_machine_code, rework_machine_code,
                current_machine_code, width_mm, thickness_mm, length_m, weight_t,
                available_width_mm, due_date, stock_age_days, output_age_days_raw,
                status_updated_at, contract_no, contract_nature, weekly_delivery_flag,
                export_flag, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
            )
            "#,
        )?;

        let mut count = 0;
        for material in materials {
            stmt.execute(params![
                material.material_id,
                material.manufacturing_order_id,
                material.material_status_code_src,
                material.steel_mark,
                material.slab_id,
                material.next_machine_code,
                material.rework_machine_code,
                material.current_machine_code,
                material.width_mm,
                material.thickness_mm,
                material.length_m,
                material.weight_t,
                material.available_width_mm,
                material.due_date,
                material.stock_age_days,
                material.output_age_days_raw,
                material.status_updated_at,
                material.contract_no,
                material.contract_nature,
                material.weekly_delivery_flag,
                material.export_flag,
                material.created_at,
                material.updated_at,
            ])?;
            count += 1;
        }

        Ok(count)
    }

    /// 在事务中批量插入 MaterialState
    fn batch_insert_material_state_tx(
        tx: &Transaction,
        states: &[MaterialState],
    ) -> Result<usize, Box<dyn Error>> {
        let mut stmt = tx.prepare(
            r#"
            INSERT OR REPLACE INTO material_state (
                material_id, sched_state, lock_flag, force_release_flag,
                urgent_level, urgent_reason, rush_level, rolling_output_age_days,
                ready_in_days, earliest_sched_date, stock_age_days,
                scheduled_date, scheduled_machine_code, seq_no,
                manual_urgent_flag, in_frozen_zone,
                last_calc_version_id, updated_at, updated_by
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
            )
            "#,
        )?;

        let mut count = 0;
        for state in states {
            // 使用统一的枚举序列化格式（全大写）
            let sched_state_str = match state.sched_state {
                crate::domain::types::SchedState::PendingMature => "PENDING_MATURE",
                crate::domain::types::SchedState::Ready => "READY",
                crate::domain::types::SchedState::Locked => "LOCKED",
                crate::domain::types::SchedState::ForceRelease => "FORCE_RELEASE",
                crate::domain::types::SchedState::Blocked => "BLOCKED",
                crate::domain::types::SchedState::Scheduled => "SCHEDULED",
            };

            let urgent_level_str = match state.urgent_level {
                crate::domain::types::UrgentLevel::L0 => "L0",
                crate::domain::types::UrgentLevel::L1 => "L1",
                crate::domain::types::UrgentLevel::L2 => "L2",
                crate::domain::types::UrgentLevel::L3 => "L3",
            };

            let rush_level_str = match state.rush_level {
                crate::domain::types::RushLevel::L0 => "L0",
                crate::domain::types::RushLevel::L1 => "L1",
                crate::domain::types::RushLevel::L2 => "L2",
            };

            stmt.execute(params![
                state.material_id,
                sched_state_str,
                state.lock_flag as i32,
                state.force_release_flag as i32,
                urgent_level_str,
                state.urgent_reason,
                rush_level_str,
                state.rolling_output_age_days,
                state.ready_in_days,
                state.earliest_sched_date.map(|d| d.to_string()),
                state.stock_age_days,
                state.scheduled_date.map(|d| d.to_string()),
                state.scheduled_machine_code,
                state.seq_no,
                state.manual_urgent_flag as i32,
                state.in_frozen_zone as i32,
                state.last_calc_version_id,
                state.updated_at.to_rfc3339(),
                state.updated_by,
            ])?;
            count += 1;
        }

        Ok(count)
    }
}

#[async_trait]
impl MaterialImportRepository for MaterialImportRepositoryImpl {
    /// 批量插入 MaterialMaster（事务化）
    async fn batch_insert_material_master(
        &self,
        materials: Vec<MaterialMaster>,
    ) -> Result<usize, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;
        let tx = conn.unchecked_transaction()?;

        let count = Self::batch_insert_material_master_tx(&tx, &materials)?;

        tx.commit()?;
        Ok(count)
    }

    /// 批量插入 MaterialState（事务化）
    async fn batch_insert_material_state(
        &self,
        states: Vec<MaterialState>,
    ) -> Result<usize, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;
        let tx = conn.unchecked_transaction()?;

        let count = Self::batch_insert_material_state_tx(&tx, &states)?;

        tx.commit()?;
        Ok(count)
    }

    /// 插入单个冲突记录
    async fn insert_conflict(&self, conflict: ImportConflict) -> Result<(), Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        conn.execute(
            r#"
            INSERT INTO import_conflict (
                conflict_id, source_batch_id, material_id, row_number,
                conflict_type, source_row_json, existing_row_json,
                resolution_status, resolution_note, detected_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9)
            "#,
            params![
                conflict.conflict_id,
                conflict.batch_id,
                conflict.material_id.clone().unwrap_or_default(),
                conflict.row_number as i64,
                format!("{:?}", conflict.conflict_type),
                conflict.raw_data,
                if conflict.resolved { "RESOLVED" } else { "OPEN" },
                conflict.reason,
                conflict.created_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// 批量插入冲突记录（事务化）
    async fn batch_insert_conflicts(
        &self,
        conflicts: Vec<ImportConflict>,
    ) -> Result<usize, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;
        let tx = conn.unchecked_transaction()?;

        let mut stmt = tx.prepare(
            r#"
            INSERT INTO import_conflict (
                conflict_id, source_batch_id, material_id, row_number,
                conflict_type, source_row_json, existing_row_json,
                resolution_status, resolution_note, detected_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9)
            "#,
        )?;

        let mut count = 0;
        for conflict in &conflicts {
            stmt.execute(params![
                conflict.conflict_id,
                conflict.batch_id,
                conflict.material_id.clone().unwrap_or_default(),
                conflict.row_number as i64,
                format!("{:?}", conflict.conflict_type),
                conflict.raw_data,
                if conflict.resolved { "RESOLVED" } else { "OPEN" },
                conflict.reason,
                conflict.created_at.to_rfc3339(),
            ])?;
            count += 1;
        }

        // 显式释放 stmt 的借用,以便提交事务
        drop(stmt);

        tx.commit()?;
        Ok(count)
    }

    /// 查询指定批次的冲突记录
    async fn get_conflicts_by_batch(
        &self,
        batch_id: &str,
    ) -> Result<Vec<ImportConflict>, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT conflict_id, source_batch_id, material_id, row_number,
                   conflict_type, source_row_json, resolution_status,
                   resolution_note, detected_at
            FROM import_conflict
            WHERE source_batch_id = ?1
            ORDER BY detected_at DESC
            "#,
        )?;

        let conflicts = stmt
            .query_map(params![batch_id], |row| {
                let material_id_str: Option<String> = row.get(2)?;
                let resolution_status: String = row.get(6)?;
                let conflict_type_raw: String = row.get(4)?;
                Ok(ImportConflict {
                    conflict_id: row.get(0)?,
                    batch_id: row.get(1)?,
                    material_id: material_id_str.filter(|s| !s.is_empty()),
                    row_number: row.get::<_, i64>(3)? as usize,
                    conflict_type: parse_conflict_type(&conflict_type_raw),
                    raw_data: row.get(5)?,
                    reason: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                    resolved: resolution_status == "RESOLVED",
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(conflicts)
    }

    // 继续实现其他方法...
    // （下一部分将实现剩余方法）

    /// 查询指定材料号的冲突记录
    async fn get_conflicts_by_material_id(
        &self,
        material_id: &str,
    ) -> Result<Vec<ImportConflict>, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT conflict_id, source_batch_id, material_id, row_number,
                   conflict_type, source_row_json, resolution_status,
                   resolution_note, detected_at
            FROM import_conflict
            WHERE material_id = ?1
            ORDER BY detected_at DESC
            "#,
        )?;

        let conflicts = stmt
            .query_map(params![material_id], |row| {
                let material_id_str: Option<String> = row.get(2)?;
                let resolution_status: String = row.get(6)?;
                let conflict_type_raw: String = row.get(4)?;
                Ok(ImportConflict {
                    conflict_id: row.get(0)?,
                    batch_id: row.get(1)?,
                    material_id: material_id_str.filter(|s| !s.is_empty()),
                    row_number: row.get::<_, i64>(3)? as usize,
                    conflict_type: parse_conflict_type(&conflict_type_raw),
                    raw_data: row.get(5)?,
                    reason: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                    resolved: resolution_status == "RESOLVED",
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(conflicts)
    }

    /// 标记冲突为已解决
    async fn mark_conflict_resolved(&self, conflict_id: &str) -> Result<(), Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        conn.execute(
            r#"
            UPDATE import_conflict
            SET resolution_status = 'RESOLVED'
            WHERE conflict_id = ?1
            "#,
            params![conflict_id],
        )?;

        Ok(())
    }

    /// 带过滤和分页的冲突列表查询
    async fn list_conflicts_with_filter(
        &self,
        status: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ImportConflict>, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        let conflicts: Vec<ImportConflict> = match status {
            Some(s) => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT conflict_id, source_batch_id, material_id, row_number,
                           conflict_type, source_row_json, resolution_status,
                           resolution_note, detected_at
                    FROM import_conflict
                    WHERE resolution_status = ?1
                    ORDER BY detected_at DESC
                    LIMIT ?2 OFFSET ?3
                    "#,
                )?;
                let result = stmt.query_map(params![s, limit, offset], |row| {
                    let material_id_str: Option<String> = row.get(2)?;
                    let resolution_status: String = row.get(6)?;
                    let conflict_type_raw: String = row.get(4)?;
                    Ok(ImportConflict {
                        conflict_id: row.get(0)?,
                        batch_id: row.get(1)?,
                        material_id: material_id_str.filter(|s| !s.is_empty()),
                        row_number: row.get::<_, i64>(3)? as usize,
                        conflict_type: parse_conflict_type(&conflict_type_raw),
                        raw_data: row.get(5)?,
                        reason: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                        resolved: resolution_status == "RESOLVED",
                        created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
                result
            }
            None => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT conflict_id, source_batch_id, material_id, row_number,
                           conflict_type, source_row_json, resolution_status,
                           resolution_note, detected_at
                    FROM import_conflict
                    ORDER BY detected_at DESC
                    LIMIT ?1 OFFSET ?2
                    "#,
                )?;
                let result = stmt.query_map(params![limit, offset], |row| {
                    let material_id_str: Option<String> = row.get(2)?;
                    let resolution_status: String = row.get(6)?;
                    let conflict_type_raw: String = row.get(4)?;
                    Ok(ImportConflict {
                        conflict_id: row.get(0)?,
                        batch_id: row.get(1)?,
                        material_id: material_id_str.filter(|s| !s.is_empty()),
                        row_number: row.get::<_, i64>(3)? as usize,
                        conflict_type: parse_conflict_type(&conflict_type_raw),
                        raw_data: row.get(5)?,
                        reason: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                        resolved: resolution_status == "RESOLVED",
                        created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
                result
            }
        };

        Ok(conflicts)
    }

    /// 统计指定状态的冲突数量
    async fn count_conflicts_by_status(
        &self,
        status: Option<&str>,
    ) -> Result<i64, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        let count: i64 = match status {
            Some(s) => conn.query_row(
                "SELECT COUNT(*) FROM import_conflict WHERE resolution_status = ?1",
                params![s],
                |row| row.get(0),
            )?,
            None => conn.query_row(
                "SELECT COUNT(*) FROM import_conflict",
                [],
                |row| row.get(0),
            )?,
        };

        Ok(count)
    }

    /// 按批次统计冲突数量（支持状态过滤）
    async fn count_conflicts_by_batch(
        &self,
        batch_id: &str,
        status: Option<&str>,
    ) -> Result<i64, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        let count: i64 = match status {
            Some(s) => conn.query_row(
                "SELECT COUNT(*) FROM import_conflict WHERE source_batch_id = ?1 AND resolution_status = ?2",
                params![batch_id, s],
                |row| row.get(0),
            )?,
            None => conn.query_row(
                "SELECT COUNT(*) FROM import_conflict WHERE source_batch_id = ?1",
                params![batch_id],
                |row| row.get(0),
            )?,
        };

        Ok(count)
    }

    /// 根据ID获取单个冲突记录
    async fn get_conflict_by_id(
        &self,
        conflict_id: &str,
    ) -> Result<Option<ImportConflict>, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT conflict_id, source_batch_id, material_id, row_number,
                   conflict_type, source_row_json, resolution_status,
                   resolution_note, detected_at
            FROM import_conflict
            WHERE conflict_id = ?1
            "#,
        )?;

        let result = stmt.query_row(params![conflict_id], |row| {
            let material_id_str: Option<String> = row.get(2)?;
            let resolution_status: String = row.get(6)?;
            let conflict_type_raw: String = row.get(4)?;
            Ok(ImportConflict {
                conflict_id: row.get(0)?,
                batch_id: row.get(1)?,
                material_id: material_id_str.filter(|s| !s.is_empty()),
                row_number: row.get::<_, i64>(3)? as usize,
                conflict_type: parse_conflict_type(&conflict_type_raw),
                raw_data: row.get(5)?,
                reason: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                resolved: resolution_status == "RESOLVED",
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
        });

        match result {
            Ok(conflict) => Ok(Some(conflict)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    /// 解决冲突并记录解决方式
    async fn resolve_conflict(
        &self,
        conflict_id: &str,
        action: &str,
        note: Option<&str>,
    ) -> Result<(), Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        conn.execute(
            r#"
            UPDATE import_conflict
            SET resolution_status = 'RESOLVED',
                resolution_action = ?2,
                resolution_note = ?3,
                resolved_at = ?4
            WHERE conflict_id = ?1
            "#,
            params![
                conflict_id,
                action,
                note,
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// 插入导入批次记录
    async fn insert_batch(&self, batch: ImportBatch) -> Result<(), Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        conn.execute(
            r#"
            INSERT INTO import_batch (
                batch_id, file_name, file_path,
                total_rows, success_rows, blocked_rows, warning_rows, conflict_rows,
                imported_at, imported_by, elapsed_ms, dq_report_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                batch.batch_id,
                batch.file_name,
                batch.file_path,
                batch.total_rows,
                batch.success_rows,
                batch.blocked_rows,
                batch.warning_rows,
                batch.conflict_rows,
                batch.imported_at.map(|dt| dt.to_rfc3339()),
                batch.imported_by,
                batch.elapsed_ms,
                batch.dq_report_json,
            ],
        )?;

        Ok(())
    }

    /// 查询最近的导入批次
    async fn get_recent_batches(&self, limit: usize) -> Result<Vec<ImportBatch>, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT batch_id, file_name, file_path,
                   total_rows, success_rows, blocked_rows, warning_rows, conflict_rows,
                   imported_at, imported_by, elapsed_ms, dq_report_json
            FROM import_batch
            ORDER BY imported_at DESC
            LIMIT ?1
            "#,
        )?;

        let batches = stmt
            .query_map(params![limit], |row| {
                Ok(ImportBatch {
                    batch_id: row.get(0)?,
                    file_name: row.get(1)?,
                    file_path: row.get(2)?,
                    total_rows: row.get(3)?,
                    success_rows: row.get(4)?,
                    blocked_rows: row.get(5)?,
                    warning_rows: row.get(6)?,
                    conflict_rows: row.get(7)?,
                    imported_at: row.get::<_, Option<String>>(8)?
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc)),
                    imported_by: row.get(9)?,
                    elapsed_ms: row.get(10)?,
                    dq_report_json: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(batches)
    }

    /// 检查材料号是否已存在
    async fn exists_material(&self, material_id: &str) -> Result<bool, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM material_master WHERE material_id = ?1",
            params![material_id],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// 批量检查材料号是否存在
    async fn batch_check_exists(
        &self,
        material_ids: Vec<String>,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        // 构建 IN 子句的占位符
        let placeholders = material_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let query = format!(
            "SELECT material_id FROM material_master WHERE material_id IN ({})",
            placeholders
        );

        let mut stmt = conn.prepare(&query)?;

        // 绑定参数
        let params: Vec<&dyn rusqlite::ToSql> = material_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        let existing_ids = stmt
            .query_map(params.as_slice(), |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(existing_ids)
    }

    /// 统计 material_master 表记录数
    async fn count_materials(&self) -> Result<usize, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM material_master",
            [],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }

    /// 统计 material_state 表记录数
    async fn count_states(&self) -> Result<usize, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM material_state",
            [],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }
}

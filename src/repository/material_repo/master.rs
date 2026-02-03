use crate::db::open_sqlite_connection;
use crate::domain::material::MaterialMaster;
use crate::repository::error::{RepositoryError, RepositoryResult};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::sync::{Arc, Mutex};

// ==========================================
// MaterialMasterRepository - 材料主数据仓储
// ==========================================
/// 材料主数据仓储
/// 职责: 管理 material_master 表的 CRUD 操作
/// 红线: 不含业务逻辑，只负责数据访问
pub struct MaterialMasterRepository {
    conn: Arc<Mutex<Connection>>,
}

impl MaterialMasterRepository {
    /// 创建新的 MaterialMasterRepository 实例
    ///
    /// # 参数
    /// - db_path: 数据库文件路径
    ///
    /// # 返回
    /// - Result<Self, RepositoryError>
    pub fn new(db_path: &str) -> RepositoryResult<Self> {
        let conn = open_sqlite_connection(db_path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 从已有连接创建仓储实例
    pub fn from_connection(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 获取数据库连接
    fn get_conn(&self) -> RepositoryResult<std::sync::MutexGuard<Connection>> {
        self.conn
            .lock()
            .map_err(|e| RepositoryError::LockError(e.to_string()))
    }

    /// 批量插入材料主数据（INSERT OR REPLACE）
    ///
    /// # 参数
    /// - materials: 材料主数据列表
    ///
    /// # 返回
    /// - Ok(usize): 成功插入的记录数
    /// - Err: 数据库错误
    ///
    /// # 说明
    /// - 使用 INSERT OR REPLACE 实现 upsert 语义
    /// - 如果 material_id 已存在，则更新记录
    /// - 使用事务确保原子性
    pub fn batch_insert_material_master(
        &self,
        materials: Vec<MaterialMaster>,
    ) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let tx = conn.unchecked_transaction()?;

        let mut count = 0;
        for material in materials {
            tx.execute(
                r#"
                INSERT OR REPLACE INTO material_master (
                    material_id, manufacturing_order_id, contract_no, due_date,
                    next_machine_code, rework_machine_code, current_machine_code,
                    width_mm, thickness_mm, length_m, weight_t, available_width_mm,
                    steel_mark, slab_id, material_status_code_src, status_updated_at,
                    output_age_days_raw, stock_age_days,
                    contract_nature, weekly_delivery_flag, export_flag,
                    created_at, updated_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
                )
                "#,
                params![
                    material.material_id,
                    material.manufacturing_order_id,
                    material.contract_no,
                    material.due_date.map(|d| d.to_string()),
                    material.next_machine_code,
                    material.rework_machine_code,
                    material.current_machine_code,
                    material.width_mm,
                    material.thickness_mm,
                    material.length_m,
                    material.weight_t,
                    material.available_width_mm,
                    material.steel_mark,
                    material.slab_id,
                    material.material_status_code_src,
                    material.status_updated_at.map(|dt| dt.to_rfc3339()),
                    material.output_age_days_raw,
                    material.stock_age_days,
                    material.contract_nature,
                    material.weekly_delivery_flag,
                    material.export_flag,
                    material.created_at.to_rfc3339(),
                    material.updated_at.to_rfc3339(),
                ],
            )?;
            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }

    /// 按 material_id 查询材料主数据
    ///
    /// # 参数
    /// - material_id: 材料号
    ///
    /// # 返回
    /// - Ok(Some(MaterialMaster)): 找到记录
    /// - Ok(None): 未找到记录
    /// - Err: 数据库错误
    pub fn find_by_id(&self, material_id: &str) -> RepositoryResult<Option<MaterialMaster>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                material_id, manufacturing_order_id, contract_no, due_date,
                next_machine_code, rework_machine_code, current_machine_code,
                width_mm, thickness_mm, length_m, weight_t, available_width_mm,
                steel_mark, slab_id, material_status_code_src, status_updated_at,
                output_age_days_raw, stock_age_days,
                contract_nature, weekly_delivery_flag, export_flag,
                created_at, updated_at
            FROM material_master
            WHERE material_id = ?1
            "#,
        )?;

        let result = stmt.query_row(params![material_id], |row| {
            Ok(MaterialMaster {
                material_id: row.get(0)?,
                manufacturing_order_id: row.get(1)?,
                contract_no: row.get(2)?,
                due_date: row
                    .get::<_, Option<String>>(3)?
                    .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                next_machine_code: row.get(4)?,
                rework_machine_code: row.get(5)?,
                current_machine_code: row.get(6)?,
                width_mm: row.get(7)?,
                thickness_mm: row.get(8)?,
                length_m: row.get(9)?,
                weight_t: row.get(10)?,
                available_width_mm: row.get(11)?,
                steel_mark: row.get(12)?,
                slab_id: row.get(13)?,
                material_status_code_src: row.get(14)?,
                status_updated_at: row
                    .get::<_, Option<String>>(15)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                output_age_days_raw: row.get(16)?,
                stock_age_days: row.get(17)?,
                contract_nature: row.get(18)?,
                weekly_delivery_flag: row.get(19)?,
                export_flag: row.get(20)?,
                created_at: row
                    .get::<_, String>(21)?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: row
                    .get::<_, String>(22)?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
        });

        match result {
            Ok(material) => Ok(Some(material)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 批量检查材料是否存在（用于冲突检测）
    ///
    /// # 参数
    /// - material_ids: 材料号列表
    ///
    /// # 返回
    /// - Ok(Vec<String>): 已存在的材料号列表
    /// - Err: 数据库错误
    pub fn batch_check_exists(
        &self,
        material_ids: Vec<String>,
    ) -> RepositoryResult<Vec<String>> {
        if material_ids.is_empty() {
            return Ok(vec![]);
        }

        let conn = self.get_conn()?;
        let placeholders = material_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT material_id FROM material_master WHERE material_id IN ({})",
            placeholders
        );

        let mut stmt = conn.prepare(&query)?;
        let params: Vec<&dyn rusqlite::ToSql> = material_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        let existing_ids = stmt
            .query_map(params.as_slice(), |row| row.get::<_, String>(0))?
            .collect::<SqliteResult<Vec<String>>>()?;

        Ok(existing_ids)
    }

    /// 按机组代码查询材料主数据
    ///
    /// # 参数
    /// - machine_code: 机组代码
    ///
    /// # 返回
    /// - Ok(Vec<MaterialMaster>): 材料列表
    /// - Err: 数据库错误
    pub fn find_by_machine(
        &self,
        machine_code: &str,
    ) -> RepositoryResult<Vec<MaterialMaster>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                material_id, manufacturing_order_id, contract_no, due_date,
                next_machine_code, rework_machine_code, current_machine_code,
                width_mm, thickness_mm, length_m, weight_t, available_width_mm,
                steel_mark, slab_id, material_status_code_src, status_updated_at,
                output_age_days_raw, stock_age_days,
                contract_nature, weekly_delivery_flag, export_flag,
                created_at, updated_at
            FROM material_master
            WHERE current_machine_code = ?1
            ORDER BY material_id
            "#,
        )?;

        let materials = stmt
            .query_map(params![machine_code], |row| {
                Ok(MaterialMaster {
                    material_id: row.get(0)?,
                    manufacturing_order_id: row.get(1)?,
                    contract_no: row.get(2)?,
                    due_date: row
                        .get::<_, Option<String>>(3)?
                        .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                    next_machine_code: row.get(4)?,
                    rework_machine_code: row.get(5)?,
                    current_machine_code: row.get(6)?,
                    width_mm: row.get(7)?,
                    thickness_mm: row.get(8)?,
                    length_m: row.get(9)?,
                    weight_t: row.get(10)?,
                    available_width_mm: row.get(11)?,
                    steel_mark: row.get(12)?,
                    slab_id: row.get(13)?,
                    material_status_code_src: row.get(14)?,
                    status_updated_at: row
                        .get::<_, Option<String>>(15)?
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc)),
                    output_age_days_raw: row.get(16)?,
                    stock_age_days: row.get(17)?,
                    contract_nature: row.get(18)?,
                    weekly_delivery_flag: row.get(19)?,
                    export_flag: row.get(20)?,
                    created_at: row
                        .get::<_, String>(21)?
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap(),
                    updated_at: row
                        .get::<_, String>(22)?
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap(),
                })
            })?
            .collect::<SqliteResult<Vec<MaterialMaster>>>()?;

        Ok(materials)
    }

    /// 查询所有材料（带分页）
    ///
    /// # 参数
    /// - limit: 返回记录数上限（0 或负数表示不限制）
    /// - offset: 偏移量（分页）
    ///
    /// # 返回
    /// - Ok(Vec<MaterialMaster>): 材料列表
    /// - Err: 数据库错误
    pub fn list_all(
        &self,
        limit: i32,
        offset: i32,
    ) -> RepositoryResult<Vec<MaterialMaster>> {
        let conn = self.get_conn()?;

        // 根据 limit 决定是否使用分页
        let sql = if limit > 0 {
            format!(
                r#"
                SELECT
                    material_id, manufacturing_order_id, contract_no, due_date,
                    next_machine_code, rework_machine_code, current_machine_code,
                    width_mm, thickness_mm, length_m, weight_t, available_width_mm,
                    steel_mark, slab_id, material_status_code_src, status_updated_at,
                    output_age_days_raw, stock_age_days,
                    contract_nature, weekly_delivery_flag, export_flag,
                    created_at, updated_at
                FROM material_master
                ORDER BY material_id
                LIMIT {} OFFSET {}
                "#,
                limit, offset
            )
        } else {
            // limit <= 0 表示不限制，返回所有数据
            r#"
            SELECT
                material_id, manufacturing_order_id, contract_no, due_date,
                next_machine_code, rework_machine_code, current_machine_code,
                width_mm, thickness_mm, length_m, weight_t, available_width_mm,
                steel_mark, slab_id, material_status_code_src, status_updated_at,
                output_age_days_raw, stock_age_days,
                contract_nature, weekly_delivery_flag, export_flag,
                created_at, updated_at
            FROM material_master
            ORDER BY material_id
            "#.to_string()
        };

        let mut stmt = conn.prepare(&sql)?;

        let materials = stmt
            .query_map([], |row| {
                Ok(MaterialMaster {
                    material_id: row.get(0)?,
                    manufacturing_order_id: row.get(1)?,
                    contract_no: row.get(2)?,
                    due_date: row
                        .get::<_, Option<String>>(3)?
                        .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                    next_machine_code: row.get(4)?,
                    rework_machine_code: row.get(5)?,
                    current_machine_code: row.get(6)?,
                    width_mm: row.get(7)?,
                    thickness_mm: row.get(8)?,
                    length_m: row.get(9)?,
                    weight_t: row.get(10)?,
                    available_width_mm: row.get(11)?,
                    steel_mark: row.get(12)?,
                    slab_id: row.get(13)?,
                    material_status_code_src: row.get(14)?,
                    status_updated_at: row
                        .get::<_, Option<String>>(15)?
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc)),
                    output_age_days_raw: row.get(16)?,
                    stock_age_days: row.get(17)?,
                    contract_nature: row.get(18)?,
                    weekly_delivery_flag: row.get(19)?,
                    export_flag: row.get(20)?,
                    created_at: row
                        .get::<_, String>(21)?
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap(),
                    updated_at: row
                        .get::<_, String>(22)?
                        .parse::<chrono::DateTime<chrono::Utc>>()
                        .unwrap(),
                })
            })?
            .collect::<SqliteResult<Vec<MaterialMaster>>>()?;

        Ok(materials)
    }

    /// 批量查询材料的出钢记号（steel_mark → 前端称为 steel_grade）
    ///
    /// # 参数
    /// - material_ids: 材料ID列表
    ///
    /// # 返回
    /// - Ok(HashMap<String, String>): material_id → steel_mark 映射
    pub fn find_steel_marks_by_ids(
        &self,
        material_ids: &[String],
    ) -> RepositoryResult<std::collections::HashMap<String, String>> {
        if material_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        const CHUNK_SIZE: usize = 900;

        let conn = self.get_conn()?;
        let mut result = std::collections::HashMap::with_capacity(material_ids.len());

        for chunk in material_ids.chunks(CHUNK_SIZE) {
            let placeholders = std::iter::repeat("?")
                .take(chunk.len())
                .collect::<Vec<_>>()
                .join(", ");

            let sql = format!(
                "SELECT material_id, steel_mark FROM material_master WHERE material_id IN ({})",
                placeholders
            );

            let mut stmt = conn.prepare(&sql)?;
            let params_vec: Vec<&dyn rusqlite::ToSql> =
                chunk.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

            let rows = stmt.query_map(params_vec.as_slice(), |row| {
                let id: String = row.get(0)?;
                let mark: Option<String> = row.get(1)?;
                Ok((id, mark))
            })?;

            for row in rows {
                if let Ok((id, Some(mark))) = row {
                    if !mark.is_empty() {
                        result.insert(id, mark);
                    }
                }
            }
        }

        Ok(result)
    }
}

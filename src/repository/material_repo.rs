// ==========================================
// 热轧精整排产系统 - 材料数据仓储
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART D 引擎铁律
// 红线: Repository 不含业务逻辑
// ==========================================

use crate::domain::material::{MaterialMaster, MaterialState};
use crate::domain::types::{RushLevel, SchedState, UrgentLevel};
use crate::repository::error::{RepositoryError, RepositoryResult};
use rusqlite::{params, Connection, Result as SqliteResult, ToSql};
use serde::{Deserialize, Serialize};
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
        let conn = Connection::open(db_path)?;
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

// ==========================================
// MaterialStateRepository - 材料状态仓储
// ==========================================
// 红线: 唯一事实层,写入必须审计
/// 材料状态仓储
/// 职责: 管理 material_state 表的 CRUD 操作
/// 红线: 唯一事实层，写入必须审计
pub struct MaterialStateRepository {
    conn: Arc<Mutex<Connection>>,
}

/// 轻量材料状态快照（用于前端解释/提示，不要求完整 MaterialState 映射）
///
/// 说明：
/// - 仅包含“挤出/可排性”解释所需的关键字段；
/// - 不包含 updated_at 等字段，避免因时间格式差异导致解析失败；
/// - 字段命名保持与 material_state 表一致（snake_case），便于前端复用现有逻辑。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialStateSnapshotLite {
    pub material_id: String,
    pub sched_state: Option<String>,
    pub urgent_level: Option<String>,
    pub rush_level: Option<String>,

    pub lock_flag: Option<bool>,
    pub force_release_flag: Option<bool>,
    pub manual_urgent_flag: Option<bool>,
    pub in_frozen_zone: Option<bool>,

    pub ready_in_days: Option<i32>,
    pub earliest_sched_date: Option<chrono::NaiveDate>,

    pub scheduled_date: Option<chrono::NaiveDate>,
    pub scheduled_machine_code: Option<String>,
    pub seq_no: Option<i32>,
}

impl MaterialStateRepository {
    /// 创建新的 MaterialStateRepository 实例
    ///
    /// # 参数
    /// - db_path: 数据库文件路径
    ///
    /// # 返回
    /// - RepositoryResult<Self>
    pub fn new(db_path: &str) -> RepositoryResult<Self> {
        let conn = Connection::open(db_path)?;
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

    // ==========================================
    // 枚举类型转换辅助方法
    // ==========================================

    /// SchedState 转字符串
    fn sched_state_to_str(state: &SchedState) -> &'static str {
        match state {
            SchedState::PendingMature => "PENDING_MATURE",
            SchedState::Ready => "READY",
            SchedState::Locked => "LOCKED",
            SchedState::ForceRelease => "FORCE_RELEASE",
            SchedState::Blocked => "BLOCKED",
            SchedState::Scheduled => "SCHEDULED",
        }
    }

    /// 字符串转 SchedState
    fn str_to_sched_state(s: &str) -> SchedState {
        match s {
            "PENDING_MATURE" => SchedState::PendingMature,
            "READY" => SchedState::Ready,
            "LOCKED" => SchedState::Locked,
            "FORCE_RELEASE" => SchedState::ForceRelease,
            "BLOCKED" => SchedState::Blocked,
            "SCHEDULED" => SchedState::Scheduled,
            _ => SchedState::Blocked, // 默认值
        }
    }

    /// UrgentLevel 转字符串
    fn urgent_level_to_str(level: &UrgentLevel) -> &'static str {
        match level {
            UrgentLevel::L0 => "L0",
            UrgentLevel::L1 => "L1",
            UrgentLevel::L2 => "L2",
            UrgentLevel::L3 => "L3",
        }
    }

    /// 字符串转 UrgentLevel
    fn str_to_urgent_level(s: &str) -> UrgentLevel {
        match s {
            "L0" => UrgentLevel::L0,
            "L1" => UrgentLevel::L1,
            "L2" => UrgentLevel::L2,
            "L3" => UrgentLevel::L3,
            _ => UrgentLevel::L0, // 默认值
        }
    }

    /// RushLevel 转字符串
    fn rush_level_to_str(level: &RushLevel) -> &'static str {
        match level {
            RushLevel::L0 => "L0",
            RushLevel::L1 => "L1",
            RushLevel::L2 => "L2",
        }
    }

    /// 字符串转 RushLevel
    fn str_to_rush_level(s: &str) -> RushLevel {
        match s {
            "L0" => RushLevel::L0,
            "L1" => RushLevel::L1,
            "L2" => RushLevel::L2,
            _ => RushLevel::L0, // 默认值
        }
    }

    /// 批量插入材料状态（INSERT OR REPLACE）
    ///
    /// # 参数
    /// - states: 材料状态列表
    ///
    /// # 返回
    /// - Ok(usize): 成功插入的记录数
    /// - Err: 数据库错误
    ///
    /// # 说明
    /// - 使用 INSERT OR REPLACE 实现 upsert 语义
    /// - 如果 material_id 已存在，则更新记录
    /// - 使用事务确保原子性
    pub fn batch_insert_material_state(
        &self,
        states: Vec<MaterialState>,
    ) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let tx = conn.unchecked_transaction()?;

        let mut count = 0;
        for state in states {
            tx.execute(
                r#"
                INSERT OR REPLACE INTO material_state (
                    material_id, sched_state, lock_flag, force_release_flag,
                    urgent_level, urgent_reason, rush_level,
                    rolling_output_age_days, ready_in_days, earliest_sched_date,
                    stock_age_days, scheduled_date, scheduled_machine_code, seq_no,
                    manual_urgent_flag, in_frozen_zone,
                    last_calc_version_id, updated_at, updated_by
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                    ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
                )
                "#,
                params![
                    state.material_id,
                    Self::sched_state_to_str(&state.sched_state),
                    state.lock_flag as i32,
                    state.force_release_flag as i32,
                    Self::urgent_level_to_str(&state.urgent_level),
                    state.urgent_reason,
                    Self::rush_level_to_str(&state.rush_level),
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
                ],
            )?;
            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }

    /// 按 material_id 查询材料状态
    ///
    /// # 参数
    /// - material_id: 材料号
    ///
    /// # 返回
    /// - Ok(Some(MaterialState)): 找到记录
    /// - Ok(None): 未找到记录
    /// - Err: 数据库错误
    pub fn find_by_id(&self, material_id: &str) -> RepositoryResult<Option<MaterialState>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                material_id, sched_state, lock_flag, force_release_flag,
                urgent_level, urgent_reason, rush_level,
                rolling_output_age_days, ready_in_days, earliest_sched_date,
                stock_age_days, scheduled_date, scheduled_machine_code, seq_no,
                manual_urgent_flag, in_frozen_zone,
                last_calc_version_id, updated_at, updated_by
            FROM material_state
            WHERE material_id = ?1
            "#,
        )?;

        let result = stmt.query_row(params![material_id], |row| {
            Ok(MaterialState {
                material_id: row.get(0)?,
                sched_state: Self::str_to_sched_state(&row.get::<_, String>(1)?),
                lock_flag: row.get::<_, i32>(2)? != 0,
                force_release_flag: row.get::<_, i32>(3)? != 0,
                urgent_level: Self::str_to_urgent_level(&row.get::<_, String>(4)?),
                urgent_reason: row.get(5)?,
                rush_level: row
                    .get::<_, Option<String>>(6)?
                    .map(|s| Self::str_to_rush_level(&s))
                    .unwrap_or(RushLevel::L0), // NULL 时默认为 L0（无催料）
                rolling_output_age_days: row.get::<_, Option<i32>>(7)?.unwrap_or(0), // NULL 时默认为 0
                ready_in_days: row.get::<_, Option<i32>>(8)?.unwrap_or(0), // NULL 时默认为 0
                earliest_sched_date: row
                    .get::<_, Option<String>>(9)?
                    .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                stock_age_days: row.get::<_, Option<i32>>(10)?.unwrap_or(0), // NULL 时默认为 0
                scheduled_date: row
                    .get::<_, Option<String>>(11)?
                    .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                scheduled_machine_code: row.get(12)?,
                seq_no: row.get(13)?,
                manual_urgent_flag: row.get::<_, i32>(14)? != 0,
                in_frozen_zone: row.get::<_, i32>(15)? != 0,
                last_calc_version_id: row.get(16)?,
                updated_at: row
                    .get::<_, String>(17)?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap(),
                updated_by: row.get(18)?,
            })
        });

        match result {
            Ok(state) => Ok(Some(state)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 批量查询材料状态快照（按 material_id 列表）
    ///
    /// 说明：
    /// - 用于“策略草案变更明细”的解释提示，避免前端逐条查库；
    /// - 由于 SQLite 参数数量限制，内部会分块查询。
    pub fn find_snapshots_by_material_ids(
        &self,
        material_ids: &[String],
    ) -> RepositoryResult<Vec<MaterialStateSnapshotLite>> {
        if material_ids.is_empty() {
            return Ok(vec![]);
        }

        // SQLite 默认变量上限通常为 999；留出余量，避免不同环境配置差异。
        const CHUNK_SIZE: usize = 900;

        let conn = self.get_conn()?;
        let mut out: Vec<MaterialStateSnapshotLite> = Vec::with_capacity(material_ids.len());

        for chunk in material_ids.chunks(CHUNK_SIZE) {
            let placeholders = std::iter::repeat("?")
                .take(chunk.len())
                .collect::<Vec<_>>()
                .join(", ");

            let sql = format!(
                r#"
                SELECT
                    material_id,
                    sched_state,
                    lock_flag,
                    force_release_flag,
                    urgent_level,
                    rush_level,
                    ready_in_days,
                    earliest_sched_date,
                    scheduled_date,
                    scheduled_machine_code,
                    seq_no,
                    manual_urgent_flag,
                    in_frozen_zone
                FROM material_state
                WHERE material_id IN ({})
                "#,
                placeholders
            );

            let mut stmt = conn.prepare(&sql)?;
            let params_vec: Vec<&dyn ToSql> = chunk.iter().map(|s| s as &dyn ToSql).collect();

            let rows = stmt.query_map(params_vec.as_slice(), |row| {
                let lock_flag_i: Option<i32> = row.get(2)?;
                let force_release_i: Option<i32> = row.get(3)?;
                let manual_urgent_i: Option<i32> = row.get(11)?;
                let in_frozen_i: Option<i32> = row.get(12)?;

                let earliest_str: Option<String> = row.get(7)?;
                let scheduled_str: Option<String> = row.get(8)?;

                Ok(MaterialStateSnapshotLite {
                    material_id: row.get(0)?,
                    sched_state: row.get(1)?,
                    lock_flag: lock_flag_i.map(|v| v != 0),
                    force_release_flag: force_release_i.map(|v| v != 0),
                    urgent_level: row.get(4)?,
                    rush_level: row.get(5)?,
                    ready_in_days: row.get(6)?,
                    earliest_sched_date: earliest_str
                        .as_deref()
                        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
                    scheduled_date: scheduled_str
                        .as_deref()
                        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
                    scheduled_machine_code: row.get(9)?,
                    seq_no: row.get(10)?,
                    manual_urgent_flag: manual_urgent_i.map(|v| v != 0),
                    in_frozen_zone: in_frozen_i.map(|v| v != 0),
                })
            })?;

            out.extend(rows.collect::<SqliteResult<Vec<_>>>()?);
        }

        Ok(out)
    }

    /// 查询适温待排材料（READY 状态）
    ///
    /// # 参数
    /// - machine_code: 可选的机组代码过滤
    ///
    /// # 返回
    /// - Ok(Vec<MaterialState>): 材料状态列表
    /// - Err: 数据库错误
    pub fn find_ready_materials(
        &self,
        machine_code: Option<&str>,
    ) -> RepositoryResult<Vec<MaterialState>> {
        let conn = self.get_conn()?;

        let query = if machine_code.is_some() {
            r#"
            SELECT
                ms.material_id, ms.sched_state, ms.lock_flag, ms.force_release_flag,
                ms.urgent_level, ms.urgent_reason, ms.rush_level,
                ms.rolling_output_age_days, ms.ready_in_days, ms.earliest_sched_date,
                ms.stock_age_days, ms.scheduled_date, ms.scheduled_machine_code, ms.seq_no,
                ms.manual_urgent_flag, ms.in_frozen_zone,
                ms.last_calc_version_id, ms.updated_at, ms.updated_by
            FROM material_state ms
            JOIN material_master mm ON ms.material_id = mm.material_id
            WHERE ms.sched_state = 'READY'
              AND mm.current_machine_code = ?1
            ORDER BY ms.urgent_level DESC, ms.stock_age_days DESC
            "#
        } else {
            r#"
            SELECT
                material_id, sched_state, lock_flag, force_release_flag,
                urgent_level, urgent_reason, rush_level,
                rolling_output_age_days, ready_in_days, earliest_sched_date,
                stock_age_days, scheduled_date, scheduled_machine_code, seq_no,
                manual_urgent_flag, in_frozen_zone,
                last_calc_version_id, updated_at, updated_by
            FROM material_state
            WHERE sched_state = 'READY'
            ORDER BY urgent_level DESC, stock_age_days DESC
            "#
        };

        let mut stmt = conn.prepare(query)?;

        let states = if let Some(code) = machine_code {
            stmt.query_map(params![code], Self::map_row_to_state)?
        } else {
            stmt.query_map([], Self::map_row_to_state)?
        }
        .collect::<SqliteResult<Vec<MaterialState>>>()?;

        Ok(states)
    }

    /// 查询未成熟材料（PENDING_MATURE 状态）
    ///
    /// # 参数
    /// - machine_code: 可选的机组代码过滤
    ///
    /// # 返回
    /// - Ok(Vec<MaterialState>): 材料状态列表
    /// - Err: 数据库错误
    pub fn find_immature_materials(
        &self,
        machine_code: Option<&str>,
    ) -> RepositoryResult<Vec<MaterialState>> {
        let conn = self.get_conn()?;

        let query = if machine_code.is_some() {
            r#"
            SELECT
                ms.material_id, ms.sched_state, ms.lock_flag, ms.force_release_flag,
                ms.urgent_level, ms.urgent_reason, ms.rush_level,
                ms.rolling_output_age_days, ms.ready_in_days, ms.earliest_sched_date,
                ms.stock_age_days, ms.scheduled_date, ms.scheduled_machine_code, ms.seq_no,
                ms.manual_urgent_flag, ms.in_frozen_zone,
                ms.last_calc_version_id, ms.updated_at, ms.updated_by
            FROM material_state ms
            JOIN material_master mm ON ms.material_id = mm.material_id
            WHERE ms.sched_state = 'PENDING_MATURE'
              AND mm.current_machine_code = ?1
            ORDER BY ms.ready_in_days ASC
            "#
        } else {
            r#"
            SELECT
                material_id, sched_state, lock_flag, force_release_flag,
                urgent_level, urgent_reason, rush_level,
                rolling_output_age_days, ready_in_days, earliest_sched_date,
                stock_age_days, scheduled_date, scheduled_machine_code, seq_no,
                manual_urgent_flag, in_frozen_zone,
                last_calc_version_id, updated_at, updated_by
            FROM material_state
            WHERE sched_state = 'PENDING_MATURE'
            ORDER BY ready_in_days ASC
            "#
        };

        let mut stmt = conn.prepare(query)?;

        let states = if let Some(code) = machine_code {
            stmt.query_map(params![code], Self::map_row_to_state)?
        } else {
            stmt.query_map([], Self::map_row_to_state)?
        }
        .collect::<SqliteResult<Vec<MaterialState>>>()?;

        Ok(states)
    }

    /// 查询指定紧急等级的材料
    ///
    /// # 参数
    /// - urgent_levels: 紧急等级列表 (如 ["L2", "L3"])
    ///
    /// # 返回
    /// - Ok(Vec<MaterialState>): 材料状态列表
    /// - Err: 数据库错误
    pub fn find_by_urgent_levels(
        &self,
        urgent_levels: &[UrgentLevel],
    ) -> RepositoryResult<Vec<MaterialState>> {
        if urgent_levels.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.get_conn()?;
        let placeholders = urgent_levels.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            r#"
            SELECT
                material_id, sched_state, lock_flag, force_release_flag,
                urgent_level, urgent_reason, rush_level,
                rolling_output_age_days, ready_in_days, earliest_sched_date,
                stock_age_days, scheduled_date, scheduled_machine_code, seq_no,
                manual_urgent_flag, in_frozen_zone,
                last_calc_version_id, updated_at, updated_by
            FROM material_state
            WHERE urgent_level IN ({})
            ORDER BY urgent_level DESC, stock_age_days DESC
            "#,
            placeholders
        );

        let mut stmt = conn.prepare(&query)?;

        // 将UrgentLevel转换为字符串,存储在Vec中
        let level_strings: Vec<String> = urgent_levels
            .iter()
            .map(|level| Self::urgent_level_to_str(level).to_string())
            .collect();

        let params: Vec<&dyn rusqlite::ToSql> = level_strings
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let states = stmt
            .query_map(params.as_slice(), Self::map_row_to_state)?
            .collect::<SqliteResult<Vec<MaterialState>>>()?;

        Ok(states)
    }

    /// 查询未排产的材料 (scheduled_date 为 NULL)
    ///
    /// # 参数
    /// - min_stock_age_days: 最小库存天数 (可选)
    ///
    /// # 返回
    /// - Ok(Vec<MaterialState>): 材料状态列表
    /// - Err: 数据库错误
    pub fn find_unscheduled_materials(
        &self,
        min_stock_age_days: Option<i32>,
    ) -> RepositoryResult<Vec<MaterialState>> {
        let conn = self.get_conn()?;

        let query = if let Some(_min_days) = min_stock_age_days {
            r#"
            SELECT
                material_id, sched_state, lock_flag, force_release_flag,
                urgent_level, urgent_reason, rush_level,
                rolling_output_age_days, ready_in_days, earliest_sched_date,
                stock_age_days, scheduled_date, scheduled_machine_code, seq_no,
                manual_urgent_flag, in_frozen_zone,
                last_calc_version_id, updated_at, updated_by
            FROM material_state
            WHERE scheduled_date IS NULL AND stock_age_days >= ?1
            ORDER BY stock_age_days DESC
            "#
        } else {
            r#"
            SELECT
                material_id, sched_state, lock_flag, force_release_flag,
                urgent_level, urgent_reason, rush_level,
                rolling_output_age_days, ready_in_days, earliest_sched_date,
                stock_age_days, scheduled_date, scheduled_machine_code, seq_no,
                manual_urgent_flag, in_frozen_zone,
                last_calc_version_id, updated_at, updated_by
            FROM material_state
            WHERE scheduled_date IS NULL
            ORDER BY stock_age_days DESC
            "#
        };

        let mut stmt = conn.prepare(query)?;

        let states = if let Some(min_days) = min_stock_age_days {
            stmt.query_map(params![min_days], Self::map_row_to_state)?
        } else {
            stmt.query_map([], Self::map_row_to_state)?
        }
        .collect::<SqliteResult<Vec<MaterialState>>>()?;

        Ok(states)
    }

    /// 辅助方法：将数据库行映射为 MaterialState
    fn map_row_to_state(row: &rusqlite::Row) -> SqliteResult<MaterialState> {
        Ok(MaterialState {
            material_id: row.get(0)?,
            sched_state: Self::str_to_sched_state(&row.get::<_, String>(1)?),
            lock_flag: row.get::<_, i32>(2)? != 0,
            force_release_flag: row.get::<_, i32>(3)? != 0,
            urgent_level: Self::str_to_urgent_level(&row.get::<_, String>(4)?),
            urgent_reason: row.get(5)?,
            rush_level: Self::str_to_rush_level(&row.get::<_, String>(6)?),
            rolling_output_age_days: row.get(7)?,
            ready_in_days: row.get(8)?,
            earliest_sched_date: row
                .get::<_, Option<String>>(9)?
                .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            stock_age_days: row.get(10)?,
            scheduled_date: row
                .get::<_, Option<String>>(11)?
                .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            scheduled_machine_code: row.get(12)?,
            seq_no: row.get(13)?,
            manual_urgent_flag: row.get::<_, i32>(14)? != 0,
            in_frozen_zone: row.get::<_, i32>(15)? != 0,
            last_calc_version_id: row.get(16)?,
            updated_at: row
                .get::<_, String>(17)?
                .parse::<chrono::DateTime<chrono::Utc>>()
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_by: row.get(18)?,
        })
    }
}

// TODO: 实现错误处理
// TODO: 实现事务支持
// TODO: 实现审计日志自动记录

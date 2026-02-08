use crate::db::open_sqlite_connection;
use crate::domain::material::MaterialState;
use crate::domain::types::{RushLevel, SchedState, UrgentLevel};
use crate::repository::error::{RepositoryError, RepositoryResult};
use rusqlite::{params, Connection, Result as SqliteResult, ToSql};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

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

/// 人工确认材料摘要（用于路径规则锚点/队列查询）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfirmedMaterialSummary {
    pub material_id: String,
    pub width_mm: f64,
    pub thickness_mm: f64,
    pub seq_no: Option<i32>,
    pub user_confirmed_at: Option<String>,
}

/// 路径规则拒绝摘要（用于重算候选过滤/提档）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathOverrideRejectionSummary {
    pub material_id: String,
    pub reject_cycle_no: Option<i32>,
    pub reject_base_sched_state: Option<String>,
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

    fn has_material_state_column(conn: &Connection, column_name: &str) -> RepositoryResult<bool> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('material_state') WHERE name = ?1",
            params![column_name],
            |row| row.get(0),
        )?;
        Ok(count > 0)
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
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO material_state (
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
                ON CONFLICT(material_id) DO UPDATE SET
                    sched_state = excluded.sched_state,
                    lock_flag = excluded.lock_flag,
                    force_release_flag = excluded.force_release_flag,
                    urgent_level = excluded.urgent_level,
                    urgent_reason = excluded.urgent_reason,
                    rush_level = excluded.rush_level,
                    rolling_output_age_days = excluded.rolling_output_age_days,
                    ready_in_days = excluded.ready_in_days,
                    earliest_sched_date = excluded.earliest_sched_date,
                    stock_age_days = excluded.stock_age_days,
                    scheduled_date = excluded.scheduled_date,
                    scheduled_machine_code = excluded.scheduled_machine_code,
                    seq_no = excluded.seq_no,
                    manual_urgent_flag = excluded.manual_urgent_flag,
                    in_frozen_zone = excluded.in_frozen_zone,
                    last_calc_version_id = excluded.last_calc_version_id,
                    updated_at = excluded.updated_at,
                    updated_by = excluded.updated_by
                "#,
            )?;

            for state in states {
                stmt.execute(
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
                , user_confirmed, user_confirmed_at, user_confirmed_by, user_confirmed_reason
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
                user_confirmed: row.get::<_, Option<i32>>(19)?.unwrap_or(0) != 0,
                user_confirmed_at: row
                    .get::<_, Option<String>>(20)?
                    .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok()),
                user_confirmed_by: row.get(21)?,
                user_confirmed_reason: row.get(22)?,
            })
        });

        match result {
            Ok(state) => Ok(Some(state)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 按机组代码查询材料状态（用于排产重算的批量读，避免 N+1 查询）
    ///
    /// # 参数
    /// - machine_code: 机组代码（material_master.current_machine_code）
    ///
    /// # 返回
    /// - Ok(Vec<MaterialState>): 该机组下所有材料状态
    /// - Err: 数据库错误
    pub fn list_by_machine_code(&self, machine_code: &str) -> RepositoryResult<Vec<MaterialState>> {
        let code = machine_code.trim();
        if code.is_empty() {
            return Err(RepositoryError::ValidationError("machine_code 不能为空".to_string()));
        }

        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                ms.material_id, ms.sched_state, ms.lock_flag, ms.force_release_flag,
                ms.urgent_level, ms.urgent_reason, ms.rush_level,
                ms.rolling_output_age_days, ms.ready_in_days, ms.earliest_sched_date,
                ms.stock_age_days, ms.scheduled_date, ms.scheduled_machine_code, ms.seq_no,
                ms.manual_urgent_flag, ms.in_frozen_zone,
                ms.last_calc_version_id, ms.updated_at, ms.updated_by
                , ms.user_confirmed, ms.user_confirmed_at, ms.user_confirmed_by, ms.user_confirmed_reason
            FROM material_state ms
            JOIN material_master mm ON ms.material_id = mm.material_id
            WHERE mm.current_machine_code = ?1
            ORDER BY ms.material_id
            "#,
        )?;

        let rows = stmt.query_map(params![code], Self::map_row_to_state)?;
        Ok(rows.collect::<SqliteResult<Vec<_>>>()?)
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
                , ms.user_confirmed, ms.user_confirmed_at, ms.user_confirmed_by, ms.user_confirmed_reason
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
                , user_confirmed, user_confirmed_at, user_confirmed_by, user_confirmed_reason
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
                , ms.user_confirmed, ms.user_confirmed_at, ms.user_confirmed_by, ms.user_confirmed_reason
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
                , user_confirmed, user_confirmed_at, user_confirmed_by, user_confirmed_reason
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
                , user_confirmed, user_confirmed_at, user_confirmed_by, user_confirmed_reason
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
                , user_confirmed, user_confirmed_at, user_confirmed_by, user_confirmed_reason
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
                , user_confirmed, user_confirmed_at, user_confirmed_by, user_confirmed_reason
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

    /// 更新材料人工确认状态（路径规则突破）
    pub fn update_user_confirmation(
        &self,
        material_id: &str,
        confirmed_by: &str,
        reason: &str,
    ) -> RepositoryResult<()> {
        let id = material_id.trim();
        if id.is_empty() {
            return Err(RepositoryError::ValidationError("material_id 不能为空".to_string()));
        }
        let by = confirmed_by.trim();
        if by.is_empty() {
            return Err(RepositoryError::ValidationError("confirmed_by 不能为空".to_string()));
        }
        let r = reason.trim();
        if r.is_empty() {
            return Err(RepositoryError::ValidationError("reason 不能为空".to_string()));
        }

        let conn = self.get_conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        let has_reject_columns = Self::has_material_state_column(&conn, "path_override_rejected")?;

        if has_reject_columns {
            conn.execute(
                r#"
                UPDATE material_state
                SET
                    user_confirmed = 1,
                    user_confirmed_at = ?2,
                    user_confirmed_by = ?3,
                    user_confirmed_reason = ?4,
                    path_override_rejected = 0,
                    path_override_rejected_at = NULL,
                    path_override_rejected_by = NULL,
                    path_override_rejected_reason = NULL,
                    path_override_reject_cycle_no = NULL,
                    path_override_reject_base_sched_state = NULL,
                    updated_at = ?2,
                    updated_by = ?3
                WHERE material_id = ?1
                "#,
                params![id, now, by, r],
            )?;

            return Ok(());
        }

        conn.execute(
            r#"
            UPDATE material_state
            SET
                user_confirmed = 1,
                user_confirmed_at = ?2,
                user_confirmed_by = ?3,
                user_confirmed_reason = ?4,
                updated_at = ?2,
                updated_by = ?3
            WHERE material_id = ?1
            "#,
            params![id, now, by, r],
        )?;

        Ok(())
    }

    /// 查询已人工确认的材料摘要（用于锚点解析/队列展示）
    pub fn list_user_confirmed_materials(
        &self,
        machine_code: &str,
    ) -> RepositoryResult<Vec<UserConfirmedMaterialSummary>> {
        let code = machine_code.trim();
        if code.is_empty() {
            return Err(RepositoryError::ValidationError("machine_code 不能为空".to_string()));
        }

        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                ms.material_id,
                COALESCE(mm.width_mm, 0.0) AS width_mm,
                COALESCE(mm.thickness_mm, 0.0) AS thickness_mm,
                ms.seq_no,
                ms.user_confirmed_at
            FROM material_state ms
            JOIN material_master mm ON ms.material_id = mm.material_id
            WHERE ms.user_confirmed = 1
              AND mm.current_machine_code = ?1
            ORDER BY ms.user_confirmed_at ASC
            "#,
        )?;

        let rows = stmt.query_map(params![code], |row| {
            Ok(UserConfirmedMaterialSummary {
                material_id: row.get(0)?,
                width_mm: row.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                thickness_mm: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                seq_no: row.get(3)?,
                user_confirmed_at: row.get(4)?,
            })
        })?;

        Ok(rows.collect::<SqliteResult<Vec<_>>>()?)
    }

    /// 更新材料人工拒绝状态（路径规则突破）
    pub fn update_path_override_rejection(
        &self,
        material_id: &str,
        rejected_by: &str,
        reason: &str,
        reject_cycle_no: i32,
        base_sched_state: &str,
    ) -> RepositoryResult<()> {
        let id = material_id.trim();
        if id.is_empty() {
            return Err(RepositoryError::ValidationError("material_id 不能为空".to_string()));
        }
        let by = rejected_by.trim();
        if by.is_empty() {
            return Err(RepositoryError::ValidationError("rejected_by 不能为空".to_string()));
        }
        let r = reason.trim();
        if r.is_empty() {
            return Err(RepositoryError::ValidationError("reason 不能为空".to_string()));
        }

        let conn = self.get_conn()?;
        let has_reject_columns = Self::has_material_state_column(&conn, "path_override_rejected")?;
        if !has_reject_columns {
            return Err(RepositoryError::ValidationError(
                "数据库缺少字段 path_override_rejected，请先执行 migrations/v0.8_path_override_reject_flow.sql"
                    .to_string(),
            ));
        }
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            r#"
            UPDATE material_state
            SET
                user_confirmed = 0,
                user_confirmed_at = NULL,
                user_confirmed_by = NULL,
                user_confirmed_reason = NULL,
                path_override_rejected = 1,
                path_override_rejected_at = ?2,
                path_override_rejected_by = ?3,
                path_override_rejected_reason = ?4,
                path_override_reject_cycle_no = ?5,
                path_override_reject_base_sched_state = ?6,
                updated_at = ?2,
                updated_by = ?3
            WHERE material_id = ?1
            "#,
            params![
                id,
                now,
                by,
                r,
                reject_cycle_no,
                base_sched_state.trim().to_uppercase()
            ],
        )?;

        Ok(())
    }

    /// 清除材料路径拒绝状态（通常在人工确认突破后调用）
    pub fn clear_path_override_rejection(
        &self,
        material_id: &str,
        cleared_by: &str,
    ) -> RepositoryResult<()> {
        let id = material_id.trim();
        if id.is_empty() {
            return Err(RepositoryError::ValidationError("material_id 不能为空".to_string()));
        }
        let by = cleared_by.trim();
        if by.is_empty() {
            return Err(RepositoryError::ValidationError("cleared_by 不能为空".to_string()));
        }

        let conn = self.get_conn()?;
        let has_reject_columns = Self::has_material_state_column(&conn, "path_override_rejected")?;
        if !has_reject_columns {
            return Ok(());
        }
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            r#"
            UPDATE material_state
            SET
                path_override_rejected = 0,
                path_override_rejected_at = NULL,
                path_override_rejected_by = NULL,
                path_override_rejected_reason = NULL,
                path_override_reject_cycle_no = NULL,
                path_override_reject_base_sched_state = NULL,
                updated_at = ?2,
                updated_by = ?3
            WHERE material_id = ?1
            "#,
            params![id, now, by],
        )?;

        Ok(())
    }

    /// 查询机组下“已拒绝路径突破”的材料摘要
    pub fn list_path_override_rejections_by_machine(
        &self,
        machine_code: &str,
    ) -> RepositoryResult<Vec<PathOverrideRejectionSummary>> {
        let code = machine_code.trim();
        if code.is_empty() {
            return Err(RepositoryError::ValidationError("machine_code 不能为空".to_string()));
        }

        let conn = self.get_conn()?;
        let has_reject_columns = Self::has_material_state_column(&conn, "path_override_rejected")?;
        if !has_reject_columns {
            return Ok(vec![]);
        }
        let mut stmt = conn.prepare(
            r#"
            SELECT
                ms.material_id,
                ms.path_override_reject_cycle_no,
                ms.path_override_reject_base_sched_state
            FROM material_state ms
            JOIN material_master mm ON ms.material_id = mm.material_id
            WHERE mm.current_machine_code = ?1
              AND COALESCE(ms.path_override_rejected, 0) = 1
            ORDER BY ms.material_id
            "#,
        )?;

        let rows = stmt.query_map(params![code], |row| {
            Ok(PathOverrideRejectionSummary {
                material_id: row.get(0)?,
                reject_cycle_no: row.get(1)?,
                reject_base_sched_state: row.get(2)?,
            })
        })?;

        Ok(rows.collect::<SqliteResult<Vec<_>>>()?)
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
            user_confirmed: row.get::<_, Option<i32>>(19)?.unwrap_or(0) != 0,
            user_confirmed_at: row
                .get::<_, Option<String>>(20)?
                .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok()),
            user_confirmed_by: row.get(21)?,
            user_confirmed_reason: row.get(22)?,
        })
    }
}

// TODO: 实现错误处理
// TODO: 实现事务支持
// TODO: 实现审计日志自动记录

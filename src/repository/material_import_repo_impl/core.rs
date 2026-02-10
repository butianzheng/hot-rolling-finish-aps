use crate::db::open_sqlite_connection;
use crate::domain::material::{MaterialMaster, MaterialState};
use rusqlite::{params, Connection, Transaction};
use std::error::Error;
use std::sync::{Arc, Mutex};

pub(super) fn parse_conflict_type(raw: &str) -> crate::domain::material::ConflictType {
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
    pub(super) conn: Arc<Mutex<Connection>>,
}

impl MaterialImportRepositoryImpl {
    /// 创建新的 Repository 实例
    ///
    /// # 参数
    /// - db_path: 数据库文件路径
    pub fn new(db_path: &str) -> Result<Self, Box<dyn Error>> {
        let conn = open_sqlite_connection(db_path)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 在事务中批量插入 MaterialMaster
    pub(super) fn batch_insert_material_master_tx(
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
                rolling_output_date, status_updated_at, contract_no, contract_nature,
                weekly_delivery_flag, export_flag, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
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
                material
                    .rolling_output_date
                    .map(|d| d.format("%Y-%m-%d").to_string()),
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
    pub(super) fn batch_insert_material_state_tx(
        tx: &Transaction,
        states: &[MaterialState],
    ) -> Result<usize, Box<dyn Error>> {
        let mut stmt = tx.prepare(
            r#"
            INSERT INTO material_state (
                material_id, sched_state, lock_flag, force_release_flag,
                urgent_level, urgent_reason, rush_level, rolling_output_age_days,
                ready_in_days, earliest_sched_date, stock_age_days,
                scheduled_date, scheduled_machine_code, seq_no,
                manual_urgent_flag, in_frozen_zone,
                last_calc_version_id, updated_at, updated_by
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
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

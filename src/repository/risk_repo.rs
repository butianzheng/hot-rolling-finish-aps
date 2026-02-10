// ==========================================
// 热轧精整排产系统 - 风险快照数据仓储
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART D 引擎铁律
// 红线: Repository 不含业务逻辑
// ==========================================

use crate::db::open_sqlite_connection;
use crate::domain::risk::RiskSnapshot;
use crate::domain::types::RiskLevel;
use crate::repository::error::{RepositoryError, RepositoryResult};
use chrono::NaiveDate;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::sync::{Arc, Mutex};

// ==========================================
// RiskSnapshotRepository - 风险快照仓储
// ==========================================
/// 风险快照仓储
/// 职责: 管理 risk_snapshot 表的 CRUD 操作
/// 用途: 驾驶舱只读数据源
/// 红线: 不含业务逻辑，只负责数据访问
pub struct RiskSnapshotRepository {
    conn: Arc<Mutex<Connection>>,
}

impl RiskSnapshotRepository {
    /// 创建新的 RiskSnapshotRepository 实例
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

    /// 批量插入风险快照（INSERT OR REPLACE）
    ///
    /// # 参数
    /// - snapshots: 风险快照列表
    ///
    /// # 返回
    /// - Ok(usize): 成功插入的记录数
    /// - Err: 数据库错误
    ///
    /// # 说明
    /// - 使用 INSERT OR REPLACE 实现 upsert 语义
    /// - 主键: (version_id, machine_code, snapshot_date)
    /// - 使用事务确保原子性
    pub fn batch_insert(&self, snapshots: Vec<RiskSnapshot>) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let tx = conn.unchecked_transaction()?;

        let mut count = 0;
        for snapshot in snapshots {
            tx.execute(
                r#"
                INSERT OR REPLACE INTO risk_snapshot (
                    version_id, machine_code, snapshot_date,
                    risk_level, risk_reasons,
                    target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t,
                    urgent_total_t, mature_backlog_t, immature_backlog_t,
                    campaign_status, created_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14
                )
                "#,
                params![
                    snapshot.version_id,
                    snapshot.machine_code,
                    snapshot.snapshot_date.to_string(),
                    format!("{:?}", snapshot.risk_level),
                    snapshot.risk_reason,
                    snapshot.target_capacity_t,
                    snapshot.used_capacity_t,
                    snapshot.limit_capacity_t,
                    snapshot.overflow_t,
                    snapshot.urgent_total_t,
                    snapshot.mature_backlog_t,
                    snapshot.immature_backlog_t,
                    snapshot.roll_status,
                    snapshot.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ],
            )?;
            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }

    /// 按 snapshot_id 查询（注意：schema中主键是version_id+machine_code+snapshot_date）
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - machine_code: 机组代码
    /// - snapshot_date: 快照日期
    ///
    /// # 返回
    /// - Ok(Some(RiskSnapshot)): 找到快照
    /// - Ok(None): 未找到
    /// - Err: 数据库错误
    pub fn find_by_key(
        &self,
        version_id: &str,
        machine_code: &str,
        snapshot_date: NaiveDate,
    ) -> RepositoryResult<Option<RiskSnapshot>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id, machine_code, snapshot_date,
                risk_level, risk_reasons,
                target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t,
                urgent_total_t, mature_backlog_t, immature_backlog_t,
                campaign_status, created_at
            FROM risk_snapshot
            WHERE version_id = ?1 AND machine_code = ?2 AND snapshot_date = ?3
            "#,
        )?;

        let result = stmt.query_row(
            params![version_id, machine_code, snapshot_date.to_string()],
            |row| {
                Ok(RiskSnapshot {
                    snapshot_id: format!(
                        "{}_{}_{}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?
                    ),
                    version_id: row.get(0)?,
                    machine_code: row.get(1)?,
                    snapshot_date: NaiveDate::parse_from_str(&row.get::<_, String>(2)?, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                    risk_level: parse_risk_level(&row.get::<_, String>(3)?),
                    risk_reason: row.get(4)?,
                    target_capacity_t: row.get(5)?,
                    used_capacity_t: row.get(6)?,
                    limit_capacity_t: row.get(7)?,
                    overflow_t: row.get(8)?,
                    urgent_total_t: row.get(9)?,
                    mature_backlog_t: row.get(10)?,
                    immature_backlog_t: row.get(11)?,
                    roll_status: row.get(12)?,
                    roll_risk: None, // TODO: 添加到schema
                    l3_count: 0,     // TODO: 添加到schema
                    l2_count: 0,     // TODO: 添加到schema
                    created_at: chrono::NaiveDateTime::parse_from_str(
                        &row.get::<_, String>(13)?,
                        "%Y-%m-%d %H:%M:%S",
                    )
                    .unwrap_or_else(|_| chrono::NaiveDateTime::default()),
                })
            },
        );

        match result {
            Ok(snapshot) => Ok(Some(snapshot)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 查询版本的所有快照
    ///
    /// # 参数
    /// - version_id: 版本ID
    ///
    /// # 返回
    /// - Ok(Vec<RiskSnapshot>): 快照列表
    /// - Err: 数据库错误
    pub fn find_by_version_id(&self, version_id: &str) -> RepositoryResult<Vec<RiskSnapshot>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id, machine_code, snapshot_date,
                risk_level, risk_reasons,
                target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t,
                urgent_total_t, mature_backlog_t, immature_backlog_t,
                campaign_status, created_at
            FROM risk_snapshot
            WHERE version_id = ?1
            ORDER BY snapshot_date ASC, machine_code ASC
            "#,
        )?;

        let snapshots = stmt
            .query_map(params![version_id], |row| {
                Ok(RiskSnapshot {
                    snapshot_id: format!(
                        "{}_{}_{}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?
                    ),
                    version_id: row.get(0)?,
                    machine_code: row.get(1)?,
                    snapshot_date: NaiveDate::parse_from_str(&row.get::<_, String>(2)?, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                    risk_level: parse_risk_level(&row.get::<_, String>(3)?),
                    risk_reason: row.get(4)?,
                    target_capacity_t: row.get(5)?,
                    used_capacity_t: row.get(6)?,
                    limit_capacity_t: row.get(7)?,
                    overflow_t: row.get(8)?,
                    urgent_total_t: row.get(9)?,
                    mature_backlog_t: row.get(10)?,
                    immature_backlog_t: row.get(11)?,
                    roll_status: row.get(12)?,
                    roll_risk: None,
                    l3_count: 0,
                    l2_count: 0,
                    created_at: chrono::NaiveDateTime::parse_from_str(
                        &row.get::<_, String>(13)?,
                        "%Y-%m-%d %H:%M:%S",
                    )
                    .unwrap_or_else(|_| chrono::NaiveDateTime::default()),
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(snapshots)
    }

    /// 查询版本指定日期的快照
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - snapshot_date: 快照日期
    ///
    /// # 返回
    /// - Ok(Vec<RiskSnapshot>): 该日期所有机组的快照
    /// - Err: 数据库错误
    pub fn find_by_version_and_date(
        &self,
        version_id: &str,
        snapshot_date: NaiveDate,
    ) -> RepositoryResult<Vec<RiskSnapshot>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id, machine_code, snapshot_date,
                risk_level, risk_reasons,
                target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t,
                urgent_total_t, mature_backlog_t, immature_backlog_t,
                campaign_status, created_at
            FROM risk_snapshot
            WHERE version_id = ?1 AND snapshot_date = ?2
            ORDER BY machine_code ASC
            "#,
        )?;

        let snapshots = stmt
            .query_map(params![version_id, snapshot_date.to_string()], |row| {
                Ok(RiskSnapshot {
                    snapshot_id: format!(
                        "{}_{}_{}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?
                    ),
                    version_id: row.get(0)?,
                    machine_code: row.get(1)?,
                    snapshot_date: NaiveDate::parse_from_str(&row.get::<_, String>(2)?, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                    risk_level: parse_risk_level(&row.get::<_, String>(3)?),
                    risk_reason: row.get(4)?,
                    target_capacity_t: row.get(5)?,
                    used_capacity_t: row.get(6)?,
                    limit_capacity_t: row.get(7)?,
                    overflow_t: row.get(8)?,
                    urgent_total_t: row.get(9)?,
                    mature_backlog_t: row.get(10)?,
                    immature_backlog_t: row.get(11)?,
                    roll_status: row.get(12)?,
                    roll_risk: None,
                    l3_count: 0,
                    l2_count: 0,
                    created_at: chrono::NaiveDateTime::parse_from_str(
                        &row.get::<_, String>(13)?,
                        "%Y-%m-%d %H:%M:%S",
                    )
                    .unwrap_or_else(|_| chrono::NaiveDateTime::default()),
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(snapshots)
    }

    /// 查询版本指定日期范围的快照
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - start_date: 起始日期（包含）
    /// - end_date: 结束日期（包含）
    ///
    /// # 返回
    /// - Ok(Vec<RiskSnapshot>): 日期范围内的快照
    /// - Err: 数据库错误
    pub fn find_by_version_and_date_range(
        &self,
        version_id: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> RepositoryResult<Vec<RiskSnapshot>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id, machine_code, snapshot_date,
                risk_level, risk_reasons,
                target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t,
                urgent_total_t, mature_backlog_t, immature_backlog_t,
                campaign_status, created_at
            FROM risk_snapshot
            WHERE version_id = ?1 AND snapshot_date >= ?2 AND snapshot_date <= ?3
            ORDER BY snapshot_date ASC, machine_code ASC
            "#,
        )?;

        let snapshots = stmt
            .query_map(
                params![version_id, start_date.to_string(), end_date.to_string()],
                |row| {
                    Ok(RiskSnapshot {
                        snapshot_id: format!(
                            "{}_{}_{}",
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?
                        ),
                        version_id: row.get(0)?,
                        machine_code: row.get(1)?,
                        snapshot_date: NaiveDate::parse_from_str(
                            &row.get::<_, String>(2)?,
                            "%Y-%m-%d",
                        )
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                        risk_level: parse_risk_level(&row.get::<_, String>(3)?),
                        risk_reason: row.get(4)?,
                        target_capacity_t: row.get(5)?,
                        used_capacity_t: row.get(6)?,
                        limit_capacity_t: row.get(7)?,
                        overflow_t: row.get(8)?,
                        urgent_total_t: row.get(9)?,
                        mature_backlog_t: row.get(10)?,
                        immature_backlog_t: row.get(11)?,
                        roll_status: row.get(12)?,
                        roll_risk: None,
                        l3_count: 0,
                        l2_count: 0,
                        created_at: chrono::NaiveDateTime::parse_from_str(
                            &row.get::<_, String>(13)?,
                            "%Y-%m-%d %H:%M:%S",
                        )
                        .unwrap_or_else(|_| chrono::NaiveDateTime::default()),
                    })
                },
            )?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(snapshots)
    }

    /// 查询版本最危险的日期（按风险等级排序）
    ///
    /// # 参数
    /// - version_id: 版本ID
    ///
    /// # 返回
    /// - Ok(Option<RiskSnapshot>): 最危险的快照
    /// - Ok(None): 无快照
    /// - Err: 数据库错误
    ///
    /// # 规则
    /// - 优先级: RED > ORANGE > YELLOW > GREEN
    /// - 相同风险等级按 overflow_t 降序
    pub fn find_most_risky_date(&self, version_id: &str) -> RepositoryResult<Option<RiskSnapshot>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id, machine_code, snapshot_date,
                risk_level, risk_reasons,
                target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t,
                urgent_total_t, mature_backlog_t, immature_backlog_t,
                campaign_status, created_at
            FROM risk_snapshot
            WHERE version_id = ?1
            ORDER BY
                CASE risk_level
                    WHEN 'Red' THEN 0
                    WHEN 'Orange' THEN 1
                    WHEN 'Yellow' THEN 2
                    WHEN 'Green' THEN 3
                    ELSE 4
                END ASC,
                overflow_t DESC,
                snapshot_date ASC
            LIMIT 1
            "#,
        )?;

        let result = stmt.query_row(params![version_id], |row| {
            Ok(RiskSnapshot {
                snapshot_id: format!(
                    "{}_{}_{}",
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?
                ),
                version_id: row.get(0)?,
                machine_code: row.get(1)?,
                snapshot_date: NaiveDate::parse_from_str(&row.get::<_, String>(2)?, "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                risk_level: parse_risk_level(&row.get::<_, String>(3)?),
                risk_reason: row.get(4)?,
                target_capacity_t: row.get(5)?,
                used_capacity_t: row.get(6)?,
                limit_capacity_t: row.get(7)?,
                overflow_t: row.get(8)?,
                urgent_total_t: row.get(9)?,
                mature_backlog_t: row.get(10)?,
                immature_backlog_t: row.get(11)?,
                roll_status: row.get(12)?,
                roll_risk: None,
                l3_count: 0,
                l2_count: 0,
                created_at: chrono::NaiveDateTime::parse_from_str(
                    &row.get::<_, String>(13)?,
                    "%Y-%m-%d %H:%M:%S",
                )
                .unwrap_or_else(|_| chrono::NaiveDateTime::default()),
            })
        });

        match result {
            Ok(snapshot) => Ok(Some(snapshot)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 查询指定风险等级的快照
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - risk_level: 风险等级
    ///
    /// # 返回
    /// - Ok(Vec<RiskSnapshot>): 指定风险等级的快照列表
    /// - Err: 数据库错误
    pub fn find_by_risk_level(
        &self,
        version_id: &str,
        risk_level: RiskLevel,
    ) -> RepositoryResult<Vec<RiskSnapshot>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id, machine_code, snapshot_date,
                risk_level, risk_reasons,
                target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t,
                urgent_total_t, mature_backlog_t, immature_backlog_t,
                campaign_status, created_at
            FROM risk_snapshot
            WHERE version_id = ?1 AND risk_level = ?2
            ORDER BY snapshot_date ASC, machine_code ASC
            "#,
        )?;

        let snapshots = stmt
            .query_map(params![version_id, format!("{:?}", risk_level)], |row| {
                Ok(RiskSnapshot {
                    snapshot_id: format!(
                        "{}_{}_{}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?
                    ),
                    version_id: row.get(0)?,
                    machine_code: row.get(1)?,
                    snapshot_date: NaiveDate::parse_from_str(&row.get::<_, String>(2)?, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                    risk_level: parse_risk_level(&row.get::<_, String>(3)?),
                    risk_reason: row.get(4)?,
                    target_capacity_t: row.get(5)?,
                    used_capacity_t: row.get(6)?,
                    limit_capacity_t: row.get(7)?,
                    overflow_t: row.get(8)?,
                    urgent_total_t: row.get(9)?,
                    mature_backlog_t: row.get(10)?,
                    immature_backlog_t: row.get(11)?,
                    roll_status: row.get(12)?,
                    roll_risk: None,
                    l3_count: 0,
                    l2_count: 0,
                    created_at: chrono::NaiveDateTime::parse_from_str(
                        &row.get::<_, String>(13)?,
                        "%Y-%m-%d %H:%M:%S",
                    )
                    .unwrap_or_else(|_| chrono::NaiveDateTime::default()),
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(snapshots)
    }

    /// 删除版本的所有快照
    ///
    /// # 参数
    /// - version_id: 版本ID
    ///
    /// # 返回
    /// - Ok(usize): 删除的记录数
    /// - Err: 数据库错误
    pub fn delete_by_version(&self, version_id: &str) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let count = conn.execute(
            "DELETE FROM risk_snapshot WHERE version_id = ?1",
            params![version_id],
        )?;
        Ok(count)
    }

    /// 查询机组风险趋势（时间序列）
    ///
    /// # 参数
    /// - version_id: 版本ID
    /// - machine_code: 机组代码
    ///
    /// # 返回
    /// - Ok(Vec<RiskSnapshot>): 按日期排序的快照列表
    /// - Err: 数据库错误
    pub fn find_risk_trend(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> RepositoryResult<Vec<RiskSnapshot>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id, machine_code, snapshot_date,
                risk_level, risk_reasons,
                target_capacity_t, used_capacity_t, limit_capacity_t, overflow_t,
                urgent_total_t, mature_backlog_t, immature_backlog_t,
                campaign_status, created_at
            FROM risk_snapshot
            WHERE version_id = ?1 AND machine_code = ?2
            ORDER BY snapshot_date ASC
            "#,
        )?;

        let snapshots = stmt
            .query_map(params![version_id, machine_code], |row| {
                Ok(RiskSnapshot {
                    snapshot_id: format!(
                        "{}_{}_{}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?
                    ),
                    version_id: row.get(0)?,
                    machine_code: row.get(1)?,
                    snapshot_date: NaiveDate::parse_from_str(&row.get::<_, String>(2)?, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                    risk_level: parse_risk_level(&row.get::<_, String>(3)?),
                    risk_reason: row.get(4)?,
                    target_capacity_t: row.get(5)?,
                    used_capacity_t: row.get(6)?,
                    limit_capacity_t: row.get(7)?,
                    overflow_t: row.get(8)?,
                    urgent_total_t: row.get(9)?,
                    mature_backlog_t: row.get(10)?,
                    immature_backlog_t: row.get(11)?,
                    roll_status: row.get(12)?,
                    roll_risk: None,
                    l3_count: 0,
                    l2_count: 0,
                    created_at: chrono::NaiveDateTime::parse_from_str(
                        &row.get::<_, String>(13)?,
                        "%Y-%m-%d %H:%M:%S",
                    )
                    .unwrap_or_else(|_| chrono::NaiveDateTime::default()),
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(snapshots)
    }
}

// ==========================================
// 辅助函数
// ==========================================

/// 解析风险等级字符串
fn parse_risk_level(s: &str) -> RiskLevel {
    match s {
        "Red" => RiskLevel::Red,
        "Orange" => RiskLevel::Orange,
        "Yellow" => RiskLevel::Yellow,
        "Green" => RiskLevel::Green,
        _ => RiskLevel::Green, // 默认值
    }
}

// ==========================================
// 热轧精整排产系统 - 换辊数据仓储
// ==========================================
// 依据: Claude_Dev_Master_Spec.md - PART D 引擎铁律
// 红线: Repository 不含业务逻辑
// ==========================================

use crate::domain::roller::RollerCampaign;
use crate::domain::types::{AnchorSource, RollStatus};
use crate::repository::error::{RepositoryError, RepositoryResult};
use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use std::sync::{Arc, Mutex};

// ==========================================
// RollerCampaignRepository - 换辊窗口仓储
// ==========================================
/// 换辊窗口仓储
/// 职责: 管理 roller_campaign 表的 CRUD 操作
/// 红线: 不含业务逻辑，只负责数据访问
pub struct RollerCampaignRepository {
    conn: Arc<Mutex<Connection>>,
}

impl RollerCampaignRepository {
    /// 创建新的 RollerCampaignRepository 实例
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

    /// 创建换辊窗口
    pub fn create(&self, campaign: &RollerCampaign) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute(
            r#"
            INSERT INTO roller_campaign (
                version_id, machine_code, campaign_no,
                start_date, end_date,
                cum_weight_t, suggest_threshold_t, hard_limit_t,
                status,
                path_anchor_material_id, path_anchor_width_mm, path_anchor_thickness_mm, anchor_source
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                campaign.version_id,
                campaign.machine_code,
                campaign.campaign_no,
                campaign.start_date.to_string(),
                campaign.end_date.map(|d| d.to_string()),
                campaign.cum_weight_t,
                campaign.suggest_threshold_t,
                campaign.hard_limit_t,
                format!("{:?}", campaign.status),
                campaign.path_anchor_material_id,
                campaign.path_anchor_width_mm,
                campaign.path_anchor_thickness_mm,
                campaign.anchor_source.map(|s| s.to_db_str().to_string()),
            ],
        )?;
        Ok(())
    }

    /// 按主键查询
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `machine_code`: 机组代码
    /// - `campaign_no`: 换辊批次号
    ///
    /// # 返回
    /// - Ok(Some(RollerCampaign)): 找到换辊窗口
    /// - Ok(None): 未找到
    /// - Err: 数据库错误
    pub fn find_by_key(
        &self,
        version_id: &str,
        machine_code: &str,
        campaign_no: i32,
    ) -> RepositoryResult<Option<RollerCampaign>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id, machine_code, campaign_no,
                start_date, end_date,
                cum_weight_t, suggest_threshold_t, hard_limit_t,
                status,
                path_anchor_material_id, path_anchor_width_mm, path_anchor_thickness_mm, anchor_source
            FROM roller_campaign
            WHERE version_id = ?1 AND machine_code = ?2 AND campaign_no = ?3
            "#,
        )?;

        let result = stmt.query_row(params![version_id, machine_code, campaign_no], |row| {
            Ok(RollerCampaign {
                version_id: row.get(0)?,
                machine_code: row.get(1)?,
                campaign_no: row.get(2)?,
                start_date: NaiveDate::parse_from_str(&row.get::<_, String>(3)?, "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                end_date: row
                    .get::<_, Option<String>>(4)?
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                cum_weight_t: row.get(5)?,
                suggest_threshold_t: row.get(6)?,
                hard_limit_t: row.get(7)?,
                status: parse_roll_status(&row.get::<_, String>(8)?),
                path_anchor_material_id: row.get(9)?,
                path_anchor_width_mm: row.get(10)?,
                path_anchor_thickness_mm: row.get(11)?,
                anchor_source: row
                    .get::<_, Option<String>>(12)?
                    .map(|s| AnchorSource::from_str(&s)),
            })
        });

        match result {
            Ok(campaign) => Ok(Some(campaign)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 查询版本的所有换辊窗口
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    ///
    /// # 返回
    /// - Ok(Vec<RollerCampaign>): 换辊窗口列表
    /// - Err: 数据库错误
    pub fn find_by_version_id(
        &self,
        version_id: &str,
    ) -> RepositoryResult<Vec<RollerCampaign>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id, machine_code, campaign_no,
                start_date, end_date,
                cum_weight_t, suggest_threshold_t, hard_limit_t,
                status,
                path_anchor_material_id, path_anchor_width_mm, path_anchor_thickness_mm, anchor_source
            FROM roller_campaign
            WHERE version_id = ?1
            ORDER BY machine_code ASC, campaign_no DESC
            "#,
        )?;

        let campaigns = stmt
            .query_map(params![version_id], |row| {
                Ok(RollerCampaign {
                    version_id: row.get(0)?,
                    machine_code: row.get(1)?,
                    campaign_no: row.get(2)?,
                    start_date: NaiveDate::parse_from_str(&row.get::<_, String>(3)?, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                    end_date: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                    cum_weight_t: row.get(5)?,
                    suggest_threshold_t: row.get(6)?,
                    hard_limit_t: row.get(7)?,
                    status: parse_roll_status(&row.get::<_, String>(8)?),
                    path_anchor_material_id: row.get(9)?,
                    path_anchor_width_mm: row.get(10)?,
                    path_anchor_thickness_mm: row.get(11)?,
                    anchor_source: row
                        .get::<_, Option<String>>(12)?
                        .map(|s| AnchorSource::from_str(&s)),
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(campaigns)
    }

    /// 查询机组当前进行中的换辊窗口
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `machine_code`: 机组代码
    ///
    /// # 返回
    /// - Ok(Some(RollerCampaign)): 找到进行中的换辊窗口
    /// - Ok(None): 未找到
    /// - Err: 数据库错误
    pub fn find_active_campaign(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> RepositoryResult<Option<RollerCampaign>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id, machine_code, campaign_no,
                start_date, end_date,
                cum_weight_t, suggest_threshold_t, hard_limit_t,
                status,
                path_anchor_material_id, path_anchor_width_mm, path_anchor_thickness_mm, anchor_source
            FROM roller_campaign
            WHERE version_id = ?1 AND machine_code = ?2 AND end_date IS NULL
            ORDER BY campaign_no DESC
            LIMIT 1
            "#,
        )?;

        let result = stmt.query_row(params![version_id, machine_code], |row| {
            Ok(RollerCampaign {
                version_id: row.get(0)?,
                machine_code: row.get(1)?,
                campaign_no: row.get(2)?,
                start_date: NaiveDate::parse_from_str(&row.get::<_, String>(3)?, "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                end_date: row
                    .get::<_, Option<String>>(4)?
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                cum_weight_t: row.get(5)?,
                suggest_threshold_t: row.get(6)?,
                hard_limit_t: row.get(7)?,
                status: parse_roll_status(&row.get::<_, String>(8)?),
                path_anchor_material_id: row.get(9)?,
                path_anchor_width_mm: row.get(10)?,
                path_anchor_thickness_mm: row.get(11)?,
                anchor_source: row
                    .get::<_, Option<String>>(12)?
                    .map(|s| AnchorSource::from_str(&s)),
            })
        });

        match result {
            Ok(campaign) => Ok(Some(campaign)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 查询机组历史换辊窗口
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `machine_code`: 机组代码
    /// - `limit`: 返回数量限制
    ///
    /// # 返回
    /// - Ok(Vec<RollerCampaign>): 历史换辊窗口列表
    /// - Err: 数据库错误
    pub fn find_by_machine(
        &self,
        version_id: &str,
        machine_code: &str,
        limit: i32,
    ) -> RepositoryResult<Vec<RollerCampaign>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id, machine_code, campaign_no,
                start_date, end_date,
                cum_weight_t, suggest_threshold_t, hard_limit_t,
                status,
                path_anchor_material_id, path_anchor_width_mm, path_anchor_thickness_mm, anchor_source
            FROM roller_campaign
            WHERE version_id = ?1 AND machine_code = ?2
            ORDER BY campaign_no DESC
            LIMIT ?3
            "#,
        )?;

        let campaigns = stmt
            .query_map(params![version_id, machine_code, limit], |row| {
                Ok(RollerCampaign {
                    version_id: row.get(0)?,
                    machine_code: row.get(1)?,
                    campaign_no: row.get(2)?,
                    start_date: NaiveDate::parse_from_str(&row.get::<_, String>(3)?, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                    end_date: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                    cum_weight_t: row.get(5)?,
                    suggest_threshold_t: row.get(6)?,
                    hard_limit_t: row.get(7)?,
                    status: parse_roll_status(&row.get::<_, String>(8)?),
                    path_anchor_material_id: row.get(9)?,
                    path_anchor_width_mm: row.get(10)?,
                    path_anchor_thickness_mm: row.get(11)?,
                    anchor_source: row
                        .get::<_, Option<String>>(12)?
                        .map(|s| AnchorSource::from_str(&s)),
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(campaigns)
    }

    /// 查询需要换辊的机组列表
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    ///
    /// # 返回
    /// - Ok(Vec<RollerCampaign>): 需要换辊的换辊窗口列表 (status = 'Suggest' or 'HardStop')
    /// - Err: 数据库错误
    pub fn find_needs_roll_change(
        &self,
        version_id: &str,
    ) -> RepositoryResult<Vec<RollerCampaign>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                version_id, machine_code, campaign_no,
                start_date, end_date,
                cum_weight_t, suggest_threshold_t, hard_limit_t,
                status,
                path_anchor_material_id, path_anchor_width_mm, path_anchor_thickness_mm, anchor_source
            FROM roller_campaign
            WHERE version_id = ?1
              AND end_date IS NULL
              AND status IN ('Suggest', 'HardStop')
            ORDER BY
                CASE status
                    WHEN 'HardStop' THEN 0
                    WHEN 'Suggest' THEN 1
                    ELSE 2
                END ASC,
                machine_code ASC
            "#,
        )?;

        let campaigns = stmt
            .query_map(params![version_id], |row| {
                Ok(RollerCampaign {
                    version_id: row.get(0)?,
                    machine_code: row.get(1)?,
                    campaign_no: row.get(2)?,
                    start_date: NaiveDate::parse_from_str(&row.get::<_, String>(3)?, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                    end_date: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                    cum_weight_t: row.get(5)?,
                    suggest_threshold_t: row.get(6)?,
                    hard_limit_t: row.get(7)?,
                    status: parse_roll_status(&row.get::<_, String>(8)?),
                    path_anchor_material_id: row.get(9)?,
                    path_anchor_width_mm: row.get(10)?,
                    path_anchor_thickness_mm: row.get(11)?,
                    anchor_source: row
                        .get::<_, Option<String>>(12)?
                        .map(|s| AnchorSource::from_str(&s)),
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(campaigns)
    }

    /// 更新累计吨位
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `machine_code`: 机组代码
    /// - `campaign_no`: 换辊批次号
    /// - `new_tonnage`: 新的累计吨位
    ///
    /// # 返回
    /// - Ok(()): 更新成功
    /// - Err: 数据库错误
    pub fn update_accumulated_tonnage(
        &self,
        version_id: &str,
        machine_code: &str,
        campaign_no: i32,
        new_tonnage: f64,
    ) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute(
            r#"
            UPDATE roller_campaign
            SET cum_weight_t = ?4
            WHERE version_id = ?1 AND machine_code = ?2 AND campaign_no = ?3
            "#,
            params![version_id, machine_code, campaign_no, new_tonnage],
        )?;
        Ok(())
    }

    /// 更新换辊状态
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `machine_code`: 机组代码
    /// - `campaign_no`: 换辊批次号
    /// - `status`: 换辊状态
    ///
    /// # 返回
    /// - Ok(()): 更新成功
    /// - Err: 数据库错误
    pub fn update_status(
        &self,
        version_id: &str,
        machine_code: &str,
        campaign_no: i32,
        status: RollStatus,
    ) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute(
            r#"
            UPDATE roller_campaign
            SET status = ?4
            WHERE version_id = ?1 AND machine_code = ?2 AND campaign_no = ?3
            "#,
            params![
                version_id,
                machine_code,
                campaign_no,
                format!("{:?}", status)
            ],
        )?;
        Ok(())
    }

    /// 结束换辊窗口
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `machine_code`: 机组代码
    /// - `campaign_no`: 换辊批次号
    /// - `end_date`: 结束日期
    ///
    /// # 返回
    /// - Ok(()): 更新成功
    /// - Err: 数据库错误
    pub fn close_campaign(
        &self,
        version_id: &str,
        machine_code: &str,
        campaign_no: i32,
        end_date: NaiveDate,
    ) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute(
            r#"
            UPDATE roller_campaign
            SET end_date = ?4
            WHERE version_id = ?1 AND machine_code = ?2 AND campaign_no = ?3
            "#,
            params![
                version_id,
                machine_code,
                campaign_no,
                end_date.to_string()
            ],
        )?;
        Ok(())
    }

    /// 更新换辊周期锚点（宽厚路径规则）
    pub fn update_campaign_anchor(
        &self,
        version_id: &str,
        machine_code: &str,
        campaign_no: i32,
        anchor_material_id: Option<&str>,
        anchor_width_mm: Option<f64>,
        anchor_thickness_mm: Option<f64>,
        anchor_source: AnchorSource,
    ) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        conn.execute(
            r#"
            UPDATE roller_campaign
            SET
                path_anchor_material_id = ?4,
                path_anchor_width_mm = ?5,
                path_anchor_thickness_mm = ?6,
                anchor_source = ?7
            WHERE version_id = ?1 AND machine_code = ?2 AND campaign_no = ?3
            "#,
            params![
                version_id,
                machine_code,
                campaign_no,
                anchor_material_id,
                anchor_width_mm,
                anchor_thickness_mm,
                anchor_source.to_db_str(),
            ],
        )?;
        Ok(())
    }

    /// 重置换辊周期（换辊时调用）：关闭当前 active campaign，并创建新 campaign
    ///
    /// 注意：Repository 不做业务判定，仅执行必要的表更新/插入。
    pub fn reset_campaign_for_roll_change(
        &self,
        version_id: &str,
        machine_code: &str,
        new_campaign_no: i32,
        start_date: NaiveDate,
    ) -> RepositoryResult<()> {
        let conn = self.get_conn()?;
        let tx = conn.unchecked_transaction()?;

        // 1) 读取当前 active campaign（若存在则关闭并继承阈值）
        let active: Option<(i32, f64, f64)> = tx
            .query_row(
                r#"
                SELECT campaign_no, suggest_threshold_t, hard_limit_t
                FROM roller_campaign
                WHERE version_id = ?1 AND machine_code = ?2 AND end_date IS NULL
                ORDER BY campaign_no DESC
                LIMIT 1
                "#,
                params![version_id, machine_code],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        let (suggest_threshold_t, hard_limit_t) = if let Some((active_no, suggest_t, hard_t)) = active {
            tx.execute(
                r#"
                UPDATE roller_campaign
                SET end_date = ?4
                WHERE version_id = ?1 AND machine_code = ?2 AND campaign_no = ?3
                "#,
                params![version_id, machine_code, active_no, start_date.to_string()],
            )?;
            (suggest_t, hard_t)
        } else {
            (1500.0, 2500.0)
        };

        // 2) 插入新 campaign（锚点重置）
        tx.execute(
            r#"
            INSERT INTO roller_campaign (
                version_id, machine_code, campaign_no,
                start_date, end_date,
                cum_weight_t, suggest_threshold_t, hard_limit_t,
                status,
                path_anchor_material_id, path_anchor_width_mm, path_anchor_thickness_mm, anchor_source
            ) VALUES (?1, ?2, ?3, ?4, NULL, 0.0, ?5, ?6, ?7, NULL, NULL, NULL, ?8)
            "#,
            params![
                version_id,
                machine_code,
                new_campaign_no,
                start_date.to_string(),
                suggest_threshold_t,
                hard_limit_t,
                format!("{:?}", RollStatus::Normal),
                AnchorSource::None.to_db_str(),
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// 查询当前活跃的换辊周期（语义别名）
    pub fn get_active_campaign(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> RepositoryResult<Option<RollerCampaign>> {
        self.find_active_campaign(version_id, machine_code)
    }

    /// 批量插入换辊窗口
    ///
    /// # 参数
    /// - `campaigns`: 换辊窗口列表
    ///
    /// # 返回
    /// - Ok(usize): 成功插入的记录数
    /// - Err: 数据库错误
    pub fn batch_insert(&self, campaigns: Vec<RollerCampaign>) -> RepositoryResult<usize> {
        let conn = self.get_conn()?;
        let tx = conn.unchecked_transaction()?;

        let mut count = 0;
        for campaign in campaigns {
            tx.execute(
                r#"
                INSERT OR REPLACE INTO roller_campaign (
                    version_id, machine_code, campaign_no,
                    start_date, end_date,
                    cum_weight_t, suggest_threshold_t, hard_limit_t,
                    status,
                    path_anchor_material_id, path_anchor_width_mm, path_anchor_thickness_mm, anchor_source
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                "#,
                params![
                    campaign.version_id,
                    campaign.machine_code,
                    campaign.campaign_no,
                    campaign.start_date.to_string(),
                    campaign.end_date.map(|d| d.to_string()),
                    campaign.cum_weight_t,
                    campaign.suggest_threshold_t,
                    campaign.hard_limit_t,
                    format!("{:?}", campaign.status),
                    campaign.path_anchor_material_id,
                    campaign.path_anchor_width_mm,
                    campaign.path_anchor_thickness_mm,
                    campaign.anchor_source.map(|s| s.to_db_str().to_string()),
                ],
            )?;
            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }
}

// ==========================================
// 辅助函数
// ==========================================

/// 解析换辊状态字符串
fn parse_roll_status(s: &str) -> RollStatus {
    match s {
        "HardStop" => RollStatus::HardStop,
        "Suggest" => RollStatus::Suggest,
        "Normal" => RollStatus::Normal,
        _ => RollStatus::Normal, // 默认值
    }
}

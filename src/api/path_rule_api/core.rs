// ==========================================
// 热轧精整排产系统 - 宽厚路径规则 API（核心） (v0.6)
// ==========================================
// 职责:
// 1) 查询/更新路径规则配置
// 2) 查询待人工确认的路径违规材料
// 3) 人工确认突破（写 material_state.user_confirmed* + action_log）
// 4) 查询/重置换辊周期锚点（roller_campaign.path_anchor_*）
// ==========================================

use std::sync::{Arc, Mutex};

use chrono::NaiveDate;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::api::error::{ApiError, ApiResult};
use crate::config::ConfigManager;
use crate::domain::action_log::{ActionLog, ActionType};
use crate::domain::types::{AnchorSource, UrgentLevel};
use crate::engine::{Anchor, PathRuleConfig, PathRuleEngine};
use crate::repository::action_log_repo::ActionLogRepository;
use crate::repository::material_repo::{MaterialMasterRepository, MaterialStateRepository};
use crate::repository::path_override_pending_repo::PathOverridePendingRepository;
use crate::repository::plan_repo::PlanItemRepository;
use crate::repository::roller_repo::RollerCampaignRepository;

// ==========================================
// DTO 定义
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRuleConfigDto {
    pub enabled: bool,
    pub width_tolerance_mm: f64,
    pub thickness_tolerance_mm: f64,
    pub override_allowed_urgency_levels: Vec<String>, // ["L2","L3"]
    pub seed_s2_percentile: f64,
    pub seed_s2_small_sample_threshold: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathOverridePendingDto {
    pub material_id: String,
    pub material_no: String,
    pub width_mm: f64,
    pub thickness_mm: f64,
    pub urgent_level: String,
    pub violation_type: String,
    pub anchor_width_mm: f64,
    pub anchor_thickness_mm: f64,
    pub width_delta_mm: f64,
    pub thickness_delta_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathOverridePendingSummaryDto {
    pub machine_code: String,
    pub plan_date: String, // YYYY-MM-DD
    pub pending_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollCycleAnchorDto {
    pub version_id: String,
    pub machine_code: String,
    pub campaign_no: i32,
    pub cum_weight_t: f64,
    pub anchor_source: String,
    pub anchor_material_id: Option<String>,
    pub anchor_width_mm: Option<f64>,
    pub anchor_thickness_mm: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfirmResultDto {
    pub success_count: i32,
    pub fail_count: i32,
    pub failed_material_ids: Vec<String>,
}

// ==========================================
// PathRuleApi
// ==========================================

pub struct PathRuleApi {
    conn: Arc<Mutex<Connection>>,
    config_manager: Arc<ConfigManager>,
    plan_item_repo: Arc<PlanItemRepository>,
    material_master_repo: Arc<MaterialMasterRepository>,
    material_state_repo: Arc<MaterialStateRepository>,
    roller_campaign_repo: Arc<RollerCampaignRepository>,
    action_log_repo: Arc<ActionLogRepository>,
    path_override_pending_repo: Arc<PathOverridePendingRepository>,
}

impl PathRuleApi {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        conn: Arc<Mutex<Connection>>,
        config_manager: Arc<ConfigManager>,
        plan_item_repo: Arc<PlanItemRepository>,
        material_master_repo: Arc<MaterialMasterRepository>,
        material_state_repo: Arc<MaterialStateRepository>,
        roller_campaign_repo: Arc<RollerCampaignRepository>,
        action_log_repo: Arc<ActionLogRepository>,
        path_override_pending_repo: Arc<PathOverridePendingRepository>,
    ) -> Self {
        Self {
            conn,
            config_manager,
            plan_item_repo,
            material_master_repo,
            material_state_repo,
            roller_campaign_repo,
            action_log_repo,
            path_override_pending_repo,
        }
    }

    fn parse_bool(raw: Option<String>, default: bool) -> bool {
        match raw.as_deref().map(|s| s.trim().to_lowercase()) {
            Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "y" | "on") => true,
            Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "n" | "off") => false,
            _ => default,
        }
    }

    fn parse_f64(raw: Option<String>, default: f64) -> f64 {
        raw.as_deref()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .unwrap_or(default)
    }

    fn parse_i32(raw: Option<String>, default: i32) -> i32 {
        raw.as_deref()
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(default)
    }

    fn parse_urgent_levels(raw: Option<String>) -> Vec<UrgentLevel> {
        let raw = raw.unwrap_or_default();
        let mut levels = Vec::new();
        for token in raw.split(',').map(|s| s.trim().to_uppercase()) {
            let level = match token.as_str() {
                "L0" => Some(UrgentLevel::L0),
                "L1" => Some(UrgentLevel::L1),
                "L2" => Some(UrgentLevel::L2),
                "L3" => Some(UrgentLevel::L3),
                _ => None,
            };
            if let Some(l) = level {
                if !levels.contains(&l) {
                    levels.push(l);
                }
            }
        }
        if levels.is_empty() {
            vec![UrgentLevel::L2, UrgentLevel::L3]
        } else {
            levels
        }
    }

    fn load_path_rule_config(&self) -> PathRuleConfig {
        PathRuleConfig {
            enabled: Self::parse_bool(
                self.config_manager
                    .get_global_config_value("path_rule_enabled")
                    .ok()
                    .flatten(),
                true,
            ),
            width_tolerance_mm: Self::parse_f64(
                self.config_manager
                    .get_global_config_value("path_width_tolerance_mm")
                    .ok()
                    .flatten(),
                50.0,
            ),
            thickness_tolerance_mm: Self::parse_f64(
                self.config_manager
                    .get_global_config_value("path_thickness_tolerance_mm")
                    .ok()
                    .flatten(),
                1.0,
            ),
            override_allowed_urgency_levels: Self::parse_urgent_levels(
                self.config_manager
                    .get_global_config_value("path_override_allowed_urgency_levels")
                    .ok()
                    .flatten(),
            ),
        }
    }

    pub fn get_path_rule_config(&self) -> ApiResult<PathRuleConfigDto> {
        let enabled = Self::parse_bool(
            self.config_manager
                .get_global_config_value("path_rule_enabled")
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
                .clone(),
            true,
        );

        let width_tolerance_mm = Self::parse_f64(
            self.config_manager
                .get_global_config_value("path_width_tolerance_mm")
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?,
            50.0,
        );

        let thickness_tolerance_mm = Self::parse_f64(
            self.config_manager
                .get_global_config_value("path_thickness_tolerance_mm")
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?,
            1.0,
        );

        let override_levels = Self::parse_urgent_levels(
            self.config_manager
                .get_global_config_value("path_override_allowed_urgency_levels")
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?,
        );

        let seed_s2_percentile = Self::parse_f64(
            self.config_manager
                .get_global_config_value("seed_s2_percentile")
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?,
            0.95,
        )
        .clamp(0.0, 1.0);

        let seed_s2_small_sample_threshold = Self::parse_i32(
            self.config_manager
                .get_global_config_value("seed_s2_small_sample_threshold")
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?,
            10,
        )
        .max(1);

        Ok(PathRuleConfigDto {
            enabled,
            width_tolerance_mm,
            thickness_tolerance_mm,
            override_allowed_urgency_levels: override_levels
                .into_iter()
                .map(|l| l.to_string())
                .collect(),
            seed_s2_percentile,
            seed_s2_small_sample_threshold,
        })
    }

    pub fn update_path_rule_config(
        &self,
        config: PathRuleConfigDto,
        operator: &str,
        reason: &str,
    ) -> ApiResult<()> {
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| ApiError::DatabaseError(format!("锁获取失败: {}", e)))?;

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let upsert = |key: &str, value: &str| -> Result<(), ApiError> {
            conn.execute(
                "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES ('global', ?1, ?2, ?3)
                 ON CONFLICT(scope_id, key) DO UPDATE SET value = ?2, updated_at = ?3",
                params![key, value, now],
            )
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
            Ok(())
        };

        upsert("path_rule_enabled", if config.enabled { "true" } else { "false" })?;
        upsert("path_width_tolerance_mm", &config.width_tolerance_mm.to_string())?;
        upsert("path_thickness_tolerance_mm", &config.thickness_tolerance_mm.to_string())?;
        upsert(
            "path_override_allowed_urgency_levels",
            &config.override_allowed_urgency_levels.join(","),
        )?;
        upsert("seed_s2_percentile", &config.seed_s2_percentile.to_string())?;
        upsert(
            "seed_s2_small_sample_threshold",
            &config.seed_s2_small_sample_threshold.to_string(),
        )?;

        drop(conn);

        let log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None,
            action_type: "UPDATE_PATH_RULE_CONFIG".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "scope_id": "global",
                "keys": [
                    "path_rule_enabled",
                    "path_width_tolerance_mm",
                    "path_thickness_tolerance_mm",
                    "path_override_allowed_urgency_levels",
                    "seed_s2_percentile",
                    "seed_s2_small_sample_threshold"
                ],
                "config": config,
                "reason": reason,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some("更新宽厚路径规则配置".to_string()),
        };

        self.action_log_repo
            .insert(&log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub fn list_path_override_pending(
        &self,
        version_id: &str,
        machine_code: &str,
        plan_date: NaiveDate,
    ) -> ApiResult<Vec<PathOverridePendingDto>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if machine_code.trim().is_empty() {
            return Err(ApiError::InvalidInput("机组代码不能为空".to_string()));
        }

        let records = self
            .path_override_pending_repo
            .list_details(version_id, machine_code, plan_date)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| PathOverridePendingDto {
                material_id: r.material_id.clone(),
                material_no: r.material_id.clone(),
                width_mm: r.width_mm,
                thickness_mm: r.thickness_mm,
                urgent_level: r.urgent_level,
                violation_type: r.violation_type,
                anchor_width_mm: r.anchor_width_mm,
                anchor_thickness_mm: r.anchor_thickness_mm,
                width_delta_mm: r.width_delta_mm,
                thickness_delta_mm: r.thickness_delta_mm,
            })
            .collect())
    }

    pub fn list_path_override_pending_summary(
        &self,
        version_id: &str,
        plan_date_from: NaiveDate,
        plan_date_to: NaiveDate,
        machine_codes: Option<&[String]>,
    ) -> ApiResult<Vec<PathOverridePendingSummaryDto>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if plan_date_to < plan_date_from {
            return Err(ApiError::InvalidInput("日期范围不合法: to < from".to_string()));
        }

        let rows = self
            .path_override_pending_repo
            .list_summary(version_id, plan_date_from, plan_date_to, machine_codes)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| PathOverridePendingSummaryDto {
                machine_code: r.machine_code,
                plan_date: r.plan_date.format("%Y-%m-%d").to_string(),
                pending_count: r.pending_count,
            })
            .collect())
    }

    pub fn confirm_path_override(
        &self,
        version_id: &str,
        material_id: &str,
        confirmed_by: &str,
        reason: &str,
    ) -> ApiResult<()> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if material_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("材料ID不能为空".to_string()));
        }
        if confirmed_by.trim().is_empty() {
            return Err(ApiError::InvalidInput("确认人不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("确认原因不能为空".to_string()));
        }

        let master = self
            .material_master_repo
            .find_by_id(material_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("材料主数据不存在: {}", material_id)))?;

        let state = self
            .material_state_repo
            .find_by_id(material_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("材料状态不存在: {}", material_id)))?;

        let machine_code = master
            .current_machine_code
            .clone()
            .unwrap_or_else(|| "UNKNOWN".to_string());

        let campaign = self
            .roller_campaign_repo
            .find_active_campaign(version_id, &machine_code)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let anchor = campaign
            .as_ref()
            .filter(|c| c.has_valid_anchor())
            .map(|c| Anchor {
                width_mm: c.path_anchor_width_mm.unwrap_or(0.0),
                thickness_mm: c.path_anchor_thickness_mm.unwrap_or(0.0),
            });

        let w = master.width_mm.unwrap_or(0.0);
        let t = master.thickness_mm.unwrap_or(0.0);

        let config = self.load_path_rule_config();
        let engine = PathRuleEngine::new(config);
        let check = engine.check(w, t, state.urgent_level, anchor.as_ref(), false);

        // 先写入 material_state.user_confirmed*
        self.material_state_repo
            .update_user_confirmation(material_id, confirmed_by, reason)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // 再写审计日志
        let pending_snapshot = self
            .path_override_pending_repo
            .find_by_key(version_id, &machine_code, material_id)
            .ok()
            .flatten();

        let (violation_type, anchor_width_mm, anchor_thickness_mm, width_delta_mm, thickness_delta_mm, urgent_level) =
            if let Some(p) = pending_snapshot {
                (
                    p.violation_type,
                    p.anchor_width_mm,
                    p.anchor_thickness_mm,
                    p.width_delta_mm,
                    p.thickness_delta_mm,
                    p.urgent_level,
                )
            } else {
                (
                    check
                        .violation_type
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "UNKNOWN".to_string()),
                    anchor.as_ref().map(|a| a.width_mm).unwrap_or(0.0),
                    anchor.as_ref().map(|a| a.thickness_mm).unwrap_or(0.0),
                    check.width_delta_mm,
                    check.thickness_delta_mm,
                    state.urgent_level.to_string(),
                )
            };

        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version_id.to_string()),
            action_type: ActionType::PathOverrideConfirm.as_str().to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: confirmed_by.to_string(),
            payload_json: Some(serde_json::json!({
                "material_id": material_id,
                "violation_type": violation_type,
                "anchor_width_mm": anchor_width_mm,
                "anchor_thickness_mm": anchor_thickness_mm,
                "material_width_mm": w,
                "material_thickness_mm": t,
                "width_delta_mm": width_delta_mm,
                "thickness_delta_mm": thickness_delta_mm,
                "urgent_level": urgent_level,
                "confirm_reason": reason,
            })),
            impact_summary_json: None,
            machine_code: Some(machine_code.clone()),
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("路径规则人工确认: {}", material_id)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub fn batch_confirm_path_override(
        &self,
        version_id: &str,
        material_ids: &[String],
        confirmed_by: &str,
        reason: &str,
    ) -> ApiResult<BatchConfirmResultDto> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if confirmed_by.trim().is_empty() {
            return Err(ApiError::InvalidInput("确认人不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("确认原因不能为空".to_string()));
        }

        let mut failed = Vec::new();
        let mut success_count = 0i32;

        for id in material_ids {
            match self
                .material_state_repo
                .update_user_confirmation(id, confirmed_by, reason)
            {
                Ok(()) => success_count += 1,
                Err(_) => failed.push(id.clone()),
            }
        }

        // 批量审计（避免为每个材料写一条日志导致噪声）
        let failed_material_ids = failed.clone();
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version_id.to_string()),
            action_type: ActionType::PathOverrideConfirm.as_str().to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: confirmed_by.to_string(),
            payload_json: Some(serde_json::json!({
                "material_ids": material_ids,
                "success_count": success_count,
                "fail_count": failed_material_ids.len(),
                "failed_material_ids": failed_material_ids,
                "confirm_reason": reason,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some("路径规则人工确认（批量）".to_string()),
        };

        if let Err(e) = self.action_log_repo.insert(&action_log) {
            tracing::warn!("写入批量 PathOverrideConfirm 审计失败: {}", e);
        }

        Ok(BatchConfirmResultDto {
            success_count,
            fail_count: failed.len() as i32,
            failed_material_ids,
        })
    }

    pub fn batch_confirm_path_override_by_range(
        &self,
        version_id: &str,
        plan_date_from: NaiveDate,
        plan_date_to: NaiveDate,
        machine_codes: Option<&[String]>,
        confirmed_by: &str,
        reason: &str,
    ) -> ApiResult<BatchConfirmResultDto> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if confirmed_by.trim().is_empty() {
            return Err(ApiError::InvalidInput("确认人不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("确认原因不能为空".to_string()));
        }
        if plan_date_to < plan_date_from {
            return Err(ApiError::InvalidInput("日期范围不合法: to < from".to_string()));
        }

        let ids = self
            .path_override_pending_repo
            .list_pending_material_ids_by_range(version_id, plan_date_from, plan_date_to, machine_codes)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        if ids.is_empty() {
            return Ok(BatchConfirmResultDto {
                success_count: 0,
                fail_count: 0,
                failed_material_ids: Vec::new(),
            });
        }

        let mut failed = Vec::new();
        let mut success_count = 0i32;
        for id in &ids {
            match self
                .material_state_repo
                .update_user_confirmation(id, confirmed_by, reason)
            {
                Ok(()) => success_count += 1,
                Err(_) => failed.push(id.clone()),
            }
        }

        let failed_material_ids = failed.clone();
        let sample_ids: Vec<String> = ids.iter().take(50).cloned().collect();
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version_id.to_string()),
            action_type: ActionType::PathOverrideConfirm.as_str().to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: confirmed_by.to_string(),
            payload_json: Some(serde_json::json!({
                "plan_date_from": plan_date_from.to_string(),
                "plan_date_to": plan_date_to.to_string(),
                "machine_codes": machine_codes.map(|v| v.to_vec()),
                "total_candidates": ids.len(),
                "success_count": success_count,
                "fail_count": failed_material_ids.len(),
                "failed_material_ids": failed_material_ids,
                "material_ids_sample": sample_ids,
                "confirm_reason": reason,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: Some(plan_date_from),
            date_range_end: Some(plan_date_to),
            detail: Some("路径规则人工确认（范围批量）".to_string()),
        };

        if let Err(e) = self.action_log_repo.insert(&action_log) {
            tracing::warn!("写入范围批量 PathOverrideConfirm 审计失败: {}", e);
        }

        Ok(BatchConfirmResultDto {
            success_count,
            fail_count: failed.len() as i32,
            failed_material_ids,
        })
    }

    pub fn get_roll_cycle_anchor(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> ApiResult<Option<RollCycleAnchorDto>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if machine_code.trim().is_empty() {
            return Err(ApiError::InvalidInput("机组代码不能为空".to_string()));
        }

        let campaign = self
            .roller_campaign_repo
            .find_active_campaign(version_id, machine_code)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(campaign.map(|c| RollCycleAnchorDto {
            version_id: c.version_id,
            machine_code: c.machine_code.clone(),
            campaign_no: c.campaign_no,
            cum_weight_t: c.cum_weight_t,
            anchor_source: c
                .anchor_source
                .unwrap_or(AnchorSource::None)
                .to_string(),
            anchor_material_id: c.path_anchor_material_id,
            anchor_width_mm: c.path_anchor_width_mm,
            anchor_thickness_mm: c.path_anchor_thickness_mm,
            status: format!("{:?}", c.status),
        }))
    }

    pub fn reset_roll_cycle(
        &self,
        version_id: &str,
        machine_code: &str,
        actor: &str,
        reason: &str,
    ) -> ApiResult<()> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if machine_code.trim().is_empty() {
            return Err(ApiError::InvalidInput("机组代码不能为空".to_string()));
        }
        if actor.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        let active = self
            .roller_campaign_repo
            .find_active_campaign(version_id, machine_code)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let Some(active) = active else {
            return Err(ApiError::NotFound(format!(
                "未找到活跃换辊周期: version_id={}, machine_code={}",
                version_id, machine_code
            )));
        };

        let previous_campaign_no = active.campaign_no;
        let new_campaign_no = previous_campaign_no + 1;
        let today = chrono::Local::now().date_naive();

        self.roller_campaign_repo
            .reset_campaign_for_roll_change(version_id, machine_code, new_campaign_no, today)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version_id.to_string()),
            action_type: ActionType::RollCycleReset.as_str().to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: actor.to_string(),
            payload_json: Some(serde_json::json!({
                "machine_code": machine_code,
                "previous_campaign_no": previous_campaign_no,
                "new_campaign_no": new_campaign_no,
                "reset_trigger": "MANUAL",
                "previous_cum_weight_t": active.cum_weight_t,
                "previous_anchor": {
                    "material_id": active.path_anchor_material_id,
                    "width_mm": active.path_anchor_width_mm,
                    "thickness_mm": active.path_anchor_thickness_mm,
                },
                "reason": reason,
            })),
            impact_summary_json: Some(serde_json::json!({
                "anchor_reset": true,
                "cum_weight_reset": true,
            })),
            machine_code: Some(machine_code.to_string()),
            date_range_start: None,
            date_range_end: None,
            detail: Some("手动换辊：重置 RollCycle 锚点与累计状态".to_string()),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

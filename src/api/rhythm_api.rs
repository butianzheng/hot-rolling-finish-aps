// ==========================================
// 热轧精整排产系统 - 每日生产节奏 API
// ==========================================
// 职责:
// - 维护“品种大类”每日节奏目标（按版本×机组×日期）
// - 提供节奏预设模板、批量应用与日内实际/目标对比
// 说明:
// - 节奏目标用于监控/评估，不直接改变排程结果（后续可接入策略/结构校正）
// ==========================================

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::api::error::{ApiError, ApiResult};
use crate::config::config_keys;
use crate::config::ConfigManager;
use crate::domain::action_log::ActionLog;
use crate::repository::action_log_repo::ActionLogRepository;
use crate::repository::plan_rhythm_repo::{
    PlanRhythmPresetEntity, PlanRhythmRepository, PlanRhythmTargetEntity,
};

pub const DIM_PRODUCT_CATEGORY: &str = "PRODUCT_CATEGORY";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRhythmPresetInfo {
    pub preset_id: String,
    pub preset_name: String,
    pub dimension: String,
    pub target_json: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub updated_by: Option<String>,
}

impl From<PlanRhythmPresetEntity> for PlanRhythmPresetInfo {
    fn from(value: PlanRhythmPresetEntity) -> Self {
        Self {
            preset_id: value.preset_id,
            preset_name: value.preset_name,
            dimension: value.dimension,
            target_json: value.target_json,
            is_active: value.is_active,
            created_at: value.created_at,
            updated_at: value.updated_at,
            updated_by: value.updated_by,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRhythmTargetInfo {
    pub version_id: String,
    pub machine_code: String,
    pub plan_date: String,
    pub dimension: String,
    pub target_json: String,
    pub preset_id: Option<String>,
    pub updated_at: String,
    pub updated_by: Option<String>,
}

impl From<PlanRhythmTargetEntity> for PlanRhythmTargetInfo {
    fn from(value: PlanRhythmTargetEntity) -> Self {
        Self {
            version_id: value.version_id,
            machine_code: value.machine_code,
            plan_date: value.plan_date,
            dimension: value.dimension,
            target_json: value.target_json,
            preset_id: value.preset_id,
            updated_at: value.updated_at,
            updated_by: value.updated_by,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyRhythmCategoryRow {
    pub category: String,
    pub scheduled_weight_t: f64,
    pub actual_ratio: f64,
    pub target_ratio: Option<f64>,
    pub diff_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyRhythmProfile {
    pub version_id: String,
    pub machine_code: String,
    pub plan_date: String,
    pub dimension: String,
    pub total_scheduled_weight_t: f64,
    pub deviation_threshold: f64,
    pub max_deviation: f64,
    pub is_violated: bool,
    pub target_preset_id: Option<String>,
    pub target_updated_at: Option<String>,
    pub target_updated_by: Option<String>,
    pub categories: Vec<DailyRhythmCategoryRow>,
}

pub struct RhythmApi {
    repo: Arc<PlanRhythmRepository>,
    action_log_repo: Arc<ActionLogRepository>,
    config_manager: Arc<ConfigManager>,
}

impl RhythmApi {
    pub fn new(
        repo: Arc<PlanRhythmRepository>,
        action_log_repo: Arc<ActionLogRepository>,
        config_manager: Arc<ConfigManager>,
    ) -> Self {
        Self {
            repo,
            action_log_repo,
            config_manager,
        }
    }

    fn normalize_date_str(value: &str) -> ApiResult<String> {
        let raw = value.trim();
        if raw.is_empty() {
            return Err(ApiError::InvalidInput("日期不能为空".to_string()));
        }
        let d = NaiveDate::parse_from_str(raw, "%Y-%m-%d")
            .map_err(|_| ApiError::InvalidInput("日期格式错误（应为 YYYY-MM-DD）".to_string()))?;
        Ok(d.format("%Y-%m-%d").to_string())
    }

    fn validate_dimension(value: &str) -> ApiResult<String> {
        let dim = value.trim();
        if dim.is_empty() {
            return Err(ApiError::InvalidInput("维度不能为空".to_string()));
        }
        if dim != DIM_PRODUCT_CATEGORY {
            return Err(ApiError::InvalidInput(format!(
                "暂不支持的节奏维度: {}（当前仅支持 {}）",
                dim, DIM_PRODUCT_CATEGORY
            )));
        }
        Ok(dim.to_string())
    }

    fn parse_and_normalize_target_json(target_json: &str) -> ApiResult<String> {
        let raw = target_json.trim();
        if raw.is_empty() {
            return Err(ApiError::InvalidInput("目标配比 JSON 不能为空".to_string()));
        }

        let parsed: serde_json::Value = serde_json::from_str(raw)
            .map_err(|e| ApiError::InvalidInput(format!("目标配比 JSON 解析失败: {}", e)))?;

        let obj = parsed.as_object().ok_or_else(|| {
            ApiError::InvalidInput("目标配比应为 JSON 对象，如 {\"普板\":0.3}".to_string())
        })?;

        if obj.is_empty() {
            // 允许空对象：表示不启用/清空目标
            return Ok("{}".to_string());
        }

        let mut ratios: BTreeMap<String, f64> = BTreeMap::new();
        for (k, v) in obj {
            let key = k.trim();
            if key.is_empty() {
                continue;
            }
            let ratio = v.as_f64().ok_or_else(|| {
                ApiError::InvalidInput(format!("目标配比 {} 的值必须为数字", key))
            })?;
            if !ratio.is_finite() {
                return Err(ApiError::InvalidInput(format!("目标配比 {} 的值非法", key)));
            }
            if ratio < 0.0 {
                return Err(ApiError::InvalidInput(format!(
                    "目标配比 {} 不能为负数",
                    key
                )));
            }
            ratios.insert(key.to_string(), ratio);
        }

        if ratios.is_empty() {
            return Ok("{}".to_string());
        }

        let sum: f64 = ratios.values().sum();
        if sum <= 0.0 {
            return Err(ApiError::InvalidInput("目标配比之和必须大于 0".to_string()));
        }

        // 自动归一化（提升手工输入容错）
        let normalized: BTreeMap<String, f64> =
            ratios.into_iter().map(|(k, v)| (k, v / sum)).collect();

        serde_json::to_string(&normalized)
            .map_err(|e| ApiError::InvalidInput(format!("目标配比序列化失败: {}", e)))
    }

    pub fn list_presets(&self, dimension: Option<&str>) -> ApiResult<Vec<PlanRhythmPresetInfo>> {
        self.list_presets_with_active(dimension, true)
    }

    pub fn list_presets_with_active(
        &self,
        dimension: Option<&str>,
        active_only: bool,
    ) -> ApiResult<Vec<PlanRhythmPresetInfo>> {
        let dim = match dimension {
            Some(v) if !v.trim().is_empty() => Some(Self::validate_dimension(v)?),
            _ => None,
        };

        let presets = self
            .repo
            .list_presets(dim.as_deref(), active_only)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(presets
            .into_iter()
            .map(PlanRhythmPresetInfo::from)
            .collect())
    }

    pub fn upsert_preset(
        &self,
        preset_id: Option<&str>,
        preset_name: &str,
        dimension: &str,
        target_json: &str,
        is_active: Option<bool>,
        operator: &str,
        reason: &str,
    ) -> ApiResult<PlanRhythmPresetInfo> {
        let name = preset_name.trim();
        if name.is_empty() {
            return Err(ApiError::InvalidInput("模板名称不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        let dim = Self::validate_dimension(dimension)?;
        let normalized_target_json = Self::parse_and_normalize_target_json(target_json)?;

        let id = preset_id
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let entity = PlanRhythmPresetEntity {
            preset_id: id.clone(),
            preset_name: name.to_string(),
            dimension: dim.clone(),
            target_json: normalized_target_json.clone(),
            is_active: is_active.unwrap_or(true),
            created_at: now.clone(),
            updated_at: now.clone(),
            updated_by: Some(operator.to_string()),
        };

        self.repo
            .upsert_preset(&entity)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None,
            action_type: "UPSERT_PLAN_RHYTHM_PRESET".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "preset_id": id,
                "preset_name": name,
                "dimension": dim,
                "target_json": normalized_target_json,
                "is_active": entity.is_active,
                "reason": reason,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("更新节奏模板: {}", name)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let saved = self
            .repo
            .find_preset_by_id(&entity.preset_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::DatabaseError("保存后未找到节奏模板".to_string()))?;

        Ok(PlanRhythmPresetInfo::from(saved))
    }

    pub fn set_preset_active(
        &self,
        preset_id: &str,
        is_active: bool,
        operator: &str,
        reason: &str,
    ) -> ApiResult<PlanRhythmPresetInfo> {
        let id = preset_id.trim();
        if id.is_empty() {
            return Err(ApiError::InvalidInput("模板ID不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        let exists = self
            .repo
            .find_preset_by_id(id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        if exists.is_none() {
            return Err(ApiError::InvalidInput(format!("未找到节奏模板: {}", id)));
        }

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let affected = self
            .repo
            .set_preset_active(id, is_active, &now, Some(operator))
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
        if affected == 0 {
            return Err(ApiError::DatabaseError("更新模板状态失败".to_string()));
        }

        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None,
            action_type: "SET_PLAN_RHYTHM_PRESET_ACTIVE".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "preset_id": id,
                "is_active": is_active,
                "reason": reason,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!(
                "{}节奏模板: {}",
                if is_active { "启用" } else { "停用" },
                id
            )),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let saved = self
            .repo
            .find_preset_by_id(id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::DatabaseError("更新后未找到节奏模板".to_string()))?;

        Ok(PlanRhythmPresetInfo::from(saved))
    }

    pub fn list_targets(
        &self,
        version_id: &str,
        dimension: &str,
        machine_codes: Option<&[String]>,
        date_from: Option<&str>,
        date_to: Option<&str>,
    ) -> ApiResult<Vec<PlanRhythmTargetInfo>> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }

        let dim = Self::validate_dimension(dimension)?;

        let date_range = match (date_from, date_to) {
            (Some(s), Some(e)) if !s.trim().is_empty() && !e.trim().is_empty() => {
                let start = Self::normalize_date_str(s)?;
                let end = Self::normalize_date_str(e)?;
                Some((start, end))
            }
            _ => None,
        };

        let range_refs = date_range.as_ref().map(|(s, e)| (s.as_str(), e.as_str()));

        let targets = self
            .repo
            .list_targets(version_id, &dim, machine_codes, range_refs)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(targets
            .into_iter()
            .map(PlanRhythmTargetInfo::from)
            .collect())
    }

    pub fn upsert_target(
        &self,
        version_id: &str,
        machine_code: &str,
        plan_date: &str,
        dimension: &str,
        target_json: &str,
        preset_id: Option<&str>,
        operator: &str,
        reason: &str,
    ) -> ApiResult<()> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if machine_code.trim().is_empty() {
            return Err(ApiError::InvalidInput("机组代码不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        let plan_date = Self::normalize_date_str(plan_date)?;
        let plan_date_dt = NaiveDate::parse_from_str(&plan_date, "%Y-%m-%d")
            .map_err(|_| ApiError::InvalidInput("日期格式错误（应为 YYYY-MM-DD）".to_string()))?;
        let dim = Self::validate_dimension(dimension)?;
        let normalized_target_json = Self::parse_and_normalize_target_json(target_json)?;

        // 校验 preset_id（可选）：若传入则必须存在
        let preset_id = match preset_id {
            Some(v) if !v.trim().is_empty() => {
                let id = v.trim();
                let preset = self
                    .repo
                    .find_preset_by_id(id)
                    .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
                if preset.is_none() {
                    return Err(ApiError::InvalidInput(format!("未找到节奏模板: {}", id)));
                }
                Some(id.to_string())
            }
            _ => None,
        };

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let entity = PlanRhythmTargetEntity {
            version_id: version_id.to_string(),
            machine_code: machine_code.to_string(),
            plan_date: plan_date.clone(),
            dimension: dim.clone(),
            target_json: normalized_target_json.clone(),
            preset_id: preset_id.clone(),
            updated_at: now.clone(),
            updated_by: Some(operator.to_string()),
        };

        self.repo
            .upsert_target(&entity)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version_id.to_string()),
            action_type: "UPSERT_PLAN_RHYTHM_TARGET".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "machine_code": machine_code,
                "plan_date": plan_date,
                "dimension": dim,
                "preset_id": preset_id,
                "target_json": normalized_target_json,
                "reason": reason,
            })),
            impact_summary_json: None,
            machine_code: Some(machine_code.to_string()),
            date_range_start: Some(plan_date_dt),
            date_range_end: Some(plan_date_dt),
            detail: Some(format!("更新每日节奏目标: {} {}", machine_code, plan_date)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub fn apply_preset(
        &self,
        version_id: &str,
        dimension: &str,
        preset_id: &str,
        machine_codes: &[String],
        date_from: &str,
        date_to: &str,
        overwrite: bool,
        operator: &str,
        reason: &str,
    ) -> ApiResult<usize> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if preset_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("模板ID不能为空".to_string()));
        }
        if machine_codes.is_empty() {
            return Err(ApiError::InvalidInput("机组列表不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        let dim = Self::validate_dimension(dimension)?;
        let start = NaiveDate::parse_from_str(&Self::normalize_date_str(date_from)?, "%Y-%m-%d")
            .map_err(|_| ApiError::InvalidInput("开始日期格式错误".to_string()))?;
        let end = NaiveDate::parse_from_str(&Self::normalize_date_str(date_to)?, "%Y-%m-%d")
            .map_err(|_| ApiError::InvalidInput("结束日期格式错误".to_string()))?;
        if end < start {
            return Err(ApiError::InvalidInput(
                "结束日期不能早于开始日期".to_string(),
            ));
        }

        let preset = self
            .repo
            .find_preset_by_id(preset_id)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ApiError::InvalidInput(format!("未找到节奏模板: {}", preset_id)))?;

        // 确保 preset 的 target_json 可解析且归一化（避免历史脏数据）
        let normalized_target_json = Self::parse_and_normalize_target_json(&preset.target_json)?;

        let start_str = start.format("%Y-%m-%d").to_string();
        let end_str = end.format("%Y-%m-%d").to_string();

        let mut existing: HashMap<(String, String), bool> = HashMap::new();
        if !overwrite {
            let targets = self
                .repo
                .list_targets(
                    version_id,
                    &dim,
                    Some(machine_codes),
                    Some((start_str.as_str(), end_str.as_str())),
                )
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
            for t in targets {
                existing.insert((t.machine_code, t.plan_date), true);
            }
        }

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut entities: Vec<PlanRhythmTargetEntity> = Vec::new();
        let mut d = start;
        while d <= end {
            let date_str = d.format("%Y-%m-%d").to_string();
            for mc in machine_codes {
                if !overwrite && existing.contains_key(&(mc.clone(), date_str.clone())) {
                    continue;
                }
                entities.push(PlanRhythmTargetEntity {
                    version_id: version_id.to_string(),
                    machine_code: mc.clone(),
                    plan_date: date_str.clone(),
                    dimension: dim.clone(),
                    target_json: normalized_target_json.clone(),
                    preset_id: Some(preset_id.to_string()),
                    updated_at: now.clone(),
                    updated_by: Some(operator.to_string()),
                });
            }
            d += Duration::days(1);
        }

        let applied = self
            .repo
            .batch_upsert_targets(&entities)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: Some(version_id.to_string()),
            action_type: "APPLY_PLAN_RHYTHM_PRESET".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "dimension": dim,
                "preset_id": preset_id,
                "machine_codes": machine_codes,
                "date_from": start_str,
                "date_to": end_str,
                "overwrite": overwrite,
                "applied_count": applied,
                "reason": reason,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: Some(start),
            date_range_end: Some(end),
            detail: Some(format!("批量应用节奏模板: {} ({} 条)", preset_id, applied)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(applied)
    }

    pub fn get_daily_profile(
        &self,
        version_id: &str,
        machine_code: &str,
        plan_date: &str,
    ) -> ApiResult<DailyRhythmProfile> {
        if version_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("版本ID不能为空".to_string()));
        }
        if machine_code.trim().is_empty() {
            return Err(ApiError::InvalidInput("机组代码不能为空".to_string()));
        }

        let plan_date = Self::normalize_date_str(plan_date)?;

        // target
        let target_entity = self
            .repo
            .find_target(version_id, machine_code, &plan_date, DIM_PRODUCT_CATEGORY)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let mut target_map: HashMap<String, f64> = HashMap::new();
        let (target_preset_id, target_updated_at, target_updated_by) =
            if let Some(t) = &target_entity {
                let json_str = t.target_json.as_str();
                if let Ok(map) = serde_json::from_str::<HashMap<String, f64>>(json_str) {
                    target_map = map;
                }
                (
                    t.preset_id.clone(),
                    Some(t.updated_at.clone()),
                    t.updated_by.clone(),
                )
            } else {
                (None, None, None)
            };

        // actual weights by category (scheduled plan items)
        let weights = self
            .repo
            .get_scheduled_weights_by_category(version_id, machine_code, &plan_date)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let total: f64 = weights.values().sum();
        let total = if total.is_finite() { total } else { 0.0 };

        let mut actual_ratio: HashMap<String, f64> = HashMap::new();
        if total > 0.0 {
            for (k, w) in &weights {
                actual_ratio.insert(k.clone(), w / total);
            }
        }

        let deviation_threshold = self
            .config_manager
            .get_global_config_value(config_keys::RHYTHM_DEVIATION_THRESHOLD)
            .ok()
            .flatten()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .or_else(|| {
                // backward compatible fallback
                self.config_manager
                    .get_global_config_value(config_keys::DEVIATION_THRESHOLD)
                    .ok()
                    .flatten()
                    .and_then(|v| v.trim().parse::<f64>().ok())
            })
            .unwrap_or(0.1);

        let max_deviation = calculate_max_deviation(&actual_ratio, &target_map);
        let is_violated = max_deviation > deviation_threshold && !target_map.is_empty();

        // build rows: union of categories
        let mut all_categories: Vec<String> = Vec::new();
        for k in actual_ratio.keys() {
            all_categories.push(k.clone());
        }
        for k in target_map.keys() {
            if !actual_ratio.contains_key(k) {
                all_categories.push(k.clone());
            }
        }

        // sort by scheduled weight desc, then name
        all_categories.sort_by(|a, b| {
            let wa = weights.get(a).copied().unwrap_or(0.0);
            let wb = weights.get(b).copied().unwrap_or(0.0);
            wb.partial_cmp(&wa)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.cmp(b))
        });
        all_categories.dedup();

        let categories = all_categories
            .into_iter()
            .map(|cat| {
                let scheduled_weight_t = weights.get(&cat).copied().unwrap_or(0.0);
                let actual = actual_ratio.get(&cat).copied().unwrap_or(0.0);
                let target = target_map.get(&cat).copied();
                let diff = target.map(|t| (actual - t).abs());
                DailyRhythmCategoryRow {
                    category: cat,
                    scheduled_weight_t,
                    actual_ratio: actual,
                    target_ratio: target,
                    diff_ratio: diff,
                }
            })
            .collect::<Vec<_>>();

        Ok(DailyRhythmProfile {
            version_id: version_id.to_string(),
            machine_code: machine_code.to_string(),
            plan_date,
            dimension: DIM_PRODUCT_CATEGORY.to_string(),
            total_scheduled_weight_t: total,
            deviation_threshold,
            max_deviation,
            is_violated,
            target_preset_id,
            target_updated_at,
            target_updated_by,
            categories,
        })
    }
}

fn calculate_max_deviation(
    actual_ratio: &HashMap<String, f64>,
    target_ratio: &HashMap<String, f64>,
) -> f64 {
    if target_ratio.is_empty() {
        return 0.0;
    }

    let mut max_dev: f64 = 0.0;
    for (k, target) in target_ratio {
        let actual = actual_ratio.get(k).copied().unwrap_or(0.0);
        max_dev = max_dev.max((actual - target).abs());
    }

    // categories present in actual but not in target: deviation=actual_ratio
    for (k, actual) in actual_ratio {
        if !target_ratio.contains_key(k) {
            max_dev = max_dev.max(*actual);
        }
    }

    max_dev
}

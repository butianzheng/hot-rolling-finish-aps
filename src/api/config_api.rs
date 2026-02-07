// ==========================================
// 热轧精整排产系统 - 配置管理 API
// ==========================================
// 职责: 配置查询、更新、快照管理
// 依据: Engine_Specs_v0.3_Integrated.md - 11. 配置项全集
// ==========================================

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection};
use std::sync::Mutex;

use crate::api::error::{ApiError, ApiResult};
use crate::config::config_manager::ConfigManager;
use crate::config::strategy_profile::{CustomStrategyParameters, CustomStrategyProfile};
use crate::domain::action_log::ActionLog;
use crate::repository::action_log_repo::ActionLogRepository;
use crate::engine::ScheduleStrategy;

// ==========================================
// ConfigApi - 配置管理 API
// ==========================================

/// 配置管理API
///
/// 职责：
/// 1. 配置查询（全部、单个）
/// 2. 配置更新（单个、批量）
/// 3. 配置快照管理
/// 4. ActionLog记录
pub struct ConfigApi {
    conn: Arc<Mutex<Connection>>,
    config_manager: Arc<ConfigManager>,
    action_log_repo: Arc<ActionLogRepository>,
}

const CUSTOM_STRATEGY_KEY_PREFIX: &str = "custom_strategy/";

impl ConfigApi {
    /// 创建新的ConfigApi实例
    pub fn new(
        conn: Arc<Mutex<Connection>>,
        config_manager: Arc<ConfigManager>,
        action_log_repo: Arc<ActionLogRepository>,
    ) -> Self {
        Self {
            conn,
            config_manager,
            action_log_repo,
        }
    }

    /// 查询所有配置
    ///
    /// # 返回
    /// - Ok(Vec<ConfigItem>): 配置列表
    /// - Err(ApiError): API错误
    pub fn list_configs(&self) -> ApiResult<Vec<ConfigItem>> {
        let conn = self.conn.lock()
            .map_err(|e| ApiError::DatabaseError(format!("锁获取失败: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT kv.scope_id, sc.scope_type, kv.key, kv.value, kv.updated_at
                 FROM config_kv kv
                 JOIN config_scope sc ON kv.scope_id = sc.scope_id
                 ORDER BY sc.scope_type, kv.scope_id, kv.key"
            )
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let configs = stmt
            .query_map([], |row| {
                Ok(ConfigItem {
                    scope_id: row.get(0)?,
                    scope_type: row.get(1)?,
                    key: row.get(2)?,
                    value: row.get(3)?,
                    updated_at: row.get(4).ok(),
                })
            })
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(configs)
    }

    /// 查询单个配置
    ///
    /// # 参数
    /// - scope_id: 作用域ID（如"global"）
    /// - key: 配置键
    ///
    /// # 返回
    /// - Ok(Some(ConfigItem)): 配置项
    /// - Ok(None): 配置不存在
    /// - Err(ApiError): API错误
    pub fn get_config(&self, scope_id: &str, key: &str) -> ApiResult<Option<ConfigItem>> {
        let conn = self.conn.lock()
            .map_err(|e| ApiError::DatabaseError(format!("锁获取失败: {}", e)))?;

        let result = conn.query_row(
            "SELECT kv.scope_id, sc.scope_type, kv.key, kv.value, kv.updated_at
             FROM config_kv kv
             JOIN config_scope sc ON kv.scope_id = sc.scope_id
             WHERE kv.scope_id = ?1 AND kv.key = ?2",
            params![scope_id, key],
            |row| {
                Ok(ConfigItem {
                    scope_id: row.get(0)?,
                    scope_type: row.get(1)?,
                    key: row.get(2)?,
                    value: row.get(3)?,
                    updated_at: row.get(4).ok(),
                })
            },
        );

        match result {
            Ok(config) => Ok(Some(config)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ApiError::DatabaseError(e.to_string())),
        }
    }

    /// 更新配置
    ///
    /// # 参数
    /// - scope_id: 作用域ID
    /// - key: 配置键
    /// - value: 配置值
    /// - operator: 操作人
    /// - reason: 操作原因
    ///
    /// # 返回
    /// - Ok(()): 成功
    /// - Err(ApiError): API错误
    pub fn update_config(
        &self,
        scope_id: &str,
        key: &str,
        value: &str,
        operator: &str,
        reason: &str,
    ) -> ApiResult<()> {
        // 参数验证
        if scope_id.trim().is_empty() {
            return Err(ApiError::InvalidInput("作用域ID不能为空".to_string()));
        }
        if key.trim().is_empty() {
            return Err(ApiError::InvalidInput("配置键不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        let conn = self.conn.lock()
            .map_err(|e| ApiError::DatabaseError(format!("锁获取失败: {}", e)))?;

        // 检查 scope_id 是否存在于 config_scope 表
        let scope_exists: bool = conn.query_row(
            "SELECT COUNT(*) FROM config_scope WHERE scope_id = ?1",
            params![scope_id],
            |row| row.get::<_, i64>(0).map(|count| count > 0),
        )
        .unwrap_or(false);

        if !scope_exists {
            return Err(ApiError::InvalidInput(format!(
                "作用域ID '{}' 在 config_scope 表中不存在",
                scope_id
            )));
        }

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 使用UPSERT语法，正确处理 updated_at 字段
        conn.execute(
            "INSERT INTO config_kv (scope_id, key, value, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(scope_id, key) DO UPDATE SET value = ?3, updated_at = ?4",
            params![scope_id, key, value, now],
        )
        .map_err(|e| ApiError::DatabaseError(format!("更新配置失败: {}", e)))?;

        drop(conn);
        self.config_manager.invalidate_cache_all();

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None,
            action_type: "UPDATE_CONFIG".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "scope_id": scope_id,
                "key": key,
                "value": value,
                "reason": reason,
            })),
            impact_summary_json: None,
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("更新配置: {}={}", key, value)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// 批量更新配置
    ///
    /// # 参数
    /// - configs: 配置列表
    /// - operator: 操作人
    /// - reason: 操作原因
    ///
    /// # 返回
    /// - Ok(usize): 更新的配置数量
    /// - Err(ApiError): API错误
    pub fn batch_update_configs(
        &self,
        configs: Vec<ConfigItem>,
        operator: &str,
        reason: &str,
    ) -> ApiResult<usize> {
        if configs.is_empty() {
            return Err(ApiError::InvalidInput("配置列表不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        let conn = self.conn.lock()
            .map_err(|e| ApiError::DatabaseError(format!("锁获取失败: {}", e)))?;

        conn.execute("BEGIN TRANSACTION", [])
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let mut count = 0;
        for config in &configs {
            let affected = conn
                .execute(
                    "INSERT INTO config_kv (scope_id, key, value) VALUES (?1, ?2, ?3)
                     ON CONFLICT(scope_id, key) DO UPDATE SET value = ?3",
                    params![config.scope_id, config.key, config.value],
                )
                .map_err(|e| {
                    let _ = conn.execute("ROLLBACK", []);
                    ApiError::DatabaseError(e.to_string())
                })?;
            count += affected;
        }

        conn.execute("COMMIT", [])
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        drop(conn);
        self.config_manager.invalidate_cache_all();

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None,
            action_type: "BATCH_UPDATE_CONFIG".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "configs": configs,
                "reason": reason,
            })),
            impact_summary_json: Some(serde_json::json!({
                "updated_count": count,
            })),
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("批量更新{}个配置", count)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(count)
    }

    /// 获取配置快照
    ///
    /// # 返回
    /// - Ok(String): 配置快照JSON
    /// - Err(ApiError): API错误
    pub fn get_config_snapshot(&self) -> ApiResult<String> {
        self.config_manager
            .get_config_snapshot()
            .map_err(|e| ApiError::InternalError(e.to_string()))
    }

    /// 从快照恢复配置
    ///
    /// # 参数
    /// - snapshot_json: 配置快照JSON
    /// - operator: 操作人
    /// - reason: 操作原因
    ///
    /// # 返回
    /// - Ok(usize): 恢复的配置数量
    /// - Err(ApiError): API错误
    pub fn restore_from_snapshot(
        &self,
        snapshot_json: &str,
        operator: &str,
        reason: &str,
    ) -> ApiResult<usize> {
        if snapshot_json.trim().is_empty() {
            return Err(ApiError::InvalidInput("快照JSON不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        let count = self
            .config_manager
            .restore_config_from_snapshot(snapshot_json)
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None,
            action_type: "RESTORE_CONFIG".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "snapshot_json": snapshot_json,
                "reason": reason,
            })),
            impact_summary_json: Some(serde_json::json!({
                "restored_count": count,
            })),
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!("从快照恢复{}个配置", count)),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(count)
    }

    // ==========================================
    // 自定义策略（P2）
    // ==========================================

    /// 保存自定义策略（持久化到 config_kv，不改表结构）
    ///
    /// 存储规则：
    /// - scope_id 固定为 'global'
    /// - key = "custom_strategy/{strategy_id}"
    /// - value = CustomStrategyProfile 的 JSON
    ///
    /// # 注意
    /// - 本接口只负责“保存 + 校验 + 审计(ActionLog)”，不直接影响排产结果；
    /// - 后续在草案对比/一键优化选择“自定义策略”时，可复用该存储。
    pub fn save_custom_strategy(
        &self,
        mut profile: CustomStrategyProfile,
        operator: &str,
        reason: &str,
    ) -> ApiResult<SaveCustomStrategyResponse> {
        if operator.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作人不能为空".to_string()));
        }
        if reason.trim().is_empty() {
            return Err(ApiError::InvalidInput("操作原因不能为空".to_string()));
        }

        profile.strategy_id = profile.strategy_id.trim().to_string();
        profile.title = profile.title.trim().to_string();
        if let Some(desc) = profile.description.as_ref() {
            let trimmed = desc.trim().to_string();
            profile.description = if trimmed.is_empty() { None } else { Some(trimmed) };
        }

        validate_custom_strategy_profile(&profile)?;

        let key = format!("{}{}", CUSTOM_STRATEGY_KEY_PREFIX, profile.strategy_id);
        let value = serde_json::to_string(&profile)
            .map_err(|e| ApiError::ValidationError(format!("序列化自定义策略失败: {}", e)))?;

        let conn = self.conn.lock()
            .map_err(|e| ApiError::DatabaseError(format!("锁获取失败: {}", e)))?;

        // 判断是新增还是覆盖（用于返回与审计提示）
        let existed: bool = conn
            .query_row(
                "SELECT 1 FROM config_kv WHERE scope_id = 'global' AND key = ?1 LIMIT 1",
                params![&key],
                |_| Ok::<_, rusqlite::Error>(()),
            )
            .is_ok();

        conn.execute(
            "INSERT INTO config_kv (scope_id, key, value) VALUES ('global', ?1, ?2)
             ON CONFLICT(scope_id, key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
            params![&key, &value],
        )
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        drop(conn);
        self.config_manager.invalidate_cache_all();

        // 记录ActionLog（可解释性红线：所有写操作必须有审计）
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: None,
            action_type: "SAVE_CUSTOM_STRATEGY".to_string(),
            action_ts: chrono::Local::now().naive_local(),
            actor: operator.to_string(),
            payload_json: Some(serde_json::json!({
                "strategy_id": profile.strategy_id,
                "title": profile.title,
                "base_strategy": profile.base_strategy,
                "parameters": profile.parameters,
                "reason": reason,
                "stored_key": key,
            })),
            impact_summary_json: Some(serde_json::json!({
                "existed": existed,
            })),
            machine_code: None,
            date_range_start: None,
            date_range_end: None,
            detail: Some(format!(
                "{}自定义策略: {} ({})",
                if existed { "更新" } else { "新增" },
                profile.title,
                profile.strategy_id
            )),
        };

        self.action_log_repo
            .insert(&action_log)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(SaveCustomStrategyResponse {
            strategy_id: profile.strategy_id,
            stored_key: key,
            existed,
            message: if existed {
                "自定义策略已更新".to_string()
            } else {
                "自定义策略已保存".to_string()
            },
        })
    }

    /// 查询所有自定义策略（从 config_kv 扫描 key 前缀）
    pub fn list_custom_strategies(&self) -> ApiResult<Vec<CustomStrategyProfile>> {
        let conn = self.conn.lock()
            .map_err(|e| ApiError::DatabaseError(format!("锁获取失败: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT key, value FROM config_kv WHERE scope_id = 'global' AND key LIKE ?1 ORDER BY key",
            )
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let like = format!("{}%", CUSTOM_STRATEGY_KEY_PREFIX);

        let rows = stmt
            .query_map(params![like], |row| {
                let _key: String = row.get(0)?;
                let value: String = row.get(1)?;
                Ok(value)
            })
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let mut out = Vec::new();
        for row in rows {
            let raw = row.map_err(|e| ApiError::DatabaseError(e.to_string()))?;
            let profile: CustomStrategyProfile = serde_json::from_str(&raw).map_err(|e| {
                ApiError::ValidationError(format!("解析自定义策略失败（请检查 config_kv 的 JSON）: {}", e))
            })?;
            // 读取时也做一次基础校验，避免脏数据污染前端
            validate_custom_strategy_profile(&profile)?;
            out.push(profile);
        }

        Ok(out)
    }
}

// ==========================================
// DTO 类型定义
// ==========================================

/// 配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigItem {
    /// 作用域ID
    pub scope_id: String,

    /// 作用域类型（GLOBAL, MACHINE, STEEL_GRADE, VERSION）
    #[serde(default)]
    pub scope_type: String,

    /// 配置键
    pub key: String,

    /// 配置值
    pub value: String,

    /// 最后更新时间（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// 保存自定义策略响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveCustomStrategyResponse {
    pub strategy_id: String,
    pub stored_key: String,
    pub existed: bool,
    pub message: String,
}

fn validate_custom_strategy_profile(profile: &CustomStrategyProfile) -> ApiResult<()> {
    if profile.strategy_id.trim().is_empty() {
        return Err(ApiError::InvalidInput("strategy_id 不能为空".to_string()));
    }
    if profile.strategy_id.len() > 64 {
        return Err(ApiError::InvalidInput("strategy_id 过长（最多64字符）".to_string()));
    }
    if !profile
        .strategy_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ApiError::InvalidInput(
            "strategy_id 只能包含字母/数字/下划线/连字符".to_string(),
        ));
    }

    if profile.title.trim().is_empty() {
        return Err(ApiError::InvalidInput("title 不能为空".to_string()));
    }
    if profile.title.len() > 80 {
        return Err(ApiError::InvalidInput("title 过长（最多80字符）".to_string()));
    }

    // base_strategy 必须是已知预设之一，避免把“临时字符串”误当作策略导致不可复现。
    profile
        .base_strategy
        .parse::<ScheduleStrategy>()
        .map_err(|e| ApiError::InvalidInput(format!("base_strategy 无效: {}", e)))?;

    validate_custom_strategy_parameters(&profile.parameters)?;

    Ok(())
}

fn validate_custom_strategy_parameters(params: &CustomStrategyParameters) -> ApiResult<()> {
    fn check_weight(name: &str, v: Option<f64>) -> ApiResult<()> {
        if let Some(x) = v {
            if !x.is_finite() {
                return Err(ApiError::InvalidInput(format!("{} 必须为有效数字", name)));
            }
            if x < 0.0 || x > 100.0 {
                return Err(ApiError::InvalidInput(format!(
                    "{} 超出范围（0~100）",
                    name
                )));
            }
        }
        Ok(())
    }

    check_weight("urgent_weight", params.urgent_weight)?;
    check_weight("capacity_weight", params.capacity_weight)?;
    check_weight("cold_stock_weight", params.cold_stock_weight)?;
    check_weight("due_date_weight", params.due_date_weight)?;
    check_weight("rolling_output_age_weight", params.rolling_output_age_weight)?;

    if let Some(days) = params.cold_stock_age_threshold_days {
        if days < 0 || days > 365 {
            return Err(ApiError::InvalidInput(
                "cold_stock_age_threshold_days 超出范围（0~365）".to_string(),
            ));
        }
    }

    if let Some(pct) = params.overflow_tolerance_pct {
        if !pct.is_finite() {
            return Err(ApiError::InvalidInput(
                "overflow_tolerance_pct 必须为有效数字".to_string(),
            ));
        }
        if pct < 0.0 || pct > 1.0 {
            return Err(ApiError::InvalidInput(
                "overflow_tolerance_pct 超出范围（0~1）".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_api_structure() {
        // 这个测试只是验证结构是否正确定义
        // 实际的集成测试在 tests/ 目录
    }

    #[test]
    fn test_config_item_deserialize_without_scope_type() {
        let raw = r#"[{"scope_id":"global","key":"k1","value":"v1"}]"#;
        let parsed: Vec<ConfigItem> = serde_json::from_str(raw).expect("反序列化应成功");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].scope_id, "global");
        assert_eq!(parsed[0].scope_type, "");
        assert_eq!(parsed[0].key, "k1");
        assert_eq!(parsed[0].value, "v1");
    }
}

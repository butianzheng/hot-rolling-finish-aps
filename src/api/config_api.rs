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
use crate::domain::action_log::ActionLog;
use crate::repository::action_log_repo::ActionLogRepository;

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
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare("SELECT scope_id, key, value FROM config_kv ORDER BY scope_id, key")
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let configs = stmt
            .query_map([], |row| {
                Ok(ConfigItem {
                    scope_id: row.get(0)?,
                    key: row.get(1)?,
                    value: row.get(2)?,
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
        let conn = self.conn.lock().unwrap();

        let result = conn.query_row(
            "SELECT scope_id, key, value FROM config_kv WHERE scope_id = ?1 AND key = ?2",
            params![scope_id, key],
            |row| {
                Ok(ConfigItem {
                    scope_id: row.get(0)?,
                    key: row.get(1)?,
                    value: row.get(2)?,
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

        let conn = self.conn.lock().unwrap();

        // 使用UPSERT语法
        conn.execute(
            "INSERT INTO config_kv (scope_id, key, value) VALUES (?1, ?2, ?3)
             ON CONFLICT(scope_id, key) DO UPDATE SET value = ?3",
            params![scope_id, key, value],
        )
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        drop(conn);

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: "N/A".to_string(),
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

        let conn = self.conn.lock().unwrap();

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

        // 记录ActionLog
        let action_log = ActionLog {
            action_id: uuid::Uuid::new_v4().to_string(),
            version_id: "N/A".to_string(),
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
            version_id: "N/A".to_string(),
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
}

// ==========================================
// DTO 类型定义
// ==========================================

/// 配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigItem {
    /// 作用域ID
    pub scope_id: String,

    /// 配置键
    pub key: String,

    /// 配置值
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_api_structure() {
        // 这个测试只是验证结构是否正确定义
        // 实际的集成测试在 tests/ 目录
    }
}

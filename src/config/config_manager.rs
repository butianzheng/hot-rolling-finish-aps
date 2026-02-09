// ==========================================
// 热轧精整排产系统 - 配置管理器
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 11. 配置项全集
// ==========================================
// 职责: 配置加载、查询、覆写管理
// 存储: config_kv 表 (key-value + scope)
// ==========================================

use crate::config::import_config_trait::ImportConfigReader;
use crate::config::strategy_profile::CustomStrategyProfile;
use crate::db::open_sqlite_connection;
use crate::domain::types::{Season, SeasonMode};
use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use rusqlite::{params, Connection};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::sync::RwLock;

#[derive(Debug, Default)]
struct ParsedConfigCache {
    season_mode: Option<SeasonMode>,
    manual_season: Option<Season>,
    winter_months: Option<Vec<u32>>,
    min_temp_days_winter: Option<i32>,
    min_temp_days_summer: Option<i32>,
    standard_finishing_machines: Option<Vec<String>>,
    machine_offset_days: Option<i32>,
    urgent_n1_days: Option<i32>,
    urgent_n2_days: Option<i32>,
}

// ==========================================
// ConfigManager - 配置管理器
// ==========================================
pub struct ConfigManager {
    conn: Arc<Mutex<Connection>>,
    cache: RwLock<HashMap<String, Option<String>>>,
    parsed_cache: RwLock<ParsedConfigCache>,
}

impl ConfigManager {
    /// 创建新的 ConfigManager 实例
    ///
    /// # 参数
    /// - db_path: 数据库文件路径
    pub fn new(db_path: &str) -> Result<Self, Box<dyn Error>> {
        let conn = open_sqlite_connection(db_path)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            cache: RwLock::new(HashMap::new()),
            parsed_cache: RwLock::new(ParsedConfigCache::default()),
        })
    }

    /// 从已有连接创建 ConfigManager
    ///
    /// 说明：为保证连接行为一致，会对传入连接再次应用统一 PRAGMA（幂等）。
    pub fn from_connection(conn: Arc<Mutex<Connection>>) -> Result<Self, Box<dyn Error>> {
        {
            let conn_guard = conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;
            crate::db::configure_sqlite_connection(&conn_guard)?;
        }

        Ok(Self {
            conn,
            cache: RwLock::new(HashMap::new()),
            parsed_cache: RwLock::new(ParsedConfigCache::default()),
        })
    }

    /// 从 config_kv 表读取配置值（scope_id='global'）
    ///
    /// # 参数
    /// - key: 配置键
    ///
    /// # 返回
    /// - Some(String): 配置值
    /// - None: 配置不存在
    fn get_config_value(&self, key: &str) -> Result<Option<String>, Box<dyn Error>> {
        if let Ok(cache) = self.cache.read() {
            if let Some(v) = cache.get(key) {
                return Ok(v.clone());
            }
        }

        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        let result = conn.query_row(
            "SELECT value FROM config_kv WHERE scope_id = 'global' AND key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        );

        let value = match result {
            Ok(value) => Some(value),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(Box::new(e)),
        };

        if let Ok(mut cache) = self.cache.write() {
            cache.insert(key.to_string(), value.clone());
        }

        Ok(value)
    }

    /// 失效缓存（配置写入后调用，确保后续读取拿到最新值）
    pub fn invalidate_cache_all(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
        if let Ok(mut parsed) = self.parsed_cache.write() {
            *parsed = ParsedConfigCache::default();
        }
    }

    /// 读取 global scope 的配置值（公开方法，供其他模块复用）
    pub fn get_global_config_value(&self, key: &str) -> Result<Option<String>, Box<dyn Error>> {
        self.get_config_value(key)
    }

    /// 读取自定义策略配置（存储于 config_kv: custom_strategy/{strategy_id}）
    pub fn get_custom_strategy_profile(
        &self,
        strategy_id: &str,
    ) -> Result<Option<CustomStrategyProfile>, Box<dyn Error>> {
        let id = strategy_id.trim();
        if id.is_empty() {
            return Ok(None);
        }

        let key = format!("custom_strategy/{}", id);
        let raw = match self.get_config_value(&key)? {
            Some(v) => v,
            None => return Ok(None),
        };

        let profile: CustomStrategyProfile = serde_json::from_str(&raw)?;
        Ok(Some(profile))
    }

    /// 从 config_kv 表读取配置值，带默认值
    ///
    /// # 参数
    /// - key: 配置键
    /// - default: 默认值
    fn get_config_or_default(&self, key: &str, default: &str) -> Result<String, Box<dyn Error>> {
        Ok(self.get_config_value(key)?.unwrap_or_else(|| default.to_string()))
    }

    /// 获取所有配置的快照（JSON格式）
    ///
    /// # 返回
    /// - Ok(String): 配置快照的JSON字符串
    /// - Err: 获取失败
    ///
    /// # 用途
    /// - 在创建/重算版本时记录配置快照
    /// - 保证版本回滚时配置一致性
    pub fn get_config_snapshot(&self) -> Result<String, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        // 查询所有global scope的配置
        let mut stmt = conn.prepare(
            "SELECT key, value FROM config_kv WHERE scope_id = 'global' ORDER BY key"
        )?;

        let mut config_map: HashMap<String, String> = HashMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
            ))
        })?;

        for row in rows {
            let (key, value) = row?;
            config_map.insert(key, value);
        }

        // 序列化为JSON
        let json_value = json!(config_map);
        Ok(serde_json::to_string(&json_value)?)
    }

    /// 从配置快照恢复配置
    ///
    /// # 参数
    /// - snapshot_json: 配置快照的JSON字符串
    ///
    /// # 返回
    /// - Ok(usize): 恢复的配置项数量
    /// - Err: 恢复失败
    ///
    /// # 注意
    /// - 此方法会覆盖现有的global配置
    /// - 仅用于版本回滚场景
    pub fn restore_config_from_snapshot(&self, snapshot_json: &str) -> Result<usize, Box<dyn Error>> {
        // 解析JSON
        let config_map: HashMap<String, String> = serde_json::from_str(snapshot_json)?;

        let conn = self.conn.lock().map_err(|e| format!("锁获取失败: {}", e))?;

        // 开启事务
        conn.execute("BEGIN TRANSACTION", [])?;

        let mut count = 0;
        for (key, value) in config_map.iter() {
            // 版本的 config_snapshot_json 里可能包含元信息（例如版本中文命名），不应回写到 config_kv。
            if key.starts_with("__meta_") {
                continue;
            }
            // 使用UPSERT语法（SQLite 3.24.0+）
            let affected = conn.execute(
                "INSERT INTO config_kv (scope_id, key, value) VALUES ('global', ?1, ?2)
                 ON CONFLICT(scope_id, key) DO UPDATE SET value = ?2",
                params![key, value],
            )?;
            count += affected;
        }

        // 提交事务
        conn.execute("COMMIT", [])?;

        // 配置被写入后，清理缓存，避免读到旧值
        self.invalidate_cache_all();

        Ok(count)
    }

    // ===== 结构校正配置 =====

    /// 获取目标钢种配比
    ///
    /// # 返回
    /// - HashMap<String, f64>: 钢种代码 → 目标配比
    ///
    /// # 说明
    /// 配置格式为 JSON: {"钢种A": 0.3, "钢种B": 0.5, "钢种C": 0.2}
    /// 如果配置不存在或格式错误，返回空 HashMap（不启用结构校正）
    pub async fn get_target_ratio(&self) -> Result<HashMap<String, f64>, Box<dyn Error>> {
        let value = self.get_config_or_default(config_keys::TARGET_RATIO, "{}")?;
        let ratio: HashMap<String, f64> = serde_json::from_str(&value)
            .unwrap_or_else(|_| {
                tracing::warn!(
                    config_key = config_keys::TARGET_RATIO,
                    raw_value = %value,
                    "目标配比配置格式错误，使用空配置"
                );
                HashMap::new()
            });
        Ok(ratio)
    }

    /// 获取结构偏差阈值
    ///
    /// # 返回
    /// - f64: 偏差阈值（默认 0.1 = 10%）
    pub async fn get_deviation_threshold(&self) -> Result<f64, Box<dyn Error>> {
        let value = self.get_config_or_default(config_keys::DEVIATION_THRESHOLD, "0.1")?;
        Ok(value.parse::<f64>().unwrap_or(0.1))
    }
}

// ==========================================
// ImportConfigReader Trait 实现
// ==========================================
#[async_trait]
impl ImportConfigReader for ConfigManager {
    // ===== 季节与适温配置 =====

    async fn get_season_mode(&self) -> Result<SeasonMode, Box<dyn Error>> {
        if let Ok(parsed) = self.parsed_cache.read() {
            if let Some(mode) = parsed.season_mode {
                return Ok(mode);
            }
        }

        let value = self.get_config_or_default("season_mode", "AUTO")?;
        let mode = match value.to_uppercase().as_str() {
            "AUTO" => SeasonMode::Auto,
            "MANUAL" => SeasonMode::Manual,
            _ => SeasonMode::Auto, // 默认 AUTO
        };

        if let Ok(mut parsed) = self.parsed_cache.write() {
            parsed.season_mode = Some(mode);
        }

        Ok(mode)
    }

    async fn get_manual_season(&self) -> Result<Season, Box<dyn Error>> {
        if let Ok(parsed) = self.parsed_cache.read() {
            if let Some(season) = parsed.manual_season {
                return Ok(season);
            }
        }

        let value = self.get_config_or_default("manual_season", "WINTER")?;
        let season = match value.to_uppercase().as_str() {
            "WINTER" => Season::Winter,
            "SUMMER" => Season::Summer,
            _ => Season::Winter, // 默认 WINTER
        };

        if let Ok(mut parsed) = self.parsed_cache.write() {
            parsed.manual_season = Some(season);
        }

        Ok(season)
    }

    async fn get_winter_months(&self) -> Result<Vec<u32>, Box<dyn Error>> {
        if let Ok(parsed) = self.parsed_cache.read() {
            if let Some(months) = parsed.winter_months.as_ref() {
                return Ok(months.clone());
            }
        }

        let value = self.get_config_or_default("winter_months", "11,12,1,2,3")?;

        let months: Vec<u32> = value
            .split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .filter(|&m| m >= 1 && m <= 12)
            .collect();

        let months = if months.is_empty() {
            vec![11, 12, 1, 2, 3] // 默认值
        } else {
            months
        };

        if let Ok(mut parsed) = self.parsed_cache.write() {
            parsed.winter_months = Some(months.clone());
        }

        Ok(months)
    }

    async fn get_min_temp_days_winter(&self) -> Result<i32, Box<dyn Error>> {
        if let Ok(parsed) = self.parsed_cache.read() {
            if let Some(v) = parsed.min_temp_days_winter {
                return Ok(v);
            }
        }

        let value = self.get_config_or_default("min_temp_days_winter", "3")?;
        let v = value.parse::<i32>().unwrap_or(3);

        if let Ok(mut parsed) = self.parsed_cache.write() {
            parsed.min_temp_days_winter = Some(v);
        }

        Ok(v)
    }

    async fn get_min_temp_days_summer(&self) -> Result<i32, Box<dyn Error>> {
        if let Ok(parsed) = self.parsed_cache.read() {
            if let Some(v) = parsed.min_temp_days_summer {
                return Ok(v);
            }
        }

        let value = self.get_config_or_default("min_temp_days_summer", "4")?;
        let v = value.parse::<i32>().unwrap_or(4);

        if let Ok(mut parsed) = self.parsed_cache.write() {
            parsed.min_temp_days_summer = Some(v);
        }

        Ok(v)
    }

    async fn get_current_min_temp_days(
        &self,
        today: chrono::NaiveDate,
    ) -> Result<i32, Box<dyn Error>> {
        let season_mode = self.get_season_mode().await?;

        let current_season = match season_mode {
            SeasonMode::Auto => {
                // 根据月份自动判断季节
                let winter_months = self.get_winter_months().await?;
                let current_month = today.month();

                if winter_months.contains(&current_month) {
                    Season::Winter
                } else {
                    Season::Summer
                }
            }
            SeasonMode::Manual => {
                // 使用手动指定的季节
                self.get_manual_season().await?
            }
        };

        match current_season {
            Season::Winter => self.get_min_temp_days_winter().await,
            Season::Summer => self.get_min_temp_days_summer().await,
        }
    }

    // ===== 机组代码配置 =====

    async fn get_standard_finishing_machines(&self) -> Result<Vec<String>, Box<dyn Error>> {
        if let Ok(parsed) = self.parsed_cache.read() {
            if let Some(machines) = parsed.standard_finishing_machines.as_ref() {
                return Ok(machines.clone());
            }
        }

        let value = self.get_config_or_default("standard_finishing_machines", "H032,H033,H034")?;

        let machines: Vec<String> = value
            .split(',')
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();

        let machines = if machines.is_empty() {
            vec!["H032".to_string(), "H033".to_string(), "H034".to_string()]
        } else {
            machines
        };

        if let Ok(mut parsed) = self.parsed_cache.write() {
            parsed.standard_finishing_machines = Some(machines.clone());
        }

        Ok(machines)
    }

    async fn get_machine_offset_days(&self) -> Result<i32, Box<dyn Error>> {
        if let Ok(parsed) = self.parsed_cache.read() {
            if let Some(v) = parsed.machine_offset_days {
                return Ok(v);
            }
        }

        let value = self.get_config_or_default("machine_offset_days", "4")?;
        let v = value.parse::<i32>().unwrap_or(4);

        if let Ok(mut parsed) = self.parsed_cache.write() {
            parsed.machine_offset_days = Some(v);
        }

        Ok(v)
    }

    // ===== 紧急等级阈值配置 =====

    async fn get_n1_threshold_days(&self) -> Result<i32, Box<dyn Error>> {
        if let Ok(parsed) = self.parsed_cache.read() {
            if let Some(v) = parsed.urgent_n1_days {
                return Ok(v);
            }
        }

        let value = self.get_config_or_default(config_keys::URGENT_N1_DAYS, "3")?;
        let v = value.parse::<i32>().unwrap_or(3);

        if let Ok(mut parsed) = self.parsed_cache.write() {
            parsed.urgent_n1_days = Some(v);
        }

        Ok(v)
    }

    async fn get_n2_threshold_days(&self) -> Result<i32, Box<dyn Error>> {
        if let Ok(parsed) = self.parsed_cache.read() {
            if let Some(v) = parsed.urgent_n2_days {
                return Ok(v);
            }
        }

        let value = self.get_config_or_default(config_keys::URGENT_N2_DAYS, "7")?;
        let v = value.parse::<i32>().unwrap_or(7);

        if let Ok(mut parsed) = self.parsed_cache.write() {
            parsed.urgent_n2_days = Some(v);
        }

        Ok(v)
    }

    // ===== 数据质量配置 =====

    async fn get_weight_anomaly_threshold(&self) -> Result<f64, Box<dyn Error>> {
        let value = self.get_config_or_default("weight_anomaly_threshold", "100.0")?;
        Ok(value.parse::<f64>().unwrap_or(100.0))
    }

    async fn get_batch_retention_days(&self) -> Result<i32, Box<dyn Error>> {
        let value = self.get_config_or_default("batch_retention_days", "90")?;
        Ok(value.parse::<i32>().unwrap_or(90))
    }
}

// ==========================================
// ConfigScope - 配置作用域
// ==========================================
#[derive(Debug, Clone)]
pub enum ConfigScope {
    Global,                                  // 全局
    Machine { machine_code: String },        // 机组
    SteelGrade { steel_grade: String },      // 钢种
    Date { date: NaiveDate },                // 日期
    MachineSteelGrade {                      // 机组+钢种
        machine_code: String,
        steel_grade: String,
    },
}

// ==========================================
// 配置键常量 (依据 Engine_Specs 11)
// ==========================================
pub mod config_keys {
    // 季节模式
    pub const SEASON_MODE: &str = "season_mode";
    pub const WINTER_MONTHS: &str = "winter_months";
    pub const MANUAL_SEASON: &str = "manual_season";

    // 适温天数
    pub const MIN_TEMP_DAYS_WINTER: &str = "min_temp_days_winter";
    pub const MIN_TEMP_DAYS_SUMMER: &str = "min_temp_days_summer";

    // 紧急等级
    pub const URGENT_N1_DAYS: &str = "urgent_n1_days";
    pub const URGENT_N2_DAYS: &str = "urgent_n2_days";

    // 换辊
    pub const ROLL_SUGGEST_THRESHOLD_T: &str = "roll_suggest_threshold_t";
    pub const ROLL_HARD_LIMIT_T: &str = "roll_hard_limit_t";
    pub const ROLL_CHANGE_DOWNTIME_MINUTES: &str = "roll_change_downtime_minutes";

    // 连续排程兜底
    pub const EMPTY_DAY_RECOVER_THRESHOLD_T: &str = "empty_day_recover_threshold_t";

    // 产能
    pub const OVERFLOW_PCT: &str = "overflow_pct";

    // 重算
    pub const RECALC_WINDOW_DAYS: &str = "recalc_window_days";
    pub const CASCADE_WINDOW_DAYS: &str = "cascade_window_days";

    // 结构校正
    pub const TARGET_RATIO: &str = "target_ratio";           // 目标钢种配比 (JSON)
    pub const DEVIATION_THRESHOLD: &str = "deviation_threshold"; // 偏差阈值

    // 每日生产节奏（品种大类等）
    // 说明：与结构校正的 deviation_threshold 口径解耦，避免相互影响。
    pub const RHYTHM_DEVIATION_THRESHOLD: &str = "rhythm_deviation_threshold";
}

// TODO: 实现错误处理
// TODO: 实现配置验证
// TODO: 实现配置导入/导出

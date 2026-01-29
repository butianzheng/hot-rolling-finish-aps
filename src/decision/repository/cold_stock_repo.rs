// ==========================================
// 热轧精整排产系统 - D3 仓储实现
// ==========================================
// 依据: DECISION_READ_MODELS.md - D3 表设计
// 职责: "哪些冷料压库" 数据访问层
// ==========================================

use crate::decision::common::{
    build_in_clause, build_optional_filter_sql, deserialize_json_array,
    deserialize_json_array_optional, serialize_json_vec,
};
use crate::decision::use_cases::d3_cold_stock::{
    AgeStockStat, ColdStockProfile, ColdStockSummary, MachineStockStat,
};
use rusqlite::{params, Connection, Result as SqlResult};
use serde_json;
use std::sync::{Arc, Mutex};

/// D3 仓储：冷料压库
pub struct ColdStockRepository {
    /// 数据库连接
    conn: Arc<Mutex<Connection>>,
}

impl ColdStockRepository {
    /// 创建新的冷料压库仓储
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 查询冷料压库概况
    ///
    /// # 参数
    /// - `version_id`: 方案版本 ID
    /// - `machine_code`: 可选机组代码 (None 表示所有机组)
    ///
    /// # 返回
    /// 按 pressure_score 降序的冷料分桶列表
    pub fn get_cold_stock_profile(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
    ) -> SqlResult<Vec<ColdStockProfile>> {
        let conn = self.conn.lock().map_err(|e| rusqlite::Error::InvalidParameterName(format!("锁获取失败: {}", e)))?;

        let base_sql = r#"
            SELECT
                version_id,
                machine_code,
                age_bin,
                age_min_days,
                age_max_days,
                count,
                weight_t,
                pressure_score,
                pressure_level,
                reasons,
                structure_gap,
                estimated_ready_date,
                can_force_release,
                suggested_actions
            FROM decision_cold_stock_profile
            WHERE version_id = ?
        "#;

        let additional_filter = machine_code.map(|_| "machine_code = ?");
        let sql = build_optional_filter_sql(base_sql, additional_filter, "pressure_score DESC");

        let map_row = |row: &rusqlite::Row| -> SqlResult<ColdStockProfile> {
            let reasons_json: String = row.get(9)?;
            let reasons: Vec<String> = deserialize_json_array(&reasons_json);

            let structure_gap: Option<String> = row.get(10)?;
            let estimated_ready_date: Option<String> = row.get(11)?;
            let can_force_release: i32 = row.get(12)?;

            let suggested_actions_json: Option<String> = row.get(13)?;
            let suggested_actions: Vec<String> =
                deserialize_json_array_optional(suggested_actions_json.as_deref());

            Ok(ColdStockProfile {
                version_id: row.get(0)?,
                machine_code: row.get(1)?,
                age_bin: row.get(2)?,
                age_min_days: row.get(3)?,
                age_max_days: row.get(4)?,
                count: row.get(5)?,
                weight_t: row.get(6)?,
                pressure_score: row.get(7)?,
                pressure_level: row.get(8)?,
                reasons,
                structure_gap,
                estimated_ready_date,
                can_force_release: can_force_release != 0,
                suggested_actions,
            })
        };

        let mut stmt = conn.prepare(&sql)?;
        let rows = if let Some(mc) = machine_code {
            stmt.query_map(params![version_id, mc], map_row)?
        } else {
            stmt.query_map(params![version_id], map_row)?
        };

        rows.collect()
    }

    /// 查询特定机组的冷料分桶
    pub fn get_machine_cold_stock(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> SqlResult<Vec<ColdStockProfile>> {
        self.get_cold_stock_profile(version_id, Some(machine_code))
    }

    /// 统计冷料总量
    pub fn get_cold_stock_summary(&self, version_id: &str) -> SqlResult<ColdStockSummary> {
        let conn = self.conn.lock().map_err(|e| rusqlite::Error::InvalidParameterName(format!("锁获取失败: {}", e)))?;

        // 查询总体统计
        let mut stmt = conn.prepare(
            r#"
            SELECT
                SUM(count) AS total_count,
                SUM(weight_t) AS total_weight_t,
                SUM(CASE WHEN pressure_level IN ('HIGH', 'CRITICAL') THEN 1 ELSE 0 END) AS high_pressure_count
            FROM (
                SELECT DISTINCT machine_code, pressure_level, count, weight_t
                FROM decision_cold_stock_profile
                WHERE version_id = ?
            )
        "#,
        )?;

        let (total_count, total_weight_t, high_pressure_machines) =
            stmt.query_row(params![version_id], |row| {
                Ok((
                    row.get::<_, Option<i32>>(0)?.unwrap_or(0),
                    row.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                    row.get::<_, Option<i32>>(2)?.unwrap_or(0),
                ))
            })?;

        // 按机组分组统计
        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code,
                SUM(count) AS count,
                SUM(weight_t) AS weight_t,
                MAX(pressure_score) AS pressure_score
            FROM decision_cold_stock_profile
            WHERE version_id = ?
            GROUP BY machine_code
            ORDER BY pressure_score DESC
        "#,
        )?;

        let by_machine: Vec<MachineStockStat> = stmt
            .query_map(params![version_id], |row| {
                Ok(MachineStockStat {
                    machine_code: row.get(0)?,
                    count: row.get(1)?,
                    weight_t: row.get(2)?,
                    pressure_score: row.get(3)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        // 按年龄分组统计
        let mut stmt = conn.prepare(
            r#"
            SELECT
                age_bin,
                SUM(count) AS count,
                SUM(weight_t) AS weight_t
            FROM decision_cold_stock_profile
            WHERE version_id = ?
            GROUP BY age_bin
            ORDER BY age_min_days
        "#,
        )?;

        let by_age: Vec<AgeStockStat> = stmt
            .query_map(params![version_id], |row| {
                Ok(AgeStockStat {
                    age_bin: row.get(0)?,
                    count: row.get(1)?,
                    weight_t: row.get(2)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        // 计算平均年龄
        let mut stmt = conn.prepare(
            r#"
            SELECT AVG(age_min_days)
            FROM decision_cold_stock_profile
            WHERE version_id = ?
        "#,
        )?;

        let avg_age_days = stmt
            .query_row(params![version_id], |row| {
                row.get::<_, Option<f64>>(0)
            })?
            .unwrap_or(0.0);

        Ok(ColdStockSummary {
            version_id: version_id.to_string(),
            total_count,
            total_weight_t,
            by_machine,
            by_age,
            high_pressure_machines,
            avg_age_days,
        })
    }

    /// 全量刷新 D3 读模型
    ///
    /// # 逻辑
    /// 1. 查询所有未排产的冷料材料 (is_mature = 0)
    /// 2. 按机组 + 年龄分桶聚合
    /// 3. 计算压库分数和等级
    /// 4. 分析结构缺口
    /// 5. 生成建议措施
    pub fn refresh_full(&self, version_id: &str) -> SqlResult<usize> {
        let conn = self.conn.lock().map_err(|e| rusqlite::Error::InvalidParameterName(format!("锁获取失败: {}", e)))?;

        // 1. 删除旧数据
        conn.execute(
            "DELETE FROM decision_cold_stock_profile WHERE version_id = ?",
            params![version_id],
        )?;

        // 2. 查询所有未排产的冷料材料并按机组 + 年龄分桶聚合
        let cold_materials = self.query_cold_materials(&conn, version_id, None)?;

        // 3. 按机组 + 年龄分桶分组
        let mut profiles = std::collections::HashMap::new();

        for material in cold_materials {
            let age_bin = determine_age_bin(material.stock_age_days);
            let key = (material.machine_code.clone(), age_bin.clone());

            let profile = profiles
                .entry(key.clone())
                .or_insert_with(|| ColdStockProfile::new(
                    version_id.to_string(),
                    material.machine_code.clone(),
                    age_bin.clone(),
                    get_age_min(age_bin.as_str()),
                    get_age_max(age_bin.as_str()),
                ));

            profile.count += 1;
            profile.weight_t += material.weight_t;
        }

        // 4. 计算压库分数、分析结构缺口、生成建议
        for ((_machine_code, _age_bin), profile) in profiles.iter_mut() {
            calculate_pressure_score(profile);
            analyze_structure_gap(&conn, version_id, profile)?;
            generate_suggestions(profile);
        }

        // 5. 插入到数据库
        let mut inserted = 0;
        for profile in profiles.values() {
            self.insert_profile(&conn, profile)?;
            inserted += 1;
        }

        Ok(inserted)
    }

    /// 增量刷新 D3 读模型 (按机组)
    ///
    /// # 参数
    /// - `version_id`: 方案版本 ID
    /// - `machine_codes`: 受影响的机组列表
    pub fn refresh_incremental(
        &self,
        version_id: &str,
        machine_codes: &[String],
    ) -> SqlResult<usize> {
        if machine_codes.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.lock().map_err(|e| rusqlite::Error::InvalidParameterName(format!("锁获取失败: {}", e)))?;

        // 1. 删除受影响机组的记录
        let in_clause = build_in_clause("machine_code", machine_codes);
        let delete_sql = format!(
            "DELETE FROM decision_cold_stock_profile WHERE version_id = ? AND {}",
            in_clause
        );

        let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&version_id];
        for mc in machine_codes {
            params_vec.push(mc);
        }

        conn.execute(&delete_sql, params_vec.as_slice())?;

        // 2. 查询受影响机组的冷料材料
        let cold_materials = self.query_cold_materials(&conn, version_id, Some(machine_codes))?;

        // 3. 按机组 + 年龄分桶分组
        let mut profiles = std::collections::HashMap::new();

        for material in cold_materials {
            let age_bin = determine_age_bin(material.stock_age_days);
            let key = (material.machine_code.clone(), age_bin.clone());

            let profile = profiles
                .entry(key.clone())
                .or_insert_with(|| ColdStockProfile::new(
                    version_id.to_string(),
                    material.machine_code.clone(),
                    age_bin.clone(),
                    get_age_min(age_bin.as_str()),
                    get_age_max(age_bin.as_str()),
                ));

            profile.count += 1;
            profile.weight_t += material.weight_t;
        }

        // 4. 计算压库分数、分析结构缺口、生成建议
        for ((_machine_code, _age_bin), profile) in profiles.iter_mut() {
            calculate_pressure_score(profile);
            analyze_structure_gap(&conn, version_id, profile)?;
            generate_suggestions(profile);
        }

        // 5. 插入到数据库
        let mut inserted = 0;
        for profile in profiles.values() {
            self.insert_profile(&conn, profile)?;
            inserted += 1;
        }

        Ok(inserted)
    }

    /// 插入冷料概况记录
    fn insert_profile(&self, conn: &Connection, profile: &ColdStockProfile) -> SqlResult<()> {
        let reasons_json = serde_json::to_string(&profile.reasons).unwrap_or_default();
        let suggested_actions_json = serialize_json_vec(&profile.suggested_actions);

        conn.execute(
            r#"
            INSERT INTO decision_cold_stock_profile (
                version_id, machine_code, age_bin,
                age_min_days, age_max_days,
                count, weight_t,
                pressure_score, pressure_level,
                reasons, structure_gap, estimated_ready_date,
                can_force_release, suggested_actions
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
            params![
                profile.version_id,
                profile.machine_code,
                profile.age_bin,
                profile.age_min_days,
                profile.age_max_days,
                profile.count,
                profile.weight_t,
                profile.pressure_score,
                profile.pressure_level,
                reasons_json,
                profile.structure_gap,
                profile.estimated_ready_date,
                if profile.can_force_release { 1 } else { 0 },
                suggested_actions_json,
            ],
        )?;

        Ok(())
    }

    /// 查询冷料材料
    fn query_cold_materials(
        &self,
        conn: &Connection,
        version_id: &str,
        machine_codes: Option<&[String]>,
    ) -> SqlResult<Vec<ColdMaterial>> {
        let mut sql = r#"
            SELECT
                ms.material_id,
                ms.machine_code,
                ms.weight_t,
                ms.stock_age_days,
                ms.spec_width_mm,
                ms.spec_thick_mm
            FROM material_state ms
            WHERE ms.is_mature = 0
              AND NOT EXISTS (
                  SELECT 1 FROM plan_item pi
                  WHERE pi.version_id = ? AND pi.material_id = ms.material_id
              )
        "#
        .to_string();

        let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&version_id];

        if let Some(mcs) = machine_codes {
            if !mcs.is_empty() {
                let in_clause = build_in_clause("ms.machine_code", mcs);
                sql.push_str(&format!(" AND {}", in_clause));
                for mc in mcs {
                    params_vec.push(mc);
                }
            }
        }

        let mut stmt = conn.prepare(&sql)?;
        let materials = stmt
            .query_map(params_vec.as_slice(), |row| {
                Ok(ColdMaterial {
                    material_id: row.get(0)?,
                    machine_code: row.get(1)?,
                    weight_t: row.get(2)?,
                    stock_age_days: row.get(3)?,
                    spec_width_mm: row.get(4)?,
                    spec_thick_mm: row.get(5)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(materials)
    }
}

/// 冷料材料信息
#[derive(Debug)]
struct ColdMaterial {
    material_id: String,
    machine_code: String,
    weight_t: f64,
    stock_age_days: i32,
    spec_width_mm: Option<f64>,
    spec_thick_mm: Option<f64>,
}

/// 确定年龄分桶
fn determine_age_bin(stock_age_days: i32) -> String {
    match stock_age_days {
        0..=7 => "0-7".to_string(),
        8..=14 => "8-14".to_string(),
        15..=30 => "15-30".to_string(),
        _ => "30+".to_string(),
    }
}

/// 获取年龄分桶的最小天数
fn get_age_min(age_bin: &str) -> i32 {
    match age_bin {
        "0-7" => 0,
        "8-14" => 8,
        "15-30" => 15,
        "30+" => 30,
        _ => 0,
    }
}

/// 获取年龄分桶的最大天数
fn get_age_max(age_bin: &str) -> Option<i32> {
    match age_bin {
        "0-7" => Some(7),
        "8-14" => Some(14),
        "15-30" => Some(30),
        "30+" => None,
        _ => None,
    }
}

/// 计算压库分数和等级
fn calculate_pressure_score(profile: &mut ColdStockProfile) {
    // 年龄因子 (60% 权重)
    let age_factor = match profile.age_min_days {
        0..=7 => 0.2,
        8..=14 => 0.4,
        15..=30 => 0.7,
        _ => 1.0,
    };

    // 数量因子 (40% 权重)
    let count_factor = match profile.count {
        0..=5 => 0.3,
        6..=10 => 0.6,
        11..=20 => 0.8,
        _ => 1.0,
    };

    // 计算压库分数
    profile.pressure_score = (age_factor * 0.6 + count_factor * 0.4) * 100.0;

    // 确定压库等级
    profile.pressure_level = match profile.pressure_score {
        s if s >= 80.0 => "CRITICAL".to_string(),
        s if s >= 60.0 => "HIGH".to_string(),
        s if s >= 40.0 => "MEDIUM".to_string(),
        _ => "LOW".to_string(),
    };

    // 生成压库原因
    let mut reasons = Vec::new();

    if profile.age_min_days >= 30 {
        reasons.push(format!("年龄超过 30 天 (平均 {:.0} 天)", profile.avg_age()));
    } else if profile.age_min_days >= 15 {
        reasons.push(format!(
            "年龄在 15-30 天区间 (平均 {:.0} 天)",
            profile.avg_age()
        ));
    }

    if profile.count >= 20 {
        reasons.push(format!("冷料数量较多 ({} 块)", profile.count));
    } else if profile.count >= 10 {
        reasons.push(format!("冷料数量中等 ({} 块)", profile.count));
    }

    if profile.weight_t >= 1000.0 {
        reasons.push(format!("冷料重量较大 ({:.1} 吨)", profile.weight_t));
    }

    profile.reasons = reasons;
}

/// 分析结构缺口
fn analyze_structure_gap(
    conn: &Connection,
    version_id: &str,
    profile: &mut ColdStockProfile,
) -> SqlResult<()> {
    // 查询该机组该年龄分桶的冷料的规格分布
    let mut stmt = conn.prepare(
        r#"
        SELECT
            ms.spec_width_mm,
            ms.spec_thick_mm,
            COUNT(*) AS count
        FROM material_state ms
        WHERE ms.machine_code = ?
          AND ms.is_mature = 0
          AND ms.stock_age_days BETWEEN ? AND ?
          AND NOT EXISTS (
              SELECT 1 FROM plan_item pi
              WHERE pi.version_id = ? AND pi.material_id = ms.material_id
          )
        GROUP BY ms.spec_width_mm, ms.spec_thick_mm
        ORDER BY count DESC
        LIMIT 1
    "#,
    )?;

    let age_max = profile.age_max_days.unwrap_or(999);
    let result = stmt.query_row(
        params![
            profile.machine_code,
            profile.age_min_days,
            age_max,
            version_id
        ],
        |row| {
            Ok((
                row.get::<_, Option<f64>>(0)?,
                row.get::<_, Option<f64>>(1)?,
                row.get::<_, i32>(2)?,
            ))
        },
    );

    if let Ok((width, thick, count)) = result {
        if count >= 5 {
            let width_str = width.map_or("未知".to_string(), |w| format!("{:.0}mm", w));
            let thick_str = thick.map_or("未知".to_string(), |t| format!("{:.1}mm", t));
            profile.structure_gap =
                Some(format!("集中在规格 {}×{} ({} 块)", width_str, thick_str, count));
        }
    }

    Ok(())
}

/// 生成建议措施
fn generate_suggestions(profile: &mut ColdStockProfile) {
    let mut suggestions = Vec::new();

    // 根据压库等级生成建议
    match profile.pressure_level.as_str() {
        "CRITICAL" => {
            suggestions.push("紧急: 立即评估强制释放冷料的可行性".to_string());
            suggestions.push("考虑联系客户协商提前适温或调整规格".to_string());
        }
        "HIGH" => {
            suggestions.push("建议: 优先安排该机组的冷料适温".to_string());
            suggestions.push("评估是否可以通过调整产能参数加快释放".to_string());
        }
        "MEDIUM" => {
            suggestions.push("监控: 关注冷料适温进度,避免长期压库".to_string());
        }
        _ => {}
    }

    // 根据年龄生成建议
    if profile.age_min_days >= 30 {
        suggestions.push("长期库存: 建议人工审核,评估报废或降级可能".to_string());
        profile.can_force_release = true;
    }

    // 根据结构缺口生成建议
    if profile.structure_gap.is_some() {
        suggestions.push("结构集中: 考虑调整产能池结构参数以释放冷料".to_string());
    }

    profile.suggested_actions = suggestions;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_determine_age_bin() {
        assert_eq!(determine_age_bin(0), "0-7");
        assert_eq!(determine_age_bin(5), "0-7");
        assert_eq!(determine_age_bin(7), "0-7");
        assert_eq!(determine_age_bin(8), "8-14");
        assert_eq!(determine_age_bin(14), "8-14");
        assert_eq!(determine_age_bin(15), "15-30");
        assert_eq!(determine_age_bin(30), "15-30");
        assert_eq!(determine_age_bin(31), "30+");
        assert_eq!(determine_age_bin(100), "30+");
    }

    #[test]
    fn test_calculate_pressure_score() {
        // 测试高压库: 30+ 天, 25 块
        let mut profile = ColdStockProfile::new(
            "V001".to_string(),
            "H032".to_string(),
            "30+".to_string(),
            30,
            None,
        );
        profile.count = 25;
        profile.weight_t = 1250.0;

        calculate_pressure_score(&mut profile);

        assert!(profile.pressure_score >= 80.0);
        assert_eq!(profile.pressure_level, "CRITICAL");
        assert!(!profile.reasons.is_empty());
    }

    #[test]
    fn test_refresh_full() {
        let conn = Connection::open_in_memory().unwrap();

        // 创建必要的表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS plan_version (
                version_id TEXT PRIMARY KEY,
                plan_id TEXT NOT NULL,
                version_no INTEGER NOT NULL,
                status TEXT NOT NULL,
                created_by TEXT NOT NULL
            )
            "#,
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS material_state (
                material_id TEXT PRIMARY KEY,
                machine_code TEXT NOT NULL,
                weight_t REAL NOT NULL,
                stock_age_days INTEGER NOT NULL DEFAULT 0,
                is_mature INTEGER NOT NULL DEFAULT 1,
                spec_width_mm REAL,
                spec_thick_mm REAL
            )
            "#,
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS plan_item (
                version_id TEXT NOT NULL,
                material_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                PRIMARY KEY (version_id, material_id)
            )
            "#,
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS decision_cold_stock_profile (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                age_bin TEXT NOT NULL,
                age_min_days INTEGER NOT NULL,
                age_max_days INTEGER,
                count INTEGER NOT NULL,
                weight_t REAL NOT NULL,
                pressure_score REAL NOT NULL,
                pressure_level TEXT NOT NULL,
                reasons TEXT NOT NULL,
                structure_gap TEXT,
                estimated_ready_date TEXT,
                can_force_release INTEGER NOT NULL DEFAULT 0,
                suggested_actions TEXT,
                refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (version_id, machine_code, age_bin)
            )
            "#,
            [],
        )
        .unwrap();

        // 插入测试数据
        conn.execute(
            "INSERT INTO plan_version VALUES ('V001', 'P001', 1, 'ACTIVE', 'test')",
            [],
        )
        .unwrap();

        // H032: 5 块冷料 (0-7天), 8 块冷料 (15-30天), 3 块冷料 (30+天)
        for i in 1..=5 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'H032', 100.0, 5, 0, 1250.0, 3.5)",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
        }

        for i in 6..=13 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'H032', 150.0, 20, 0, 1250.0, 3.5)",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
        }

        for i in 14..=16 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'H032', 120.0, 35, 0, 1500.0, 4.0)",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
        }

        // H033: 10 块冷料 (8-14天)
        for i in 17..=26 {
            conn.execute(
                "INSERT INTO material_state VALUES (?, 'H033', 80.0, 10, 0, 1000.0, 2.5)",
                params![format!("MAT{:03}", i)],
            )
            .unwrap();
        }

        let conn_arc = Arc::new(Mutex::new(conn));
        let repo = ColdStockRepository::new(conn_arc);

        // 执行全量刷新
        let result = repo.refresh_full("V001").unwrap();

        // 验证结果
        assert!(result > 0);

        // 查询冷料概况
        let profiles = repo.get_cold_stock_profile("V001", None).unwrap();
        assert!(profiles.len() > 0);

        // 验证按压库分数降序排列
        for i in 1..profiles.len() {
            assert!(profiles[i - 1].pressure_score >= profiles[i].pressure_score);
        }
    }
}

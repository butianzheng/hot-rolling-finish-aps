// ==========================================
// 热轧精整排产系统 - D4 机组堵塞仓储
// ==========================================
// 依据: spec/DecisionApi_Contract_v1.0.md - D4 用例
// 职责: 查询机组堵塞概况数据
// ==========================================
// P2 阶段: 优先从 decision_machine_bottleneck 读模型表读取
//         如果读模型表为空，回退到 capacity_pool/plan_item 实时计算
// ==========================================

use crate::decision::common::sql_builder::SqlQueryBuilder;
use crate::decision::use_cases::d4_machine_bottleneck::{
    BottleneckHeatmap, MachineBottleneckProfile,
};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

/// D4 机组堵塞仓储
///
/// 职责: 查询机组堵塞概况数据
/// 策略: 优先从 decision_machine_bottleneck 读模型表读取，回退到 capacity_pool/plan_item 实时计算
pub struct BottleneckRepository {
    conn: Arc<Mutex<Connection>>,
}

impl BottleneckRepository {
    /// 创建新的 BottleneckRepository 实例
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 查询机组堵塞概况
    ///
    /// 策略: 优先从 decision_machine_bottleneck 读模型表读取，如果为空则回退到实时计算
    ///
    /// # 参数
    /// - version_id: 方案版本 ID
    /// - machine_code: 机组代码（可选）
    /// - start_date: 开始日期
    /// - end_date: 结束日期
    ///
    /// # 返回
    /// - Ok(Vec<MachineBottleneckProfile>): 机组堵塞概况列表，按堵塞分数降序排列
    /// - Err: 数据库错误
    pub fn get_bottleneck_profile(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<MachineBottleneckProfile>, Box<dyn Error>> {
        // 优先尝试从读模型表读取
        if let Ok(profiles) = self.get_bottleneck_from_read_model(version_id, machine_code, start_date, end_date) {
            if !profiles.is_empty() {
                tracing::debug!(
                    version_id = version_id,
                    count = profiles.len(),
                    "D4: 从 decision_machine_bottleneck 读模型表读取"
                );
                return Ok(profiles);
            }
        }

        // 回退到实时计算
        tracing::debug!(
            version_id = version_id,
            "D4: 回退到 capacity_pool/plan_item 实时计算"
        );
        self.get_bottleneck_realtime(version_id, machine_code, start_date, end_date)
    }

    /// 从 decision_machine_bottleneck 读模型表读取（P2 优先路径）
    fn get_bottleneck_from_read_model(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<MachineBottleneckProfile>, Box<dyn Error>> {
        let conn = self.conn.lock().expect("锁获取失败");

        let machine_filter = machine_code.map(|_| "machine_code = ?");

        let sql = SqlQueryBuilder::new(
            r#"SELECT
                machine_code,
                plan_date,
                bottleneck_score,
                bottleneck_level,
                bottleneck_types,
                reasons,
                remaining_capacity_t,
                capacity_utilization,
                needs_roll_change,
                structure_violations,
                pending_materials,
                suggested_actions
            FROM decision_machine_bottleneck"#,
        )
        .where_clause("version_id = ?")
        .where_clause("plan_date >= ?")
        .where_clause("plan_date <= ?")
        .and_if(machine_filter)
        .order_by("bottleneck_score DESC")
        .build();

        let mut stmt = conn.prepare(&sql)?;

        // 构建参数
        let profiles = if let Some(mc) = machine_code {
            stmt.query_map(params![version_id, start_date, end_date, mc], |row| {
                Self::map_read_model_row(row, version_id)
            })?
            .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![version_id, start_date, end_date], |row| {
                Self::map_read_model_row(row, version_id)
            })?
            .collect::<Result<Vec<_>, _>>()?
        };

        Ok(profiles)
    }

    /// 映射读模型表行到 MachineBottleneckProfile
    fn map_read_model_row(row: &rusqlite::Row, version_id: &str) -> rusqlite::Result<MachineBottleneckProfile> {
        let machine_code: String = row.get(0)?;
        let plan_date: String = row.get(1)?;
        let bottleneck_score: f64 = row.get(2)?;
        let bottleneck_level: String = row.get(3)?;
        let bottleneck_types: String = row.get(4)?;
        let reasons: String = row.get(5)?;
        let remaining_capacity_t: f64 = row.get(6)?;
        let capacity_utilization: f64 = row.get(7)?;
        let needs_roll_change: i32 = row.get(8)?;
        let structure_violations: i32 = row.get(9)?;
        let pending_materials: i32 = row.get(10)?;
        let suggested_actions: Option<String> = row.get(11)?;

        let mut profile = MachineBottleneckProfile::new(
            version_id.to_string(),
            machine_code,
            plan_date,
        );

        profile.bottleneck_score = bottleneck_score;
        profile.bottleneck_level = bottleneck_level;
        profile.remaining_capacity_t = remaining_capacity_t;
        profile.capacity_utilization = capacity_utilization;
        profile.needs_roll_change = needs_roll_change != 0;
        profile.structure_violations = structure_violations;
        profile.pending_materials = pending_materials;

        // 解析 bottleneck_types (String -> BottleneckType)
        if let Ok(type_strings) = serde_json::from_str::<Vec<String>>(&bottleneck_types) {
            use crate::decision::use_cases::d4_machine_bottleneck::BottleneckType;
            profile.bottleneck_types = type_strings
                .into_iter()
                .filter_map(|s| match s.as_str() {
                    "Capacity" => Some(BottleneckType::Capacity),
                    "Structure" => Some(BottleneckType::Structure),
                    "RollChange" => Some(BottleneckType::RollChange),
                    "ColdStock" => Some(BottleneckType::ColdStock),
                    "Mixed" => Some(BottleneckType::Mixed),
                    _ => None,
                })
                .collect();
        }

        // 解析 reasons
        if let Ok(reason_list) = serde_json::from_str::<Vec<serde_json::Value>>(&reasons) {
            for reason in reason_list {
                if let (Some(code), Some(desc), Some(severity), Some(impact)) = (
                    reason.get("code").and_then(|v| v.as_str()),
                    reason.get("description").and_then(|v| v.as_str()),
                    reason.get("severity").and_then(|v| v.as_f64()),
                    reason.get("impact_t").and_then(|v| v.as_i64()),
                ) {
                    profile.add_reason(code.to_string(), desc.to_string(), severity, impact as i32);
                }
            }
        }

        // 解析 suggested_actions
        if let Some(actions) = suggested_actions {
            if let Ok(action_list) = serde_json::from_str::<Vec<String>>(&actions) {
                for action in action_list {
                    profile.add_suggested_action(action);
                }
            }
        }

        Ok(profile)
    }

    /// 从 capacity_pool/plan_item 表实时计算（P1 回退路径）
    fn get_bottleneck_realtime(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<MachineBottleneckProfile>, Box<dyn Error>> {
        let conn = self.conn.lock().expect("锁获取失败");

        // 按机组-日聚合数据
        let mut bottleneck_map: HashMap<(String, String), MachineBottleneckAggregateData> =
            HashMap::new();

        // 根据是否指定 machine_code 选择不同的查询路径
        if let Some(mc) = machine_code {
            self.query_capacity_for_machine(&conn, mc, start_date, end_date, &mut bottleneck_map)?;
        } else {
            self.query_capacity_for_all(&conn, start_date, end_date, &mut bottleneck_map)?;
        }

        // 查询 plan_item 表以获取已排材料数据
        self.enrich_with_plan_items(&conn, version_id, start_date, end_date, &mut bottleneck_map)?;

        // 查询 material_state 表以获取真实的待排材料数据
        self.enrich_with_pending_materials(&conn, &mut bottleneck_map)?;

        // 转换为 MachineBottleneckProfile 并排序
        let mut profiles: Vec<MachineBottleneckProfile> = bottleneck_map
            .into_values()
            .map(|data| data.into_profile(version_id.to_string()))
            .collect();

        // 按堵塞分数降序排序
        profiles.sort_by(|a, b| {
            b.bottleneck_score
                .partial_cmp(&a.bottleneck_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(profiles)
    }

    /// 查询指定机组的产能数据
    fn query_capacity_for_machine(
        &self,
        conn: &Connection,
        machine_code: &str,
        start_date: &str,
        end_date: &str,
        bottleneck_map: &mut HashMap<(String, String), MachineBottleneckAggregateData>,
    ) -> Result<(), Box<dyn Error>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code,
                plan_date,
                target_capacity_t,
                limit_capacity_t,
                used_capacity_t,
                overflow_t,
                frozen_capacity_t,
                accumulated_tonnage_t,
                roll_campaign_id
            FROM capacity_pool
            WHERE machine_code = ?1
              AND plan_date BETWEEN ?2 AND ?3
            ORDER BY plan_date ASC
            "#,
        )?;

        let rows = stmt.query_map(params![machine_code, start_date, end_date], |row| {
            Ok((
                row.get::<_, String>(0)?, // machine_code
                row.get::<_, String>(1)?, // plan_date
                row.get::<_, f64>(2)?,    // target_capacity_t
                row.get::<_, f64>(3)?,    // limit_capacity_t
                row.get::<_, f64>(4)?,    // used_capacity_t
                row.get::<_, f64>(5)?,    // overflow_t
                row.get::<_, f64>(6)?,    // frozen_capacity_t
                row.get::<_, f64>(7)?,    // accumulated_tonnage_t
                row.get::<_, Option<String>>(8)?, // roll_campaign_id
            ))
        })?;

        self.process_capacity_rows(rows, bottleneck_map)?;
        Ok(())
    }

    /// 查询所有机组的产能数据
    fn query_capacity_for_all(
        &self,
        conn: &Connection,
        start_date: &str,
        end_date: &str,
        bottleneck_map: &mut HashMap<(String, String), MachineBottleneckAggregateData>,
    ) -> Result<(), Box<dyn Error>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code,
                plan_date,
                target_capacity_t,
                limit_capacity_t,
                used_capacity_t,
                overflow_t,
                frozen_capacity_t,
                accumulated_tonnage_t,
                roll_campaign_id
            FROM capacity_pool
            WHERE plan_date BETWEEN ?1 AND ?2
            ORDER BY machine_code ASC, plan_date ASC
            "#,
        )?;

        let rows = stmt.query_map(params![start_date, end_date], |row| {
            Ok((
                row.get::<_, String>(0)?, // machine_code
                row.get::<_, String>(1)?, // plan_date
                row.get::<_, f64>(2)?,    // target_capacity_t
                row.get::<_, f64>(3)?,    // limit_capacity_t
                row.get::<_, f64>(4)?,    // used_capacity_t
                row.get::<_, f64>(5)?,    // overflow_t
                row.get::<_, f64>(6)?,    // frozen_capacity_t
                row.get::<_, f64>(7)?,    // accumulated_tonnage_t
                row.get::<_, Option<String>>(8)?, // roll_campaign_id
            ))
        })?;

        self.process_capacity_rows(rows, bottleneck_map)?;
        Ok(())
    }

    /// 处理产能查询结果行
    fn process_capacity_rows(
        &self,
        rows: rusqlite::MappedRows<impl FnMut(&rusqlite::Row) -> rusqlite::Result<(String, String, f64, f64, f64, f64, f64, f64, Option<String>)>>,
        bottleneck_map: &mut HashMap<(String, String), MachineBottleneckAggregateData>,
    ) -> Result<(), Box<dyn Error>> {
        for row_result in rows {
            let (
                machine_code,
                plan_date,
                target_capacity_t,
                limit_capacity_t,
                used_capacity_t,
                overflow_t,
                frozen_capacity_t,
                accumulated_tonnage_t,
                roll_campaign_id,
            ) = row_result?;

            let key = (machine_code.clone(), plan_date.clone());
            let entry = bottleneck_map
                .entry(key)
                .or_insert_with(|| MachineBottleneckAggregateData::new(machine_code, plan_date));

            entry.set_capacity_data(
                target_capacity_t,
                limit_capacity_t,
                used_capacity_t,
                overflow_t,
                frozen_capacity_t,
            );

            entry.set_roll_campaign_data(accumulated_tonnage_t, roll_campaign_id);
        }
        Ok(())
    }

    /// 从 plan_item 表查询已排材料数据并填充到聚合数据中
    fn enrich_with_plan_items(
        &self,
        conn: &Connection,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        bottleneck_map: &mut HashMap<(String, String), MachineBottleneckAggregateData>,
    ) -> Result<(), Box<dyn Error>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT
                machine_code,
                plan_date,
                COUNT(*) as material_count,
                SUM(weight_t) as total_weight_t,
                SUM(CASE WHEN violation_flags IS NOT NULL AND violation_flags != '' THEN 1 ELSE 0 END) as violation_count
            FROM plan_item
            WHERE version_id = ?1
              AND plan_date BETWEEN ?2 AND ?3
            GROUP BY machine_code, plan_date
            "#,
        )?;

        let rows = stmt.query_map(params![version_id, start_date, end_date], |row| {
            Ok((
                row.get::<_, String>(0)?,  // machine_code
                row.get::<_, String>(1)?,  // plan_date
                row.get::<_, i32>(2)?,     // material_count (已排材料数)
                row.get::<_, f64>(3)?,     // total_weight_t (已排材料重量)
                row.get::<_, i32>(4)?,     // violation_count
            ))
        })?;

        for row_result in rows {
            let (machine_code, plan_date, scheduled_count, scheduled_weight, violation_count) =
                row_result?;

            let key = (machine_code, plan_date);
            if let Some(entry) = bottleneck_map.get_mut(&key) {
                // plan_item 数据代表已排材料，直接赋值给 scheduled 字段
                // pending 字段将从 material_state 表查询，不在此处赋值
                entry.set_plan_item_data(
                    0,                   // pending_materials（暂不赋值，由 enrich_with_pending_materials 填充）
                    0.0,                 // pending_weight_t（暂不赋值）
                    violation_count,
                    scheduled_count,     // scheduled_materials（已排材料数）
                    scheduled_weight,    // scheduled_weight_t（已排材料重量）
                );
            }
        }

        Ok(())
    }

    /// 从 material_state 表查询真实的待排材料数据并填充到聚合数据中
    ///
    /// # 说明
    /// - 查询 sched_state IN ('READY', 'FORCE_RELEASE') 的材料（真正的待排状态）
    /// - 通过 JOIN material_master 获取机组代码和材料重量
    /// - 按机组聚合统计，将结果填充到每个(机组, 日期)的 pending_materials 和 pending_weight_t
    ///
    /// # 业务含义
    /// - 待排材料是按机组级别统计的，不分日期（因为还未排入具体日期）
    /// - 因此，同一机组的所有日期点将显示相同的待排材料数量和重量
    fn enrich_with_pending_materials(
        &self,
        conn: &Connection,
        bottleneck_map: &mut HashMap<(String, String), MachineBottleneckAggregateData>,
    ) -> Result<(), Box<dyn Error>> {
        // 查询待排材料：从 material_state 中筛选 READY/FORCE_RELEASE 状态的材料
        // JOIN material_master 获取机组代码和重量
        let mut stmt = conn.prepare(
            r#"
            SELECT
                mm.next_machine_code as machine_code,
                COUNT(*) as pending_count,
                SUM(mm.weight_t) as pending_weight_t
            FROM material_state ms
            INNER JOIN material_master mm ON ms.material_id = mm.material_id
            WHERE ms.sched_state IN ('READY', 'FORCE_RELEASE')
              AND mm.next_machine_code IS NOT NULL
              AND mm.next_machine_code != ''
            GROUP BY mm.next_machine_code
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,  // machine_code
                row.get::<_, i32>(1)?,     // pending_count
                row.get::<_, f64>(2)?,     // pending_weight_t
            ))
        })?;

        // 构建机组 -> (待排数, 待排重量) 的映射
        let mut machine_pending_map: HashMap<String, (i32, f64)> = HashMap::new();
        for row_result in rows {
            let (machine_code, pending_count, pending_weight) = row_result?;
            machine_pending_map.insert(machine_code, (pending_count, pending_weight));
        }

        // 将待排数据填充到每个(机组, 日期)的聚合数据中
        for ((machine_code, _plan_date), entry) in bottleneck_map.iter_mut() {
            if let Some((pending_count, pending_weight)) = machine_pending_map.get(machine_code) {
                entry.pending_materials = *pending_count;
                entry.pending_weight_t = *pending_weight;
            }
        }

        Ok(())
    }

    /// 查询最堵塞的 N 个机组-日组合
    pub fn get_top_bottlenecks(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        top_n: usize,
    ) -> Result<Vec<MachineBottleneckProfile>, Box<dyn Error>> {
        let mut profiles = self.get_bottleneck_profile(version_id, None, start_date, end_date)?;
        profiles.truncate(top_n);
        Ok(profiles)
    }

    /// 获取机组堵塞热力图数据
    pub fn get_bottleneck_heatmap(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<BottleneckHeatmap, Box<dyn Error>> {
        let profiles = self.get_bottleneck_profile(version_id, None, start_date, end_date)?;

        let mut heatmap = BottleneckHeatmap::new(
            version_id.to_string(),
            start_date.to_string(),
            end_date.to_string(),
        );

        for profile in profiles {
            heatmap.add_cell(
                profile.machine_code,
                profile.plan_date,
                profile.bottleneck_score,
                profile.bottleneck_level,
            );
        }

        Ok(heatmap)
    }
}

/// 机组堵塞聚合数据（中间结构）
struct MachineBottleneckAggregateData {
    machine_code: String,
    plan_date: String,
    target_capacity_t: f64,
    limit_capacity_t: f64,
    used_capacity_t: f64,
    overflow_t: f64,
    frozen_capacity_t: f64,
    accumulated_tonnage_t: f64,
    roll_campaign_id: Option<String>,
    pending_materials: i32,
    pending_weight_t: f64,
    structure_violations: i32,
    scheduled_materials: i32,
    scheduled_weight_t: f64,
}

impl MachineBottleneckAggregateData {
    fn new(machine_code: String, plan_date: String) -> Self {
        Self {
            machine_code,
            plan_date,
            target_capacity_t: 0.0,
            limit_capacity_t: 0.0,
            used_capacity_t: 0.0,
            overflow_t: 0.0,
            frozen_capacity_t: 0.0,
            accumulated_tonnage_t: 0.0,
            roll_campaign_id: None,
            pending_materials: 0,
            pending_weight_t: 0.0,
            structure_violations: 0,
            scheduled_materials: 0,
            scheduled_weight_t: 0.0,
        }
    }

    fn set_capacity_data(
        &mut self,
        target_capacity_t: f64,
        limit_capacity_t: f64,
        used_capacity_t: f64,
        overflow_t: f64,
        frozen_capacity_t: f64,
    ) {
        self.target_capacity_t = target_capacity_t;
        self.limit_capacity_t = limit_capacity_t;
        self.used_capacity_t = used_capacity_t;
        self.overflow_t = overflow_t;
        self.frozen_capacity_t = frozen_capacity_t;
    }

    fn set_roll_campaign_data(
        &mut self,
        accumulated_tonnage_t: f64,
        roll_campaign_id: Option<String>,
    ) {
        self.accumulated_tonnage_t = accumulated_tonnage_t;
        self.roll_campaign_id = roll_campaign_id;
    }

    fn set_plan_item_data(
        &mut self,
        pending_materials: i32,
        pending_weight_t: f64,
        structure_violations: i32,
        scheduled_materials: i32,
        scheduled_weight_t: f64,
    ) {
        self.pending_materials = pending_materials;
        self.pending_weight_t = pending_weight_t;
        self.structure_violations = structure_violations;
        self.scheduled_materials = scheduled_materials;
        self.scheduled_weight_t = scheduled_weight_t;
    }

    fn into_profile(self, version_id: String) -> MachineBottleneckProfile {
        let mut profile =
            MachineBottleneckProfile::new(version_id, self.machine_code, self.plan_date);

        // 计算产能利用率
        let capacity_utilization = if self.limit_capacity_t > 0.0 {
            self.used_capacity_t / self.limit_capacity_t
        } else {
            0.0
        };

        // 计算剩余产能
        let remaining_capacity_t = self.limit_capacity_t - self.used_capacity_t;

        // 设置产能信息
        profile.set_capacity_info(remaining_capacity_t, capacity_utilization);

        // 设置结构信息
        profile.set_structure_info(self.structure_violations);

        // 设置待排材料数量和重量
        profile.pending_materials = self.pending_materials;
        profile.pending_weight_t = self.pending_weight_t;

        // 设置已排材料数量和重量
        profile.scheduled_materials = self.scheduled_materials;
        profile.scheduled_weight_t = self.scheduled_weight_t;

        // 添加堵塞原因
        // 数据一致性校验：如果显示超限但无已排材料，添加警告原因
        if self.overflow_t > 0.0 && self.scheduled_materials == 0 {
            profile.add_reason(
                "DATA_INCONSISTENCY_WARNING".to_string(),
                format!(
                    "数据不一致：容量池显示超限 {:.1}t，但无已排材料。请检查容量配置或排程状态",
                    self.overflow_t
                ),
                0.3,
                0,
            );
        }

        if self.overflow_t > 0.0 {
            profile.add_reason(
                "CAPACITY_OVERFLOW".to_string(),
                format!(
                    "产能池超限 {:.1}t，利用率 {:.1}%",
                    self.overflow_t,
                    capacity_utilization * 100.0
                ),
                0.9,
                0,
            );
        }

        if capacity_utilization >= 0.95 && capacity_utilization < 1.0 {
            profile.add_reason(
                "HIGH_UTILIZATION".to_string(),
                format!("产能利用率高 {:.1}%", capacity_utilization * 100.0),
                0.7,
                0,
            );
        }

        if self.structure_violations > 0 {
            profile.add_reason(
                "STRUCTURE_CONFLICT".to_string(),
                format!(
                    "结构矛盾导致 {} 个材料无法排入",
                    self.structure_violations
                ),
                0.8,
                self.structure_violations,
            );
        }

        if self.pending_materials > 20 {
            profile.add_reason(
                "HIGH_PENDING_COUNT".to_string(),
                format!("待排产材料数量较多 {} 个", self.pending_materials),
                0.5,
                self.pending_materials,
            );
        }

        if remaining_capacity_t < 100.0 && self.pending_weight_t > 0.0 {
            profile.add_reason(
                "LOW_REMAINING_CAPACITY".to_string(),
                format!(
                    "剩余产能不足 {:.1}t，待排产 {:.1}t",
                    remaining_capacity_t, self.pending_weight_t
                ),
                0.6,
                0,
            );
        }

        // 添加建议措施
        if profile.is_severe() {
            if self.overflow_t > 0.0 {
                profile.add_suggested_action("调整产能池上限".to_string());
            }
            if self.structure_violations > 0 {
                profile.add_suggested_action("优先处理结构冲突材料".to_string());
            }
            if self.pending_materials > 20 {
                profile.add_suggested_action("将部分材料转移至其他机组".to_string());
            }
        }

        profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();

        // 创建 capacity_pool 表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS capacity_pool (
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                target_capacity_t REAL NOT NULL,
                limit_capacity_t REAL NOT NULL,
                used_capacity_t REAL NOT NULL DEFAULT 0.0,
                overflow_t REAL NOT NULL DEFAULT 0.0,
                frozen_capacity_t REAL NOT NULL DEFAULT 0.0,
                accumulated_tonnage_t REAL NOT NULL DEFAULT 0.0,
                roll_campaign_id TEXT,
                PRIMARY KEY (machine_code, plan_date)
            )
            "#,
            [],
        )
        .unwrap();

        // 创建 plan_item 表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS plan_item (
                version_id TEXT NOT NULL,
                material_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                seq_no INTEGER NOT NULL,
                weight_t REAL NOT NULL,
                source_type TEXT NOT NULL,
                locked_in_plan INTEGER NOT NULL DEFAULT 0,
                force_release_in_plan INTEGER NOT NULL DEFAULT 0,
                violation_flags TEXT,
                PRIMARY KEY (version_id, material_id)
            )
            "#,
            [],
        )
        .unwrap();

        // 创建 material_master 表（用于待排材料查询）
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS material_master (
                material_id TEXT PRIMARY KEY,
                manufacturing_order_id TEXT,
                contract_no TEXT,
                due_date TEXT,
                next_machine_code TEXT,
                rework_machine_code TEXT,
                current_machine_code TEXT,
                width_mm REAL,
                thickness_mm REAL,
                length_m REAL,
                weight_t REAL,
                available_width_mm REAL,
                steel_mark TEXT,
                slab_id TEXT,
                material_status_code_src TEXT,
                status_updated_at TEXT,
                output_age_days_raw INTEGER,
                stock_age_days INTEGER,
                contract_nature TEXT,
                weekly_delivery_flag TEXT,
                export_flag TEXT,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )
            "#,
            [],
        )
        .unwrap();

        // 创建 material_state 表（用于待排材料查询）
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS material_state (
                material_id TEXT PRIMARY KEY,
                sched_state TEXT NOT NULL DEFAULT 'READY',
                lock_flag INTEGER NOT NULL DEFAULT 0,
                force_release_flag INTEGER NOT NULL DEFAULT 0,
                urgent_level TEXT NOT NULL DEFAULT 'L0',
                urgent_reason TEXT,
                rush_level TEXT DEFAULT 'L0',
                rolling_output_age_days INTEGER DEFAULT 0,
                ready_in_days INTEGER DEFAULT 0,
                earliest_sched_date TEXT,
                stock_age_days INTEGER DEFAULT 0,
                scheduled_date TEXT,
                scheduled_machine_code TEXT,
                seq_no INTEGER,
                manual_urgent_flag INTEGER NOT NULL DEFAULT 0,
                in_frozen_zone INTEGER NOT NULL DEFAULT 0,
                last_calc_version_id TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_by TEXT
            )
            "#,
            [],
        )
        .unwrap();

        Arc::new(Mutex::new(conn))
    }

    fn insert_test_capacity_data(conn: &Connection) {
        // H032: 高利用率
        conn.execute(
            r#"
            INSERT INTO capacity_pool (
                machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                "H032",
                "2026-01-24",
                1500.0,
                2000.0,
                1950.0,
                0.0,
                100.0,
                15000.0,
                "RC001"
            ],
        )
        .unwrap();

        // H033: 产能超载
        conn.execute(
            r#"
            INSERT INTO capacity_pool (
                machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                "H033",
                "2026-01-24",
                1500.0,
                2000.0,
                2300.0,
                300.0,
                150.0,
                18000.0,
                "RC002"
            ],
        )
        .unwrap();
    }

    fn insert_test_plan_items(conn: &Connection) {
        // H032: 10 个材料，其中 2 个有结构违规
        for i in 1..=10 {
            let violation_flags = if i <= 2 { "STRUCT_CONFLICT" } else { "" };
            conn.execute(
                r#"
                INSERT INTO plan_item (
                    version_id, material_id, machine_code, plan_date, seq_no, weight_t,
                    source_type, locked_in_plan, force_release_in_plan, violation_flags
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    "V001",
                    format!("MAT{:03}", i),
                    "H032",
                    "2026-01-24",
                    i,
                    150.0,
                    "AUTO",
                    0,
                    0,
                    violation_flags
                ],
            )
            .unwrap();
        }

        // H033: 25 个材料，其中 5 个有结构违规
        for i in 11..=35 {
            let violation_flags = if i <= 15 { "STRUCT_CONFLICT" } else { "" };
            conn.execute(
                r#"
                INSERT INTO plan_item (
                    version_id, material_id, machine_code, plan_date, seq_no, weight_t,
                    source_type, locked_in_plan, force_release_in_plan, violation_flags
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    "V001",
                    format!("MAT{:03}", i),
                    "H033",
                    "2026-01-24",
                    i - 10,
                    100.0,
                    "AUTO",
                    0,
                    0,
                    violation_flags
                ],
            )
            .unwrap();
        }
    }

    fn insert_test_material_master(conn: &Connection) {
        // H032: 插入 10 个已排材料（对应 plan_item）
        for i in 1..=10 {
            conn.execute(
                r#"
                INSERT INTO material_master (
                    material_id, current_machine_code, next_machine_code, weight_t,
                    manufacturing_order_id, contract_no, due_date,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "H031",
                    "H032",
                    150.0,
                    format!("MO{:03}", i),
                    format!("C{:03}", i),
                    "2026-02-01",
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:00:00Z"
                ],
            )
            .unwrap();
        }

        // H033: 插入 25 个已排材料（对应 plan_item）
        for i in 11..=35 {
            conn.execute(
                r#"
                INSERT INTO material_master (
                    material_id, current_machine_code, next_machine_code, weight_t,
                    manufacturing_order_id, contract_no, due_date,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "H031",
                    "H033",
                    100.0,
                    format!("MO{:03}", i),
                    format!("C{:03}", i),
                    "2026-02-01",
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:00:00Z"
                ],
            )
            .unwrap();
        }

        // H032: 插入 3 个待排材料（READY 状态的 material_master 记录）
        for i in 36..=38 {
            conn.execute(
                r#"
                INSERT INTO material_master (
                    material_id, current_machine_code, next_machine_code, weight_t,
                    manufacturing_order_id, contract_no, due_date,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "H031",
                    "H032",
                    120.0,
                    format!("MO{:03}", i),
                    format!("C{:03}", i),
                    "2026-02-05",
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:00:00Z"
                ],
            )
            .unwrap();
        }

        // H033: 插入 5 个待排材料（READY 状态的 material_master 记录）
        for i in 39..=43 {
            conn.execute(
                r#"
                INSERT INTO material_master (
                    material_id, current_machine_code, next_machine_code, weight_t,
                    manufacturing_order_id, contract_no, due_date,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "H031",
                    "H033",
                    80.0,
                    format!("MO{:03}", i),
                    format!("C{:03}", i),
                    "2026-02-05",
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:00:00Z"
                ],
            )
            .unwrap();
        }
    }

    fn insert_test_material_state(conn: &Connection) {
        // H032: 3 个待排材料（READY 状态）
        for i in 36..=38 {
            conn.execute(
                r#"
                INSERT INTO material_state (
                    material_id, sched_state, lock_flag, force_release_flag,
                    urgent_level, updated_at, updated_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "READY",
                    0,
                    0,
                    "L0",
                    "2026-01-01T00:00:00Z",
                    "SYSTEM"
                ],
            )
            .unwrap();
        }

        // H033: 5 个待排材料（READY 状态）
        for i in 39..=43 {
            conn.execute(
                r#"
                INSERT INTO material_state (
                    material_id, sched_state, lock_flag, force_release_flag,
                    urgent_level, updated_at, updated_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "READY",
                    0,
                    0,
                    "L0",
                    "2026-01-01T00:00:00Z",
                    "SYSTEM"
                ],
            )
            .unwrap();
        }

        // 已排材料（SCHEDULED 状态）
        for i in 1..=35 {
            let (machine_code, plan_date) = if i <= 10 {
                ("H032", "2026-01-24")
            } else {
                ("H033", "2026-01-24")
            };
            conn.execute(
                r#"
                INSERT INTO material_state (
                    material_id, sched_state, lock_flag, force_release_flag,
                    urgent_level, scheduled_machine_code, scheduled_date,
                    updated_at, updated_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    "SCHEDULED",
                    0,
                    0,
                    "L0",
                    machine_code,
                    plan_date,
                    "2026-01-01T00:00:00Z",
                    "SYSTEM"
                ],
            )
            .unwrap();
        }
    }

    #[test]
    fn test_get_bottleneck_profile() {
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().expect("锁获取失败");
            insert_test_capacity_data(&conn);
            insert_test_plan_items(&conn);
            insert_test_material_master(&conn);
            insert_test_material_state(&conn);
        }

        let repo = BottleneckRepository::new(conn_arc);
        let profiles = repo
            .get_bottleneck_profile("V001", None, "2026-01-24", "2026-01-24")
            .unwrap();

        assert_eq!(profiles.len(), 2);

        // 第一个应该是 H033（产能超载，堵塞分数最高）
        let h033 = &profiles[0];
        assert_eq!(h033.machine_code, "H033");
        assert!(h033.bottleneck_score > 0.0);
        assert!(h033.is_severe());
        // pending_materials 来自 material_state（READY 状态），H033 有 5 个待排
        assert_eq!(h033.pending_materials, 5);
        assert_eq!(h033.structure_violations, 5);
        // scheduled_materials 来自 plan_item
        assert_eq!(h033.scheduled_materials, 25);

        // 第二个应该是 H032（高利用率）
        let h032 = &profiles[1];
        assert_eq!(h032.machine_code, "H032");
        assert!(h032.bottleneck_score > 0.0);
        // pending_materials 来自 material_state（READY 状态），H032 有 3 个待排
        assert_eq!(h032.pending_materials, 3);
        assert_eq!(h032.structure_violations, 2);
        // scheduled_materials 来自 plan_item
        assert_eq!(h032.scheduled_materials, 10);
    }

    #[test]
    fn test_get_top_bottlenecks() {
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().expect("锁获取失败");
            insert_test_capacity_data(&conn);
            insert_test_plan_items(&conn);
            insert_test_material_master(&conn);
            insert_test_material_state(&conn);
        }

        let repo = BottleneckRepository::new(conn_arc);
        let profiles = repo
            .get_top_bottlenecks("V001", "2026-01-24", "2026-01-24", 1)
            .unwrap();

        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].machine_code, "H033");
    }

    #[test]
    fn test_get_bottleneck_heatmap() {
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().expect("锁获取失败");
            insert_test_capacity_data(&conn);
            insert_test_plan_items(&conn);
            insert_test_material_master(&conn);
            insert_test_material_state(&conn);
        }

        let repo = BottleneckRepository::new(conn_arc);
        let heatmap = repo
            .get_bottleneck_heatmap("V001", "2026-01-24", "2026-01-24")
            .unwrap();

        assert_eq!(heatmap.machines.len(), 2);
        assert_eq!(heatmap.data.len(), 2);
        assert!(heatmap.max_score > 0.0);
        assert!(heatmap.avg_score > 0.0);

        // 验证可以获取特定机组-日的分数
        let h033_score = heatmap.get_score("H033", "2026-01-24");
        assert!(h033_score.is_some());
        assert!(h033_score.unwrap() > 0.0);
    }

    #[test]
    fn test_filter_by_machine_code() {
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().expect("锁获取失败");
            insert_test_capacity_data(&conn);
            insert_test_plan_items(&conn);
            insert_test_material_master(&conn);
            insert_test_material_state(&conn);
        }

        let repo = BottleneckRepository::new(conn_arc);
        let profiles = repo
            .get_bottleneck_profile("V001", Some("H032"), "2026-01-24", "2026-01-24")
            .unwrap();

        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].machine_code, "H032");
        assert_eq!(profiles[0].pending_materials, 3);  // H032 有 3 个待排材料
        assert_eq!(profiles[0].scheduled_materials, 10);  // H032 有 10 个已排材料
    }

    #[test]
    fn test_pending_materials_from_material_state() {
        // 新测试：验证待排材料正确地从 material_state 查询
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().expect("锁获取失败");
            insert_test_capacity_data(&conn);
            insert_test_plan_items(&conn);
            insert_test_material_master(&conn);
            insert_test_material_state(&conn);
        }

        let repo = BottleneckRepository::new(conn_arc);
        let profiles = repo
            .get_bottleneck_profile("V001", None, "2026-01-24", "2026-01-24")
            .unwrap();

        // 验证 pending 和 scheduled 不相等（之前的 bug）
        for profile in profiles {
            if profile.machine_code == "H032" {
                // 待排：3 个，已排：10 个
                assert_eq!(profile.pending_materials, 3);
                assert_eq!(profile.scheduled_materials, 10);
                assert_eq!(profile.pending_weight_t, 3.0 * 120.0);  // 3 个 × 120t
                assert_eq!(profile.scheduled_weight_t, 10.0 * 150.0);  // 10 个 × 150t
            } else if profile.machine_code == "H033" {
                // 待排：5 个，已排：25 个
                assert_eq!(profile.pending_materials, 5);
                assert_eq!(profile.scheduled_materials, 25);
                assert_eq!(profile.pending_weight_t, 5.0 * 80.0);  // 5 个 × 80t
                assert_eq!(profile.scheduled_weight_t, 25.0 * 100.0);  // 25 个 × 100t
            }
        }
    }

    #[test]
    fn test_data_inconsistency_warning() {
        // 新测试：验证数据一致性校验（超限但无已排材料）
        let conn_arc = setup_test_db();
        {
            let conn = conn_arc.lock().expect("锁获取失败");
            // 插入产能超限的 capacity_pool 数据
            conn.execute(
                r#"
                INSERT INTO capacity_pool (
                    machine_code, plan_date, target_capacity_t, limit_capacity_t,
                    used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    "H034",
                    "2026-01-25",
                    1500.0,
                    2000.0,
                    2300.0,
                    300.0,  // 超限
                    0.0,
                    0.0,
                    "RC003"
                ],
            )
            .unwrap();
            // 故意不插入 plan_item（没有已排材料）
        }

        let repo = BottleneckRepository::new(conn_arc);
        let profiles = repo
            .get_bottleneck_profile("V001", Some("H034"), "2026-01-25", "2026-01-25")
            .unwrap();

        assert_eq!(profiles.len(), 1);
        let h034 = &profiles[0];

        // 应该包含数据不一致警告原因
        let warning_reason = h034.reasons.iter()
            .find(|r| r.code == "DATA_INCONSISTENCY_WARNING");
        assert!(warning_reason.is_some(), "应该包含数据不一致警告");
        assert_eq!(h034.scheduled_materials, 0);  // 没有已排材料
    }
}

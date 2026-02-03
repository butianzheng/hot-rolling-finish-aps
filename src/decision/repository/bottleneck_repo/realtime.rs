use super::core::BottleneckRepository;
use crate::decision::use_cases::d4_machine_bottleneck::MachineBottleneckProfile;
use rusqlite::{params, Connection};
use std::collections::{BTreeSet, HashMap};
use std::error::Error;

impl BottleneckRepository {
    /// 从 capacity_pool/plan_item 表实时计算（P1 回退路径）
    pub(super) fn get_bottleneck_realtime(
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
            self.query_capacity_for_machine(&conn, version_id, mc, start_date, end_date, &mut bottleneck_map)?;
        } else {
            self.query_capacity_for_all(&conn, version_id, start_date, end_date, &mut bottleneck_map)?;
        }

        // 查询 plan_item 表以获取已排材料数据
        self.enrich_with_plan_items(&conn, version_id, start_date, end_date, &mut bottleneck_map)?;

        // 查询 material_state 表以获取真实的待排材料数据
        self.enrich_with_pending_materials(&conn, version_id, start_date, end_date, &mut bottleneck_map)?;

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
        version_id: &str,
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
            WHERE version_id = ?1
              AND machine_code = ?2
              AND plan_date BETWEEN ?3 AND ?4
            ORDER BY plan_date ASC
            "#,
        )?;

        let rows = stmt.query_map(params![version_id, machine_code, start_date, end_date], |row| {
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
                target_capacity_t,
                limit_capacity_t,
                used_capacity_t,
                overflow_t,
                frozen_capacity_t,
                accumulated_tonnage_t,
                roll_campaign_id
            FROM capacity_pool
            WHERE version_id = ?1
              AND plan_date BETWEEN ?2 AND ?3
            ORDER BY machine_code ASC, plan_date ASC
            "#,
        )?;

        let rows = stmt.query_map(params![version_id, start_date, end_date], |row| {
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

    /// 计算“缺口（到当日仍未排入 <= 当日 的量）”并填充到聚合数据中
    ///
    /// # 说明
    /// - gap(D) = max(0, demand_ready_cum(<=D) - scheduled_cum(<=D))
    /// - demand_ready_cum：按 effective_earliest_date 累计（FORCE_RELEASE 视为 start_date）
    /// - scheduled_cum：来自 plan_item（按 version_id 累计）
    ///
    /// # 业务含义
    /// - 缺口是按机组×日期统计的，会随日期推进逐步收敛（若后续日期排入足够材料）
    fn enrich_with_pending_materials(
        &self,
        conn: &Connection,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        bottleneck_map: &mut HashMap<(String, String), MachineBottleneckAggregateData>,
    ) -> Result<(), Box<dyn Error>> {
        let machine_codes: Vec<String> = {
            let mut list: Vec<String> = bottleneck_map.keys().map(|(mc, _)| mc.clone()).collect();
            list.sort();
            list.dedup();
            list
        };
        if machine_codes.is_empty() {
            return Ok(());
        }

        let mut profile_dates: HashMap<String, BTreeSet<String>> = HashMap::new();
        for ((mc, plan_date), _) in bottleneck_map.iter() {
            profile_dates
                .entry(mc.clone())
                .or_default()
                .insert(plan_date.clone());
        }

        // scheduled: plan_item（按机组×日期聚合，用于 prefix sum）
        let scheduled_daily_map =
            Self::query_scheduled_by_machine_date(conn, version_id, &machine_codes, start_date, end_date)?;
        let scheduled_before_map =
            Self::query_scheduled_before_date(conn, version_id, &machine_codes, start_date)?;

        // demand increments: READY/FORCE_RELEASE/LOCKED + 本版本 plan_item 的 union
        let demand_incr_map =
            Self::query_ready_demand_increments(conn, version_id, &machine_codes, start_date, end_date)?;

        let gap_map = Self::compute_gap_map(
            &machine_codes,
            &profile_dates,
            &scheduled_before_map,
            &scheduled_daily_map,
            &demand_incr_map,
        );

        for ((machine_code, plan_date), entry) in bottleneck_map.iter_mut() {
            if let Some((gap_cnt, gap_weight)) =
                gap_map.get(&(machine_code.clone(), plan_date.clone()))
            {
                entry.pending_materials = *gap_cnt;
                entry.pending_weight_t = *gap_weight;
            } else {
                entry.pending_materials = 0;
                entry.pending_weight_t = 0.0;
            }
        }

        Ok(())
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
                format!("缺口材料数量较多 {} 个（到当日仍未排入≤当日）", self.pending_materials),
                0.5,
                self.pending_materials,
            );
        }

        if remaining_capacity_t < 100.0 && self.pending_weight_t > 0.0 {
            profile.add_reason(
                "LOW_REMAINING_CAPACITY".to_string(),
                format!(
                    "剩余产能不足 {:.1}t，缺口 {:.1}t（到当日仍未排入≤当日）",
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
                profile.add_suggested_action("将部分材料转移至其他机组或延后至后续日期".to_string());
            }
        }

        profile
    }
}


use super::core::BottleneckRepository;
use crate::decision::use_cases::d4_machine_bottleneck::MachineBottleneckProfile;
use rusqlite::{params, Connection};
use std::collections::{BTreeSet, HashMap};
use std::error::Error;

#[derive(Debug, Clone)]
struct BottleneckScoringConfig {
    capacity_hard_threshold: f64,
    capacity_full_threshold: f64,
    structure_dev_threshold: f64,
    structure_dev_full_multiplier: f64,
    structure_small_category_threshold: f64,
    structure_violation_full_count: f64,
    low_threshold: f64,
    medium_threshold: f64,
    high_threshold: f64,
    critical_threshold: f64,
}

impl Default for BottleneckScoringConfig {
    fn default() -> Self {
        Self {
            capacity_hard_threshold: 0.95,
            capacity_full_threshold: 1.0,
            structure_dev_threshold: 0.1,
            structure_dev_full_multiplier: 2.0,
            structure_small_category_threshold: 0.05,
            structure_violation_full_count: 10.0,
            low_threshold: 0.3,
            medium_threshold: 0.6,
            high_threshold: 0.9,
            critical_threshold: 0.95,
        }
    }
}

impl BottleneckScoringConfig {
    fn load(conn: &Connection) -> Self {
        fn read_f64(conn: &Connection, key: &str) -> Option<f64> {
            let value: rusqlite::Result<String> = conn.query_row(
                "SELECT value FROM config_kv WHERE scope_id = 'global' AND key = ?1 LIMIT 1",
                params![key],
                |row| row.get(0),
            );
            match value {
                Ok(raw) => raw.trim().parse::<f64>().ok(),
                Err(_) => None,
            }
        }

        let mut cfg = BottleneckScoringConfig::default();
        if let Some(v) = read_f64(conn, "d4_capacity_hard_threshold") {
            cfg.capacity_hard_threshold = v;
        }
        if let Some(v) = read_f64(conn, "d4_capacity_full_threshold") {
            cfg.capacity_full_threshold = v;
        }
        if let Some(v) = read_f64(conn, "d4_structure_dev_threshold") {
            cfg.structure_dev_threshold = v;
        }
        if let Some(v) = read_f64(conn, "d4_structure_dev_full_multiplier") {
            cfg.structure_dev_full_multiplier = v;
        }
        if let Some(v) = read_f64(conn, "d4_structure_small_category_threshold") {
            cfg.structure_small_category_threshold = v;
        }
        if let Some(v) = read_f64(conn, "d4_structure_violation_full_count") {
            cfg.structure_violation_full_count = v;
        }
        if let Some(v) = read_f64(conn, "d4_bottleneck_low_threshold") {
            cfg.low_threshold = v;
        }
        if let Some(v) = read_f64(conn, "d4_bottleneck_medium_threshold") {
            cfg.medium_threshold = v;
        }
        if let Some(v) = read_f64(conn, "d4_bottleneck_high_threshold") {
            cfg.high_threshold = v;
        }
        if let Some(v) = read_f64(conn, "d4_bottleneck_critical_threshold") {
            cfg.critical_threshold = v;
        }

        cfg
    }

    fn capacity_severity(&self, utilization: f64) -> f64 {
        if !utilization.is_finite() {
            return 0.0;
        }
        if utilization <= self.capacity_hard_threshold {
            return 0.0;
        }
        if self.capacity_full_threshold <= self.capacity_hard_threshold {
            return 1.0;
        }
        clamp01((utilization - self.capacity_hard_threshold) / (self.capacity_full_threshold - self.capacity_hard_threshold))
    }

    fn structure_deviation_severity(&self, weighted_dev: f64) -> f64 {
        if !weighted_dev.is_finite() {
            return 0.0;
        }
        if self.structure_dev_threshold <= 0.0 {
            return clamp01(weighted_dev);
        }
        if weighted_dev <= self.structure_dev_threshold {
            return 0.0;
        }
        if self.structure_dev_full_multiplier <= 1.0 {
            return 1.0;
        }
        let full_threshold = self.structure_dev_threshold * self.structure_dev_full_multiplier;
        if full_threshold <= self.structure_dev_threshold {
            return 1.0;
        }
        clamp01((weighted_dev - self.structure_dev_threshold) / (full_threshold - self.structure_dev_threshold))
    }

    fn structure_violation_severity(&self, violation_count: i32) -> f64 {
        if self.structure_violation_full_count <= 0.0 {
            return 0.0;
        }
        clamp01(violation_count.max(0) as f64 / self.structure_violation_full_count)
    }
}

fn clamp01(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    if value < 0.0 {
        0.0
    } else if value > 1.0 {
        1.0
    } else {
        value
    }
}

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

        // 评分参数（来自 config_kv）
        let scoring_cfg = BottleneckScoringConfig::load(&conn);

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

        // 查询结构偏差（加权偏差）
        self.enrich_with_structure_deviation(
            &conn,
            version_id,
            start_date,
            end_date,
            scoring_cfg.structure_small_category_threshold,
            &mut bottleneck_map,
        )?;

        // 查询 material_state 表以获取真实的待排材料数据
        self.enrich_with_pending_materials(&conn, version_id, start_date, end_date, &mut bottleneck_map)?;

        // 转换为 MachineBottleneckProfile 并排序
        let mut profiles: Vec<MachineBottleneckProfile> = bottleneck_map
            .into_values()
            .map(|data| data.into_profile(version_id.to_string(), &scoring_cfg))
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

    /// 计算“加权节奏偏差”并填充到聚合数据中
    fn enrich_with_structure_deviation(
        &self,
        conn: &Connection,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        small_category_threshold: f64,
        bottleneck_map: &mut HashMap<(String, String), MachineBottleneckAggregateData>,
    ) -> Result<(), Box<dyn Error>> {
        if bottleneck_map.is_empty() {
            return Ok(());
        }

        let has_rhythm_table: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='plan_rhythm_target'",
            [],
            |row| row.get(0),
        )?;
        if has_rhythm_table == 0 {
            return Ok(());
        }

        let has_product_category_col: i32 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('material_master') WHERE name = 'product_category'",
            [],
            |row| row.get(0),
        )?;
        let category_expr = if has_product_category_col > 0 {
            "COALESCE(mm.product_category, '未分类')"
        } else {
            "COALESCE(mm.steel_mark, '未分类')"
        };

        let sql = format!(
            r#"
            WITH target AS (
                SELECT
                    machine_code,
                    plan_date,
                    target_json
                FROM plan_rhythm_target
                WHERE version_id = ?1
                  AND dimension = 'PRODUCT_CATEGORY'
                  AND plan_date BETWEEN ?2 AND ?3
                  AND target_json IS NOT NULL
                  AND TRIM(target_json) != ''
                  AND TRIM(target_json) != '{{}}'
            ),
            target_dates AS (
                SELECT DISTINCT machine_code, plan_date FROM target
            ),
            actual_total AS (
                SELECT
                    pi.machine_code,
                    pi.plan_date,
                    COALESCE(SUM(pi.weight_t), 0) AS total_weight_t
                FROM plan_item pi
                WHERE pi.version_id = ?1
                  AND pi.plan_date BETWEEN ?2 AND ?3
                GROUP BY pi.machine_code, pi.plan_date
            ),
            actual AS (
                SELECT
                    pi.machine_code,
                    pi.plan_date,
                    {category_expr} AS category,
                    COALESCE(SUM(pi.weight_t), 0) / NULLIF(at.total_weight_t, 0) AS actual_ratio
                FROM plan_item pi
                JOIN material_master mm ON mm.material_id = pi.material_id
                JOIN actual_total at ON at.machine_code = pi.machine_code AND at.plan_date = pi.plan_date
                WHERE pi.version_id = ?1
                  AND at.total_weight_t > 0
                GROUP BY pi.machine_code, pi.plan_date, category
            ),
            target_kv AS (
                SELECT
                    t.machine_code,
                    t.plan_date,
                    je.key AS category,
                    CAST(je.value AS REAL) AS target_ratio
                FROM target t, json_each(t.target_json) je
            ),
            diff_target_keys AS (
                SELECT
                    tk.machine_code,
                    tk.plan_date,
                    tk.category,
                    tk.target_ratio,
                    COALESCE(a.actual_ratio, 0) AS actual_ratio,
                    ABS(COALESCE(a.actual_ratio, 0) - tk.target_ratio) AS diff
                FROM target_kv tk
                LEFT JOIN actual a
                  ON a.machine_code = tk.machine_code
                 AND a.plan_date = tk.plan_date
                 AND a.category = tk.category
            ),
            diff_actual_only AS (
                SELECT
                    a.machine_code,
                    a.plan_date,
                    a.category,
                    0 AS target_ratio,
                    a.actual_ratio AS actual_ratio,
                    a.actual_ratio AS diff
                FROM actual a
                LEFT JOIN target_kv tk
                  ON tk.machine_code = a.machine_code
                 AND tk.plan_date = a.plan_date
                 AND tk.category = a.category
                WHERE tk.category IS NULL
            ),
            diff_all AS (
                SELECT * FROM diff_target_keys
                UNION ALL
                SELECT * FROM diff_actual_only
            ),
            diff_filtered AS (
                SELECT
                    d.machine_code,
                    d.plan_date,
                    d.diff
                FROM diff_all d
                WHERE ?4 <= 0
                   OR (CASE WHEN d.target_ratio >= d.actual_ratio THEN d.target_ratio ELSE d.actual_ratio END) >= ?4
            ),
            sumdiff AS (
                SELECT
                    machine_code,
                    plan_date,
                    COALESCE(SUM(diff), 0) AS sum_diff
                FROM diff_filtered
                GROUP BY machine_code, plan_date
            ),
            weighted_dev AS (
                SELECT
                    td.machine_code,
                    td.plan_date,
                    CASE
                        WHEN COALESCE(at.total_weight_t, 0) <= 0 THEN 0.0
                        ELSE COALESCE(sd.sum_diff, 0) / 2.0
                    END AS weighted_dev
                FROM target_dates td
                LEFT JOIN sumdiff sd
                  ON sd.machine_code = td.machine_code
                 AND sd.plan_date = td.plan_date
                LEFT JOIN actual_total at
                  ON at.machine_code = td.machine_code
                 AND at.plan_date = td.plan_date
            )
            SELECT
                machine_code,
                plan_date,
                weighted_dev
            FROM weighted_dev
            "#
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(
            params![version_id, start_date, end_date, small_category_threshold],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                ))
            },
        )?;

        for row in rows {
            let (machine_code, plan_date, deviation) = row?;
            if let Some(entry) = bottleneck_map.get_mut(&(machine_code, plan_date)) {
                entry.set_structure_deviation(deviation);
            }
        }

        Ok(())
    }

    /// 计算“未排材料（到当日仍未排入 <= 当日 的量）”并填充到聚合数据中
    ///
    /// # 说明
    /// - pending(D) = sum(ready_unscheduled_incr(<=D))
    /// - ready_unscheduled_incr：按 effective_earliest_date 累计（FORCE_RELEASE 视为 start_date）
    /// - unscheduled：当前版本内未出现在 plan_item 的材料
    ///
    /// # 业务含义
    /// - 未排材料按机组×日期统计，会随日期推进逐步收敛（若后续日期排入足够材料）
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

        // pending increments: READY/FORCE_RELEASE/LOCKED 且未排入 plan_item
        let pending_incr_map =
            Self::query_unscheduled_ready_increments(conn, version_id, &machine_codes, start_date, end_date)?;

        let pending_map = Self::compute_pending_map(
            &machine_codes,
            &profile_dates,
            &pending_incr_map,
        );

        for ((machine_code, plan_date), entry) in bottleneck_map.iter_mut() {
            if let Some((pending_cnt, pending_weight)) =
                pending_map.get(&(machine_code.clone(), plan_date.clone()))
            {
                entry.pending_materials = *pending_cnt;
                entry.pending_weight_t = *pending_weight;
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
    structure_deviation: f64,
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
            structure_deviation: 0.0,
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

    fn set_structure_deviation(&mut self, deviation: f64) {
        self.structure_deviation = deviation;
    }

    fn into_profile(self, version_id: String, cfg: &BottleneckScoringConfig) -> MachineBottleneckProfile {
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
        profile.set_capacity_info_with_threshold(
            remaining_capacity_t,
            capacity_utilization,
            cfg.capacity_hard_threshold,
        );

        let structure_dev_sev = cfg.structure_deviation_severity(self.structure_deviation);
        let violation_sev = cfg.structure_violation_severity(self.structure_violations);

        // 设置结构信息
        profile.set_structure_info(self.structure_violations, structure_dev_sev > 0.0);

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

        let capacity_sev = cfg.capacity_severity(capacity_utilization);
        profile.add_reason(
            "CAPACITY_UTILIZATION".to_string(),
            format!(
                "产能利用率 {:.1}%（硬阈值 {:.1}%）",
                capacity_utilization * 100.0,
                cfg.capacity_hard_threshold * 100.0
            ),
            capacity_sev,
            0,
        );

        if self.structure_deviation > 0.0 {
            profile.add_reason(
                "STRUCTURE_DEVIATION".to_string(),
                format!(
                    "加权节奏偏差 {:.1}%（阈值 {:.1}%，小类<{:.1}%忽略）",
                    self.structure_deviation * 100.0,
                    cfg.structure_dev_threshold * 100.0,
                    cfg.structure_small_category_threshold * 100.0
                ),
                structure_dev_sev,
                0,
            );
        }

        if self.structure_violations > 0 {
            profile.add_reason(
                "STRUCTURE_VIOLATION".to_string(),
                format!("结构违规 {} 个", self.structure_violations),
                violation_sev,
                self.structure_violations,
            );
        }

        if self.pending_materials > 20 {
            profile.add_reason(
                "HIGH_PENDING_COUNT".to_string(),
                format!("未排材料数量较多 {} 个（到当日仍未排入≤当日）", self.pending_materials),
                0.0,
                self.pending_materials,
            );
        }

        if remaining_capacity_t < 100.0 && self.pending_weight_t > 0.0 {
            profile.add_reason(
                "LOW_REMAINING_CAPACITY".to_string(),
                format!(
                    "剩余产能不足 {:.1}t，未排 {:.1}t（到当日仍未排入≤当日）",
                    remaining_capacity_t, self.pending_weight_t
                ),
                0.0,
                0,
            );
        }

        // 使用配置阈值重新计算等级
        profile.apply_scoring_thresholds(
            cfg.low_threshold,
            cfg.medium_threshold,
            cfg.high_threshold,
            cfg.critical_threshold,
        );

        // 添加建议措施
        if profile.is_severe() {
            if self.overflow_t > 0.0 {
                profile.add_suggested_action("调整产能池上限".to_string());
            }
            if self.structure_violations > 0 || self.structure_deviation > 0.0 {
                profile.add_suggested_action("优先处理结构冲突/节奏偏差材料".to_string());
            }
            if self.pending_materials > 20 {
                profile.add_suggested_action("将部分材料转移至其他机组或延后至后续日期".to_string());
            }
        }

        profile
    }
}

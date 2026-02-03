use super::*;
use crate::decision::use_cases::d4_machine_bottleneck::MachineBottleneckProfile;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn setup_test_db() -> Arc<Mutex<Connection>> {
    let conn = Connection::open_in_memory().unwrap();

    // 创建 capacity_pool 表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS capacity_pool (
            version_id TEXT NOT NULL,
            machine_code TEXT NOT NULL,
            plan_date TEXT NOT NULL,
            target_capacity_t REAL NOT NULL,
            limit_capacity_t REAL NOT NULL,
            used_capacity_t REAL NOT NULL DEFAULT 0.0,
            overflow_t REAL NOT NULL DEFAULT 0.0,
            frozen_capacity_t REAL NOT NULL DEFAULT 0.0,
            accumulated_tonnage_t REAL NOT NULL DEFAULT 0.0,
            roll_campaign_id TEXT,
            PRIMARY KEY (version_id, machine_code, plan_date)
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
            version_id, machine_code, plan_date, target_capacity_t, limit_capacity_t,
            used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            "V001",
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
            version_id, machine_code, plan_date, target_capacity_t, limit_capacity_t,
            used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            "V001",
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
    // pending_materials 口径：缺口（到当日仍未排入≤当日）
    assert_eq!(h033.pending_materials, 5);
    assert_eq!(h033.structure_violations, 5);
    // scheduled_materials 来自 plan_item
    assert_eq!(h033.scheduled_materials, 25);

    // 第二个应该是 H032（高利用率）
    let h032 = &profiles[1];
    assert_eq!(h032.machine_code, "H032");
    assert!(h032.bottleneck_score > 0.0);
    // pending_materials 口径：缺口（到当日仍未排入≤当日）
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
fn test_pending_materials_gap_by_date() {
    // 新测试：验证缺口（到当日仍未排入≤当日）按日期随排产累计收敛
    let conn_arc = setup_test_db();
    {
        let conn = conn_arc.lock().expect("锁获取失败");
        // capacity_pool: H032 两天
        conn.execute(
            r#"
            INSERT INTO capacity_pool (
                version_id, machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
            ) VALUES
                ('V001', 'H032', '2026-01-24', 1500.0, 2000.0, 1500.0, 0.0, 0.0, 0.0, NULL),
                ('V001', 'H032', '2026-01-25', 1500.0, 2000.0, 1500.0, 0.0, 0.0, 0.0, NULL)
            "#,
            [],
        )
        .unwrap();

        // material_master: 4 件需求（其中 1 件未排）
        for i in 1..=4 {
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

        // material_state: 3 件 2026-01-24 起可排，1 件 2026-01-25 起可排
        for i in 1..=4 {
            let earliest = if i == 3 { "2026-01-25" } else { "2026-01-24" };
            let sched_state = if i == 1 { "SCHEDULED" } else { "READY" };
            conn.execute(
                r#"
                INSERT INTO material_state (
                    material_id, sched_state, lock_flag, force_release_flag,
                    urgent_level, earliest_sched_date,
                    updated_at, updated_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("MAT{:03}", i),
                    sched_state,
                    0,
                    0,
                    "L0",
                    earliest,
                    "2026-01-01T00:00:00Z",
                    "SYSTEM"
                ],
            )
            .unwrap();
        }

        // plan_item: 2026-01-24 排 1 件；2026-01-25 排 2 件（第 4 件不排）
        conn.execute(
            r#"
            INSERT INTO plan_item (
                version_id, material_id, machine_code, plan_date, seq_no, weight_t,
                source_type, locked_in_plan, force_release_in_plan, violation_flags
            ) VALUES
                ('V001', 'MAT001', 'H032', '2026-01-24', 1, 100.0, 'AUTO', 0, 0, ''),
                ('V001', 'MAT002', 'H032', '2026-01-25', 1, 100.0, 'AUTO', 0, 0, ''),
                ('V001', 'MAT003', 'H032', '2026-01-25', 2, 100.0, 'AUTO', 0, 0, '')
            "#,
            [],
        )
        .unwrap();
    }

    let repo = BottleneckRepository::new(conn_arc);
    let profiles = repo
        .get_bottleneck_profile("V001", Some("H032"), "2026-01-24", "2026-01-25")
        .unwrap();

    assert_eq!(profiles.len(), 2);

    let mut by_date: HashMap<String, MachineBottleneckProfile> = HashMap::new();
    for p in profiles {
        by_date.insert(p.plan_date.clone(), p);
    }

    let p_24 = by_date.get("2026-01-24").expect("missing 2026-01-24");
    // 2026-01-24：需求 3（MAT001/MAT002/MAT004），已排 1（MAT001）→ 缺口 2
    assert_eq!(p_24.pending_materials, 2);
    assert_eq!(p_24.pending_weight_t, 200.0);
    assert_eq!(p_24.scheduled_materials, 1);
    assert_eq!(p_24.scheduled_weight_t, 100.0);

    let p_25 = by_date.get("2026-01-25").expect("missing 2026-01-25");
    // 2026-01-25：需求 4（+MAT003），已排累计 3（+MAT002/MAT003）→ 缺口 1（MAT004）
    assert_eq!(p_25.pending_materials, 1);
    assert_eq!(p_25.pending_weight_t, 100.0);
    assert_eq!(p_25.scheduled_materials, 2);
    assert_eq!(p_25.scheduled_weight_t, 200.0);
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
                version_id, machine_code, plan_date, target_capacity_t, limit_capacity_t,
                used_capacity_t, overflow_t, frozen_capacity_t, accumulated_tonnage_t, roll_campaign_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                "V001",
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

#[test]
fn test_read_model_reason_parsing_and_enrich_fields() {
    // 回归测试：读模型 reasons 字段使用 affected_materials 时应能正常解析；
    // 同时补齐 scheduled/pending 的数量与重量，避免前端出现“利用率很高但材料数为 0 / 原因为空”。
    let conn_arc = setup_test_db();
    {
        let conn = conn_arc.lock().expect("锁获取失败");

        // 创建读模型表（最小字段集）
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS decision_machine_bottleneck (
                version_id TEXT NOT NULL,
                machine_code TEXT NOT NULL,
                plan_date TEXT NOT NULL,
                bottleneck_score REAL NOT NULL,
                bottleneck_level TEXT NOT NULL,
                bottleneck_types TEXT NOT NULL,
                reasons TEXT NOT NULL,
                remaining_capacity_t REAL NOT NULL,
                capacity_utilization REAL NOT NULL,
                needs_roll_change INTEGER NOT NULL DEFAULT 0,
                structure_violations INTEGER NOT NULL DEFAULT 0,
                pending_materials INTEGER NOT NULL DEFAULT 0,
                suggested_actions TEXT
            )
            "#,
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            INSERT INTO decision_machine_bottleneck (
                version_id, machine_code, plan_date,
                bottleneck_score, bottleneck_level,
                bottleneck_types, reasons,
                remaining_capacity_t, capacity_utilization,
                needs_roll_change, structure_violations, pending_materials, suggested_actions
            ) VALUES (
                'V001', 'H034', '2026-01-31',
                95.0, 'CRITICAL',
                '["Capacity"]',
                '[{"code":"CAPACITY_UTILIZATION","description":"产能利用率: 114.7%","severity":1.147,"affected_materials":0}]',
                -100.0, 1.147,
                0, 0, 0, '[]'
            )
            "#,
            [],
        )
        .unwrap();

        // 插入 plan_item（已排 2 件，共 200t）
        for i in 1..=2 {
            // 对齐真实 schema：plan_item.material_id 通常引用 material_master.material_id
            conn.execute(
                r#"
                INSERT INTO material_master (
                    material_id, current_machine_code, next_machine_code, weight_t,
                    manufacturing_order_id, contract_no, due_date,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    format!("PM{:03}", i),
                    "H031",
                    "H034",
                    100.0,
                    format!("MO_PM{:03}", i),
                    format!("C_PM{:03}", i),
                    "2026-02-01",
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:00:00Z"
                ],
            )
            .unwrap();

            conn.execute(
                r#"
                INSERT INTO plan_item (
                    version_id, material_id, machine_code, plan_date, seq_no, weight_t,
                    source_type, locked_in_plan, force_release_in_plan, violation_flags
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    "V001",
                    format!("PM{:03}", i),
                    "H034",
                    "2026-01-31",
                    i,
                    100.0,
                    "AUTO",
                    0,
                    0,
                    ""
                ],
            )
            .unwrap();
        }

        // 插入待排材料（READY 1 件，共 50t，next_machine_code = H034）
        conn.execute(
            r#"
            INSERT INTO material_master (
                material_id, current_machine_code, next_machine_code, weight_t,
                manufacturing_order_id, contract_no, due_date,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                "PENDING_001",
                "H031",
                "H034",
                50.0,
                "MO_P001",
                "C_P001",
                "2026-02-01",
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:00:00Z"
            ],
        )
        .unwrap();

        conn.execute(
            r#"
            INSERT INTO material_state (
                material_id, sched_state, lock_flag, force_release_flag,
                urgent_level, updated_at, updated_by
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                "PENDING_001",
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

    let repo = BottleneckRepository::new(conn_arc);
    let profiles = repo
        .get_bottleneck_profile("V001", Some("H034"), "2026-01-31", "2026-01-31")
        .unwrap();

    assert_eq!(profiles.len(), 1);
    let p = &profiles[0];
    assert_eq!(p.machine_code, "H034");
    assert_eq!(p.plan_date, "2026-01-31");
    assert_eq!(p.bottleneck_level, "CRITICAL");
    assert_eq!(p.scheduled_materials, 2);
    assert_eq!(p.scheduled_weight_t, 200.0);
    assert_eq!(p.pending_materials, 1);
    assert_eq!(p.pending_weight_t, 50.0);
    assert!(
        p.reasons.iter().any(|r| r.code == "CAPACITY_UTILIZATION"),
        "应能解析读模型 reasons 字段"
    );
}

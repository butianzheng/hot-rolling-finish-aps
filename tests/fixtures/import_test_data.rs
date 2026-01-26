// 导入测试数据到数据库
use chrono::Utc;
use rusqlite::{params, Connection};
use std::error::Error;
use uuid::Uuid;

fn main() -> Result<(), Box<dyn Error>> {
    println!("开始导入测试数据到数据库...");

    let conn = Connection::open("hot_rolling_aps.db")?;

    // 清空现有数据
    println!("清空现有数据...");
    conn.execute("DELETE FROM material_state", [])?;
    conn.execute("DELETE FROM material_master", [])?;

    // 生成测试数据
    let today = Utc::now().date_naive();

    // 场景1: 正常材料（50条）
    println!("生成场景1: 正常材料（50条）...");
    for i in 1..=50 {
        let material_id = format!("MAT{:06}", i);
        let order_id = format!("ORDER{:04}", i);

        // 插入 material_master
        conn.execute(
            "INSERT INTO material_master (material_id, manufacturing_order_id, steel_mark,
             thickness_mm, width_mm, weight_t, current_machine_code, due_date, rush_flag,
             created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &material_id,
                &order_id,
                "Q235B",
                3.0 + (i as f64 * 0.1),
                1500.0,
                25.0 + (i as f64 * 0.5),
                "M01",
                (today + chrono::Duration::days((i % 10) as i64)).format("%Y-%m-%d").to_string(),
                "N",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )?;

        // 插入 material_state
        let sched_state = if i % 10 == 0 { "frozen" } else { "ready" };
        let urgent_level = match i % 4 {
            0 => "L0",
            1 => "L1",
            2 => "L2",
            _ => "L3",
        };
        let in_frozen = if i % 10 == 0 { 1 } else { 0 };

        conn.execute(
            "INSERT INTO material_state (material_id, sched_state, urgent_level, lock_flag,
             in_frozen_zone, earliest_sched_date, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &material_id,
                sched_state,
                urgent_level,
                0,
                in_frozen,
                (today + chrono::Duration::days((i % 5) as i64)).format("%Y-%m-%d").to_string(),
                Utc::now().to_rfc3339(),
            ],
        )?;
    }

    // 场景2: 紧急材料（20条，全部L0）
    println!("生成场景2: 紧急材料（20条）...");
    for i in 51..=70 {
        let material_id = format!("URGENT{:06}", i);
        let order_id = format!("URGENT{:04}", i);

        conn.execute(
            "INSERT INTO material_master (material_id, manufacturing_order_id, steel_mark,
             thickness_mm, width_mm, weight_t, current_machine_code, due_date, rush_flag,
             created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &material_id,
                &order_id,
                "Q345B",
                4.0,
                1800.0,
                30.0,
                "M02",
                today.format("%Y-%m-%d").to_string(),
                "Y",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )?;

        conn.execute(
            "INSERT INTO material_state (material_id, sched_state, urgent_level, lock_flag,
             in_frozen_zone, manual_urgent_flag, earliest_sched_date, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &material_id,
                "ready",
                "L0",
                0,
                0,
                1,
                today.format("%Y-%m-%d").to_string(),
                Utc::now().to_rfc3339(),
            ],
        )?;
    }

    // 场景3: 锁定材料（10条）
    println!("生成场景3: 锁定材料（10条）...");
    for i in 71..=80 {
        let material_id = format!("LOCKED{:06}", i);
        let order_id = format!("LOCKED{:04}", i);

        conn.execute(
            "INSERT INTO material_master (material_id, manufacturing_order_id, steel_mark,
             thickness_mm, width_mm, weight_t, current_machine_code, due_date, rush_flag,
             created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &material_id,
                &order_id,
                "SPCC",
                2.5,
                1200.0,
                20.0,
                "M01",
                (today + chrono::Duration::days(5)).format("%Y-%m-%d").to_string(),
                "N",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )?;

        conn.execute(
            "INSERT INTO material_state (material_id, sched_state, urgent_level, lock_flag,
             in_frozen_zone, earliest_sched_date, updated_at, updated_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &material_id,
                "ready",
                "L2",
                1,
                0,
                today.format("%Y-%m-%d").to_string(),
                Utc::now().to_rfc3339(),
                "admin",
            ],
        )?;
    }

    // 场景4: 冻结材料（15条）
    println!("生成场景4: 冻结材料（15条）...");
    for i in 81..=95 {
        let material_id = format!("FROZEN{:06}", i);
        let order_id = format!("FROZEN{:04}", i);

        conn.execute(
            "INSERT INTO material_master (material_id, manufacturing_order_id, steel_mark,
             thickness_mm, width_mm, weight_t, current_machine_code, due_date, rush_flag,
             created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &material_id,
                &order_id,
                "Q235B",
                3.5,
                1600.0,
                28.0,
                "M03",
                (today + chrono::Duration::days(15)).format("%Y-%m-%d").to_string(),
                "N",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )?;

        conn.execute(
            "INSERT INTO material_state (material_id, sched_state, urgent_level, lock_flag,
             in_frozen_zone, earliest_sched_date, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &material_id,
                "frozen",
                "L3",
                0,
                1,
                (today + chrono::Duration::days(10)).format("%Y-%m-%d").to_string(),
                Utc::now().to_rfc3339(),
            ],
        )?;
    }

    // 场景5: 未适温材料（10条）
    println!("生成场景5: 未适温材料（10条）...");
    for i in 96..=105 {
        let material_id = format!("IMMATURE{:06}", i);
        let order_id = format!("IMMATURE{:04}", i);

        conn.execute(
            "INSERT INTO material_master (material_id, manufacturing_order_id, steel_mark,
             thickness_mm, width_mm, weight_t, current_machine_code, due_date, rush_flag,
             created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &material_id,
                &order_id,
                "Q345B",
                4.5,
                1900.0,
                35.0,
                "M02",
                (today + chrono::Duration::days(7)).format("%Y-%m-%d").to_string(),
                "N",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )?;

        conn.execute(
            "INSERT INTO material_state (material_id, sched_state, urgent_level, lock_flag,
             in_frozen_zone, ready_in_days, earliest_sched_date, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &material_id,
                "ready",
                "L1",
                0,
                0,
                3,
                (today + chrono::Duration::days(3)).format("%Y-%m-%d").to_string(),
                Utc::now().to_rfc3339(),
            ],
        )?;
    }

    // 场景6: 多机组材料
    println!("生成场景6: 多机组材料（15条）...");
    let machines = ["M01", "M02", "M03"];
    for i in 106..=120 {
        let material_id = format!("MULTI{:06}", i);
        let order_id = format!("MULTI{:04}", i);
        let machine = machines[(i - 106) % 3];

        conn.execute(
            "INSERT INTO material_master (material_id, manufacturing_order_id, steel_mark,
             thickness_mm, width_mm, weight_t, current_machine_code, due_date, rush_flag,
             created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &material_id,
                &order_id,
                "SPCC",
                2.0 + ((i - 106) as f64 * 0.2),
                1400.0,
                22.0,
                machine,
                (today + chrono::Duration::days(3)).format("%Y-%m-%d").to_string(),
                "N",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )?;

        conn.execute(
            "INSERT INTO material_state (material_id, sched_state, urgent_level, lock_flag,
             in_frozen_zone, earliest_sched_date, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &material_id,
                "ready",
                "L2",
                0,
                0,
                today.format("%Y-%m-%d").to_string(),
                Utc::now().to_rfc3339(),
            ],
        )?;
    }

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM material_master", [], |row| row.get(0))?;

    println!("✓ 数据导入完成！");
    println!("  - 总计导入: {} 条材料", count);
    println!("  - 场景1: 正常材料 50条");
    println!("  - 场景2: 紧急材料 20条（L0）");
    println!("  - 场景3: 锁定材料 10条");
    println!("  - 场景4: 冻结材料 15条");
    println!("  - 场景5: 未适温材料 10条");
    println!("  - 场景6: 多机组材料 15条");
    println!("\n现在可以在应用程序中查看这些数据了！");

    Ok(())
}

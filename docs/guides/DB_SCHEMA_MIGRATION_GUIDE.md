# 数据库 Schema / 迁移统一约定（v0.6）

## 目标

- 统一“建库/迁移”的权威来源与执行路径，避免出现：代码已升级但数据库结构未升级，导致隐性运行错误。
- 让 `schema_version` 具备可用的最低语义：能用于启动时提示数据库是否明显过旧。

---

## 1) 新建/重置开发库（推荐）

开发环境以 `scripts/dev_db/schema.sql` 为**全量建库脚本**（Single Source for dev reset）。

常用方式：

- 使用 bin 一键重置+播种（会备份旧库）
  - `cargo run --bin reset_and_seed_full_scenario_db -- <db_path> <material_count>`

该流程会：
- 执行 `scripts/dev_db/schema.sql`
- 写入 `schema_version=6`（用于启动提示）

---

## 2) 现有数据库升级（手工迁移）

增量迁移脚本集中放在 `migrations/`，按版本号顺序执行：

1. `migrations/v0.2_importer_schema.sql`
2. `migrations/v0.3_material_state_enhancement.sql`
3. `migrations/v0.4_decision_layer.sql`
4. `migrations/v0.5_strategy_draft.sql`
5. `migrations/v0.6_path_rule_extension.sql`
6. `migrations/v0.6_path_override_pending.sql`

示例（请先备份数据库）：

```bash
cp hot_rolling_aps.db hot_rolling_aps.db.bak.$(date +%Y%m%d_%H%M%S)
sqlite3 hot_rolling_aps.db < migrations/v0.2_importer_schema.sql
sqlite3 hot_rolling_aps.db < migrations/v0.3_material_state_enhancement.sql
sqlite3 hot_rolling_aps.db < migrations/v0.4_decision_layer.sql
sqlite3 hot_rolling_aps.db < migrations/v0.5_strategy_draft.sql
sqlite3 hot_rolling_aps.db < migrations/v0.6_path_rule_extension.sql
sqlite3 hot_rolling_aps.db < migrations/v0.6_path_override_pending.sql
```

迁移完成后建议更新 `schema_version`：

```bash
sqlite3 hot_rolling_aps.db "INSERT OR REPLACE INTO schema_version (version, applied_at) VALUES (6, datetime('now'));"
```

> 注意：当前应用启动会读取 `schema_version` 做**告警提示**，不会自动执行迁移；因此生产库升级仍需按上面步骤人工执行。

---

## 3) 关于 `scripts/migrations/*`

`scripts/migrations/` 目前主要用于历史数据库的“专项迁移脚本”（例如 capacity_pool 版本化），不作为全量升级链路的唯一依据。

如需在生产升级链路中继续保留该类脚本，建议后续将其迁移为 `migrations/` 的一部分，并统一维护 `schema_version` 语义。


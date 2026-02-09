# 数据库迁移指南

> 本目录包含从旧版本升级到新版本的增量迁移脚本。

## 快速参考

### 何时使用增量迁移

| 场景 | 推荐操作 |
|------|---------|
| **新建开发库** | 使用 `cargo run --bin reset_and_seed_full_scenario_db`，不需要迁移 |
| **升级现有开发库** | 执行本目录中的增量迁移脚本 |
| **升级生产库** | 执行增量迁移 + 人工验证 + 备份 |

### 权威 Schema 来源

- **新建库**：`scripts/dev_db/schema.sql`（全量，包含所有 v0.2-v0.10 特性）
- **增量升级**：本目录的 `v0.*.sql` 文件

## 迁移文件清单

| 文件 | 版本 | 功能 | 依赖 |
|------|------|------|------|
| `v0.2_importer_schema.sql` | 0→2 | 导入功能增强（import_batch 表、material_master 扩展） | 基础 schema |
| `v0.3_material_state_enhancement.sql` | 2→3 | 物料状态完整化（8 个新字段 + 4 个索引） | v0.2 |
| `v0.4_decision_layer.sql` | 3→4 | 决策读模型 D1-D6（6 个新表） | v0.3 |
| `v0.5_strategy_draft.sql` | 4→5 | 策略草案持久化（decision_strategy_draft 表） | v0.4 |
| `v0.6_path_rules_complete.sql` | 5→6 | 宽厚路径规则完整实现（合并版本） | v0.5 |
| `v0.8_path_override_reject_flow.sql` | 6→8 | 路径规则拒绝闭环（拒绝态字段 + 索引） | v0.6 |
| `v0.9_material_management_coverage_alert_threshold.sql` | 8→9 | 物料管理机组覆盖异常阈值配置化（默认4） | v0.8 |
| `v0.10_empty_day_recover_threshold.sql` | 9→10 | 连续排程空白日兜底阈值配置化（默认200吨） | v0.9 |

### ⚠️ 弃用文件

以下文件已弃用，请使用 `v0.6_path_rules_complete.sql` 代替：
- ~~`v0.6_path_override_pending.sql`~~ → 已合并到 `v0.6_path_rules_complete.sql`
- ~~`v0.6_path_rule_extension.sql`~~ → 已合并到 `v0.6_path_rules_complete.sql`

## 执行顺序

必须按版本号顺序执行：

```bash
# 1. 备份现有数据库
cp hot_rolling_aps.db "hot_rolling_aps.db.bak.$(date +%Y%m%d_%H%M%S)"

# 2. 执行迁移（按顺序）
sqlite3 hot_rolling_aps.db < migrations/v0.2_importer_schema.sql
sqlite3 hot_rolling_aps.db < migrations/v0.3_material_state_enhancement.sql
sqlite3 hot_rolling_aps.db < migrations/v0.4_decision_layer.sql
sqlite3 hot_rolling_aps.db < migrations/v0.5_strategy_draft.sql
sqlite3 hot_rolling_aps.db < migrations/v0.6_path_rules_complete.sql
sqlite3 hot_rolling_aps.db < migrations/v0.8_path_override_reject_flow.sql
sqlite3 hot_rolling_aps.db < migrations/v0.9_material_management_coverage_alert_threshold.sql
sqlite3 hot_rolling_aps.db < migrations/v0.10_empty_day_recover_threshold.sql

# 3. 验证版本
sqlite3 hot_rolling_aps.db "SELECT * FROM schema_version;"
# 应显示 version = 10
```

## 迁移特性说明

### v0.2: 导入功能增强

- `material_master` 新增字段：`contract_nature`、`weekly_delivery_flag`、`export_flag`
- 新增表：`import_batch`（批次元信息）
- `import_conflict` 新增字段：`row_number`

### v0.3: 物料状态完整化

- `material_state` 新增 8 个字段：
  - `rush_level`、`stock_age_days`
  - `scheduled_date`、`scheduled_machine_code`、`seq_no`
  - `manual_urgent_flag`、`in_frozen_zone`、`updated_by`
- 新增 4 个性能索引

### v0.4: 决策读模型 (D1-D6)

- D1: `decision_day_summary` — 哪天最危险
- D2: `decision_order_failure_set` — 哪些紧急单无法完成
- D3: `decision_cold_stock_profile` — 哪些冷料压库
- D4: `decision_machine_bottleneck` — 哪个机组最堵
- D5: `decision_roll_campaign_alert` — 换辊是否异常
- D6: `decision_capacity_opportunity` — 是否存在产能优化空间
- 辅助表：`decision_refresh_queue`、`decision_refresh_log`

### v0.5: 策略草案持久化

- 新增表：`decision_strategy_draft`（策略草案 JSON 存储）

### v0.6: 宽厚路径规则完整实现

- 新增表：`path_override_pending`（待人工确认的路径突破清单）
- `material_state` 新增字段：`user_confirmed`、`user_confirmed_at`、`user_confirmed_by`、`user_confirmed_reason`
- `roller_campaign` 新增字段：`path_anchor_material_id`、`path_anchor_width_mm`、`path_anchor_thickness_mm`、`anchor_source`
- 默认配置初始化：路径规则参数（6 条配置项）

### v0.8: 路径规则拒绝闭环

- `material_state` 新增拒绝字段：
  - `path_override_rejected`
  - `path_override_rejected_at`
  - `path_override_rejected_by`
  - `path_override_rejected_reason`
  - `path_override_reject_cycle_no`
  - `path_override_reject_base_sched_state`
- 新增索引：`idx_material_state_path_override_rejected`

### v0.9: 物料管理机组覆盖异常阈值配置化

- 新增配置项：`material_management_coverage_alert_threshold`
- 默认值：`4`（global scope）
- 用途：控制“物料管理”页面机组覆盖异常红色告警阈值

### v0.10: 连续排程空白日兜底阈值配置化

- 新增配置项：`empty_day_recover_threshold_t`
- 默认值：`200`（global scope）
- 用途：作为连续排程“最小可排量阈值（开机阈值）”。当某机组当日直接可排量低于阈值，且“直接可排量+仅因拒绝待下一周期恢复而阻塞的吨位”达到阈值时，自动后移一套换辊周期并重试当日排程

## 幂等性说明

迁移脚本设计为**部分幂等**：

- ✅ **表创建**：使用 `CREATE TABLE IF NOT EXISTS`，可安全重复执行
- ✅ **数据插入**：使用 `INSERT OR IGNORE`，可安全重复执行
- ⚠️ **表结构修改**：`ALTER TABLE ADD COLUMN` 在列已存在时会报错（SQLite 限制）

**重要提示**：
- 迁移脚本设计用于从 v(n-1) 升级到 vn，**不应在已完成迁移的数据库上重复执行**
- 如果误执行，可能会遇到 "duplicate column" 错误（无害，但会中断后续语句）
- 请先用 `SELECT * FROM schema_version;` 检查当前版本，避免重复迁移

## 回滚策略

1. **推荐**：使用迁移前的备份恢复
   ```bash
   cp hot_rolling_aps.db.bak.YYYYMMDD_HHMMSS hot_rolling_aps.db
   ```

2. **手工回滚**：删除新增的表和字段（较复杂，不推荐）

## 版本检查

应用启动时会检查 `schema_version` 表：

- 若版本低于 `CURRENT_SCHEMA_VERSION`（当前为 10），会输出警告日志
- 不会自动执行迁移，需要人工确认

## 历史迁移脚本

`scripts/migrations/` 目录包含历史性的一次性迁移脚本，与本目录的增量迁移分开管理：

- `001_capacity_pool_versioning.sql` — 历史性的 capacity_pool 版本化迁移（现已整合到全量 schema）

---

**更新日期**：2026-02-08
**当前版本**：v0.10 (schema_version = 10)

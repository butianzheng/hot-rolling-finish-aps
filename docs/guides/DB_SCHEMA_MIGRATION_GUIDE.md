# 数据库 Schema / 迁移统一约定（v0.6）

## 目标

- 统一"建库/迁移"的权威来源与执行路径，避免出现：代码已升级但数据库结构未升级，导致隐性运行错误。
- 让 `schema_version` 具备可用的最低语义：能用于启动时提示数据库是否明显过旧。
- 明确 `ensure_schema`（自动建表）与 `migrations`（手动升级）的职责分工。

---

## 0) 职责分工（重要）

### ensure_schema（自动，仅首次建表）

- **触发时机**：应用启动时，如果检测到 `schema_version` 表不存在
- **执行内容**：自动执行 `scripts/dev_db/schema.sql` 完整建表脚本
- **版本标记**：自动插入当前版本号（`CURRENT_SCHEMA_VERSION = 6`）
- **适用场景**：
  - 开发环境首次启动（无需手动执行 SQL）
  - 生产环境首次部署（无需手动执行 SQL）
  - 测试环境快速初始化

### migrations/*.sql（手动，增量升级）

- **触发时机**：人工确认后手动执行
- **执行内容**：增量升级脚本（v0→v2, v2→v3, ..., v5→v6）
- **版本标记**：每个脚本内部插入对应版本号
- **适用场景**：
  - 现有数据库从旧版本升级到新版本
  - 生产环境版本升级（需先备份，人工验证）

### 原则

- **不自动执行增量迁移**：即使检测到版本过旧，应用只会告警，不会自动执行 migrations/*.sql
- **符合工业系统要求**：避免自动迁移导致的数据风险，保留人工确认环节

---

## 1) 新建/重置开发库（推荐）

开发环境以 `scripts/dev_db/schema.sql` 为**全量建库脚本**（Single Source for dev reset）。

### 方式 1：使用 bin 一键重置+播种（推荐）

```bash
cargo run --bin reset_and_seed_full_scenario_db -- <db_path> <material_count>
```

该流程会：
- 备份现有数据库（如果存在）
- 执行 `scripts/dev_db/schema.sql`
- 播种测试数据
- 写入 `schema_version=6`

### 方式 2：首次启动自动建表（新增）

**直接启动应用**，如果数据库不存在或 `schema_version` 表缺失，应用会自动：
- 执行 `scripts/dev_db/schema.sql` 完整建表
- 写入 `schema_version=6`
- 启动成功（但无测试数据，需手动导入或播种）

---

## 2) 首次部署（生产环境）

### 场景：第一次部署到生产服务器，数据库完全不存在

**推荐方式**：直接启动应用，`ensure_schema()` 会自动创建完整表结构。

```bash
# 1. 启动应用（会自动建表）
./hot_rolling_aps

# 2. 验证版本
sqlite3 hot_rolling_aps.db "SELECT * FROM schema_version;"
# 应显示 version = 6
```

**替代方式**：手动执行完整建表脚本（适用于对自动化不信任的场景）

```bash
# 1. 手动建表
sqlite3 hot_rolling_aps.db < scripts/dev_db/schema.sql

# 2. 手动插入版本号
sqlite3 hot_rolling_aps.db "INSERT INTO schema_version (version, applied_at) VALUES (6, datetime('now'));"

# 3. 启动应用
./hot_rolling_aps
```

---

## 3) 现有数据库升级（手工迁移）

增量迁移脚本集中放在 `migrations/`，按版本号顺序执行。

**详细文档**：参见 [`migrations/README.md`](../../migrations/README.md)

**迁移文件清单**：

1. `migrations/v0.2_importer_schema.sql` (v0→v2)
2. `migrations/v0.3_material_state_enhancement.sql` (v2→v3)
3. `migrations/v0.4_decision_layer.sql` (v3→v4)
4. `migrations/v0.5_strategy_draft.sql` (v4→v5)
5. `migrations/v0.6_path_rules_complete.sql` (v5→v6, 合并版本)

⚠️ **注意**：v0.6 已合并为单一文件 `v0.6_path_rules_complete.sql`，旧的 `v0.6_path_override_pending.sql` 和 `v0.6_path_rule_extension.sql` 已弃用。

示例（请先备份数据库）：

```bash
# 1. 备份
cp hot_rolling_aps.db "hot_rolling_aps.db.bak.$(date +%Y%m%d_%H%M%S)"

# 2. 执行迁移
sqlite3 hot_rolling_aps.db < migrations/v0.2_importer_schema.sql
sqlite3 hot_rolling_aps.db < migrations/v0.3_material_state_enhancement.sql
sqlite3 hot_rolling_aps.db < migrations/v0.4_decision_layer.sql
sqlite3 hot_rolling_aps.db < migrations/v0.5_strategy_draft.sql
sqlite3 hot_rolling_aps.db < migrations/v0.6_path_rules_complete.sql

# 3. 验证版本
sqlite3 hot_rolling_aps.db "SELECT * FROM schema_version;"
# 应显示 version = 6
```

> 注意：当前应用启动会读取 `schema_version` 做**告警提示**，不会自动执行迁移；因此生产库升级仍需按上面步骤人工执行。

---

## 4) 关于 `scripts/migrations/*`

`scripts/migrations/` 目前主要用于历史数据库的"专项迁移脚本"（例如 capacity_pool 版本化），不作为全量升级链路的唯一依据。

如需在生产升级链路中继续保留该类脚本，建议后续将其迁移为 `migrations/` 的一部分，并统一维护 `schema_version` 语义。

---

## 5) 生产环境部署检查清单

### 首次部署

- [ ] 确认数据库文件路径配置正确（环境变量或配置文件）
- [ ] 启动应用前备份现有数据库（如果存在）
- [ ] 启动应用，观察日志确认 `ensure_schema` 执行成功
- [ ] 执行 `SELECT * FROM schema_version;` 验证版本号为 6
- [ ] 验证关键表是否创建成功：
  ```sql
  SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;
  ```
- [ ] 导入初始数据或执行播种脚本
- [ ] 执行功能测试验证系统正常运行

### 版本升级

- [ ] **先备份数据库**（必须，无备份不升级）
  ```bash
  cp hot_rolling_aps.db "hot_rolling_aps.db.bak.$(date +%Y%m%d_%H%M%S)"
  ```
- [ ] 检查当前版本号：
  ```sql
  SELECT MAX(version) FROM schema_version;
  ```
- [ ] 根据当前版本号，按顺序执行缺失的迁移脚本（参见 `migrations/README.md`）
- [ ] 验证迁移后的版本号：
  ```sql
  SELECT * FROM schema_version ORDER BY version;
  ```
- [ ] 执行数据完整性检查：
  ```sql
  PRAGMA integrity_check;
  PRAGMA foreign_key_check;
  ```
- [ ] 启动应用，观察日志确认无版本告警
- [ ] 执行功能测试验证系统正常运行
- [ ] 如果验证失败，回滚到备份：
  ```bash
  cp hot_rolling_aps.db.bak.YYYYMMDD_HHMMSS hot_rolling_aps.db
  ```

### 版本号告警处理

如果应用启动时出现以下告警：

```
数据库 schema_version=X 低于当前要求 6，可能需要执行迁移 (migrations/ v0.*.sql) 或重置开发库
```

**处理方法**：

- **开发环境**：使用 `cargo run --bin reset_and_seed_full_scenario_db` 重置
- **生产环境**：按"版本升级"检查清单执行增量迁移脚本

**注意**：应用不会自动执行迁移，只会告警并继续启动，但可能会因表结构不匹配导致运行错误。

---

## 6) 常见问题（FAQ）

### Q1: 为什么不自动执行增量迁移？

**A**: 符合工业系统要求：
- 生产数据库变更必须人工确认，避免自动化导致数据丢失
- 迁移脚本可能包含数据转换逻辑，需要验证
- 保留回滚能力（自动迁移难以回滚）

### Q2: 如何在测试环境快速初始化？

**A**: 两种方式：
1. 直接启动应用，`ensure_schema()` 自动建表
2. 使用 `cargo run --bin reset_and_seed_full_scenario_db` 获得完整测试数据

### Q3: schema.sql 与 migrations/*.sql 如何保持一致？

**A**: 维护原则：
- `scripts/dev_db/schema.sql` 是"当前最新版本的完整表结构"（包含 v0.6 所有特性）
- `migrations/v0.*.sql` 是"从旧版本升级到新版本的增量脚本"
- 每次发布新版本时，应同步更新 schema.sql 和创建新的 migration 脚本

### Q4: 如何检查数据库是否需要迁移？

**A**: 查询版本号：
```sql
SELECT MAX(version) FROM schema_version;
```
- 如果返回 `6`：无需迁移
- 如果返回 `< 6`：需要执行缺失的迁移脚本
- 如果表不存在或查询失败：数据库未初始化或损坏


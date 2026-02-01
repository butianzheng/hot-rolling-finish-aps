# capacity_pool 版本化迁移指南

## 概述

本文档描述 `capacity_pool` 表从非版本化到版本化的数据迁移流程。

### 迁移内容

- **当前状态**: `capacity_pool` 主键为 `(machine_code, plan_date)`
- **目标状态**: `capacity_pool` 主键为 `(version_id, machine_code, plan_date)`
- **当前数据量**: 120 行
- **风险等级**: 中等（涉及表结构变更和数据迁移）

---

## 迁移前准备

### 1. 环境检查

```bash
# 1. 确认数据库文件位置
ls -lh hot_rolling_aps.db

# 2. 检查当前表结构
sqlite3 hot_rolling_aps.db "PRAGMA table_info(capacity_pool);"

# 3. 统计当前数据量
sqlite3 hot_rolling_aps.db "SELECT COUNT(*) FROM capacity_pool;"

# 4. 确认是否有激活版本（迁移将使用此版本 ID）
sqlite3 hot_rolling_aps.db "SELECT version_id, status FROM plan_version WHERE status = 'ACTIVE';"
```

### 2. 停止应用程序

⚠️ **重要**: 在迁移前停止所有访问数据库的应用程序，避免数据不一致。

```bash
# 停止 Tauri 应用（如果正在运行）
pkill -f "hot-rolling-finish-aps"
```

### 3. 创建备份目录

```bash
mkdir -p backups
```

---

## 迁移流程

### 方式 1: 使用自动化脚本（推荐）

#### 步骤 1: 执行迁移

```bash
cd /Users/butianzheng/Documents/trae_projects/hot-rolling-finish-aps

# 执行迁移脚本
./scripts/migrations/run_migration.sh
```

**脚本会自动完成以下操作**:

1. ✓ 检查数据库文件存在
2. ✓ 创建时间戳备份到 `backups/hot_rolling_aps_YYYYMMDD_HHMMSS.db`
3. ✓ 执行迁移前检查（表结构、数据量、版本信息）
4. ✓ 询问用户确认
5. ✓ 执行迁移 SQL（`001_capacity_pool_versioning.sql`）
6. ✓ 执行迁移后验证（表结构、数据完整性、索引）
7. ✓ 更新 `schema_version` 表

#### 步骤 2: 验证迁移结果

```bash
# 独立验证脚本（10 项检查）
./scripts/migrations/verify_migration.sh
```

**验证项目**:

- [x] 表结构包含 `version_id` 字段
- [x] 主键为 `(version_id, machine_code, plan_date)`
- [x] 所有行都有有效的 `version_id`
- [x] 数据与 `plan_item` 一致性检查
- [x] 外键约束验证
- [x] 索引存在性检查
- [x] `used_capacity_t` 聚合正确性
- [x] 版本分布统计
- [x] 数据量一致性
- [x] 抽样数据检查

#### 步骤 3: 启动应用验证

```bash
# 启动 Tauri 应用
npm run tauri dev
```

**功能验证清单**:

1. [ ] 工作台页面加载正常
2. [ ] 堵塞矩阵显示正确（利用率 vs 已排数量一致）
3. [ ] 切换版本后产能数据隔离（无跨版本污染）
4. [ ] 决策面板 D4（机组堵塞）数据正确
5. [ ] 版本对比功能正常

---

### 方式 2: 手动执行（高级用户）

#### 步骤 1: 手动备份

```bash
cp hot_rolling_aps.db backups/hot_rolling_aps_manual_$(date +%Y%m%d_%H%M%S).db
```

#### 步骤 2: 执行迁移 SQL

```bash
sqlite3 hot_rolling_aps.db < scripts/migrations/001_capacity_pool_versioning.sql
```

#### 步骤 3: 手动验证

```bash
# 1. 检查表结构
sqlite3 hot_rolling_aps.db "SELECT sql FROM sqlite_master WHERE type='table' AND name='capacity_pool';"

# 2. 检查数据量
sqlite3 hot_rolling_aps.db "SELECT COUNT(*) FROM capacity_pool;"

# 3. 检查版本分布
sqlite3 hot_rolling_aps.db "SELECT version_id, COUNT(*) FROM capacity_pool GROUP BY version_id;"

# 4. 检查外键约束
sqlite3 hot_rolling_aps.db "PRAGMA foreign_key_check(capacity_pool);"
```

---

## 迁移验证

### 数据库层面验证

#### 1. 表结构验证

**预期结果**:

```sql
CREATE TABLE capacity_pool (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  plan_date TEXT NOT NULL,
  target_capacity_t REAL NOT NULL,
  limit_capacity_t REAL NOT NULL,
  used_capacity_t REAL NOT NULL DEFAULT 0.0,
  overflow_t REAL NOT NULL DEFAULT 0.0,
  frozen_capacity_t REAL NOT NULL DEFAULT 0.0,
  accumulated_tonnage_t REAL NOT NULL DEFAULT 0.0,
  roll_campaign_id TEXT,
  PRIMARY KEY (version_id, machine_code, plan_date)
);
```

#### 2. 数据完整性验证

```bash
# 验证所有行都有 version_id
sqlite3 hot_rolling_aps.db <<EOF
SELECT COUNT(*) AS rows_without_version_id
FROM capacity_pool
WHERE version_id IS NULL OR version_id = '';
EOF
```

**预期结果**: `0`

#### 3. 与 plan_item 一致性验证

```bash
sqlite3 hot_rolling_aps.db <<EOF
SELECT
    cp.version_id,
    cp.machine_code,
    cp.plan_date,
    cp.used_capacity_t AS pool_used,
    COALESCE(SUM(pi.weight_t), 0) AS actual_used,
    ABS(cp.used_capacity_t - COALESCE(SUM(pi.weight_t), 0)) AS diff
FROM capacity_pool cp
LEFT JOIN plan_item pi ON
    cp.version_id = pi.version_id AND
    cp.machine_code = pi.machine_code AND
    cp.plan_date = pi.plan_date
GROUP BY cp.version_id, cp.machine_code, cp.plan_date
HAVING ABS(diff) > 0.01
LIMIT 10;
EOF
```

**预期结果**: 空结果集（或差异在可接受范围内）

### 应用层面验证

#### 1. 堵塞矩阵验证（P0-3 修复验证）

**操作步骤**:

1. 打开工作台页面
2. 选择机组（如 H032）
3. 查看堵塞矩阵

**验证点**:

- [ ] 利用率 = `used_capacity_t / target_capacity_t`
- [ ] 已排数量 = 当天 `plan_item` 的材料数量
- [ ] 不应出现"利用率高但已排为 0"的矛盾

#### 2. 版本隔离验证（P1-1 核心目标）

**操作步骤**:

1. 创建新版本（一键重算）
2. 切换到旧版本
3. 切换回新版本

**验证点**:

- [ ] 不同版本的 `capacity_pool` 数据独立
- [ ] 切换版本不会污染产能数据
- [ ] 决策面板（D1-D6）数据随版本切换正确更新

#### 3. 决策刷新验证（P0-2 联动验证）

**操作步骤**:

1. 锁定/解锁材料
2. 检查决策刷新队列
3. 查看 D4（机组堵塞）是否更新

**验证点**:

- [ ] 物料操作触发 `decision_refresh_queue`
- [ ] D4 决策表从 `capacity_pool` 正确读取（带 `version_id` 条件）
- [ ] 堵塞分数随产能变化更新

---

## 回滚方案

### 方式 1: 从备份恢复（推荐）

```bash
# 自动化回滚脚本
./scripts/migrations/rollback_migration.sh

# 选择选项 1: 从备份恢复
# 脚本会自动找到最新备份并恢复
```

### 方式 2: 手动回滚

```bash
# 查找最新备份
ls -lt backups/hot_rolling_aps_*.db | head -1

# 恢复备份（替换 <TIMESTAMP> 为实际时间戳）
cp backups/hot_rolling_aps_<TIMESTAMP>.db hot_rolling_aps.db

# 验证恢复
sqlite3 hot_rolling_aps.db "SELECT sql FROM sqlite_master WHERE type='table' AND name='capacity_pool';"
```

### 方式 3: SQL 回滚（去除 version_id）

⚠️ **警告**: 此方式会合并多版本数据，可能导致数据丢失。

```bash
# 使用回滚脚本的选项 2
./scripts/migrations/rollback_migration.sh
```

---

## 常见问题 (FAQ)

### Q1: 迁移后发现 used_capacity_t 不准确怎么办？

**A**: 这是正常现象，因为旧数据可能已过时。解决方案：

```bash
# 方法 1: 重新执行一键重算（推荐）
# 在应用中点击"一键重算"按钮

# 方法 2: 手动触发产能重算（通过 Tauri 命令）
# 见 src/api/plan_api.rs:recalculate_capacity_pool_for_version()
```

### Q2: 迁移后应用启动报错怎么办？

**A**: 检查以下几点：

1. 确认所有 Rust 代码已更新（见 P1-1 修改清单）
2. 重新编译应用：`npm run tauri build`
3. 检查日志：`~/Library/Logs/com.yourapp.dev/main.log`

### Q3: 如何验证跨版本隔离是否生效？

**A**: 执行以下 SQL：

```sql
-- 查看不同版本的产能池
SELECT version_id, COUNT(*) AS row_count
FROM capacity_pool
GROUP BY version_id;

-- 验证同一天不同版本的产能值不同
SELECT version_id, machine_code, plan_date, used_capacity_t
FROM capacity_pool
WHERE machine_code = 'H032' AND plan_date = '2026-01-25'
ORDER BY version_id;
```

### Q4: 迁移失败了怎么办？

**A**: 按以下步骤处理：

1. **不要 panic**：备份已自动创建在 `backups/` 目录
2. **查看错误信息**：脚本会输出详细错误
3. **回滚到备份**：运行 `./scripts/migrations/rollback_migration.sh`
4. **联系开发者**：提供错误日志和数据库状态

---

## 迁移清单

### 迁移前

- [ ] 已阅读本文档
- [ ] 已停止应用程序
- [ ] 已创建 `backups/` 目录
- [ ] 已确认激活版本存在

### 迁移中

- [ ] 执行 `./scripts/migrations/run_migration.sh`
- [ ] 确认备份已创建
- [ ] 迁移脚本成功完成
- [ ] 执行 `./scripts/migrations/verify_migration.sh`
- [ ] 所有验证通过

### 迁移后

- [ ] 启动应用程序
- [ ] 工作台页面正常加载
- [ ] 堵塞矩阵显示正确
- [ ] 切换版本功能正常
- [ ] 决策面板数据正确
- [ ] 一键重算功能正常

---

## 技术参考

### 相关文件

| 文件路径 | 说明 |
|---------|------|
| `scripts/migrations/001_capacity_pool_versioning.sql` | 迁移 SQL 脚本 |
| `scripts/migrations/run_migration.sh` | 自动化迁移工具 |
| `scripts/migrations/verify_migration.sh` | 验证工具 |
| `scripts/migrations/rollback_migration.sh` | 回滚工具 |
| `scripts/dev_db/schema.sql` | 新表结构定义 |

### 相关代码修改（P1-1）

| 文件 | 修改说明 |
|------|----------|
| `src/domain/capacity.rs` | CapacityPool 增加 version_id 字段 |
| `src/repository/capacity_repo.rs` | 所有方法增加 version_id 参数 |
| `src/api/plan_api.rs` | recalculate_capacity_pool_for_version 增加清零逻辑 |
| `src/decision/services/refresh_service.rs` | D4/D6 刷新 SQL 增加 version_id 条件 |
| `src/engine/risk.rs` | RiskEngine 测试更新 |

### 迁移原理

```
旧表 (machine_code, plan_date)
          ↓
    备份到临时表
          ↓
创建新表 (version_id, machine_code, plan_date)
          ↓
    数据迁移：
    - 使用 ACTIVE 版本 ID
    - 如无 ACTIVE 使用最新版本
    - 如都无则使用 'DEFAULT_VERSION'
          ↓
    删除旧表 → 重命名新表
          ↓
    创建索引 → 验证数据
```

---

## 支持

如有问题，请参考：

- 计划文档：`~/.claude/plans/ancient-stargazing-wozniak.md`
- 评估报告：`docs/reports/DATA_SYNC_ASSESSMENT_REPORT_2026-02-01.md`
- 项目 Wiki：(待补充)

---

**最后更新**: 2026-02-01
**版本**: 1.0
**作者**: Claude Code (自动生成)

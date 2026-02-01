# capacity_pool 迁移快速执行指南

## 🚀 快速开始（3 分钟完成迁移）

### 前提条件

- ✅ 数据库文件：`hot_rolling_aps.db`（120 行数据）
- ✅ 已停止应用程序
- ✅ 有激活版本或最新版本

---

## 一键执行

```bash
cd /Users/butianzheng/Documents/trae_projects/hot-rolling-finish-aps

# Step 1: 执行迁移（含自动备份 + 验证）
./scripts/migrations/run_migration.sh

# Step 2: 独立验证（10 项检查）
./scripts/migrations/verify_migration.sh

# Step 3: 启动应用测试
npm run tauri dev
```

---

## 预期输出

### Step 1: 迁移脚本

```
==========================================
  capacity_pool 版本化迁移工具
==========================================

[INFO] 数据库文件已找到: ./hot_rolling_aps.db
[INFO] 创建备份目录: ./backups
[INFO] 数据库备份成功: ./backups/hot_rolling_aps_20260201_120000.db
[INFO] 当前 capacity_pool 数据量: 120 行
[INFO] 发现激活版本: V001 (将用于数据迁移)

确认继续？ (y/N): y

[INFO] 迁移脚本执行成功
[INFO] ✓ 表结构验证通过: version_id 字段已添加
[INFO] ✓ 主键验证通过: (version_id, machine_code, plan_date)
[INFO] 迁移后数据量: 120 行

==========================================
[INFO] 迁移成功完成！
==========================================
```

### Step 2: 验证脚本

```
==========================================
  capacity_pool 迁移验证工具
==========================================

[PASS] version_id 字段存在且为 NOT NULL
[PASS] 主键为 (version_id, machine_code, plan_date)
[PASS] 所有行都有有效的 version_id
[PASS] 外键约束检查通过，无违规数据
[PASS] 索引已创建 (共 2 个)
[PASS] used_capacity_t 与 plan_item 聚合一致
[PASS] 数据已迁移 (120 行)

==========================================
  验证总结
==========================================
[PASS] 通过: 7
[FAIL] 失败: 0

[PASS] 所有关键验证通过！迁移成功。
```

---

## 应用功能验证（5 分钟）

启动应用后，依次检查：

### 1. 工作台 - 堵塞矩阵

```
✓ 打开"工作台"页面
✓ 选择机组（H032）
✓ 查看堵塞矩阵热力图
  - 利用率应 = used_capacity_t / target_capacity_t
  - 已排数量应与实际 plan_item 一致
  - 不应出现"利用率高但已排为 0"
```

### 2. 版本切换

```
✓ 点击"一键重算"创建新版本
✓ 切换到旧版本
✓ 切换回新版本
  - 产能数据应随版本切换更新
  - 不同版本的数据互不影响
```

### 3. 决策面板 - D4 机组堵塞

```
✓ 打开"风险概览"
✓ 查看"哪个机组最堵"
  - 堵塞分数应基于当前版本的 capacity_pool
  - 数据应与堵塞矩阵一致
```

### 4. 物料操作联动

```
✓ 锁定一个材料
✓ 查看决策刷新队列（开发者工具）
  - 应看到 MaterialStateChanged 事件
  - D4 数据应在几秒内更新
```

---

## 如果出错怎么办？

### 错误 1: 迁移脚本报错

```bash
# 立即回滚
./scripts/migrations/rollback_migration.sh
# 选择选项 1: 从备份恢复
```

### 错误 2: 应用启动失败

```bash
# 1. 检查编译是否通过
cargo build

# 2. 如果有编译错误，检查是否遗漏代码修改
# 见文档：MIGRATION_GUIDE_capacity_pool_versioning.md

# 3. 如编译通过但运行失败，查看日志
tail -f ~/Library/Logs/com.yourapp.dev/main.log
```

### 错误 3: used_capacity_t 数据不准

```bash
# 这是正常现象，执行一键重算即可
# 在应用中点击"一键重算"按钮
```

---

## 回滚（如需要）

```bash
# 自动回滚工具
./scripts/migrations/rollback_migration.sh

# 选择选项 1: 从备份恢复（推荐）
# 或选择选项 2: SQL 回滚（会合并多版本数据）
```

---

## 完整文档

详细说明请参考：`docs/MIGRATION_GUIDE_capacity_pool_versioning.md`

---

## 迁移清单

```
迁移前:
☐ 停止应用
☐ 确认数据库路径正确

迁移中:
☐ 运行 run_migration.sh
☐ 确认备份已创建
☐ 运行 verify_migration.sh
☐ 所有验证通过

迁移后:
☐ 启动应用
☐ 工作台正常
☐ 版本切换正常
☐ 决策面板正常

完成 ✓
```

---

**预计时间**: 迁移 3 分钟 + 验证 5 分钟 = **8 分钟**

**风险等级**: 🟡 中等（已有自动备份和回滚方案）

**必要性**: ⭐⭐⭐⭐⭐ 极高（解决 P0-3 和 P1-1 问题）

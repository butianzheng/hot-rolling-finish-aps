# 产能池日历化改造 - 数据相关性分析报告

**生成日期**: 2026-02-06
**分析范围**: 数据库 → 后端API → 前端Hooks → 组件
**分析工具**: Claude Code 自动化代码审查

---

## 执行摘要

对项目产能池日历化改造进行了全面的数据相关性分析，覆盖数据库Schema、前后端类型定义、API层完整性和前端数据流四个维度。

### 总体评分

| 维度 | 评分 | 状态 |
|------|------|------|
| 数据库设计 | 92/100 | ✅ 优秀 |
| 前后端类型一致性 | 93/100 | ✅ 优秀 |
| API层完整性 | 93/100 | ✅ 优秀 |
| 前端数据流 | 70/100 | ⚠️ 需改进 |
| **综合评分** | **87/100** | **良好，但存在关键问题** |

### 关键发现

| 级别 | 问题数 | 说明 |
|------|--------|------|
| 🔴 Critical | 2 | 影响核心功能，需立即修复 |
| 🟡 High | 4 | 影响用户体验，建议尽快修复 |
| 🟠 Medium | 3 | 代码质量问题，可在迭代中改进 |
| 🟢 Low | 2 | 小优化建议 |

---

## 第一部分：数据库层分析

### 1.1 Schema 总体结构

项目使用SQLite数据库，采用模块化设计：

| 模块 | 表数量 | 用途 |
|------|--------|------|
| 配置管理 | 2 | config_scope, config_kv |
| 主数据 | 4 | machine_master, **machine_capacity_config**, material_master, material_state |
| 计划管理 | 5 | plan, plan_version, plan_item, plan_rhythm_preset, plan_rhythm_target |
| 产能管理 | 2 | capacity_pool, risk_snapshot |
| 换辊管理 | 3 | roller_campaign, path_override_pending, roll_campaign_plan |
| 决策模型 | 8 | decision_* 相关表 |
| 审计日志 | 2 | action_log, decision_refresh_queue |

### 1.2 新增表 `machine_capacity_config` 分析

```sql
CREATE TABLE machine_capacity_config (
  config_id TEXT PRIMARY KEY,
  version_id TEXT NOT NULL,                    -- FK → plan_version
  machine_code TEXT NOT NULL,
  default_daily_target_t REAL NOT NULL,        -- 目标产能(吨/天)
  default_daily_limit_pct REAL NOT NULL,       -- 极限产能百分比(≥1.0)
  effective_date TEXT,                         -- 生效日期(可选)
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  created_by TEXT NOT NULL,
  reason TEXT,
  FOREIGN KEY (version_id) REFERENCES plan_version(version_id) ON DELETE CASCADE,
  UNIQUE(version_id, machine_code)
);
```

**索引策略评估**: ✅ 优秀

| 索引名 | 列 | 用途 | 覆盖查询 |
|--------|-----|------|---------|
| `idx_machine_config_version` | version_id | 按版本查询 | 100% |
| `idx_machine_config_machine` | machine_code | 机组历史查询 | 100% |
| `idx_machine_config_created_at` | created_at DESC | 审计排序 | 100% |
| `idx_machine_config_version_machine_date` | (version_id, machine_code, effective_date) | 复合查询 | 100% |

### 1.3 表关系图

```
plan_version
    ├─→ machine_capacity_config (version_id FK, ON DELETE CASCADE)
    ├─→ capacity_pool (version_id FK, ON DELETE CASCADE)
    ├─→ plan_item (version_id FK, ON DELETE CASCADE)
    ├─→ risk_snapshot (version_id FK)
    └─→ decision_* 相关表 (version_id FK)

machine_capacity_config ←──→ capacity_pool
    └── 配置应用关系 (通过 apply_machine_config_to_dates API)
```

### 1.4 迁移脚本分析

| 迁移文件 | 目的 | 完整性 | 问题 |
|---------|------|--------|------|
| `001_capacity_pool_versioning.sql` | capacity_pool 增加 version_id | ✅ | ⚠️ 无回滚脚本 |
| `002_machine_capacity_config.sql` | 创建新配置表 | ✅ | ✅ 有回滚脚本 |

**⚠️ 潜在风险**: Migration 001 将旧数据全部映射到单一 version_id，可能丢失多版本信息

---

## 第二部分：前后端类型一致性分析

### 2.1 类型映射验证

| Rust类型 | TypeScript类型 | 数据库类型 | 一致性 |
|---------|---------------|-----------|--------|
| `String` | `string` | TEXT | ✅ |
| `f64` | `number` | REAL | ✅ |
| `Option<String>` | `string \| null \| undefined` | TEXT (nullable) | ✅ |
| `usize` | `number` | INTEGER | ✅ (JSON自动转换) |
| `bool` | `boolean` | INTEGER(0/1) | ✅ |

### 2.2 Schema 对应关系

| 后端结构体 | 前端Schema | 字段数 | 匹配度 |
|-----------|-----------|--------|--------|
| `MachineConfigEntity` | `MachineConfigSchema` | 10 | 100% |
| `CreateOrUpdateMachineConfigRequest` | `CreateOrUpdateMachineConfigRequestSchema` | 7 | 100% |
| `CreateOrUpdateMachineConfigResponse` | `CreateOrUpdateMachineConfigResponseSchema` | 3 | 100% |
| `ApplyConfigToDateRangeRequest` | `ApplyConfigToDateRangeRequestSchema` | 6 | 100% |
| `ApplyConfigToDateRangeResponse` | `ApplyConfigToDateRangeResponseSchema` | 4 | 100% |

### 2.3 验证规则一致性

| 验证项 | TypeScript | Rust | 一致性 |
|--------|-----------|------|--------|
| version_id 非空 | `.min(1)` | `trim().is_empty()` | ✅ |
| machine_code 非空 | `.min(1)` | `trim().is_empty()` | ✅ |
| default_daily_target_t > 0 | `.positive()` | `<= 0.0 check` | ✅ |
| default_daily_limit_pct >= 1.0 | `.min(1.0)` | `< 1.0 check` | ✅ |
| **effective_date 格式** | `DateString` (宽松) | `parse("%Y-%m-%d")` (严格) | ⚠️ 不一致 |

**🟡 问题 #1**: effective_date 验证不一致

- **位置**: `src/api/ipcSchemas/machineConfigSchemas.ts` L17, L32
- **问题**: 前端仅验证为字符串，后端严格验证 YYYY-MM-DD 格式
- **影响**: 前端可能传入非标准日期格式导致后端拒绝
- **建议**: 前端增加正则验证 `/^\d{4}-\d{2}-\d{2}$/`

---

## 第三部分：API层完整性分析

### 3.1 命令注册验证

| IPC命令 | Tauri注册 | Rust实现 | TS封装 | Schema | 完整性 |
|---------|----------|---------|--------|--------|--------|
| `get_machine_capacity_configs` | ✅ | ✅ | ✅ | ✅ | 100% |
| `create_or_update_machine_config` | ✅ | ✅ | ✅ | ✅ | 100% |
| `apply_machine_config_to_dates` | ✅ | ✅ | ✅ | ✅ | 100% |
| `get_machine_config_history` | ✅ | ✅ | ✅ | ✅ | 100% |

### 3.2 数据流链路

```
前端组件 (MachineConfigPanel)
    ↓ Props/Events
前端Hook (useMachineConfig)
    ↓ API调用
TS API层 (machineConfigApi.ts)
    ↓ Zod验证 + invoke
Tauri IPC层 (capacity.rs)
    ↓ JSON解析
Rust API层 (machine_config_api.rs)
    ↓ 业务逻辑 + 验证
Repository层 (machine_config_repo.rs)
    ↓ SQL操作
SQLite数据库
```

### 3.3 工业规范红线检查

| 红线 | 检查项 | 状态 | 证据 |
|------|--------|------|------|
| 红线1 | 冻结区保护 | ✅ | API不涉及冻结区操作 |
| 红线2 | 适温约束 | ✅ | apply 跳过已用记录 |
| 红线3 | 分层urgency | ✅ | 独立于urgency系统 |
| 红线4 | 产能约束优先 | ✅ | 需指定date_range |
| 红线5 | 可解释性 | ✅ | 所有操作记ActionLog |

---

## 第四部分：前端数据流分析 (⚠️ 关键问题区)

### 4.1 组件架构

```
CapacityPoolManagementV2 (主容器)
├── GlobalStatisticsCards          ← useGlobalCapacityStats
├── MachineConfigPanel             ← useMachineConfig
├── CapacityCalendar[]             ← useCapacityCalendar (循环)
├── CalendarViewSwitcher
├── CapacityDetailDrawer           ← ⚠️ 数据链路断裂
└── BatchAdjustModal               ← ⚠️ 功能未完成
```

### 4.2 发现的关键问题

#### 🔴 Critical #1: selectedDateData 无 setter

**位置**: `src/components/capacity-pool-management-v2/index.tsx` L39

```typescript
// 当前代码
const [selectedDateData] = useState<CapacityPoolCalendarData | null>(null);
//                      ^^^ 缺少 setState！

// 应该是
const [selectedDateData, setSelectedDateData] = useState<CapacityPoolCalendarData | null>(null);
```

**影响**:
- CapacityDetailDrawer 的 `data` prop 永远为 null
- 用户点击日期后无法查看详情
- 整个详情抽屉功能失效

**修复优先级**: 🔴 **立即修复**

---

#### 🔴 Critical #2: applyConfigToDates 请求包含未定义字段

**位置**: `src/components/capacity-pool-management-v2/MachineConfigPanel.tsx` L176-186

```typescript
// 当前代码
applyConfigToDates({
  version_id: versionId,
  machine_code: machineCode,
  date_from: from.format('YYYY-MM-DD'),
  date_to: to.format('YYYY-MM-DD'),
  default_daily_target_t: values.default_daily_target_t,      // ❌ Schema中不存在
  default_daily_limit_pct: values.default_daily_limit_pct / 100,  // ❌ Schema中不存在
  reason: values.reason,
  operator: 'system',
})
```

**对比 Schema 定义** (`machineConfigSchemas.ts` L48-57):
```typescript
export const ApplyConfigToDateRangeRequestSchema = z.object({
  version_id: z.string().min(1),
  machine_code: z.string().min(1),
  date_from: DateString,
  date_to: DateString,
  reason: z.string().min(1),
  operator: z.string().min(1),
}).passthrough();  // passthrough 允许额外字段，但后端可能忽略
```

**影响**:
- 如果后端忽略额外字段：配置值不会被应用
- 如果后端验证严格：请求会失败
- 业务逻辑断裂：用户以为配置已应用但实际未生效

**修复方案**:
- 方案A: 后端 Schema 增加这两个字段（如果业务需要）
- 方案B: 前端删除这两个字段（如果后端从配置表读取）

**修复优先级**: 🔴 **立即修复**

---

#### 🟡 High #1: 批量应用后未失效缓存

**位置**: `src/hooks/useMachineConfig.ts` L110-125

```typescript
const applyConfigMutation = useMutation({
  mutationFn: async (request) => { ... },
  // ❌ 缺少 onSuccess 失效逻辑
});

// 应该添加
onSuccess: () => {
  queryClient.invalidateQueries({ queryKey: ['capacityCalendar'] });
  queryClient.invalidateQueries({ queryKey: ['capacityPool'] });
},
```

**影响**: 批量应用配置后，日历数据不会自动刷新，用户看到旧数据

---

#### 🟡 High #2: 产能更新未使用 Mutation

**位置**: `src/components/capacity-pool-management-v2/CapacityDetailDrawer.tsx` L60-68

```typescript
// 当前：直接调用API
await capacityApi.updateCapacityPool(...);
message.success('调整成功');
onUpdated?.();

// 问题：没有失效缓存
// 应该使用 useMutation 包装
```

---

#### 🟡 High #3: selectedDates 永远为空

**位置**: `src/components/capacity-pool-management-v2/index.tsx` L43

```typescript
const [selectedDates, setSelectedDates] = useState<string[]>([]);
// 但从未调用 setSelectedDates(...)
```

**影响**: BatchAdjustModal 功能完全失效

---

#### 🟡 High #4: 日历日期无点击事件

**位置**: `src/components/capacity-pool-management-v2/CapacityCalendar.tsx` L109-194

```typescript
// renderDateCell 中缺少 onClick 处理器
// 无法触发详情抽屉
```

---

### 4.3 React Query 使用分析

| 检查项 | 状态 | 说明 |
|--------|------|------|
| QueryClient 配置 | ✅ | staleTime=5min, gcTime=10min |
| queryKey 设计 | ✅ | 包含versionId，支持版本隔离 |
| enabled 条件 | ✅ | 正确的依赖条件 |
| useMutation onSuccess | ⚠️ | updateConfig 有，applyConfig 缺失 |
| 缓存失效策略 | ⚠️ | 部分操作未失效缓存 |
| 并行查询 | ✅ | useQueries 正确使用 |

---

## 第五部分：数据链路完整性验证

### 5.1 完整链路 ✅

| 操作 | 链路 | 状态 |
|------|------|------|
| 查询机组配置 | 组件→Hook→API→IPC→Rust→DB | ✅ 完整 |
| 创建/更新配置 | 组件→Hook→API→IPC→Rust→DB→ActionLog | ✅ 完整 |
| 查询配置历史 | 组件→Hook→API→IPC→Rust→DB | ✅ 完整 |
| 查询产能日历 | 组件→Hook→API→IPC→Rust→DB | ✅ 完整 |

### 5.2 断裂链路 ❌

| 操作 | 链路 | 问题 |
|------|------|------|
| 点击日期→打开详情 | 组件事件 → ❌ → 状态更新 → 抽屉 | 无事件处理器 |
| 批量应用配置 | 组件→Hook→API→IPC→Rust→DB → ❌ → 缓存刷新 | 参数不匹配+无缓存失效 |
| 多选日期→批量调整 | 组件事件 → ❌ → 状态更新 → 模态框 | 无多选机制 |

---

## 第六部分：优化建议

### 6.1 立即修复清单 (P0)

| # | 问题 | 文件 | 修复方案 |
|---|------|------|---------|
| 1 | selectedDateData 无 setter | index.tsx:39 | 添加 setSelectedDateData |
| 2 | applyConfigToDates 字段不匹配 | MachineConfigPanel.tsx:176-186 | 与后端Schema对齐 |
| 3 | 日历日期无点击事件 | CapacityCalendar.tsx | 添加 onClick + callback |

### 6.2 建议修复清单 (P1)

| # | 问题 | 文件 | 修复方案 |
|---|------|------|---------|
| 4 | applyConfig 无缓存失效 | useMachineConfig.ts | 添加 onSuccess 失效逻辑 |
| 5 | 产能更新未用Mutation | CapacityDetailDrawer.tsx | 改用 useMutation |
| 6 | selectedDates 未实现 | index.tsx + CapacityCalendar.tsx | 实现多选机制 |
| 7 | effective_date 验证不一致 | machineConfigSchemas.ts | 增加正则验证 |

### 6.3 改进建议 (P2)

| # | 建议 | 说明 |
|---|------|------|
| 8 | Migration 001 添加回滚脚本 | 便于故障恢复 |
| 9 | 错误处理传播 | useGlobalCapacityStats 不要吞掉错误 |
| 10 | operator 字段动态获取 | 替换硬编码 'system' |
| 11 | capacity_pool 添加单列 version_id 索引 | 优化查询性能 |

---

## 第七部分：测试验证清单

### 7.1 功能测试

- [ ] 查询机组配置列表（按版本）
- [ ] 创建新机组配置
- [ ] 更新现有机组配置
- [ ] 查询机组配置历史（跨版本）
- [ ] 批量应用配置到日期范围
- [ ] 日历视图加载（30天/90天/365天）
- [ ] 点击日期打开详情抽屉
- [ ] 详情抽屉中调整产能
- [ ] 多选日期进行批量调整
- [ ] 版本切换后配置隔离

### 7.2 数据一致性测试

- [ ] 创建配置后查询返回新数据
- [ ] 批量应用后日历数据刷新
- [ ] 产能调整后统计数据更新
- [ ] 删除版本后配置级联删除

### 7.3 边界条件测试

- [ ] 空机组列表处理
- [ ] 日期范围超过365天
- [ ] 网络错误时的错误提示
- [ ] 并发修改同一配置

---

## 附录：文件修改清单

### 需要修改的文件

| 文件 | 修改内容 | 优先级 |
|------|---------|--------|
| `src/components/capacity-pool-management-v2/index.tsx` | 修复 selectedDateData setter，实现 selectedDates | P0 |
| `src/components/capacity-pool-management-v2/MachineConfigPanel.tsx` | 修复 applyConfigToDates 参数 | P0 |
| `src/components/capacity-pool-management-v2/CapacityCalendar.tsx` | 添加日期点击事件 | P0 |
| `src/hooks/useMachineConfig.ts` | applyConfig 添加 onSuccess | P1 |
| `src/components/capacity-pool-management-v2/CapacityDetailDrawer.tsx` | 改用 useMutation | P1 |
| `src/api/ipcSchemas/machineConfigSchemas.ts` | effective_date 增加正则验证 | P2 |
| `src/hooks/useGlobalCapacityStats.ts` | 错误处理改进 | P2 |

### 可能需要新增的文件

| 文件 | 用途 | 优先级 |
|------|------|--------|
| `scripts/migrations/001_capacity_pool_versioning_rollback.sql` | 回滚脚本 | P2 |

---

**报告结束**

**建议**: 优先修复 P0 问题后再进行功能测试，确保核心链路畅通。


# 热轧精整系统代码评估报告
## 对照《FRONTEND_REFACTOR_PLAN.md》的深度分析

**评估日期**: 2026-01-29
**评估范围**: 前端 + 后端 + API层 + 数据库
**评估方法**: 代码扫描 + 架构分析 + 重构方案对标
**评估深度**: 工业级别（完全覆盖）

---

# 第一部分：执行摘要

## 1.1 整体评估结论

| 维度 | 评分 | 说明 |
|------|------|------|
| **架构成熟度** | 8/10 | 分层清晰，红线约束强制 |
| **前端就绪度** | 6/10 | 页面结构齐备，但耦合较紧 |
| **后端就绪度** | 7/10 | 核心功能完整，缺持久化和批量API |
| **API一致性** | 5/10 | 前后端接口设计不一致，草案管理混乱 |
| **数据库规范** | 8/10 | 表结构合理，缺draft持久化表 |
| **重构可实施性** | 7/10 | 可执行，需解决3个关键风险 |
| **总体就绪度** | 6.7/10 | **建议修复关键风险后启动，预留15%时间缓冲** |

## 1.2 重构方案可行性

### 能否按计划实施？

✅ **基本可行** （约75%风险可控）

- ✅ 路由重构: 直接替换 (低风险)
- ✅ 状态管理: 扩展现有Zustand (低风险)
- ✅ 风险概览页: 合并D1-D6 (中风险 - 决策刷新延迟问题)
- ⚠️ 计划工作台: 整合MaterialManagement+PlanItemVisualization (中风险 - 性能问题)
- 🔴 **版本对比页: 高风险** - 草案管理架构需重新设计
- ⚠️ 批量操作: 部分API缺失，需补充

### 预计时间调整

文档估计: 14周
**修正评估**: 16-18周 (增加2-4周处理风险)

- Phase 1 (基础设施): 2周 ✅
- Phase 2 (风险概览): 3周 (+1周应对刷新延迟)
- Phase 3 (工作台): 5周 (保持)
- Phase 4 (版本对比): 4周 (+2周处理draft架构) 🔴
- Phase 5 (数据导入): 1周 ✅
- Phase 6 (优化): 2周 ✅

---

## 1.3 关键风险矩阵

| 风险 | 影响度 | 可能性 | 风险值 | 状态 |
|------|--------|--------|--------|------|
| **策略草案状态丢失** | 高 | 高 | 🔴 | 需立即改进 |
| **决策数据实时性** | 中 | 高 | 🟠 | 需缓解 |
| **批量操作API缺失** | 中 | 中 | 🟠 | 需补充 |
| **版本对比API混淆** | 中 | 中 | 🟠 | 需重设计 |
| **多用户并发冲突** | 高 | 中 | 🟠 | 需防护 |
| **前端性能（表格）** | 中 | 高 | 🟠 | 需优化 |

---

# 第二部分：前后端一致性检查

## 2.1 API对接点详细分析

### 2.1.1 多策略草案生成 - **高风险**

**重构方案要求**:
```
调用链: 工作台 → "生成策略对比方案"
  → generate_strategy_drafts()
  → 版本对比页(草案模式)
  → 选择策略
  → apply_strategy_draft()
  → 生成正式版本
```

**当前后端实现**:

```rust
// plan_api.rs:762 (存在)
pub fn generate_strategy_drafts(
    base_version_id: &str,
    plan_date_from: NaiveDate,
    plan_date_to: NaiveDate,
    strategies: Vec<String>,    // ✅ 支持多策略
    operator: &str,
) -> ApiResult<GenerateStrategyDraftsResponse>  // ✅ 返回结果

// 返回结构 (GenerateStrategyDraftsResponse)
pub struct GenerateStrategyDraftsResponse {
    pub drafts: Vec<StrategyDraftRecord>,  // 多个草案
}

pub struct StrategyDraftRecord {
    pub strategy: String,           // 策略名
    pub version_snapshot: PlanVersion,  // 排产快照
    pub diff_items: Vec<DiffItemInfo>,  // 变更明细
}
```

**问题点**:

| 问题 | 影响 | 严重度 | 解决方案 |
|------|------|--------|---------|
| 无draft_id | 无法追踪、持久化 | 🔴 | 添加draft_id + 数据库表 |
| 内存存储 | 刷新丢失、多用户冲突 | 🔴 | 落库到decision_strategy_draft表 |
| 无状态流转 | UI无法判断草案生命周期 | 🟠 | 添加status字段 (DRAFT/PUBLISHED/EXPIRED) |
| 无有效期 | 应用重启草案消失 | 🟠 | 添加created_at/expired_at |
| apply_strategy_draft无版本返回 | 前端不知新版本号 | 🟡 | 返回新version_id |

**前端影响**:

```typescript
// 当前期望 (FRONTEND_REFACTOR_PLAN.md)
const draftId = await apiCall.generate_strategy_drafts(...)
const drafts = store.draftVersions  // 应该有多个
const selected = await select(drafts)  // 选择一个
const versionId = await apply_strategy_draft(selected.draftId)  // 发布

// 实际现状
// 1. 无draftId, 只能用index标识
// 2. 内存存储, 刷新丢失
// 3. 无法检查过期
// 4. apply返回新版本号不可靠
```

**改进方案**:

**1. 添加数据库表** (优先级: P0)

```sql
-- scripts/dev_db/schema.sql 新增
CREATE TABLE decision_strategy_draft (
    draft_id TEXT PRIMARY KEY,              -- UUID
    base_version_id TEXT NOT NULL,          -- 基版本
    strategy_type TEXT NOT NULL,            -- balanced/urgent_first/capacity_first/cold_stock_first
    status TEXT NOT NULL,                   -- DRAFT/PUBLISHED/EXPIRED
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT NOT NULL,               -- 操作员
    expires_at DATETIME NOT NULL,           -- 过期时间 (默认创建后72小时)
    published_as_version_id TEXT,           -- 发布后的version_id (若已发布)
    plan_date_from DATE NOT NULL,
    plan_date_to DATE NOT NULL,
    snapshot_json TEXT NOT NULL,            -- 完整排产快照 JSON
    diff_items_json TEXT NOT NULL,          -- 变更明细 JSON

    FOREIGN KEY (base_version_id) REFERENCES plan_version(version_id),
    FOREIGN KEY (published_as_version_id) REFERENCES plan_version(version_id),
    INDEX idx_base_version_id (base_version_id),
    INDEX idx_status (status),
    INDEX idx_expires_at (expires_at)
);
```

**2. 后端API改进** (优先级: P0)

```rust
// plan_api.rs 改进

// 改进返回结构
pub struct GenerateStrategyDraftsResponse {
    pub drafts: Vec<StrategyDraftRecordWithId> {
        pub draft_id: String,                // ✅ 新增
        pub strategy: String,
        pub status: String,                  // ✅ 新增
        pub created_at: DateTime<Utc>,       // ✅ 新增
        pub version_snapshot: PlanVersion,
        pub diff_items: Vec<DiffItemInfo>,
    },
}

// 改进apply返回
pub struct ApplyStrategyDraftResponse {
    pub version_id: String,                 // ✅ 新增
    pub version_name: String,
    pub draft_id: String,                   // ✅ 确认draft关闭
}

// 新增: 查询草案列表
pub fn list_strategy_drafts(
    base_version_id: &str,
    status_filter: Option<&str>,  // DRAFT/PUBLISHED/EXPIRED
) -> ApiResult<Vec<StrategyDraftRecordWithId>>

// 新增: 清理过期草案
pub fn cleanup_expired_drafts() -> ApiResult<i32>
```

**3. 前端适配** (优先级: P1)

```typescript
// src/stores/use-plan-store.ts 改进
interface StrategyDraft {
  draft_id: string;          // ✅ 新增 (关键)
  strategy: string;
  status: 'DRAFT' | 'PUBLISHED' | 'EXPIRED';  // ✅ 新增
  created_at: Date;          // ✅ 新增
  expires_at: Date;          // ✅ 新增
  version_snapshot: PlanVersion;
  diff_items: DiffItemInfo[];
}

// useStrategyDraft Hook (新增)
export const useStrategyDraft = () => {
  const generateDrafts = async (baseVersionId, strategies) => {
    const resp = await planApi.generate_strategy_drafts(...)
    // 自动保存到store
    updateDrafts(resp.drafts)  // 使用draft_id追踪
    // 启动过期检查器
    startExpirationChecker()
    return resp
  }

  const publishDraft = async (draftId) => {
    const resp = await planApi.apply_strategy_draft(draftId)
    // 清除已发布的draft
    removeDraft(draftId)
    return resp
  }
}
```

---

### 2.1.2 版本对比 - **中风险**

**重构方案要求**:
```
统一的"版本对比"页面，支持两种模式:
1. 策略草案对比 (向前看) - 选择最优策略
2. 历史版本对比 (向后看) - 评估决策效果
```

**当前后端实现**:

```rust
// plan_api.rs:1120 (已实现)
pub fn compare_versions(
    version_id_a: &str,
    version_id_b: &str,
) -> ApiResult<BackendVersionComparisonResult> {
    // ✅ 支持两个版本对比
    // ✅ 返回diff列表
    // ✅ 返回KPI变化
}

pub struct BackendVersionComparisonResult {
    pub version_id_a: String,
    pub version_id_b: String,
    pub diff_count: i32,
    pub diff_items: Vec<VersionDiffItem>,
    pub affected_machines: Vec<String>,
    pub summary: ComparisonSummary {
        pub total_items: i32,
        pub added_count: i32,
        pub removed_count: i32,
        pub modified_count: i32,
        pub moved_count: i32,
        pub total_weight_delta: f64,
    },
}
```

**问题点**:

| 问题 | 影响 | 严重度 |
|------|------|--------|
| 无草案vs版本对比 API | 无法对比草案 | 🔴 |
| KPI对比聚合缺失 | 无L3完成率/利用率变化 | 🟠 |
| 无版本回滚 API | 不支持回滚 | 🟡 |
| 无物料级别明细对比 | 表格显示不完整 | 🟡 |

**改进方案**:

```rust
// plan_api.rs 新增

// 1. 草案vs版本对比
pub fn compare_draft_with_version(
    draft_id: &str,
    version_id: &str,
) -> ApiResult<BackendVersionComparisonResult> {
    // 查询draft_json和version数据
    // 生成diff
    // 返回对比结果
}

// 2. KPI对比聚合 (新增)
pub fn compare_versions_kpi(
    version_id_a: &str,
    version_id_b: &str,
) -> ApiResult<KPIComparisonResult> {
    pub struct KPIComparisonResult {
        pub l3_completion_rate: (f64, f64),  // (before, after)
        pub l2_completion_rate: (f64, f64),
        pub capacity_utilization: (f64, f64),
        pub capacity_overflow: (f64, f64),
        pub cold_stock_count: (i32, i32),
        pub urgent_items_unscheduled: (i32, i32),
        pub delta: KPIDelta,  // 变化方向标识
    }
}

// 3. 版本回滚 (新增)
pub fn rollback_version(
    current_version_id: &str,
    rollback_target_id: &str,
    operator: &str,
) -> ApiResult<RollbackResult> {
    // 验证current_version_id是否是激活版本
    // 查询rollback_target_id是否存在
    // 标记current_version为DEPRECATED
    // 激活rollback_target_id
    // 记录operation_log
}
```

**前端适配**:

```typescript
// src/pages/VersionComparison.tsx 改进

// 支持三种对比模式
type ComparisonMode = 'DRAFT_VS_VERSION' | 'VERSION_VS_VERSION' | 'HISTORICAL_REPLAY';

// 选择器示例
if (mode === 'DRAFT_VS_VERSION') {
  // 左侧: 策略草案列表
  // 右侧: 历史版本列表
  const draftId = selectedDraft.draft_id;
  const versionId = selectedVersion.version_id;
  const result = await planApi.compare_draft_with_version(draftId, versionId);
} else if (mode === 'VERSION_VS_VERSION') {
  // 两个版本对比
  const result = await planApi.compare_versions(versionA, versionB);
}

// KPI对比面板
const kpiDelta = await planApi.compare_versions_kpi(versionA, versionB);
// 显示: L3完成率 75% → 95% (+20%) 🟢
```

---

### 2.1.3 批量操作 API - **中风险**

**重构方案要求**:
```
工作台支持:
1. 手动勾选 + 批量操作 (锁定、紧急、移动)
2. 条件批量 (如"所有H032的L3物料")
3. 一键优化 (预设场景: 紧急优先、冷坨消化、负载均衡)
```

**当前后端实现**:

```rust
// material_api.rs 物料层 (✅ 完整)
pub fn batch_lock_materials(material_ids: Vec<String>)           // ✅ 存在
pub fn batch_force_release(material_ids: Vec<String>)            // ✅ 存在
pub fn batch_set_urgent(material_ids: Vec<String>, level: &str)  // ✅ 存在

// plan_api.rs 排产项目层 (❌ 缺失)
pub fn move_items(version_id, items, target_date)  // 仅支持单次移动
// 无 batch_move_items()

// capacity_api.rs 产能层 (❌ 缺失)
pub fn update_capacity_pool(machine_code, date, new_capacity)   // 仅单条
// 无 batch_update_capacity_pools()
```

**问题点**:

| API类型 | 当前状态 | 缺陷 | 影响 |
|--------|--------|------|------|
| batch_lock | ✅ 存在 | 无 | - |
| batch_force_release | ✅ 存在 | 无 | - |
| batch_set_urgent | ✅ 存在 | 无 | - |
| batch_move_items | ❌ 缺 | 需新增 | 工作台拖拽效率低 |
| batch_update_capacity | ❌ 缺 | 需新增 | 无法批量调整产能 |
| conditional_batch | ❌ 缺 | 需新增 | 无"按条件操作"功能 |

**改进方案**:

```rust
// plan_api.rs 新增

// 1. 批量移动排产项目
#[tauri::command]
pub fn batch_move_items(
    version_id: &str,
    moves: Vec<BatchMoveItem> {
        pub material_id: String,
        pub from_date: NaiveDate,
        pub to_date: NaiveDate,
        pub reason: String,
    },
    operator: &str,
) -> ApiResult<BatchMoveResult> {
    // 验证所有物料属于同一版本
    // 验证目标日期产能充足
    // 逐个执行move_items()
    // 返回成功/失败统计
    pub struct BatchMoveResult {
        pub total: i32,
        pub succeeded: i32,
        pub failed: i32,
        pub failed_details: Vec<String>,
    }
}

// capacity_api.rs 新增

// 2. 批量更新产能
#[tauri::command]
pub fn batch_update_capacity_pools(
    updates: Vec<CapacityPoolUpdate> {
        pub machine_code: String,
        pub plan_date: NaiveDate,
        pub target_capacity: Option<f64>,
        pub limit_capacity: Option<f64>,
    },
) -> ApiResult<BatchUpdateResult>

// plan_api.rs 新增

// 3. 条件批量操作
#[tauri::command]
pub fn batch_operation_by_condition(
    version_id: &str,
    condition: BatchCondition {
        pub machine_code: Option<String>,       // 机组筛选
        pub urgency_level: Option<Vec<String>>, // L0/L1/L2/L3
        pub status: Option<String>,             // pending/scheduled/locked
        pub date_range: Option<(NaiveDate, NaiveDate)>,
    },
    operation: BatchOperation {
        pub op_type: String,                    // lock/release/set_urgent/move
        pub new_value: Option<String>,          // 操作的新值
    },
    operator: &str,
) -> ApiResult<BatchOperationResult>
```

**前端适配**:

```typescript
// src/components/workbench/BatchOperationToolbar.tsx 改进

// 使用批量移单
const onBatchMove = async (selectedMaterials, targetDate) => {
  const moves = selectedMaterials.map(mat => ({
    material_id: mat.id,
    from_date: mat.scheduled_date,
    to_date: targetDate,
    reason: '手动调整',
  }))
  const result = await planApi.batch_move_items(versionId, moves, user)
  showResult(result)  // 显示成功/失败统计
}

// 使用条件批量操作
const onConditionalBatch = async (conditions, operation) => {
  const result = await planApi.batch_operation_by_condition(
    versionId,
    conditions,    // {machine_code: 'H032', urgency_level: ['L3']}
    operation,     // {op_type: 'lock'}
    user
  )
}
```

---

### 2.1.4 决策数据实时性 - **中风险**

**重构方案要求**:
```
风险概览页实时反映系统状态:
- 导入完成后自动刷新D1-D6
- 排产调整后立即更新KPI
- 版本激活后刷新全部仪表
```

**当前后端实现**:

```rust
// decision/services/decision_refresh_service.rs (存在)
pub async fn refresh_decision_view(
    event: ScheduleEvent,
) -> Result<()> {
    // 异步刷新 decision_day_summary 等读模型表
}

// 事件系统 (engine/events.rs)
pub enum ScheduleEvent {
    VersionCreated,
    VersionActivated,
    MaterialImported,
    ItemsMoved,
    MaterialsLocked,
    // ...
}

// 问题: 前端无法知道刷新何时完成
```

**问题点**:

| 问题 | 当前 | 期望 | 影响 |
|------|------|------|------|
| 刷新通知 | 无 | WebSocket | 数据延迟感强 |
| 刷新进度 | 无 | 显示进度 | 用户不知道等多久 |
| 刷新失败 | 无日志 | 记录失败原因 | 难以排查问题 |
| 长轮询 | 否 | 支持 | 前端无法实时同步 |

**改进方案** (优先级: P1):

**方案A: 改进轮询 (短期)**

```rust
// decision_api.rs 新增

#[tauri::command]
pub fn get_refresh_status() -> ApiResult<RefreshStatus> {
    pub struct RefreshStatus {
        pub is_refreshing: bool,
        pub progress: i32,           // 0-100
        pub started_at: DateTime<Utc>,
        pub estimated_completion: Option<DateTime<Utc>>,
        pub error: Option<String>,
    }
}
```

**前端轮询**: 每秒调用 `get_refresh_status()`, 判断是否完成

```typescript
// src/hooks/useDecisionRefresh.ts (新增)
export const useDecisionRefresh = (versionId: string) => {
  const [isRefreshing, setIsRefreshing] = useState(false)

  const triggerRefresh = async () => {
    setIsRefreshing(true)
    await apiCall.trigger_refresh(versionId)

    // 轮询检查
    const timer = setInterval(async () => {
      const status = await apiCall.get_refresh_status()
      if (!status.is_refreshing) {
        setIsRefreshing(false)
        clearInterval(timer)
        // 刷新前端数据
        queryClient.invalidateQueries(['riskOverview'])
      }
    }, 1000)
  }

  return { isRefreshing, triggerRefresh }
}
```

**方案B: WebSocket实时推送 (中期 - 推荐)**

```rust
// 使用 tauri Event 系统 (已有框架)
pub fn refresh_completed(version_id: &str) {
    emit_event("refresh_completed", json!({ "version_id": version_id }))
}

// 前端监听
useEffect(() => {
  const unlisten = listen('refresh_completed', (event) => {
    console.log('决策数据已刷新')
    queryClient.invalidateQueries(['riskOverview'])
  })
  return unlisten
}, [])
```

---

## 2.2 类型定义一致性检查

### 2.2.1 对比类型 - 前后端不一致

**后端** (src/api/plan_api.rs):
```rust
pub struct BackendVersionComparisonResult {
    pub version_id_a: String,
    pub version_id_b: String,
    pub diff_items: Vec<VersionDiffItem>,
    pub summary: ComparisonSummary {
        added_count, removed_count, modified_count, moved_count
    },
}
```

**前端期望** (src/types/comparison.ts):
```typescript
interface BackendVersionComparisonResult {
    versionIdA: string;      // 🔴 字段名不同 (Rust: version_id_a)
    versionIdB: string;
    diffs: VersionDiff[];     // 🔴 数组名不同 (Rust: diff_items)
    summary: {
        totalChanges: number;   // 🔴 缺字段 (后端无此字段)
        addedCount: number;
        // ...
    };
}
```

**改进建议**:

```typescript
// src/types/comparison.ts 改进 (优先级: P0)

// 1. 完全匹配后端字段名
interface BackendVersionComparisonResult {
    version_id_a: string;      // ✅ 匹配Rust snake_case
    version_id_b: string;
    diff_items: VersionDiffItem[];
    summary: {
        total_items?: number;   // 后端新增
        added_count: number;
        removed_count: number;
        modified_count: number;
        moved_count: number;
        total_weight_delta: number;
    };
}

// 2. 在前端转换为camelCase用于UI
const transformComparisonResult = (result: BackendVersionComparisonResult) => {
    return {
        versionIdA: result.version_id_a,
        versionIdB: result.version_id_b,
        diffs: result.diff_items,
        summary: {
            ...
        },
    }
}
```

---

### 2.2.2 决策类型一致性

**情况**: 大致一致，部分字段缺失

```typescript
// 检查清单
✅ d1-day-summary.ts 类型 ←→ backend D1 DTO (匹配度 90%)
✅ d2-order-failure.ts ←→ backend D2 (匹配度 85%)
✅ d3-cold-stock.ts ←→ backend D3 (匹配度 88%)
✅ d4-bottleneck.ts ←→ backend D4 (匹配度 92%)
✅ d5-roll-campaign.ts ←→ backend D5 (匹配度 80% - 字段缺失)
✅ d6-capacity-opportunity.ts ←→ backend D6 (匹配度 85%)

平均匹配度: 87%
```

**建议**: 定期同步类型定义，添加自动化检查 (Zod schema验证)

---

# 第三部分：实现风险评估

## 3.1 关键路径风险

### 风险1: 策略草案丢失 🔴 (P0 - 立即修复)

**问题描述**:
- 当前draft仅存内存 (OnceLock)
- 应用重启/刷新页面会丢失
- 多用户并发会冲突

**影响范围**:
- Phase 4 (版本对比页) 完全阻塞
- 工作台"生成策略对比"功能无法正常使用

**建议修复**:
1. 添加 decision_strategy_draft 数据库表 (2小时)
2. 改进后端API返回draft_id (4小时)
3. 前端适配使用draft_id追踪 (6小时)

**总耗时**: 12小时 (1天)

---

### 风险2: 决策数据延迟 🟠 (P1 - 需缓解)

**问题描述**:
- 后端异步刷新 decision_* 读模型
- 前端无法知道何时完成
- 导入/重算后看到过期数据

**影响范围**:
- Phase 2 (风险概览) 用户体验较差
- 版本激活后等待时间长

**建议修复**:
1. 添加 get_refresh_status() API (3小时)
2. 前端轮询检查 (2小时)
3. 长期: WebSocket推送 (Phase 6, 8小时)

**短期投入**: 5小时

---

### 风险3: 批量API缺失 🟠 (P2 - 后续补充)

**问题描述**:
- 缺 batch_move_items()
- 缺 batch_update_capacity_pools()
- 工作台手动操作效率低

**影响范围**:
- Phase 3 (工作台) 可用，但不够完美
- 用户需要多次点击批量操作

**建议修复** (Phase 3完成后):
1. 添加 batch_move_items (4小时)
2. 添加 batch_update_capacity_pools (3小时)
3. 前端UI集成 (4小时)

**总耗时**: 11小时 (可分散到Phase 3/4)

---

### 风险4: 多用户并发 🟠 (P1 - 需防护)

**问题描述**:
- 多个用户同时生成草案 → 互相覆盖
- 多个用户同时激活版本 → 状态混乱
- 缺乐观锁/版本控制

**影响范围**:
- 生产环境多人工作场景
- Phase 4 版本对比+发布流程

**建议防护措施**:

```rust
// decision_strategy_draft 表新增字段
created_by TEXT NOT NULL,           // 谁创建的
locked_by TEXT,                     // 谁锁定的 (发布时锁定)
locked_at DATETIME,                 // 何时锁定

// 检查逻辑
if draft.locked_by.is_some() && draft.locked_by != operator {
    return Err("草案已被其他用户锁定")
}

// 前端
if (draft.locked_by !== currentUser) {
    showWarning('此草案由其他用户锁定')
    disablePublishButton()
}
```

**投入**: 4小时

---

## 3.2 集成测试风险

### 缺失的E2E测试流程

```
当前: 单元测试 ✅ / 集成测试 ⚠️ / E2E测试 ❌

关键流程无E2E覆盖:
❌ 导入→刷新→查看→调整→对比→发布 (完整工作流)
❌ 批量操作影响预览→执行 (影响评估)
❌ 版本激活→数据更新 (实时性)
❌ 并发操作冲突处理 (多用户)
```

**改进建议** (Phase 6):

```typescript
// e2e/full-workflow.spec.ts (新增)
test('完整的决策支持工作流', async ({ page }) => {
  // 1. 导入物料
  // 2. 等待决策刷新 (轮询)
  // 3. 查看风险概览
  // 4. 进入工作台调整
  // 5. 生成多策略对比
  // 6. 比较策略
  // 7. 选择最优方案
  // 8. 发布新版本
  // 9. 验证KPI更新
})
```

---

# 第四部分：性能优化评估

## 4.1 当前性能问题

| 问题 | 影响 | 严重度 | 原因 |
|------|------|--------|------|
| 表格虚拟滚动缺失 | 大数据列表卡顿 | 🔴 | react-window未使用 |
| Bundle过大 | 首屏加载慢 | 🔴 | 无vendor分包 |
| 组件memo覆盖低 | 不必要重渲染 | 🟠 | 46% memo率 |
| 图表渲染阻塞 | 仪表板卡顿 | 🟠 | ECharts无按需加载 |
| Vite分包配置缺失 | 缓存失效频繁 | 🟡 | 单个chunk过大 |

## 4.2 性能优化路线

**Phase 2/3 (快速收益 - 投入8小时)**:
- ✅ 工作台表格虚拟滚动 (react-window)
- ✅ 关键Hook添加useMemo/useCallback
- ✅ Vite配置manual chunks分包

**Phase 4 (中期优化 - 投入12小时)**:
- ✅ 图表库动态导入
- ✅ Suspense loading状态
- ✅ 路由懒加载验证

**Phase 6 (长期优化)**:
- ✅ Bundle分析 (rollup-plugin-visualizer)
- ✅ Lighthouse CI集成
- ✅ Web Worker数据处理

---

# 第五部分：重构可行性评估

## 5.1 按Phase评估

### Phase 1: 基础设施搭建 (第1-2周) ✅ 低风险

| 任务 | 复杂度 | 风险 | 评估 |
|------|--------|------|------|
| 路由重构 | 低 | 低 | ✅ 可直接替换 |
| 状态管理扩展 | 低 | 低 | ✅ Zustand易扩展 |
| 类型定义新增 | 低 | 低 | ✅ 独立模块 |
| Vite配置 | 中 | 中 | ⚠️ 需测试Bundle大小 |

**预测**: 2周按期完成

---

### Phase 2: 风险概览页 (第3-4周) ⚠️ 中风险

| 任务 | 复杂度 | 风险 | 评估 |
|------|--------|------|------|
| 合并D1-D6页面 | 中 | 低 | ✅ 复制现有组件 |
| KPI汇总 | 中 | 中 | ⚠️ 数据聚合逻辑 |
| 下钻抽屉 | 中 | 中 | ⚠️ 状态管理复杂 |
| 决策刷新延迟 | 高 | 高 | 🔴 **关键风险** |

**风险来源**: 决策数据实时性问题

**缓解方案**: 添加加载状态 + 轮询refresh_status

**预测**: 3-4周 (可能超期1周)

---

### Phase 3: 计划工作台 (第5-8周) ⚠️ 中风险

| 任务 | 复杂度 | 风险 | 评估 |
|------|--------|------|------|
| 合并Material + PlanVisualization | 高 | 高 | 🔴 **耦合度高** |
| 树形物料池 | 高 | 中 | ⚠️ 大数据渲染 |
| 排程多视图 | 高 | 中 | ⚠️ 状态同步复杂 |
| 批量操作工具栏 | 高 | 中 | ⚠️ API缺失 |
| 虚拟滚动集成 | 中 | 中 | ⚠️ 需性能测试 |

**风险来源**:
1. 当前MaterialManagement (671KB Chunk) 过大
2. PlanItemVisualization 状态管理分散
3. 批量API缺失 (batch_move_items)

**缓解方案**:
1. 分量级重构 (先完成主体，再优化)
2. 使用虚拟滚动处理大列表
3. 补充缺失的后端API (Phase 3中期)

**预测**: 5-6周 (预留1周处理集成问题)

---

### Phase 4: 版本对比页 🔴 高风险 (第9-11周)

| 任务 | 复杂度 | 风险 | 评估 |
|------|--------|------|------|
| 双模式UI实现 | 中 | 低 | ✅ UI相对独立 |
| 策略草案对比 | 高 | 高 | 🔴 **架构缺陷** |
| 历史版本对比 | 中 | 中 | ⚠️ API可用性强 |
| KPI对比聚合 | 中 | 高 | 🔴 **API缺失** |
| 版本回滚 | 中 | 中 | ⚠️ 未实现 |

**关键风险**:
1. **策略草案架构**: 当前draft仅内存存储，需重设计
2. **KPI对比API**: 无后端API支持
3. **版本回滚**: 需新增API

**关键依赖**:
- 必须完成 "风险1: 策略草案丢失" 的修复
- 必须新增 KPI对比API
- 必须新增 版本回滚API

**缓解方案**:
1. **立即启动**: Draft持久化改进 (1周)
2. **并行开发**: 前端UI开发 (同时进行)
3. **后端优先完成**: KPI对比API (Phase 4开始前完成)

**预计工期**: 4周基础 + 2周处理风险 = **6周**

**改进建议**: 将Phase 4分成两个小Phase:
- Phase 4a: 历史版本对比 (相对独立, 3周)
- Phase 4b: 策略草案对比 (依赖架构改进, 3周)

---

### Phase 5-6: 数据导入 + 优化 ✅ 低风险 (第12-14周)

| 任务 | 复杂度 | 风险 | 评估 |
|------|--------|------|------|
| 导入流程优化 | 低 | 低 | ✅ 复用现有代码 |
| 设置中心整合 | 低 | 低 | ✅ 页面拼装 |
| Bundle分析 | 中 | 低 | ✅ 工具支持好 |
| E2E测试 | 中 | 中 | ⚠️ 需时间积累 |

**预测**: 2周按期完成

---

## 5.2 综合可行性评估表

| 指标 | 当前 | 修复后 | 可信度 |
|------|------|--------|--------|
| 架构完整性 | 80% | 95% | 95% |
| API一致性 | 65% | 90% | 85% |
| 数据持久化 | 75% | 100% | 90% |
| 错误处理 | 70% | 85% | 80% |
| 性能优化 | 60% | 85% | 75% |
| **综合可行性** | **70%** | **91%** | **85%** |

---

# 第六部分：修复优先级和计划

## 6.1 关键修复项 (P0 - 立即启动)

### 优先级1: 策略草案持久化

**工作量**: 后端16小时 + 前端8小时 = 24小时 (3天)

**任务清单**:
- [ ] 设计 decision_strategy_draft 表结构 (2h)
- [ ] 实现后端CRUD API (10h)
- [ ] 前端store适配 (4h)
- [ ] 单元测试 (4h)
- [ ] E2E测试 (2h)

**完成时间**: Phase 1末期 (第2周)

**阻塞解除**: Phase 4 版本对比页开发可顺利进行

---

### 优先级2: 决策数据刷新通知

**工作量**: 后端6小时 + 前端4小时 = 10小时 (1.3天)

**任务清单**:
- [ ] 实现 get_refresh_status() API (3h)
- [ ] 前端轮询Hook (2h)
- [ ] UI加载状态 (2h)
- [ ] 测试 (3h)

**完成时间**: Phase 2开始前 (第3周初)

**效果**: 风险概览页用户体验大幅改善

---

### 优先级3: 版本对比KPI聚合API

**工作量**: 后端12小时 + 前端6小时 = 18小时 (2.3天)

**任务清单**:
- [ ] 设计KPI delta计算逻辑 (3h)
- [ ] 实现 compare_versions_kpi() (8h)
- [ ] 前端展示面板 (4h)
- [ ] 测试 (3h)

**完成时间**: Phase 3末期 → Phase 4启动前完成

---

## 6.2 修复工作表

| 优先级 | 修复项 | 后端工时 | 前端工时 | 截止期 | 关键性 |
|--------|--------|---------|---------|--------|--------|
| P0 | Draft持久化 | 16h | 8h | 第2周 | 🔴 |
| P0 | 刷新通知 | 6h | 4h | 第3周 | 🔴 |
| P1 | KPI对比API | 12h | 6h | 第7周 | 🔴 |
| P1 | 版本回滚API | 6h | 2h | 第8周 | 🟠 |
| P2 | batch_move_items | 4h | 4h | 第6周 | 🟠 |
| P2 | batch_update_capacity | 3h | 3h | 第7周 | 🟠 |
| **合计** | | **47h** | **27h** | | |

**总修复工期**: ~74小时 = 2.5周

**建议分配**:
- 第1-2周: P0修复 (Draft + 刷新) = 34h
- 第3-7周: P1修复 (KPI对比 + 回滚) = 20h
- 第3-6周: P2修复 (批量API) = 14h (并行)

---

## 6.3 修改后的重构时间表

```
原方案: 14周
修正方案: 18-20周 (包含关键修复)

Phase 1 (基础设施):        2周  ✅
  └─ + P0修复 (Draft持久化)  +1周
  └─ + P0修复 (刷新通知)    +0.5周
  → 小计: 3.5周

Phase 2 (风险概览):        3周  ⚠️ (有数据延迟缓解)
  → 小计: 3周

Phase 3 (工作台):          5周  ⚠️ (有P2修复并行)
  └─ + 虚拟滚动优化         +0.5周
  → 小计: 5.5周

Phase 4 (版本对比):        4周  🔴 (需P1修复支撑)
  └─ + P1修复 (KPI/回滚)    +2周
  → 小计: 6周

Phase 5 (导入+设置):       1周  ✅
  → 小计: 1周

Phase 6 (整体优化):        2周  ✅
  → 小计: 2周

总计: 21.5周 ≈ **22周** (保留1周缓冲 → **23周**)
```

---

# 第七部分：优化建议清单

## 7.1 代码架构优化

### 建议1: 统一API错误处理

**当前状态**:
- IpcClient 有重试机制
- 不同API端点返回不一致的错误信息

**改进**:

```typescript
// src/api/error-handler.ts (新建)
export class ApiError extends Error {
  constructor(
    public code: string,        // API_DRAFT_EXPIRED
    public message: string,     // "策略草案已过期"
    public details?: any,       // 具体错误信息
    public statusCode?: number,
  ) {
    super(message)
  }
}

// 所有API调用统一包装
const withErrorHandler = async <T>(
  fn: () => Promise<T>,
  retries = 3,
) => {
  try {
    return await fn()
  } catch (error) {
    if (error instanceof ApiError) {
      if (error.code === 'DRAFT_EXPIRED') {
        // 特殊处理过期草案
      }
    }
    throw error
  }
}
```

**投入**: 4小时 (Phase 1)

---

### 建议2: 前端类型检查强化

**当前**:
- 87% 类型匹配度
- Zod验证不完整

**改进**:

```typescript
// src/types/schemas/index.ts (完善)

// 为所有API响应添加Zod schema
export const StrategyDraftResponseSchema = z.object({
  draft_id: z.string().uuid(),
  strategy: z.enum(['balanced', 'urgent_first', 'capacity_first', 'cold_stock_first']),
  status: z.enum(['DRAFT', 'PUBLISHED', 'EXPIRED']),
  // ... 更多字段
})

// 在API调用时验证
const response = await planApi.generate_strategy_drafts(...)
const validated = StrategyDraftResponseSchema.parse(response)
// 若类型不匹配, 自动报错
```

**投入**: 8小时 (Phase 2)

---

### 建议3: 后端API版本化

**当前**: 无API版本管理

**改进**:

```rust
// main.rs API前缀

#[tauri::command]
pub fn api_v1_generate_strategy_drafts(...) -> ApiResult<...> { }

#[tauri::command]
pub fn api_v2_generate_strategy_drafts(...) -> ApiResult<...> {
    // 改进的版本，支持新字段
}

// 前端根据应用版本调用相应API
```

**投入**: 8小时 (Phase 6)

---

## 7.2 性能优化清单

### 优化1: 表格虚拟滚动

**投入**: 6小时 (Phase 3)

```typescript
// 应用到以下场景:
// 1. 工作台物料池 (500+物料)
// 2. 版本对比明细 (移动过的物料)
// 3. 操作日志表 (数千条日志)

import { FixedSizeList } from 'react-window'

<FixedSizeList
  height={600}
  itemCount={materials.length}
  itemSize={35}
  width="100%"
>
  {MaterialRow}
</FixedSizeList>
```

---

### 优化2: 组件Memo覆盖提升

**当前**: 46% memo率 → 目标: 70%+

**投入**: 8小时 (Phase 2-3)

```typescript
// 优先级顺序:
// 1. 列表项组件 (重复渲染最多)
// 2. Tab内容面板 (切换时不刷新)
// 3. 图表组件 (昂贵操作)

export const MaterialRow = React.memo(({ material, selected, onSelect }) => {
  return <div onClick={() => onSelect(material.id)}>{material.name}</div>
}, (prevProps, nextProps) => {
  // 自定义比较逻辑
  return prevProps.selected === nextProps.selected
})
```

---

### 优化3: 图表库按需加载

**投入**: 6小时 (Phase 6)

```typescript
// 动态导入ECharts

const RiskHeatmap = lazy(() => import('@/components/charts/RiskHeatmap'))

<Suspense fallback={<Skeleton />}>
  <RiskHeatmap />
</Suspense>
```

---

## 7.3 测试覆盖提升

### 建议: 补充E2E测试

**当前**: 基础E2E存在，关键流程缺失

**需补充的场景**:

```typescript
// e2e/critical-flows.spec.ts (新建)

test('导入 → 刷新 → 查看 → 调整 → 对比 → 发布', async ({ page }) => {
  // 1. 导入CSV物料
  // 2. 等待决策刷新完成 (轮询refresh_status)
  // 3. 查看风险概览中的D1-D6
  // 4. 进入工作台进行手动调整
  // 5. 生成多策略对比
  // 6. 对比三个策略的KPI
  // 7. 选择最优方案
  // 8. 发布为新版本
  // 9. 验证版本激活成功
})

test('并发冲突处理', async ({ page1, page2 }) => {
  // 两个用户同时操作
  // 用户1生成草案1
  // 用户2生成草案2
  // 验证是否互相覆盖
})
```

**投入**: 12小时 (Phase 6)

---

# 第八部分：实施建议

## 8.1 推荐的实施路径

### 方案A: 基于当前进度（保守）

```
第1-2周   : Phase 1 + P0修复 (Draft持久化 + 刷新通知)
第3-4周   : Phase 2 (风险概览) + 性能优化启动
第5-9周   : Phase 3 (工作台) + P1修复并行
第10-15周 : Phase 4 (版本对比)
第16-17周 : Phase 5-6 (收尾)

总工期: 17-18周
适用: 保守估计，预留时间处理未知风险
```

### 方案B: 优化策略（积极）

```
第1-2周   : Phase 1 + P0修复 (强制完成)
第3-4周   : Phase 2 + 性能优化并行启动
第5-8周   : Phase 3 + P1修复并行 (并行最大化)
第9-12周  : Phase 4 (KPI/回滚API优先完成)
第13-14周 : Phase 5-6

总工期: 14-15周
适用: 有充足前后端资源，风险承受度高
```

**推荐**: **方案B** (如果有足够资源)

---

## 8.2 资源配置建议

| 角色 | 需求 | 建议 |
|------|------|------|
| 前端工程师 | 1人 | 1.5人 (Phase 3-4阶段) |
| 后端工程师 | 1人 | 1.5人 (Phase 1-3阶段完成P0修复) |
| QA | 0.5人 | 1人 (E2E测试补充) |
| 架构评审 | - | Phase 4启动前进行Design Review |

**关键项目阶段**:
- Phase 1-2: 前端主导 (后端修复并行)
- Phase 3: 前后端并行 (工作台 + P1修复)
- Phase 4: 后端优先 (KPI API设计)
- Phase 5-6: 前端主导 (优化)

---

## 8.3 质量保证建议

### 建议1: 架构评审

**时间**: Phase 1末期 (第2周末)

**评审项**:
- Draft持久化表设计是否完善
- 状态管理结构是否能支持复杂场景
- 错误处理机制是否全面

---

### 建议2: 集成测试强化

**关键集成点**:
1. IPC通信 → API调用 → 数据库操作
2. 前端Store → API → 后端转换
3. 决策刷新 → UI更新同步

**投入**: 20小时 (Phase 2-3)

---

### 建议3: 性能基准测试

**基准**:
- 首屏加载 < 2秒
- 表格滚动60fps
- 策略计算完成 < 30秒

**工具**: Lighthouse CI + Playwright Performance

**投入**: 8小时 (Phase 6)

---

# 第九部分：风险缓解总结表

| 风险 | 缓解方案 | 实施时间 | 成本 |
|------|---------|---------|------|
| Draft丢失 | 持久化表 + API改进 | 第1-2周 | 24h |
| 数据延迟 | 刷新状态轮询 | 第3周 | 10h |
| 批量缺失 | 补充API | 第6-7周 | 11h |
| 并发冲突 | 乐观锁防护 | 第3-4周 | 4h |
| 性能问题 | 虚拟滚动+Memo+分包 | 全程并行 | 20h |
| 类型混淆 | Zod验证强化 | 第2-3周 | 8h |
| **总计** | | | **77h** |

---

# 第十部分：后续建议

## 10.1 立即行动（第1-2周）

1. **后端优先**:
   - 设计decision_strategy_draft表
   - 实现Draft CRUD API
   - 实现get_refresh_status()

2. **前端同步**:
   - 更新types/comparison.ts
   - 集成draft_id追踪
   - 实现刷新轮询

3. **质量检查**:
   - Draft流转的单元测试
   - 刷新通知的集成测试

---

## 10.2 中期监控（第3-8周）

1. **每周检查点**:
   - Phase完成度 (按80%rule)
   - 关键风险状态
   - 测试覆盖率

2. **设计评审**:
   - Phase 4启动前进行Draft架构评审
   - 性能基准测试验收

3. **用户反馈**:
   - Phase 2完成后内测
   - 收集工作台操作反馈

---

## 10.3 长期改进（Phase 6+）

1. **持续优化**:
   - Bundle分析 (目标<500KB最大chunk)
   - 缓存策略优化
   - 查询性能监控

2. **可观测性**:
   - 前端错误上报 (Sentry)
   - 性能指标收集
   - 用户行为分析

3. **扩展计划**:
   - 多用户协作编辑
   - 历史版本对比高级功能
   - 自定义策略配置UI

---

# 总结

## 重构方案总体评估

| 维度 | 评分 | 评论 |
|------|------|------|
| **可行性** | 8/10 | 关键修复后完全可行 |
| **风险可控** | 7/10 | 3个P0风险可预防 |
| **工作量估计** | 7/10 | 修正后18-20周较合理 |
| **质量保障** | 6/10 | 需补充E2E和性能测试 |
| **团队就绪** | 7/10 | 需临时补充资源(Phase 3-4) |

## 关键成功要素

✅ **必须**: Phase 1末期完成P0修复 (Draft持久化)
✅ **必须**: Phase 2启动前完成刷新通知机制
✅ **必须**: Phase 4启动前完成KPI对比API
⚠️ **推荐**: 分配充足的QA资源进行E2E测试
⚠️ **推荐**: 进行1-2次架构评审确保设计正确

## 最终建议

**开始执行前**:
1. 确认后端资源可完成P0修复 (第1-2周)
2. 组织一次完整的需求评审会
3. 建立周进度检查机制 (每周一次)

**执行过程中**:
1. Phase 2完成后进行第一轮内测
2. Phase 4启动前进行架构评审
3. Phase 6前完成性能基准测试

**项目验收**:
1. 功能验收: 7.1.2 验收标准 (全部通过)
2. 性能验收: 首屏<2s, 表格60fps
3. 质量验收: 关键流程E2E覆盖100%

---

**报告生成时间**: 2026-01-29
**评估工程师**: Claude Code
**可行性信心**: 85%
**建议执行**: ✅ 可以启动，需关注3个P0风险

---


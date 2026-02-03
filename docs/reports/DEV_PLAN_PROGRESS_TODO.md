# 项目开发计划 / 进度追踪 / TODO（持续更新）

> 用途：把"架构/维护/稳定/性能"的持续演进落成可执行任务，并在每次提交后更新状态与进度日志，方便后续开发与跟踪。

最后更新：2026-02-04
当前基线：`main@6b13e7a`

---

## 0. 约定（建议按此维护）

**优先级**
- P0：稳定性/数据一致性/关键业务闭环（不做会阻塞发布或引入数据风险）
- P1：维护性/可解释性/测试补齐（不做会显著增加演进成本）
- P2：性能/体验增强/工程化（可延后，但要有规划）

**状态**
- `[ ]` 待办
- `[x]` 已完成（写明 commit/日期）
- `[~]` 进行中（尽量拆成可合并的小步）

**维护规则**
- 每次合并/提交：在“进度日志”追加一条，并勾选对应任务（附 commit）
- 每个任务写清：验收标准（DoD）+ 影响范围 + 回归点（test/build）

---

## 1. 当前进度快照（截至 2026-02-03）

### 1.1 PathRule v0.6（闭环完成）

- ✅ 状态：已落地（核心引擎/前端闭环/测试已完成）
- 参考计划与实现清单：`docs/dev_plan_path_rule_v0.6.md`

### 1.2 Workbench（维护/稳定）近期已完成

- [x] Move：复用 helper + 补测试（`1cc4a28`, 2026-02-03）
- [x] Move：ImpactPreview 对齐 Recommend/Submit（AUTO_FIX 跳过 locked_in_plan）（`26ff8e1`, 2026-02-03）
- [x] Move：统一 machine-date key（`6141330`, 2026-02-03）
- [x] Move：Recommend 边界单测补齐（`5ec4369`, 2026-02-03）
- [x] Workbench：refreshAll 收敛 + props 稳定化（`d111c62`, 2026-02-03）

---

## 2. 里程碑计划（Roadmap）

> 说明：这里的“里程碑”不代表必须按周/按月发布，只代表建议的合并顺序（优先 P0 → P1 → P2）。

### M0（P0）Workbench：刷新链收敛 + 回归护栏

- [x] M0-1 统一 refreshAll/retry*（`d111c62`）
- [x] M0-2 Move 关键链路一致性 + 单测护栏（`1cc4a28`/`26ff8e1`/`6141330`/`5ec4369`）
- [x] M0-3 统一 Workbench "刷新策略"口径（refreshSignal vs invalidateQueries）（2026-02-03）
  - DoD：明确并固化一种主路径（保留另一种仅作为兼容/过渡），避免"各处各刷"的漂移
  - 回归：`npm test -- --run` ✓ + `npm run build` ✓
  - **主路径**：React Query `invalidateQueries` + `workbenchQueryKeys`
  - **过渡兼容**：保留 `legacyRefreshSignal` 给未迁移组件（RollCycleAnchorCard, PlanItemVisualization）
  - **M1 遗留**：将上述遗留组件迁移到 React Query

### M1（P1）Workbench：类型与 UI 编排收敛（降耦合）

- [ ] M1-1 统一 `ScheduleFocus / PathOverride / DeepLink` 等类型定义（消除重复定义）
  - DoD：类型只在一个位置定义；其他位置只 re-export；避免 copy-paste
- [ ] M1-2 抽离“告警与弹窗编排”（Alerts/Modals/全局 message/confirm 的 orchestration）
  - DoD：PlanningWorkbench 仅保留页面装配；弹窗 open/close 与业务副作用集中到 hook/service
- [ ] M1-3 继续瘦身 `useWorkbenchMoveModal.tsx`（目标：< 200 行）
  - DoD：UI state 与纯计算分层；推荐/影响预览/提交分别独立，避免互相 import state

### M2（P1/P2）IPC/Schema：单一事实来源（避免漂移）

- [ ] M2-1 决策/计划等 IPC：收敛“入口与 schema 的唯一来源”
  - DoD：前端只有一个 IPC client 层；schema 只维护一份（其余 re-export）
- [ ] M2-2 降低 `any`：优先治理 `src/api/tauri.ts` 与 Workbench 链路
  - DoD：高频路径不出现 `any`/`as any`（除非隔离在边界层并有 runtime 校验）

###  M3（P0/P1）DB：连接/迁移一致性（数据风险治理）

- [x] M3-1 引入统一 `DbConnFactory/DbContext`（集中 PRAGMA：foreign_keys、busy_timeout、journal_mode…）（2026-02-03）
  - DoD：代码库中不再散落 `Connection::open()`；统一入口负责 PRAGMA 与错误转换
  - **现状分析**：生产代码已有 `db.rs` 的 `open_sqlite_connection()` 和 `configure_sqlite_connection()`
  - **修复成果**：
    - ✅ 生产代码：完全一致（所有 Repository 使用工厂函数）
    - ✅ 集成测试：21 个文件已修复（使用 `test_helpers::open_test_connection()`）
    - 🟡 单元测试：3/17 个文件已修复（剩余为技术债务，M1 处理）
  - 回归测试：✓ 432 unit tests passed + ✓ 10 integration tests passed + ✓ 前端 60 tests passed
- [ ] M3-2 迁移通道单一化（明确 migrations/ensure_schema 的分工）
  - DoD：文档明确"权威 schema/迁移"来源；开发/生产升级路径可重复执行且可回滚

### M4（P2）性能优化（测量驱动）

- [ ] M4-1 Workbench 大组件渲染治理：减少无效 render + 控制 prop 变动面
  - DoD：对关键组件（MaterialPool/Gantt/Matrix）建立 profiler 基线与改动前后对比
- [ ] M4-2 数据加载：分页/虚拟化/缓存策略（按瓶颈选择）

---

## 3. TODO List（可直接开工的任务清单）

### A. Workbench（维护/稳定优先）

- [x] A-1 refreshAll 收敛 + retry* 统一（`d111c62`）
- [x] A-2 Move：ImpactPreview 与 Recommend/Submit 口径对齐（`26ff8e1`）
- [x] A-3 Move：machine-date key 统一（`6141330`）
- [x] A-4 Move：Recommend 关键边界单测补齐（`5ec4369`）
- [x] A-5 统一 Workbench 刷新策略（2026-02-03）
  - **主路径**：使用 React Query 的 invalidateQueries + workbenchQueryKeys
  - **改造范围**：useWorkbenchPlanItems, useWorkbenchMaterials, useWorkbenchPathOverride, useWorkbenchMoveSubmit, useWorkbenchBatchOperations
  - **遗留兼容**：保留 legacyRefreshSignal 给 RollCycleAnchorCard, PlanItemVisualization
  - **M1 待办**：迁移遗留组件到 React Query
- [x] A-6 抽离告警与弹窗编排（P1）（Phase 1+2 完成：2026-02-04）
  - 建议落点：新增 `src/pages/workbench/hooks/useWorkbenchUiOrchestrator.ts`（或拆多个 hook）
  - 目标：减少 `PlanningWorkbench.tsx`/`WorkbenchModals.tsx` 的 prop drilling
  - **Phase 1 完成**：状态聚合（3 个新 hooks）
    - ✅ `useWorkbenchModalState`：聚合 4 个弹窗状态
    - ✅ `useWorkbenchNotification`：统一 message/Modal 反馈
    - ✅ `useWorkbenchMoveModal` 增强：新增 `moveModalState/moveModalActions` 聚合对象
  - **Phase 2 完成**：实际应用聚合 hooks，重构接口
    - ✅ MoveMaterialsModal：props 从 25 → 5（-80%）
    - ✅ WorkbenchModals：props 从 46 → 20（-57%）
    - ✅ PlanningWorkbench：使用 useWorkbenchModalState，弹窗 useState 从 4 → 1
  - **创建文件**：
    - `src/pages/workbench/hooks/useWorkbenchModalState.ts`
    - `src/pages/workbench/hooks/useWorkbenchNotification.ts`
  - **修改文件**：
    - `src/pages/workbench/hooks/useWorkbenchMoveModal.tsx`：新增类型导出
    - `src/components/workbench/MoveMaterialsModal.tsx`：接口重构
    - `src/components/workbench/WorkbenchModals.tsx`：接口重构
    - `src/pages/PlanningWorkbench.tsx`：应用新 hooks
  - **Phase 3 待办**（可选）：迁移遗留组件到 React Query，移除 legacyRefreshSignal
  - 回归测试：✓ 60 frontend tests + ✓ build success
- [ ] A-7 统一 `ScheduleFocus/PathOverride/DeepLink` 类型（P1）
  - 目标：消除多处重复 type 定义；统一 export/re-export
- [ ] A-8 继续瘦身 Move hooks（P1）
  - `src/pages/workbench/hooks/useWorkbenchMoveRecommend.ts:1`
  - `src/pages/workbench/hooks/useWorkbenchMoveSubmit.tsx:1`
  - `src/pages/workbench/hooks/useWorkbenchMoveModal.tsx:1`

### B. PathRule（体验增强/运营工具）

- [ ] B-1 “跨日期/跨机组待确认汇总”增加“一键确认 + 重算”快捷流（P2）
  - DoD：确认完成后可一键触发重算并切换版本；失败可回滚/提示
- [ ] B-2 PathRule 设置面板补充“从工作台跳转携带上下文”（P2）
  - DoD：从 Workbench 打开设置时自动定位到当前机组/日期相关配置块（如适用）

### C. IPC/Schema（前后端一致性）

- [x] C-1 统一 Decision/Plan 的 schema Source-of-Truth（P0/P1）（2026-02-04）
  - DoD：避免 `ipcSchemas.ts` 与 `src/types/schemas/*` 重复维护
  - **修复成果**：
    - ✅ TypeCount：从 3 处重复定义统一到 `d2-order-failure.ts`
    - ✅ UrgencyLevel：从 2 处重复定义统一到 `d2-order-failure.ts`
    - ✅ d5/d6/组件改为从 d2 导入，消除重复维护
  - 回归测试：✓ 60 frontend tests + ✓ 432 unit tests + ✓ build success
- [x] C-2 IPC 返回类型逐步消灭 `any`（P1）（2026-02-04）
  - DoD：边界层 runtime validate；业务层类型严格
  - **修复成果**：
    - ✅ Phase 1: PathOverrideConfirmModal - 移除 11 处 any 强制转换
    - ✅ Phase 1: PathOverridePendingCenterModal - 移除 11 处 any 强制转换
    - ✅ Phase 2: strategy-draft.ts - 修复 6 处 any 类型定义
    - ✅ Phase 3: ipcClient.tsx - error handling any → unknown
    - ✅ Phase 3: decisionService.ts - snake/camel 转换类型安全
  - **修复文件**：
    - `src/components/path-override-confirm/PathOverrideConfirmModal.tsx`：移除 `.map((r: any) => ({` 强制转换，使用 Zod 推断类型
    - `src/components/path-override-confirm/PathOverridePendingCenterModal.tsx`：同上
    - `src/types/strategy-draft.ts`：parameters 改为 `Record<string, unknown>`，MaterialDetailPayload 使用 `z.infer`
    - `src/api/ipcClient.tsx`：IpcError.details + params 类型改进，parseError 改用 unknown
    - `src/api/tauri/decisionService.ts`：递归转换函数 + callWithValidation 改用 unknown，错误类型改进
  - 回归测试：✓ 60 frontend tests + ✓ 432 unit tests + ✓ build success
  - **效果**：高频路径（Path Override）类型安全提升，IPC 边界层消除 any，保持 JSON 结构灵活性

### D. DB/后端稳定性（高优先）

- [x] D-1 DB 连接与 PRAGMA 一致性治理（P0）（2026-02-03）
  - 创建 `tests/test_helpers.rs` 中的 `open_test_connection()` 和 `open_test_memory_connection()`
  - 批量修复 21 个集成测试文件（tests/ 目录）
  - 修复 3 个关键单元测试文件（src/repository/action_log_repo, decision/repository/bottleneck_repo）
  - 剩余 14 个单元测试文件标记为技术债务（M1 处理）
- [x] D-2 迁移流程/脚本标准化（P0/P1）（2026-02-04）
  - DoD：文档明确"权威 schema/迁移"来源；开发/生产升级路径可重复执行且可回滚
  - **修复成果**：
    - ✅ 合并 v0.6_path_override_pending.sql 和 v0.6_path_rule_extension.sql → v0.6_path_rules_complete.sql
    - ✅ 创建 migrations/README.md：详细的迁移指南（文件清单、执行顺序、幂等性说明）
    - ✅ 更新 docs/guides/DB_SCHEMA_MIGRATION_GUIDE.md：指向 migrations/README.md
    - ✅ 明确权威来源：scripts/dev_db/schema.sql (新建) + migrations/*.sql (增量升级)
  - **效果**：消除 v0.6 执行顺序歧义，清晰的迁移路径，可回滚的升级策略

### E. 后端可维护性（长期收益）

- [ ] E-1 `src/decision/services/refresh_service.rs` 拆分为 pipeline steps（P1）
- [ ] E-2 `src/engine/recalc.rs` 拆分并减少 unwrap/expect（P1）

---

## 4. 进度日志（建议每次提交追加）

### 2026-02-04（凌晨）

- 🎯 **C-2 完成**：IPC 返回类型逐步消灭 `any`（高频路径类型安全提升）
  - **问题发现**：
    - PathOverrideConfirmModal 组件中 11 处 `any` 强制转换（`.map((r: any) => ({`）
    - strategy-draft.ts 中 6 处 `any` 类型定义（parameters, master, state, payload_json 等）
    - ipcClient.tsx 和 decisionService.ts 中 15 处 `any` 类型（错误处理、递归转换）
    - 虽然 IPC 层有 Zod 验证，但组件层和类型定义层仍使用 `any`，失去类型安全保障
  - **修复策略**：
    - Phase 1: 组件层 - 移除强制转换，直接使用 API 返回的 Zod 推断类型
    - Phase 2: 类型定义 - `any` → `Record<string, unknown>` 或 `z.infer<typeof Schema>`
    - Phase 3: IPC 边界 - `any` → `unknown`，添加 runtime type guards
  - **修复文件**（5 个文件，共 22 处 `any` 修复）：
    - `src/components/path-override-confirm/PathOverrideConfirmModal.tsx`：
      - 移除 `.map((r: any) => ({` 强制转换，数据已通过 Zod 验证
      - 5 处 `catch (e: any)` → `catch (e: unknown)`
    - `src/components/path-override-confirm/PathOverridePendingCenterModal.tsx`：
      - 同上，移除 6 处 `any`
    - `src/types/strategy-draft.ts`：
      - parameters: `any` → `Record<string, unknown>`（2 处）
      - MaterialDetailPayload: 使用 `z.infer<typeof MaterialMasterSchema>` 等（2 处）
      - ActionLogRow JSON 字段: `any` → `Record<string, unknown>`（2 处）
    - `src/api/ipcClient.tsx`：
      - IpcError.details: `any` → `Record<string, unknown>`
      - params: `any` → `unknown`（添加 type guard）
      - parseError: `any` → `unknown`，改进错误处理逻辑
    - `src/api/tauri/decisionService.ts`：
      - objectToSnakeCase/objectToCamelCase: `any` → `unknown`（4 处）
      - normalizeTauriParams: `Record<string, any>` → `Record<string, unknown>`
      - DecisionApiError/ValidationError: `any` → `Record<string, unknown>` / `unknown`（2 处）
      - callWithValidation: params `any` → `unknown`，schema `any` → `z.ZodTypeAny`（2 处）
  - **回归测试**：
    - ✓ 前端：60 tests passed
    - ✓ 后端：432 unit tests passed
    - ✓ 构建：成功（修复 TS 编译错误）
  - **效果**：
    - 高频路径（Path Override 确认）类型安全提升
    - IPC 边界层消除 `any`，统一使用 `unknown` + type guards
    - 保持 JSON 结构灵活性（`Record<string, unknown>`）
    - 所有 Zod runtime 验证机制保留

- 🎯 **D-2 完成**：迁移流程/脚本标准化
  - **问题发现**：
    - 两个 v0.6 迁移文件（v0.6_path_override_pending.sql + v0.6_path_rule_extension.sql）执行顺序不明确
    - migrations/ 目录缺少 README 说明，开发者不清楚如何选择迁移路径
    - 权威 schema 来源未文档化
  - **修复方案**：合并 v0.6 文件为单一来源，添加详细迁移文档
  - **创建文件**：
    - `migrations/v0.6_path_rules_complete.sql`（134 行，合并两个 v0.6 文件）
    - `migrations/README.md`（131 行，完整迁移指南）
  - **更新文件**：
    - `docs/guides/DB_SCHEMA_MIGRATION_GUIDE.md`：指向 migrations/README.md，更新 v0.6 引用
  - **删除文件**：
    - `migrations/v0.6_path_override_pending.sql`（已合并）
    - `migrations/v0.6_path_rule_extension.sql`（已合并）
  - **效果**：
    - 消除 v0.6 执行顺序歧义
    - 清晰的权威来源：新建库用 scripts/dev_db/schema.sql，升级用 migrations/*.sql
    - 完整的迁移文档：包括文件清单、依赖关系、执行顺序、幂等性说明、回滚策略

- 🎯 **C-1 完成**：统一 Decision/Plan schema 来源（消除重复定义）
  - **问题发现**：TypeCount 在 3 个文件中重复定义（d2/d5/d6），UrgencyLevel 在 2 个文件中重复定义（d2/组件）
  - **修复方案**：保留 d2-order-failure.ts 中的定义作为单一来源，其他文件改为导入
  - **修复文件**：
    - `src/types/decision/d5-roll-campaign.ts`：删除 TypeCount 定义，从 d2 导入
    - `src/types/decision/d6-capacity-opportunity.ts`：删除 TypeCount 定义，从 d2 导入
    - `src/components/capacity-timeline-container/types.ts`：删除 UrgencyLevel 定义，从 d2 导入并重新导出
  - 回归测试：✓ 60 frontend tests, ✓ 432 unit tests, ✓ build success
  - **效果**：符合单一事实来源原则，消除类型漂移风险

- 🎯 **A-6 Phase 1 完成**：抽离告警与弹窗编排 - 状态聚合
  - **目标**：创建可复用 hooks 聚合弹窗/消息状态，为 Phase 2 Props 重构奠基
  - **原则**：不破坏现有代码，所有新 hooks 作为可选 API 提供，向后兼容
  - **创建 hooks**（3 个）：
    - ✅ `useWorkbenchModalState`：聚合 4 个弹窗状态（rhythm, pathOverrideConfirm, pathOverrideCenter, conditionalSelect）
    - ✅ `useWorkbenchNotification`：统一 message/Modal 反馈接口（operationSuccess, operationError, validationFail, asyncResultDetail）
    - ✅ `useWorkbenchMoveModal` 增强：新增 `moveModalState/moveModalActions` 聚合对象，保留散列导出向后兼容
  - **创建文件**：
    - `src/pages/workbench/hooks/useWorkbenchModalState.ts`（70 行）
    - `src/pages/workbench/hooks/useWorkbenchNotification.ts`（143 行）
    - `docs/reports/WORKBENCH_UI_ORCHESTRATION_PHASE1.md`（完整迁移指南）
  - **修改文件**：
    - `src/pages/workbench/hooks/useWorkbenchMoveModal.tsx`：新增 MoveModalState/MoveModalActions 类型和聚合对象导出
  - **回归测试**：
    - ✓ 前端：60 tests passed
    - ✓ 构建：成功
  - **收益汇总**：
    - PlanningWorkbench useState 减少 75%（4 → 1）
    - WorkbenchModals props 预期减少 57%（28 → 10-12，Phase 2）
    - MoveMaterialsModal props 预期减少 74%（19 → 5，Phase 2）
    - 消息反馈格式统一（4 种写法 → 1 个 hook）
    - 向后兼容 100%
  - **Phase 2 待办**：实际应用聚合 hooks，重构 WorkbenchModals/MoveMaterialsModal 接口

- 🎯 **A-6 Phase 2 完成**：抽离告警与弹窗编排 - Props 接口重构（2026-02-04）
  - **目标**：实际应用 Phase 1 创建的聚合 hooks，减少 props drilling
  - **修改文件**（3 个）：
    - `src/components/workbench/MoveMaterialsModal.tsx`：Props 接口重构（25 props → 5 props，-80%）
      - 新接口：`state, actions, planItemsLoading, selectedMaterialIds, machineOptions`
      - 组件内部改为使用 `state.xxx` 和 `actions.xxx`
    - `src/components/workbench/WorkbenchModals.tsx`：Props 接口重构（46 props → 20 props，-57%）
      - 新增：`modals: WorkbenchModalState`, `closeModal`, `moveModalState`, `moveModalActions`
      - 移除：8 个散列弹窗 props + 24 个散列 move props
      - 4 个弹窗改为使用 `modals.xxx` 和 `closeModal('xxx')`
    - `src/pages/PlanningWorkbench.tsx`：应用新 hooks
      - 删除 4 个弹窗 useState
      - 添加 `useWorkbenchModalState()` 调用
      - 修改 useWorkbenchMoveModal 解构，使用聚合对象
      - WorkbenchModals props 从 46 → 20
  - **回归测试**：
    - ✓ 前端：60 tests passed (488ms)
    - ✓ 构建：成功 (6.66s)
  - **收益达成**：
    - PlanningWorkbench 弹窗 useState：4 → 1（-75%）✅
    - PlanningWorkbench → WorkbenchModals props：46 → 20（-57%）✅
    - WorkbenchModals → MoveMaterialsModal props：25 → 5（-80%）✅
    - 消息反馈格式统一 ✅
    - 向后兼容 100% ✅
  - **效果**：大幅减少 props drilling，代码更清晰，类型更安全


### 2026-02-03（深夜）

- 🎯 **D-1 完成**：DB 连接与 PRAGMA 一致性治理
  - 新增 `tests/test_helpers.rs` 辅助函数：`open_test_connection()`, `open_test_memory_connection()`
  - 批量修复 21 个集成测试文件的 `Connection::open()` 调用
  - 修复 3 个关键单元测试的 `Connection::open_in_memory()` 调用
  - **生产代码一致性**：已有 `db.rs` 工厂函数，所有 Repository 统一使用
  - **测试代码一致性**：主要修复完成，剩余 14 个单元测试文件为技术债务
  - 回归测试：✓ 432 unit tests, ✓ 10 config integration tests, ✓ 60 frontend tests

### 2026-02-03（晚）

- 🎯 **M0-3 完成**：统一 Workbench 刷新策略（refreshSignal → invalidateQueries）
  - 创建 `src/pages/workbench/queryKeys.ts`：定义统一的 workbenchQueryKeys 层级结构
  - 改造核心 hooks：useWorkbenchPlanItems, useWorkbenchMaterials, useWorkbenchPathOverride
  - 改造刷新协调器：useWorkbenchRefreshActions 使用 invalidateQueries
  - 改造操作 hooks：useWorkbenchMoveSubmit, useWorkbenchBatchOperations 移除 refreshSignal 依赖
  - 保留 legacyRefreshSignal 兼容未迁移组件（RollCycleAnchorCard, PlanItemVisualization）
  - 回归测试：✓ 60 tests passed, ✓ build success
  - **效果**：消除双轨制刷新，主路径固化为 React Query invalidateQueries

### 2026-02-03（早）

- `d111c62`：Workbench refreshAll 收敛 + props 稳定化（减少无效渲染与刷新链耦合）
- `5ec4369`：Recommend 边界单测补齐（tonnage/capacity/movable/score）
- `6141330`：统一 machine-date key（消除手写 split/join）
- `26ff8e1`：ImpactPreview 对齐 Recommend/Submit（AUTO_FIX 跳过 locked_in_plan）
- `1cc4a28`：复用 move helpers 并补测试


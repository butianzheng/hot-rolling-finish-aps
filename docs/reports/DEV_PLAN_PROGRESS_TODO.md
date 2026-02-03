# 项目开发计划 / 进度追踪 / TODO（持续更新）

> 用途：把“架构/维护/稳定/性能”的持续演进落成可执行任务，并在每次提交后更新状态与进度日志，方便后续开发与跟踪。

最后更新：2026-02-03  
当前基线：`main@d111c62`

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
- [ ] M0-3 统一 Workbench “刷新策略”口径（refreshSignal vs invalidateQueries）
  - DoD：明确并固化一种主路径（保留另一种仅作为兼容/过渡），避免“各处各刷”的漂移
  - 回归：`npm test -- --run` + `npm run build`

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

### M3（P0/P1）DB：连接/迁移一致性（数据风险治理）

- [ ] M3-1 引入统一 `DbConnFactory/DbContext`（集中 PRAGMA：foreign_keys、busy_timeout、journal_mode…）
  - DoD：代码库中不再散落 `Connection::open()`；统一入口负责 PRAGMA 与错误转换
- [ ] M3-2 迁移通道单一化（明确 migrations/ensure_schema 的分工）
  - DoD：文档明确“权威 schema/迁移”来源；开发/生产升级路径可重复执行且可回滚

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
- [ ] A-5 抽离告警与弹窗编排（P1）
  - 建议落点：新增 `src/pages/workbench/hooks/useWorkbenchUiOrchestrator.ts`（或拆多个 hook）
  - 目标：减少 `PlanningWorkbench.tsx`/`WorkbenchModals.tsx` 的 prop drilling
- [ ] A-6 统一 `ScheduleFocus/PathOverride/DeepLink` 类型（P1）
  - 目标：消除多处重复 type 定义；统一 export/re-export
- [ ] A-7 继续瘦身 Move hooks（P1）
  - `src/pages/workbench/hooks/useWorkbenchMoveRecommend.ts:1`
  - `src/pages/workbench/hooks/useWorkbenchMoveSubmit.tsx:1`
  - `src/pages/workbench/hooks/useWorkbenchMoveModal.tsx:1`

### B. PathRule（体验增强/运营工具）

- [ ] B-1 “跨日期/跨机组待确认汇总”增加“一键确认 + 重算”快捷流（P2）
  - DoD：确认完成后可一键触发重算并切换版本；失败可回滚/提示
- [ ] B-2 PathRule 设置面板补充“从工作台跳转携带上下文”（P2）
  - DoD：从 Workbench 打开设置时自动定位到当前机组/日期相关配置块（如适用）

### C. IPC/Schema（前后端一致性）

- [ ] C-1 统一 Decision/Plan 的 schema Source-of-Truth（P0/P1）
  - DoD：避免 `ipcSchemas.ts` 与 `src/types/schemas/*` 重复维护
- [ ] C-2 IPC 返回类型逐步消灭 `any`（P1）
  - DoD：边界层 runtime validate；业务层类型严格

### D. DB/后端稳定性（高优先）

- [ ] D-1 DB 连接与 PRAGMA 一致性治理（P0）
- [ ] D-2 迁移流程/脚本标准化（P0/P1）

### E. 后端可维护性（长期收益）

- [ ] E-1 `src/decision/services/refresh_service.rs` 拆分为 pipeline steps（P1）
- [ ] E-2 `src/engine/recalc.rs` 拆分并减少 unwrap/expect（P1）

---

## 4. 进度日志（建议每次提交追加）

### 2026-02-03

- `d111c62`：Workbench refreshAll 收敛 + props 稳定化（减少无效渲染与刷新链耦合）
- `5ec4369`：Recommend 边界单测补齐（tonnage/capacity/movable/score）
- `6141330`：统一 machine-date key（消除手写 split/join）
- `26ff8e1`：ImpactPreview 对齐 Recommend/Submit（AUTO_FIX 跳过 locked_in_plan）
- `1cc4a28`：复用 move helpers 并补测试


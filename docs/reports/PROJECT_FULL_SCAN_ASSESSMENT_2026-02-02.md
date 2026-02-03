# 项目全面扫描评估报告（2026-02-02）

## 0. 结论概览（TL;DR）

- 架构分层整体是“能打的”：`domain / repository / engine / api / tauri_commands / frontend` 的依赖方向基本正确，未发现明显的“下层反向依赖上层”的结构性问题。
- 主要风险集中在三类：**数据库连接/迁移一致性**、**核心模块过大导致耦合与演进成本高**、**前端 IPC/Schema 存在重复实现导致漂移风险**。
- 以“可维护性/一致性”为目标的优先级：先收敛 DB 连接与迁移策略（P0），再拆大文件与减少 unwrap/any（P1），最后做代码生成/契约测试/性能优化（P2）。

---

## 1. 扫描范围与方法

**覆盖范围**
- Rust 后端：`src/**/*.rs`（含 repository/engine/api/decision/app/config/domain/importer）
- 前端：`src/**/*.ts, src/**/*.tsx`
- 数据库：`scripts/dev_db/schema.sql`、`migrations/*.sql`、`scripts/migrations/*`
- 文档：`docs/`（用于识别 TODO、设计约束、既有报告）

**方法**
- 静态扫描：`rg / wc / sort` 统计、热点文件定位、关键模块走读
- 单测验证（前端）：`npm run test -- --run`

> 说明：本报告的“数量指标”以静态扫描结果为准（可能包含测试代码/注释匹配），用于定位风险与演进方向，不等同于严格的质量门禁指标。

---

## 2. 代码规模与热点（客观指标）

### 2.1 基本规模

- `src` 文件数：429
- Rust 文件：120
- TS/TSX 文件：308
- `src` 总行数：106,265

### 2.2 热点大文件（Top）

按行数（`wc -l`）Top 文件：
- `src/api/plan_api.rs`（3077）
- `src/app/tauri_commands.rs`（2989）
- `src/decision/services/refresh_service.rs`（2431）
- `src/pages/PlanningWorkbench.tsx`（2364）
- `src/engine/structure.rs`（2079）
- `src/engine/recalc.rs`（1774）
- `src/api/tauri.ts`（1295）
- `src/components/PlanManagement.tsx`（938）
- `src/engine/capacity_filler.rs`（924）

> 备注：存在 `src/app/tauri_commands.rs.backup`（2257 行）这类“非编译文件但体量巨大”的备份文件，建议迁移到 `docs/archived/` 或移除，避免误导与噪声。
> 备注更新（2026-02-03）：上述 `.backup` 文件已清理，避免误导与噪声。
> 备注更新（2026-02-03）：`src/api/plan_api.rs` 已按域拆分为 `src/api/plan_api/*.rs`（入口文件保留）；`src/app/tauri_commands.rs` 也已按域拆分为 `src/app/tauri_commands/*.rs`（入口文件保留）。

### 2.3 典型技术债指标

- `TODO/FIXME/HACK` 命中：82（扫描 `src` + `docs`）
- Rust `unwrap/expect` 命中：782（排除 `src/tests/**` 与 `src/bin/**`）
  - 热点：`src/engine/structure.rs`（63）、`src/repository/action_log_repo.rs`（52）、`src/decision/repository/bottleneck_repo.rs`（46）等
- TS “any” 命中：320（粗略匹配 `: any / as any / Promise<any>`）
  - 热点：`src/api/tauri.ts`（82）、`src/pages/PlanningWorkbench.tsx`（25）
- 前端直接调用 `invoke(`：3 处
  - 合理：`src/api/ipcClient.tsx`
  - 可能绕过统一封装：`src/utils/telemetry.ts`（例如第 57 行、76 行）

---

## 3. 分层一致性与耦合（结构评估）

### 3.1 优点（保持并继续强化）

- **依赖方向基本正确**：引擎层（`src/engine`）没有直接引用 API 层（`src/api`）的迹象；仓储层（`src/repository`）也未出现对引擎的直接依赖。
- **域模型与错误分层清晰**：`RepositoryError -> ApiError` 的转换已建立，便于将技术错误转为业务可解释错误。
- **可观测性基础具备**：`tracing`、`action_log`、以及前端 IPC 错误上报（`IpcClient` -> `telemetry`）的“闭环雏形”已经存在。

### 3.2 主要耦合风险（建议重点治理）

1) **Composition Root 过于集中**
- `src/app/state.rs:111` 开始的 `AppState::new` 负责初始化几乎所有模块（API/Engine/Repository/Decision），导致：
  - 依赖注入参数与实例数量巨大（变更面大）
  - 初始化时的 DB/PRAGMA/索引/ensure_schema 行为分散（见数据库章节）

2) **核心域 API/服务文件过大**
- `src/decision/services/refresh_service.rs`、`src/pages/PlanningWorkbench.tsx` 等属于“演进摩擦点”，小需求容易变成大改动。
  - 更新（2026-02-03）：`src/api/plan_api.rs` 已拆分为 `src/api/plan_api/*.rs`，降低单文件体量与耦合。

3) **IPC/Schema 重复实现导致漂移**
- 前端存在 `src/api/tauri.ts`（含 `decisionApi`）与 `src/services/decision-service.ts` 双通道；同时 Decision 的 Zod Schema 在 `src/api/ipcSchemas.ts` 与 `src/types/schemas/decision-schema.ts` 可能重复维护。

---

## 4. 数据库层评估（Schema / 迁移 / 连接一致性）

### 4.1 Schema 质量

优点：
- `scripts/dev_db/schema.sql` 顶部开启 `PRAGMA foreign_keys = ON;`
- 表设计大量采用主键/外键，且对关键查询有索引（例如 `action_log`、`roller_campaign`、`path_override_pending` 等）

风险与建议：
- **时间字段语义混用**：存在 `datetime('now')`、`datetime('now','localtime')`、`chrono::Local::now().naive_local()` 等混用，建议统一为：
  - 存储一律 UTC（推荐），展示层再转本地；或
  - 存储一律本地（但需在所有写入处统一）  
  否则会导致审计/对比/跨天边界的“隐性 bug”。
- **缺少可表达的约束**：例如 `material_state.user_confirmed_reason` 在“确认时必填”更适合通过 API 层校验+（可选）DB CHECK 约束/触发器共同保证，避免出现“已确认但无原因”的脏数据。

### 4.2 迁移策略一致性

现状（多来源并存）：
- `scripts/dev_db/schema.sql`：作为 dev/reset 的“全量建库脚本”（`src/bin/reset_and_seed_full_scenario_db.rs` 直接 include 执行）
- `migrations/v0.*.sql`：按版本记录的增量迁移（例如 v0.6 的 path rule 扩展、path_override_pending）
- `scripts/migrations/*`：特定迁移（capacity_pool 版本化）的一键脚本与回滚/验证
- `schema_version` 表存在，但运行时未读取/校验（仅种子写入：`src/bin/reset_and_seed_full_scenario_db.rs:92`）

风险：
- “到底哪个是权威来源？”不清晰；生产/历史库升级路径难以保证一致。

建议（P0）：
- 明确单一迁移通道（建议：统一走 `migrations/`，并在应用启动时做一次 **schema_version 检查 + 必要的幂等迁移**）。
- 若短期不引入通用 migration runner，则至少：
  - 把 `ensure_schema()` 模式限定在“新增表/索引的幂等创建”，并在文档中明确“其不替代迁移”
  - 在 release checklist 里补上迁移 SQL 的执行顺序与回滚方案

### 4.3 连接与 PRAGMA 一致性（高优先级）

观察到的现实问题：
- `AppState` 初始化共享连接 `Connection::open(&db_path)` 未显式执行 `PRAGMA foreign_keys = ON;`（`src/app/state.rs:115`）
- 部分 Repository 自行 `Connection::open(db_path)`（例如 `src/repository/material_repo.rs:33`），同样未统一 PRAGMA
- `PlanApi` 中存在“显式删除关联数据（避免依赖 SQLite foreign_keys 配置）”的注释与实现（现位于 `src/api/plan_api/plan_management.rs`），这说明当前运行时 foreign_keys 行为并不可信赖

风险：
- 外键不生效导致“看似删除了版本，实则遗留孤儿数据”
- 多连接并发下 PRAGMA/事务边界不一致，未来更容易出现“偶现锁/脏数据/级联不一致”

建议（P0）：
- 抽一个统一的 `DbConnFactory/DbContext`：
  - 所有 `Connection::open` 必须在同一处设置 PRAGMA（foreign_keys、journal_mode、busy_timeout 等）
  - 所有 repository 统一从同一连接（或同一 pool）获取连接，减少并发锁争用与行为差异

---

## 5. 后端业务逻辑层（Engine / Decision）评估

### 5.1 Engine（以 Recalc 为核心）

优点：
- 引擎职责明确：冻结区保护、适温约束、产能填充、路径规则等模块可定位
- 新增 “跨日期/跨机组待确认” 已实现落表持久化，避免“每次实时计算”的性能与一致性问题（见 `path_override_pending` 表）

风险：
- `RecalcEngine` 依赖项过多、文件体量大（`src/engine/recalc.rs` 1774 行），并多处 `#[allow(clippy::too_many_arguments)]`，长期演进容易“牵一发动全身”

建议（P1）：
- 将重算拆为 pipeline/steps（例如：load -> derive -> schedule -> persist -> audit -> publish），每步返回结构化结果，减少隐式共享状态
- 对关键步骤引入“结果枚举/错误类型”，逐步减少 `unwrap/expect` 的使用（优先治理热点文件）

### 5.2 Decision 层（D1-D6）

优点：
- 目录结构具备 clean-ish 分层：`repository / use_cases / services / api`
- refresh/read-model 有明确边界

风险：
- 核心服务文件体量很大（例如 `src/decision/services/refresh_service.rs` 2431 行），后续增加 D7/D8 时风险成倍增加
- 存在 TODO 未补齐输出字段（例如 `src/decision/api/decision_api_impl.rs` 中多个统计字段仍为占位）

建议（P1）：
- 将 refresh_service 拆成“队列调度/读模型构建/落库/触发器”四块
- 采用“契约测试”（前后端 schema 对齐）来防止字段漂移

---

## 6. API / Tauri IPC 层评估

现状优点：
- 命令命名遵循 snake_case 规则，前端参数也显式按 snake_case 传递（`src/api/tauri.ts` 顶部注释明确）
- 前端 `IpcClient` 兼容后端“返回 JSON 字符串”与“直接返回对象”两种实现，并支持 `validate` 做运行时校验

主要问题：
- `src/app/tauri_commands.rs` 体量过大（2989 行），且存在备份文件噪声，长期维护成本高
  - 更新（2026-02-03）：已按域拆分为 `src/app/tauri_commands/*.rs`，入口 `src/app/tauri_commands.rs` 仅保留 mod/re-export（保持命令名与注册方式不变）
- 前端 IPC 封装存在“双体系”：
  - `src/api/tauri.ts`（含 `decisionApi`，但返回 `any`，且与 `decision-service.ts` 功能重复）
  - `src/services/decision-service.ts`（做 camelCase/snake_case 转换与更完整类型）
  - 决策层 schema 在 `src/api/ipcSchemas.ts` 与 `src/types/schemas/decision-schema.ts` 可能重复维护

建议（P0/P1）：
- 选定一个“前端 IPC 入口”（推荐以 `src/api/tauri.ts` 为唯一入口，decision-service 作为其内部实现或直接合并）
- 选定一个“Schema Source-of-Truth”（推荐集中在 `src/api/ipcSchemas.ts`，其余位置只 re-export，不再复制）
- 将 `tauri_commands.rs` 按域拆文件（plan/material/config/decision/path_rule/roller…），并在 `mod.rs` 统一注册（已完成，见上方更新）

---

## 7. 前端代码质量评估

### 7.1 维护性热点

- `src/pages/PlanningWorkbench.tsx`（2364 行）：典型“工作台巨石组件”，UI、状态、业务操作、IPC 调用混杂
- `src/components/PlanManagement.tsx`（938 行）：计划管理仍偏大
- any 使用热点：`src/api/tauri.ts`、`src/pages/PlanningWorkbench.tsx`

建议（P1）：
- 继续沿用仓库内的工作台拆分指南（`WORKBENCH_REFACTOR_GUIDE.md`）推进：
  - 抽 hooks（数据加载/提交/缓存失效）
  - 抽纯组件（表格/弹窗/筛选器/工具条）
  - 将“领域操作”集中到 `src/api/tauri.ts`，组件只关心“调用与状态”

### 7.2 IPC 一致性与错误处理

- 建议避免在业务代码中直接 `invoke`：例如 `src/utils/telemetry.ts` 目前直接调用（第 57/76 行），可改为复用 `IpcClient` 的错误处理与 JSON 解析（保留 best-effort 语义即可）。

### 7.3 测试状态

- 前端单测已通过：`npm run test -- --run`（2026-02-02）
- 已修复一个明确的行为问题：非 UUID 的 `version_id` 不应被截断（`src/components/comparison/utils.ts:40`）

---

## 8. 风险清单与改进优先级（可直接转 TODO）

### P0（高优先级：一致性/数据安全/可升级）

1) 统一 DB Connection 与 PRAGMA 初始化（foreign_keys、busy_timeout、journal_mode 等）
2) 统一迁移策略：明确 schema.sql vs migrations 的权威来源，并建立 schema_version 校验机制
3) 收敛前端 IPC 与 Schema：单一入口、单一 schema 源，杜绝重复维护

### P1（中优先级：可维护性/可测试性）

1) 拆分巨型文件：`plan_api.rs`、`tauri_commands.rs`、`refresh_service.rs`、`PlanningWorkbench.tsx`
   - 更新（2026-02-03）：`plan_api.rs`、`tauri_commands.rs` 已完成按域拆分；后续优先 `refresh_service.rs`、`PlanningWorkbench.tsx`。
2) 分批清理 unwrap/expect（按热点文件逐步替换为可解释错误）
3) 分批减少 TS any：优先 `src/api/tauri.ts`，用 zod inference + 泛型约束替代

### P2（低优先级：工程化提升/性能）

1) 引入契约测试（前后端 schema 对齐），避免 IPC 漂移
2) 增加“关键路径”性能基准（重算耗时、决策刷新耗时、工作台加载耗时）
3) 建立统一 lint/format 规则与质量门禁（clippy、eslint、prettier、sqlfmt 等按需）

---

## 9. 建议路线图（可执行）

**第 1 周（稳定性优先）**
- DB 连接初始化统一化（抽 `DbContext` + 全面替换散落的 `Connection::open`）
- 迁移策略定稿（输出一份“生产升级手册”+ schema_version 方案）
- 前端 IPC 入口与 schema 收敛方案定稿（不一定立刻全量迁移，但要锁定方向）

**第 2-3 周（维护性优先）**
- 拆 `tauri_commands.rs`、拆 `PlanningWorkbench.tsx`（先做结构拆分，不做业务改动）
- 为拆分后的模块补最低限度单测/契约测试

**第 4 周+（工程化与性能）**
- unwrap/any 清理进入常态化（每周固定配额）
- 引入基准与门禁，降低回归成本

---

## 附录 A：本次扫描用到的命令（摘录）

```bash
rg --files src | wc -l
rg --files src -g'*.rs' | wc -l
rg --files src -g'*.ts' -g'*.tsx' | wc -l
rg -n "TODO|FIXME|HACK" src docs | wc -l
rg -n "\\bunwrap\\(|\\bexpect\\(" src -g'*.rs' --glob '!src/tests/**' --glob '!src/bin/**' | wc -l
rg -n ":\\s*any\\b|as any\\b|Promise<any>" src | wc -l
rg --files src | tr '\\n' '\\0' | xargs -0 wc -l | sort -nr | head -20
npm run test -- --run
```

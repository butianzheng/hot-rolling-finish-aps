# 热轧精整排产系统：数据同步管理评估报告 & 修改建议方案

日期：2026-02-01  
范围：风险概览、计划工作台、版本对比、设置中心（含相关后端链路与数据表）

## 0. 结论摘要（给测试/产品/开发的共识）

当前项目的数据一致性问题并不是“某一个页面取数错了”，而是**同一业务指标在不同页面/链路下，依赖了不同的数据源（capacity_pool / plan_item / risk_snapshot / decision_* / material_state），且刷新触发与前端缓存失效不统一**，导致出现：

- 堵塞矩阵可出现“利用率很高但已排/待排为 0、原因为空”的组合（典型：H034 @ 2026-01-31）。
- 删除非激活版本时出现 `FOREIGN KEY constraint failed`（依赖表未清理/未解绑或数据库版本/约束与代码假设不一致）。
- 堵塞矩阵偶发“响应数据验证失败: get_machine_bottleneck_profile”（前端 Zod 契约与后端响应在某些数据形态下不一致；项目内存在两套 IPC schema，增加漂移概率）。
- 跨页面联动（风险概览/工作台/版本对比/设置中心）在“版本切换、导入、物料状态改动、产能池参数改动、配置改动”等操作后，**缺少统一的后端刷新触发 + 前端缓存/本地状态失效策略**。

这类问题属于**体系性同步管理缺陷**。建议按本文的 P0/P1/P2 分级修复：先保证“数据源一致 + 刷新必达”，再做“版本化/重构/统一契约”。

---

## 1. 端到端运行路径总览（从操作到页面数据刷新）

### 1.1 数据分层（建议统一口径）

- **主数据（Master）**：`material_master`、`machine_master`、`config_kv` 等  
  用途：真实物料属性、机组能力参数、系统配置。
- **计划数据（Plan / Versioned）**：`plan_version`、`plan_item`（按 `version_id` 版本化）  
  用途：某版本的排程结果。
- **派生数据（Derived / Read Models）**
  - `capacity_pool`：当前实现为**全局表**（PK = `(machine_code, plan_date)`，无 `version_id`），同时存“参数(target/limit)”与“派生(used/overflow)”。
  - `risk_snapshot`：**版本化**（PK = `(version_id, machine_code, snapshot_date)`），用于版本对比 KPI 风险汇总。
  - `decision_*`：**版本化决策读模型**（D1~D6），由刷新服务写入。

### 1.2 后端刷新链路（读模型刷新）

后端存在“引擎事件 → 决策刷新队列 → 同步执行刷新”的链路：

1. 业务 API / Engine 发布 `ScheduleEvent`（例如：`PlanItemChanged`、`ManualTrigger`）。  
2. `RefreshQueueAdapter` 把事件写入 `decision_refresh_queue`，并在当前实现中**同步执行** `process_all()`，最终调用 `DecisionRefreshService.refresh_all()` 刷新 `decision_*` 表。
3. 前端通过 `get_refresh_status` 轮询/订阅刷新完成，并在完成后 `invalidateQueries(decisionQueryKeys.all)` 触发重新取数。

关键点：**只要事件没发布（或发布的 trigger 不覆盖该类变化），decision_* 就不会刷新；前端即使 refetch，也只能拿到旧读模型。**

### 1.3 前端缓存与事件联动（现状）

前端同时存在两种“同步机制”：

- React Query 缓存（决策查询、KPI 等）。  
  - 版本切换时：`invalidateQueries(decisionQueryKeys.all)`（只覆盖决策与 globalKpi）。
  - 决策刷新完成时：再次失效 `decisionQueryKeys.all` + `['globalKpi', versionId]`。
- Tauri 前端事件总线（`plan_updated` / `risk_snapshot_updated` / `material_state_changed` 等）。  
  现状：事件订阅分散在少量组件中，主要刷新 **globalKpi**、部分组件局部刷新；工作台的不同视图刷新策略不一致。

---

## 2. 页面级数据流与同步管理盘点（4 个页面）

### 2.1 风险概览（/overview）

**主要数据源**

- 决策面板数据：来自 `src/services/decision-service.ts`（带 Zod 严格校验 + camel/snake 转换）  
  - D1 `decision_day_summary`（risk 日历/风险热力概览）
  - D4 `decision_machine_bottleneck`（堵塞矩阵）
  - D2/D3/D5/D6（订单失败、冷料压库、换辊、机会）
- 顶部 KPI：`useGlobalKPI` 混用两条链路：
  - `dashboardApi.getMostRiskyDate/getColdStockMaterials`（走 `src/api/tauri.ts` + `src/api/ipcSchemas.ts`）
  - `decisionService.getAllRollCampaignAlerts`（走 `decision-service.ts` + `types/schemas`）
  - `materialApi.listMaterialsByUrgentLevel`（走 `tauri.ts`）

**同步触发（现状）**

- `DecisionRefreshStatus` 轮询到刷新完成 → 失效 `decisionQueryKeys.all` + `globalKpi`。
- App 级事件：`plan_updated` / `risk_snapshot_updated` / `material_state_changed` 只会触发 **globalKpi refetch**，并不会直接失效/刷新决策查询。

**风险点**

- “KPI 与决策面板”来自两套 IPC schema 与两条 API 封装路径，容易出现**同一指标不同口径/不同校验规则**。
- 依赖 `decision_*` 的部分（尤其 D4）若后端未刷新，会长时间停留旧数据（staleTime 5min），导致“看起来没联动”。

### 2.2 计划工作台（/workbench）

**主要数据源**

- 物料池：`materialApi.listMaterials({ limit: 1000, offset: 0 })`，queryKey 固定为 `['materials']`（无版本/筛选维度）。  
- 排程明细：`planApi.listPlanItems(activeVersionId)`，queryKey 为 `['planItems', activeVersionId]`。
- 产能时间线：通过 `capacityApi.getCapacityPools(...)`（但后端忽略 version_id）。

**同步触发（现状）**

- 页面内很多操作后会手动 `refetch()`（例如锁定/紧急/强放/移动等）。
- **事件订阅不一致**：矩阵/可视化中的 `PlanItemVisualization` 子模块订阅 `plan_updated` 会重载，但甘特/其他视图依赖父级 query，不一定联动。

**风险点**

- `limit=1000` + 无分页：测试数据增大后，“工作台显示的物料子集”与“后端聚合/决策读模型统计”可能出现肉眼不一致。
- `capacity_pool` 无版本：工作台的产能视角与版本数据天然存在“跨版本污染”风险（详见第 3 节）。

### 2.3 版本对比（/comparison）

**主要数据源**

- 历史版本对比：`PlanManagement` 组件使用 `planApi`（tauri.ts）拉取版本列表、对比结果、对比 KPI。
- KPI 对比：后端聚合接口 `compare_versions_kpi` **强依赖** `risk_snapshot`（没有就返回 null 指标并提示）。

**风险点**

- 当前系统的 Recalc Engine/流程中 **risk_snapshot 基本未生成/未维护**（详见第 3.2 节），因此版本对比 KPI 的风险相关字段经常为空。  
  这会造成用户感知上的“风险概览有数，但版本对比风险 KPI 为 null”。

### 2.4 设置中心（/settings）

**主要数据源**

- 系统配置：`configApi.listConfigs/updateConfig/...`（无统一事件/刷新触发）。
- 机组配置（产能池）：`capacityApi.getCapacityPools/updateCapacityPool/batchUpdateCapacityPools`  
  - 前端会传 `activeVersionId`，但 `get_capacity_pools` 后端参数 `_version_id` 未使用。
  - 更新产能池时若传 `version_id`，后端会 best-effort 触发 `manual_refresh_decision`（有一定补救价值）。
- 物料管理：订阅了 `material_state_changed`/`plan_updated`，属于少数有事件联动的页面。

**风险点**

- 配置修改不会自动触发决策刷新或相关页面缓存失效：配置变化对“重算逻辑/决策口径/阈值”等有影响，但系统没有统一的“配置变更 → 刷新”路径。
- 机组选项来源于 `listMaterials(limit=1000)`，数据量大时可能漏掉机组。

---

## 3. 关键问题根因分析（对应你反馈的现象）

### 3.1 堵塞矩阵出现“利用率114.7%但已排/待排为0、原因空”

现象示例（测试日：2026-02-01，页面默认取“昨天起未来 N 天”）：  
H034 @ 2026-01-31 显示“极度堵塞（利用率114.7%）”，但已排材料数/待排材料数为 0，原因栏为空。

**高概率根因（可由代码直接解释）**

1) `capacity_pool` **非版本化**，且同时存“参数+派生(used/overflow)”  
`capacity_pool` 主键仅 `(machine_code, plan_date)`，没有 `version_id`。不同版本/不同操作会互相覆盖 `used_capacity_t`。  
这会导致：版本 A 的 `plan_item` 已经变了，但 `capacity_pool.used_capacity_t` 可能还是版本 B 的残留值。  

2) 版本激活时的产能池同步只“更新有排程的 key”，不会把其它日期清零  
`recalculate_capacity_pool_for_version()` 只遍历该版本 `plan_item` 聚合出来的 (machine,date) 组合做 upsert；**没有覆盖到的 (machine,date) 仍保留旧 used_capacity_t**。  
因此会出现：某天在版本 A 没有任何 `plan_item`，但 `capacity_pool` 还留着之前版本的 used 值 → “利用率高但已排为 0”。  

3) D4 读模型刷新依赖 `capacity_pool`，并把“利用率”计算口径绑定到 `capacity_pool.used/target`  
D4 刷新写入 `decision_machine_bottleneck` 时，从 `capacity_pool` 直接取 used/target 作为 `capacity_utilization`。  
只要 capacity_pool 没同步到位，就会把错误的“利用率”固化到决策读模型中。

**为什么“原因栏会空”**

原因项来自读模型列 `decision_machine_bottleneck.reasons`（JSON），后端需要 parse 后再转 DTO。  
当 reasons JSON 无法解析/字段缺失（例如 severity 为空或 JSON 格式不符合预期）时，后端会返回空 reasons 数组，前端就显示为空。  
（建议按 3.3 的“契约校验失败定位方法”抓取 rawResponse 与 zodError，能一次性确定是哪个字段导致原因丢失。）

### 3.2 版本对比 KPI 风险字段为 null / 与风险概览不一致

根因是 **risk_snapshot 缺失/未维护**：

- VersionComparison KPI 聚合使用 `risk_snapshot`，若为空则风险 KPI 返回空并提示“部分版本缺少 risk_snapshot”。  
- Recalc Engine 目前明确写了“试算不写 risk_snapshot”，且生产模式也没有实际写入 risk_snapshot 的实现（TODO 处于未落地状态）。  
- 决策 D1 虽然能在 `risk_snapshot` 缺失时 fallback 到 `capacity_pool` 兜底聚合，因此风险概览仍可能有“风险日/风险分数”。

结果：同一个版本在风险概览可看到风险相关信息，但版本对比 KPI 风险部分为 null。

### 3.3 “响应数据验证失败: get_machine_bottleneck_profile”（堵塞矩阵加载失败）

这是前端 `decision-service.ts` 的 Zod 校验在运行期失败导致。该模块在失败时会输出：

- `rawResponse`（后端返回原始对象）
- `zodError`（具体哪个字段类型/范围/可空性不匹配）

**为什么项目更容易出现这种失败（结构性原因）**

- 项目内存在两套 IPC schema：
  - `src/services/decision-service.ts` + `src/types/schemas/decision-schema.ts`（更严格）
  - `src/api/tauri.ts` + `src/api/ipcSchemas.ts`（大量 `.passthrough()`，且部分字段允许 `.nullable()`）
- 同一条后端命令可能在不同页面走不同 schema，导致：
  - A 页面能过校验、B 页面失败
  - 同一命令的“可空字段约束”不一致（例如 optional vs nullable）

**建议的定位步骤（无需改代码）**

1. 打开前端控制台（或 tauri-dev log），触发一次失败。  
2. 找到 `[DecisionService] Validation failed for get_machine_bottleneck_profile:` 日志块。  
3. 把其中的 `rawResponse` 与 `zodError` 保存下来（这是定位唯一必要的信息）。  
4. 对照 `MachineBottleneckProfileResponseSchema`（decision-schema 与 ipcSchemas 各一份）确认差异点。  

### 3.4 删除非激活版本时报外键约束失败（FOREIGN KEY constraint failed）

代码层面 `delete_version` 已尝试显式删除/解绑多个关联表，但仍可能因为以下原因触发 FK 失败：

1) **数据库 schema 与代码假设不一致**（测试库可能是旧 schema / 手工改过 / migrations 未完全应用），导致：
   - 某些引用表没有 ON DELETE CASCADE
   - 或新增了引用表但 `delete_version` 未覆盖
2) 删除过程不在单事务中，若中途失败，可能留下部分数据，二次删除更容易 FK 失败。
3) `action_log.version_id` 外键为 “引用但不级联”，删除前必须置空（代码已有 detach，但若 DB 列名/表结构不一致会失败）。

**建议的快速判定方法（无需改代码）**

- 对出问题的库执行 `PRAGMA foreign_key_check;`，它会列出具体是哪张表、哪条记录违反约束。  
  该结果可直接指向需要补充清理/解绑的表。

---

## 4. 同步触发矩阵（操作 → 写库 → 后端刷新 → 前端联动）

> 目的：明确“为什么某些页面不联动”，以及“要补哪一个触发点”。

说明：下表中“后端刷新”指是否发布 `ScheduleEvent` 并触发 `DecisionRefreshService` 刷新 `decision_*`。

| 操作场景 | 主要写入表 | 后端 ScheduleEvent | 前端事件 | 前端缓存失效/刷新（现状） | 风险/缺口 |
|---|---|---|---|---|---|
| 导入材料 `import_materials` | material_master / material_state / import_batch | 否 | material_state_changed | App 仅 refetch globalKpi；决策查询不一定失效 | D2/D3/D4 等依赖 material_state 的读模型不会自动刷新 |
| 物料锁定/解锁/紧急/强放 | material_state + action_log | 否 | material_state_changed | 工作台内会手动 refetch；风险概览/决策读模型不刷新 | 决策读模型长期停留旧值 |
| 一键重算（生产） | plan_version / plan_item / capacity_pool / material_state（及日志） | 是（RecalcEngine 发布 PlanItemChanged） | plan_updated + risk_snapshot_updated | 刷新完成后决策查询会失效；globalKpi 也会失效 | 若 capacity_pool 同步不全，仍可能出现残留利用率 |
| 版本激活/回滚 | plan_version 状态 +（激活时会同步部分 capacity_pool.used） | 是（ManualTrigger） | plan_updated + risk_snapshot_updated | 决策查询会在刷新完成后失效；globalKpi refetch | capacity_pool 同步只覆盖有排程 key，易残留 |
| 移动排产项 move_items | plan_item + action_log | 是（PlanItemChanged） | （通常会 emit plan_updated） | 决策查询会失效；部分工作台视图订阅 plan_updated | 工作台不同视图刷新策略不一致 |
| 修改产能池参数 | capacity_pool + action_log | best-effort（manual_refresh_decision） | risk_snapshot_updated | 触发决策刷新后会失效决策查询 | `capacity_pool` 非版本化，参数与派生混在一起导致跨版本污染 |
| 修改系统配置 update_config | config_kv + action_log（如有） | 否 | 无 | 仅设置中心本页 reload | 配置变更不会驱动“决策刷新/页面重算口径更新” |
| 删除非激活版本 delete_version | plan_version +（尝试清理关联表） | 否（删除不需要） | 无 | 仅本页刷新 | schema 漂移/遗漏引用表会导致 FK 失败 |

---

## 5. 修改建议方案（不改代码的前提下给出可落地的改法）

### 5.1 P0（必须先修，否则测试会持续遇到“看起来不一致”）

**P0-1：修复 material_state 的 INSERT OR REPLACE 覆盖问题（会破坏决策刷新依赖字段）**

- 问题：当前 `material_state` 的写入使用 `INSERT OR REPLACE`，只写入部分列；SQLite 的 REPLACE 语义是 delete+insert，未写到的列会被置空/默认值。  
  而决策刷新 SQL（D2/D3/D6 等）依赖 `material_state` 中的“反范式列”（如 contract_no、due_date、weight_t、is_mature、machine_code 等）。  
- 影响：一旦发生锁定/强放/紧急等操作，可能把这些列清空，导致 D2/D3 统计突然异常或为 0。  
- 建议改法（优先级从高到低）：
  1) 把写入改为 **UPDATE**（或 SQLite UPSERT 的 DO UPDATE），只更新状态相关列，保留其它列；  
  2) 或者彻底移除决策刷新对 material_state 反范式列的依赖：刷新 SQL 改为 join `material_master` 获取 contract_no/weight 等（更根治）。

**P0-2：补齐“物料/导入/配置变更”的后端 ScheduleEvent 发布**

- 目标：只要改变了决策读模型依赖的数据源，就必须发布对应 trigger（至少 `MaterialStateChanged` / `ManualTrigger`）。  
- 建议：
  - 物料 API（锁定/强放/紧急/导入）在成功写库后发布 `ScheduleEventType::MaterialStateChanged`；  
  - 配置更新后发布 `ManualTrigger`（配置影响口径广，建议全量刷新或按需刷新）。  

**P0-3：修复 capacity_pool.used 的残留（导致“利用率高但已排为 0”）**

- 最小改法：版本激活/切换时，同步产能池不能只更新“有排程 key”，还要对窗口内其它 key 做清零/重算。  
- 更合理的长期改法见 P1-1（capacity_pool 版本化或把 used 从表中剥离）。

### 5.2 P1（中期修复：提升一致性与版本能力）

**P1-1：解决 capacity_pool 非版本化导致的跨版本污染**

可选路径：

1) **capacity_pool 版本化**：给 capacity_pool 增加 `version_id` 作为主键的一部分，并把 used/overflow 变为 version 维度；  
2) **拆表**：capacity_pool 只保留 target/limit（参数）；新增 versioned 的 `capacity_usage(version_id, machine_code, plan_date, used, overflow, ...)`；  
3) **不落库 used/overflow**：直接由 `plan_item` 聚合实时计算（最一致，但需评估性能）。

**P1-2：落地 risk_snapshot 的生成与维护，或统一替换版本对比 KPI 的风险数据源**

- 若继续使用 `risk_snapshot`：需要在“重算/发布版本/激活”链路中生成 snapshot，并明确刷新时机。  
- 若不使用 `risk_snapshot`：版本对比 KPI 应改为使用 `decision_day_summary` 或直接聚合 `plan_item + capacity_pool(target/limit)`，避免空值。

### 5.3 P2（治理/重构：降低以后出“验证失败/口径漂移”的概率）

**P2-1：统一 IPC 调用与 Schema（消除双轨制）**

- 建议收敛为一条主链路：  
  - 要么全面使用 `decision-service.ts`（严格、统一转换与错误处理）  
  - 要么把 `tauri.ts + ipcSchemas.ts` 也统一到同一份 schema/类型源  
- 目标：同一后端命令只有一份 schema（避免 A 过 B 不过）。

**P2-2：规范 queryKey 与分页策略（避免 1000 上限导致“肉眼不一致”）**

- `['materials']` 应至少包含筛选条件/分页信息；否则多个页面/条件会互相污染缓存。  
- 去掉硬编码 limit=1000，改为分页/滚动加载；同时在聚合指标处明确口径（全量 vs 当前页）。

**P2-3：观测性与回归保障**

- 在后端决策刷新失败时，把失败原因与“影响表/影响范围”写入 action_log/refresh_log。  
- 增加契约回归测试：对关键 IPC 命令（get_machine_bottleneck_profile 等）做 JSON contract snapshot 测试，避免改动后前端 Zod 直接报错。

---

## 6. 测试数据建议（你提出的“1000+ 全场景数据”）

项目已包含一个可用的“重置并生成全场景数据”的工具入口（Rust bin）：

- `src/bin/reset_and_seed_full_scenario_db.rs`  
  - 默认生成 2000 条物料（最小 1000）  
  - 会备份旧 db 为 `.bak.YYYYMMDD_HHMMSS` 后重建 schema 并 seed  
  - 会刷新 D2/D3/D5/D6 决策读模型，便于风险概览直接有数

建议把它作为测试基线库，后续再追加“导入冲突/版本多轮变更/产能调整/配置切换”等脚本化场景。

---

## 7. 验收建议（修复后如何确认“同步管理变好了”）

建议按“操作 → 预期联动页面”做验收回归：

1) 物料锁定/强放/紧急  
   - 预期：风险概览（D2/D3/D4）、工作台物料池、设置中心物料管理应在决策刷新完成后自动一致。  
2) 版本切换/回滚/重算  
   - 预期：D4 不再出现“利用率高但已排为 0”；工作台各视图在 `plan_updated` 后一致刷新。  
3) 配置修改  
   - 预期：配置影响到的口径（阈值/策略）在触发刷新/重算后能在风险概览和版本对比体现一致变化。  
4) 删除非激活版本  
   - 预期：不再出现 FK 失败；若仍失败，错误信息能指出具体引用表与记录（便于修复）。

---

## 8. 关键代码与数据点索引（便于开发定位）

> 只列“最影响一致性”的点。

- capacity_pool 非版本化：`scripts/dev_db/schema.sql`（capacity_pool 定义，PK 无 version_id）  
- 激活版本时只更新部分 capacity_pool：`src/api/plan_api.rs`（`recalculate_capacity_pool_for_version`）  
- D4 刷新从 capacity_pool 计算并写入读模型：`src/decision/services/refresh_service.rs`（`refresh_d4`）  
- material_state 使用 INSERT OR REPLACE 且只写部分列：`src/repository/material_repo.rs`（`batch_insert_material_state`）  
- material API 操作未发布 ScheduleEvent：`src/api/material_api.rs`（batch_lock / force_release / set_urgent 等）  
- 配置更新无刷新触发：`src/app/tauri_commands.rs`（`update_config` / `batch_update_configs`）  
- 风险快照缺失导致版本对比 KPI 风险为空：`src/engine/recalc.rs`（risk_snapshot TODO）+ `src/api/plan_api.rs`（`compare_versions_kpi` 依赖 risk_snapshot）  
- 前端两套 schema：`src/services/decision-service.ts` vs `src/api/tauri.ts` + `src/api/ipcSchemas.ts`


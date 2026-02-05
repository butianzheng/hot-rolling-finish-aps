# Workbench 大数据量优化 TODO（30k~50k+）

## 背景
- 目标数据量：开发全场景 30,000+（已支持），生产预计 50,000+
- 主要问题：
  - Workbench 初始加载/切换机组时拉取全量 `plan_item`、`material_*` → IPC 传输与前端渲染压力大，易触发 `Timeout`
  - 缺少可观测性：无法快速定位“慢 SQL / N+1 / 哪个 IPC 最慢”
  - 旧数据库 schema 版本过低导致运行期 `no such column`（如 `material_state.user_confirmed`）

## 已完成（本轮）
### P1：按范围/分页（关键链路）
- [x] 后端：`plan_item` 日期边界查询（用于 AUTO 范围，无需全量拉取）
  - IPC：`get_plan_item_date_bounds`
- [x] 后端：`material_master` + `material_state` JOIN 一次取列表，支持过滤/分页（根治 N+1）
  - IPC：`list_materials`（新增 `sched_state/urgent_level/lock_status/query_text` 可选过滤）
- [x] 后端：物料池树汇总（机组 × 状态）
  - IPC：`get_material_pool_summary`
- [x] 前端：Workbench `planItems` 按日期范围拉取 + 内部分页（5k/页聚合），避免一次性超大 JSON
- [x] 前端：Workbench 物料池树使用后端汇总（不再依赖“当前页 materials”推导数量）
- [x] 前端：Workbench materials 使用 `useInfiniteQuery`（分页加载 + “加载更多”按钮）
- [x] 前端：矩阵视图（PlanItemVisualization）改为按范围/分页拉取（跟随工作台默认范围），避免全量

### P1：可观测性（elapsed_ms / SQL 计数 / 慢查询）
- [x] 后端：`PerfGuard`（`elapsed_ms`、`sql_count`、`slow_sql_count`）
- [x] 后端：SQLite tracing/profile（慢 SQL 日志 target=`slow_sql`）
- [x] 关键 IPC 已加 `PerfGuard`：
  - `list_materials`、`get_material_pool_summary`
  - `get_plan_item_date_bounds`、`list_plan_items`、`list_items_by_date`
  - `get_capacity_pools`
  - `get_refresh_status`、`manual_refresh_decision`

### DB：重置/全场景数据
- [x] `scripts/dev_db/reset_and_seed.sh` 支持一键重置并生成全场景数据（支持参数 material_count）
- [x] 已实测重置 dev DB：`material_master/material_state` 30,000 行，`plan_item` 46,890 行

## 使用说明（开发）
### 重置并生成全场景 30k 数据
```bash
bash scripts/dev_db/reset_and_seed.sh "$HOME/Library/Application Support/hot-rolling-aps-dev/hot_rolling_aps.db" 30000
```

### 打开性能日志/慢 SQL
```bash
export HOT_ROLLING_APS_PERF_SQL=1
export HOT_ROLLING_APS_SLOW_SQL_MS=50
```

## 待办（按优先级）
### P0：业务逻辑正确性（2 个）
- [ ] 单日排产量超过“极限产能”时，超出部分应自动顺延到后续日期（避免违背物理/业务规律）
  - 需梳理：产能约束口径（目标/极限/溢出比例）、冻结区材料是否占用产能池
  - 建议：引擎侧强约束 + UI 侧明确标记“超限”并提供一键修复（后移）
- [ ] 适温时间随排产日期动态变化的核实与修复
  - 示例：产出 0 天 + 冬季室温 3 天 → 第 3 天后应进入 READY（而非静态一次性计算）
  - 建议：把“就绪判定”从静态字段升级为“随时间推演”的派生视图/计算函数（减少手工同步错误）

### P1：海量数据性能（补强）
- [ ] Materials：增加 `count_materials` IPC（与过滤口径一致）用于分页总数/进度展示
- [ ] Materials：滚动触底自动 `fetchNextPage`（替代按钮，避免频繁点击）
- [ ] PlanItems：考虑提供“按机组×日期聚合”的轻量接口（甘特图先展示汇总，点开单元格再拉明细）
- [ ] Strategy Draft/发布链路：对 `Timeout` 进行链路分解（拆分长任务、增加进度上报、避免单次 IPC 30s 卡死）

### P2：UI/交互优化（6 个）
- [ ] 产能池管理支持批量调整（目标/极限常用模板、按机组复制、批量填充、导入导出）
- [ ] 所有前端重量数字统一四舍五入保留小数点后两位（含表格/图表/提示）
- [x] 排程视图矩阵物料清单显示厚度、宽度等完整信息（后端已补齐 plan_item 快照字段；前端需确认全部列已接入）
- [ ] “重算窗口天数”的影响说明 + 作为一键优化/重算的可选参数（并与默认策略参数联动）
- [ ] 策略配置画面：预设策略参数“只读展示”，便于自定义策略参考
- [ ] 文案统一：所有“冷坨消化”改为“冷料消化”

## 风险与验证清单
- 性能：以 30k/50k 数据库分别验证 Workbench 首屏加载、切换机组、打开草案对比、发布方案的 IPC 延迟与慢 SQL 输出
- 正确性：重点回归“冻结区保护/强制放行/紧急等级/适温约束/产能池超限”相关红线


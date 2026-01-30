# 代码评估修复计划与 TODO（持续更新）

依据文档：
- `docs/REVIEW_SUMMARY.md`
- `docs/CODE_REVIEW_REPORT.md`
- `docs/IMPLEMENTATION_GUIDE.md`

目标：把评估报告中提出的 P0/P1/P2 风险修复落到可执行的任务清单，并在每次修复完成后同步更新状态。

状态约定：`[x] 已完成` / `[ ] 待办` / `[~] 进行中` / `[?] 待确认`

最后更新：2026-01-29

---

## 当前现状对照（基于现有代码）

以下条目在当前代码中**已部分/全部落地**，后续计划将以“补齐缺口/升级为可维护版本”为主：

- [x] 草案生成/发布已改为落库持久化（`decision_strategy_draft`）；待补：草案列表/恢复入口 + 过期清理
- [x] 排产“移动项”已支持批量（`move_items(moves: Vec<...>)`）
- [x] 工作台物料池已使用虚拟列表（`react-window`），大列表卡顿风险已显著下降
- [x] 前端错误上报/遥测已接入（写入 `action_log`，便于排查）

---

## 修复计划（按优先级）

### P0（必须：阻塞/高风险）

#### P0-1 策略草案持久化（替换内存草案存储）

目的：解决“重启丢失/刷新丢失/多人并发冲突”，让草案具备可追溯生命周期（DRAFT/PUBLISHED/EXPIRED）。

交付物：
- 新增表：`decision_strategy_draft`（不改现有核心表；如需与 action_log 关联，优先放在 payload_json，避免硬改表结构）
- 新增仓储：StrategyDraftRepository（CRUD + 过期清理 + 软锁）
- 改造 API：
  - `generate_strategy_drafts`：生成后落库（并支持 `custom:xxx` 策略键）
  - `get_strategy_draft_detail`：从库中读取（支持截断/总数）
  - `apply_strategy_draft`：读取草案 profile 发布，发布后标记草案状态
  - （可选）`list_strategy_drafts`：支持按 base_version/status/created_by 查询
  - （可选）`cleanup_expired_drafts`：清理过期草案（启动时 best-effort）
- 测试：Repo/Api/E2E 覆盖草案生成→查询→发布→过期/清理的核心链路

验收标准：
- 草案生成后重启应用仍可查询/发布
- 发布结果可复现（草案内固化 `strategy_key/base_strategy/params_json`）
- 过期草案不可发布；可被清理；状态可解释

风险/注意：
- 设计要兼容当前已上线的草案 DTO（前端已使用 `strategy: string`）
- 冻结区/适温/产能硬约束不受草案参数影响（仅等级内排序参数化）

#### P0-2 决策数据刷新通知（刷新状态可见）

目的：解决“导入/重算后决策读模型数据延迟，前端无法确认何时刷新完成”的体验问题。

交付物：
- 后端新增 API：`get_refresh_status(version_id)`（聚合 decision_refresh_queue / decision_refresh_log）
- 前端新增 Hook：`useDecisionRefreshStatus(versionId)`（按需轮询；离线/后台降频）
- UI 指示：Header/页面级提示“决策刷新中/已完成/失败可重试”（与现有事件 `plan_updated` 联动）
- 测试：刷新入队→状态变化→UI 展示的集成验证（至少后端 API + 关键场景）

验收标准：
- 触发导入/重算/移单后，用户能看到“刷新进行中”，完成后自动消失/提示完成
- 刷新失败可见（含错误信息），且不影响主流程（best-effort）

---

### P1（重要：质量/可维护/性能）

#### P1-1 版本对比 KPI 聚合 API（减少前端全量拉取与本地计算）

目的：为“历史版本对比”提供稳定、可复用的 KPI 汇总接口，降低前端计算/网络压力。

交付物：
- 后端新增 `compare_versions_kpi(version_a, version_b, date_range?)`（或扩展现有 `compare_versions`）
- 指标建议（优先“无需额外查库”的口径）：
  - plan_items_count / total_weight_t
  - mature/immature（如可在重排计算中得出则直出，否则定义可解释口径）
  - overflow_days / overflow_t（基于 capacity_pool 或聚合 used/limit）
  - moved/added/removed/squeezed_out（已有，可复用）
- 前端：PlanManagement/VersionComparison 使用该 API 展示 KPI 总览；明细仍可按需加载
- 测试：对比结果稳定性（相同输入输出一致）

验收标准：
- 历史对比页 KPI 不再依赖全量 plan_item 拉取即可展示“对比总览”
- 超大版本数据下页面首屏明显更快（可通过简单计时/日志验证）

#### P1-2 版本回滚 API（审计可追溯）

目的：补齐“回滚到历史版本”的决策闭环能力（评估报告列为缺失项）。

交付物：
- 后端新增 `rollback_version(plan_id, target_version_id, operator, reason?)`
  - 规则：仅允许回滚到同 plan 的历史版本；写 action_log；发布刷新事件
- 前端：在历史版本对比页提供“回滚/激活”入口（含二次确认与风险提示）
- 测试：回滚后 active_version 变化、读模型刷新、审计记录齐全

---

### P2（增强：体验/性能/工程化）

#### P2-1 批量产能池更新（评估报告项，当前待确认是否必要）

- [x] 已实现 `batch_update_capacity_pools`：支持表格多选批量设置 target/limit（留空保持原值）+ 审计日志 +（可选）触发决策刷新

#### P2-2 性能与工程化清单（按收益择机推进）

- [x] Vite 分包/缓存优化：补 `vite.config.ts` 的 `manualChunks` + 调整 `chunkSizeWarningLimit`（解决 vendor chunk 过大告警）
- [x] 关键列表分页/增量加载：`list_plan_items` / `get_recent_actions` 提供分页/时间窗参数（并保持旧调用兼容）
- [x] Zod 契约校验：为关键 IPC 响应补 schema（优先：草案/对比/刷新状态；校验失败走统一错误弹窗/遥测）

---

## TODO List（执行追踪）

### P0-1 草案持久化
- [x] 设计 `decision_strategy_draft` 表结构 + 索引（兼容 `custom:...`）
- [x] migrations + `scripts/dev_db/schema.sql` 同步
- [x] 新增 `StrategyDraftRepository`（CRUD/过期清理/锁）
- [x] PlanApi：草案生成/查询/发布改为落库（替换 OnceLock）
- [x] Tauri commands：补 `list_strategy_drafts` / `cleanup_expired_strategy_drafts`
- [x] 前端：草案对比页支持“可恢复草案”（刷新/重启后自动拉取同范围最新草案）
- [x] 测试：生成→查询→发布→过期/清理（Repo+E2E）

### P0-2 刷新状态通知
- [x] 后端：实现 `get_refresh_status(version_id)`（聚合 decision_refresh_queue / decision_refresh_log）
- [x] 前端：`useDecisionRefreshStatus` + 轮询策略（离线/后台降频）
- [x] UI：Header 刷新提示（刷新中/失败可重试/刚完成）；刷新完成后自动无效化 decision/globalKpi 查询
- [x] 测试：关键路径覆盖（`tests/decision_refresh_status_test.rs`）

### P1-1 对比 KPI 聚合
- [x] 后端：新增版本对比 KPI 接口 `compare_versions_kpi`（plan_item 聚合 + risk_snapshot 聚合）
- [x] 前端：历史对比页接入 KPI 总览（默认不拉取全量 plan_item；需要时手动“加载明细”）
- [x] 测试：补齐 `compare_versions_kpi` 的最小集成用例 + 通过构建

### P1-2 版本回滚
- [x] 后端：实现 `rollback_version(plan_id, target_version_id, operator, reason)`（可选恢复 config_snapshot_json 到 config_kv）+ ActionLog + 刷新事件
- [x] 前端：历史对比页/版本列表提供“回滚/激活”入口（必填原因 + 二次确认 + 风险提示）
- [x] 测试：补齐回滚链路最小用例（`tests/plan_api_test.rs`）+ 通过构建

### P2-1 批量产能池更新
- [x] 需求确认：按表格勾选多行批量设置；支持“留空保持原值”；暂不做导入模板/撤销
- [x] 后端：新增 `batch_update_capacity_pools`（批量 upsert + ActionLog + 可选触发决策刷新）
- [x] 后端：`update_capacity_pool` 支持可选 `version_id` 并触发决策刷新
- [x] 前端：产能池页支持多选 + 批量调整弹窗 + 传递 operator/version_id
- [x] 校验：`cargo test` + `npm run build` 通过

### P2-2 性能与工程化
- [x] Vite `manualChunks` 分包优化（`npm run build` 不再输出 chunk > 500k 告警）
- [x] 关键查询分页/时间窗（PlanItem/ActionLog 支持增量加载参数；`cargo test` + `npm run build` 通过）
- [x] Zod 契约校验补齐（优先：草案/对比/刷新状态；新增 `src/api/ipcSchemas.ts` 并接入 `IpcClient.validate`；`npm run build` 通过）

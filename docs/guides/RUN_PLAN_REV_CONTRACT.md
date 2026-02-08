# 重算与版本一致性运行规约（run_id / plan_rev）

> 适用范围：`importer / engine / decision / api / workbench / deep link` 全链路。  
> 目标：防止并发重算覆盖、前后端版本错位、导入后仍展示旧计划。

---

## 1. 核心约束（MUST）

1. **每次重算必须带唯一 `run_id`**
   - 前端触发重算前调用 `createRunId(...)`。
   - 调 `planApi.recalcFull(...)` 必须传 `run_id`。

2. **前端只认 latest run 的结果**
   - 触发前先走 `beginLatestRun`（含触发时间比较与 TTL 覆盖策略）。
   - 回包后先 `markLatestRunDone/Failed`，再校验 `latestRun.runId === responseRunId`，不匹配直接丢弃 UI 更新。

3. **所有可展示查询必须绑定 `plan_rev`**
   - decision/workbench 查询必须携带 `expected_plan_rev`。
   - query key 必须包含 `activePlanRev`，确保缓存分区正确。

4. **`STALE_PLAN_REV` 必须集中处理**
   - 禁止在各组件散落处理。
   - 统一走 `handleStalePlanRevError`：
     - toast 节流（4s cooldown）
     - single-flight refresh
     - 自动拉取最新 `PlanContext(version_id, plan_rev)` 并失效相关 queries
     - deep link 场景提示“已切换到最新计划”

5. **`plan_updated` 事件必须安全回填**
   - 有 `run_id`：先 `markLatestRunDone`。
   - 若刷新后丢失 run 跟踪上下文，可在满足以下条件时回填 `PlanContext`：
     - 当前无 tracked run
     - `version_id` 匹配当前版本（或当前为空）
     - `incoming plan_rev >= current plan_rev`（禁止回退）

---

## 2. latest run 状态机与 TTL

- 状态：`IDLE -> PENDING -> RUNNING -> DONE/FAILED`，超时进入 `EXPIRED`。
- TTL 默认：`120s`。
- 覆盖策略：
  - 仅允许更晚触发的 run 覆盖运行中 run。
  - `DONE/FAILED/EXPIRED` 可被新 run 覆盖。
- 前端每 `5s` 调用一次 `expireLatestRunIfNeeded`，避免页面长驻卡在 `RUNNING/PENDING`。

---

## 3. 后端/前端契约要点

### 3.1 `recalc_full`

- 请求：支持 `run_id`。
- 响应：必须可读 `run_id / version_id / plan_rev`。
- 事件：`plan_updated` 需带 `version_id`，建议带 `run_id + plan_rev`。

### 3.2 `STALE_PLAN_REV`

- 错误码：`STALE_PLAN_REV`。
- details：`version_id / expected_plan_rev / actual_plan_rev`。
- 前端拿到后不得静默失败，必须走统一 handler。

---

## 4. 开发检查清单（PR 自检）

- [ ] 新增/修改重算触发点时，已传 `run_id` 且接入 `latestRun` gate。
- [ ] 新增/修改可展示查询时，已传 `expected_plan_rev` 且 query key 含 `activePlanRev`。
- [ ] 未在组件内重复实现 `STALE_PLAN_REV` 处理逻辑。
- [ ] `plan_updated` 消费逻辑未绕过 run gate 与 plan_rev 防回退判断。
- [ ] 至少补 1 条回归测试覆盖新增链路。

---

## 5. 现有回归测试（基线）

- `src/stores/latestRun.test.ts`
- `src/stores/use-global-store.run-gating.test.ts`
- `src/hooks/useDecisionStalePlanRev.test.tsx`
- `src/App.plan-updated-fallback.test.ts`

> 若未来修改 run/plan_rev 协议，以上用例需同步更新。


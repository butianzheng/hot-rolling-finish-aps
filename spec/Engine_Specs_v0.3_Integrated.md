# 排产引擎工程规格书（整合版 v0.3）

整合来源：Engine_Specs_v0.1 + Engine_Specs_v0.2  
技术栈：Tauri + Rust + SQLite  
范围：MVP（精整机组局部优化），合同不独立建表（合同字段为材料影子列）  
依赖文档：Field_Mapping_Spec_v0.1.md、schema_v0.1.sql、data_dictionary_v0.1.md  

---

# 0. 冻结结论（当前版本必须遵循）

## 0.1 季节模式与适温阈值（统一版）

- season_mode = AUTO | MANUAL（并行支持）  
- AUTO：按月份判断季节  
  - 默认冬季：11,12,1,2,3（可配置）  
- MANUAL：人工指定 WINTER / SUMMER，覆盖 AUTO  

适温默认：  
- 冬季：min_temp_days = 3  
- 夏季：min_temp_days = 4  

支持按 全局 / 机组 / 钢种 / 日期 覆写。

---

## 0.2 紧急等级体系（冻结）

紧急等级：  
- L0 正常  
- L1 关注  
- L2 紧急  
- L3 红线  

阈值：  
- N1 ∈ {2,3,5} 天（可配置）  
- N2 ∈ {7,10,14} 天（可配置）  

冻结区规则：  
- 冻结区材料：urgent_level ≥ L2（强制）

---

## 0.3 催料等级（v0.2 新冻结，替代原 rush_flag 直映射）

新增中间变量：rush_level (L0/L1/L2)  
由 material_master 影子字段组合计算：

字段：  
- contract_nature  
- weekly_delivery_flag  
- export_flag  

判定顺序：

1. contract_nature 非空 且 首字母 ∉ {Y,X} 且 weekly_delivery_flag = 'D'  
   → rush_level = L2  

2. contract_nature 非空 且 首字母 ∉ {Y,X} 且 weekly_delivery_flag = 'A' 且 export_flag = '1'  
   → rush_level = L1  

3. 其他 → L0  

说明：  
rush_level 仅作为紧急等级抬升因子，不等于最终 urgent_level。

---

## 0.4 轧制产出时间反推（天口径冻结）

- 若 current_machine_code ∉ {H032,H033,H034}  
  → rolling_output_age_days = output_age_days_raw + 4  

- 否则  
  → rolling_output_age_days = output_age_days_raw  

---

## 0.5 库存压力口径冻结

- stock_age_days = 状态时间（天）

---

# 1. 总体引擎架构与数据流

## 1.1 计算主流程（批量）

1) 导入/更新材料主数据 → 写 material_master  
2) 生成/更新系统状态 → 写 material_state  
3) 生成/更新产能池 → 写 capacity_pool  
4) 生成排产方案版本（草稿/沙盘） → 写 plan、plan_version  
5) 执行排产计算（N 天窗口 + 冻结区保护） → 写 plan_item  
6) 生成换辊窗口 → 写 roller_campaign  
7) 生成风险快照 → 写 risk_snapshot  
8) 记录操作与影响摘要 → 写 action_log  

---

## 1.2 模块拆分

- Eligibility Engine：锁定过滤 + 适温准入  
- Urgency Engine：紧急等级判定（L0–L3）  
- Priority Sorter：等级内次排序（冷料/库存）  
- Capacity Filler：产能池填充（吨位驱动）  
- Structure Corrector：结构软控制与违规标记（MVP 以提示为主）  
- Roll Campaign Engine：换辊窗口计算与硬停止  
- Recalc Engine：一键重算 / 局部重排 / 联动窗口  
- Risk Engine：驾驶舱指标生成（risk_snapshot）  
- Impact Summary Engine：调整影响摘要  

---

# 2. Eligibility Engine（可排池生成器）

## 2.1 输入

- material_master（材料行）  
- material_state（锁定/强制放行等系统字段）  
- 配置：min_temp_days（按季节 + 覆写规则）  

## 2.2 输出

更新 material_state：  
- rolling_output_age_days  
- ready_in_days  
- earliest_sched_date  
- sched_state（PENDING_MATURE / READY / LOCKED / FORCE_RELEASE / BLOCKED）  

## 2.3 规则

1) 锁定优先：lock_flag=1 → sched_state=LOCKED  
2) 强制放行：force_release_flag=1 → sched_state=FORCE_RELEASE  
3) 适温判断：  
   - rolling_output_age_days 按 0.4 规则派生  
   - ready_in_days = max(0, min_temp_days - rolling_output_age_days)  
   - earliest_sched_date = today + ready_in_days  
   - ready_in_days > 0 → PENDING_MATURE，否则 READY  

## 2.4 异常与DQ

- output_age_days_raw 缺失或 <0 → BLOCKED + 风险提示  
- current_machine_code 缺失 → BLOCKED

---

# 3. Urgency Engine（紧急等级判定器）

## 3.1 中间变量

- rush_level（由催料组合规则计算）

## 3.2 最终 urgent_level 判定顺序（冻结版）

1) 人工红线 → L3  
2) 冻结区材料 → 至少 L2  
3) due_date < today → L3  
4) 适温阻断红线：due_date ≤ today+N1 且 earliest_sched_date > due_date → L3  
5) 临近交期：  
   - due_date ≤ today+N1 → L2  
   - due_date ≤ today+N2 → L1  
6) 业务抬升：urgent_level = max(urgent_level, rush_level)  
7) 默认 → L0  

urgent_reason 必须记录触发分支（FREEZE / LATE / TEMP_BLOCK / N1 / N2 / RUSH_RULE 等）。

---

# 4. Priority Sorter（等级内排序）

排序键（同机组、同 urgent_level）：

1) FORCE_RELEASE 优先  
2) LOCKED 优先  
3) stock_age_days 降序  
4) rolling_output_age_days 降序  
5) due_date 升序  

---

# 5. Capacity Filler（产能池填充器）

## 5.1 输入

- capacity_pool（target / limit）
- 候选材料列表
- 冻结区落位材料
- 换辊窗口约束
- 结构目标
- **[v0.4+] PathRuleEngine 实例**
- **[v0.4+] 当前锚点状态（RollCycleState，见章节 14）**

## 5.2 输出

- plan_item
- material_state.sched_state = SCHEDULED
- **[v0.4+] violation_flags（JSON，含路径违规标记）**

## 5.3 行为规则

- 冻结区材料优先且不改变
- **[v0.4+] 每个候选材料填充前，调用 PathRuleEngine.check() 进行路径门控（见章节 15）**
- **[v0.4+] 路径违规时，根据 urgent_level 判断是否允许人工突破**
- 计算区填充至 target_capacity_t
- 允许填充到 limit_capacity_t（需风险标记）
- 允许结构跳过，但锁定材料不可跳过
- **[v0.4+] 每次成功填充后，更新路径锚点为当前材料的 width_mm / thickness_mm**

## 5.4 填充伪代码（v0.4+ PathRule 集成）

```python
def fill_with_path_rule(capacity_pool, candidates, frozen_items, roll_cycle_state):
    # 1. 先添加冻结区材料
    for item in frozen_items:
        add_to_pool(item)

    # 2. 解析锚点（优先级见章节 16）
    anchor = resolve_anchor(frozen_items, locked_items, user_confirmed_items, candidates)

    # 3. 填充计算区
    for candidate in candidates:
        # 路径门控
        path_result = path_rule_engine.check(candidate, anchor)

        if not path_result.is_feasible:
            if path_result.override_allowed:
                if candidate.user_confirmed:
                    # 允许突破，标记违规
                    candidate.violation_flags.path_violation = path_result.violation_type
                    proceed = True
                else:
                    skip(candidate, "PATH_OVERRIDE_REQUIRED")
                    continue
            else:
                skip(candidate, "PATH_HARD_VIOLATION")
                continue

        # 产能门控
        if not capacity_pool.can_add(candidate.weight_t):
            skip(candidate, "CAPACITY_EXCEEDED")
            continue

        # 添加材料
        add_to_pool(candidate)

        # 更新锚点
        anchor = (candidate.width_mm, candidate.thickness_mm)

    return plan_items, skipped_materials
```

---

# 6. Recalc Engine（重算/联动）

- 一键重算未来 N 天 → 新增 plan_version  
- 冻结区：保持不动  
- 计算区：联动窗口重排  
- 联动窗口：3 / 5 / 7 / 自定义  

---

# 7. Roll Campaign Engine（换辊窗口）

- suggest_threshold_t：建议阈值  
- hard_limit_t：强制上限  
- 超过 hard_limit_t → HARD_STOP  

---

# 8. Risk Engine（风险快照）

指标：  
- used_capacity_t  
- overflow_t  
- urgent_total_t（L2+L3）  
- mature_backlog_t  
- immature_backlog_t  
- risk_level（GREEN / YELLOW / ORANGE / RED，可解释）  

---

# 9. Impact Summary Engine（调整影响摘要）

写入 action_log.impact_summary_json：  
- 影响日期范围  
- 被挤出/移动/新增材料数量  
- 风险等级变化  
- used/target/limit delta  
- 锁定冲突与结构补偿建议  

---

# 10. 事务与一致性

- 导入/重算：BEGIN IMMEDIATE  
- 驾驶舱只读 risk_snapshot  
- 所有写入必须记录 action_log  

---

# 11. 配置项全集

| key | scope | default | note |
|-----|-------|---------|------|
| season_mode | GLOBAL | AUTO | AUTO / MANUAL |
| winter_months | GLOBAL | 11,12,1,2,3 | |
| manual_season | GLOBAL | WINTER | |
| min_temp_days_winter | GLOBAL | 3 | |
| min_temp_days_summer | GLOBAL | 4 | |
| urgent_n1_days | GLOBAL | 2 | 2/3/5 |
| urgent_n2_days | GLOBAL | 7 | 7/10/14 |
| roll_suggest_threshold_t | MACHINE | 1500 | |
| roll_hard_limit_t | MACHINE | 2500 | |
| overflow_pct | MACHINE | 0.05 | |
| recalc_window_days | VERSION | 7 | |

### 11.1 宽厚路径规则配置（v0.4+）

| key | scope | default | note |
|-----|-------|---------|------|
| path_rule_enabled | GLOBAL | true | 是否启用宽厚路径规则 |
| path_width_tolerance_mm | GLOBAL | 50.0 | 宽度容差阈值 (mm)，超过锚点此值视为违规 |
| path_thickness_tolerance_mm | GLOBAL | 1.0 | 厚度容差阈值 (mm)，超过锚点此值视为违规 |
| path_override_allowed_urgency_levels | GLOBAL | L2,L3 | 允许人工突破的紧急等级列表（逗号分隔） |
| seed_s2_percentile | GLOBAL | 0.95 | S2 种子策略上沿分位点 |
| seed_s2_small_sample_threshold | GLOBAL | 10 | S2 小样本阈值，低于此值用 max |

说明：
- 阈值策略：**固定阈值，全局生效，前端可配置**
- 不做产品族/钢种差异化（MVP 阶段）
- 配置存储于 `config_kv` 表，`scope_id = 'global'`

---

# 12. 数据落地约束

material_master 必须包含：  
- contract_nature  
- weekly_delivery_flag  
- export_flag  

缺失处理：  
- rush_level = L0  
- 风险/DQ 提示“催料字段缺失”

---

# 13. 规则扩展建议

紧急等级建议拆为：
- 参数层（config_kv）
- 组合规则层（rule_set / rule_item 或 JSON 规则）

v0.3–v0.4 推荐引入规则表，避免 Rust 硬编码。

---

# 14. RollCycle State Model（换辊周期状态模型）[v0.4+]

## 14.1 概述

换辊周期（Roll Campaign/Cycle）是热轧精整线的核心物理约束。每个换辊周期内，轧辊累计吨位不断增加，当达到阈值时需要换辊。**换辊时，宽厚路径规则的累计状态与锚点需要重置。**

## 14.2 状态定义

RollCycleState（运行时状态，与 roller_campaign 表对齐）：

| 字段 | 类型 | 说明 |
|------|------|------|
| version_id | TEXT | 关联排产版本 |
| machine_code | TEXT | 机组代码 |
| campaign_no | INTEGER | 当前换辊批次号 |
| cum_weight_t | REAL | 当前周期累计吨位 |
| path_anchor_material_id | TEXT [NEW] | 当前路径锚点材料ID |
| path_anchor_width_mm | REAL [NEW] | 锚点宽度 (mm) |
| path_anchor_thickness_mm | REAL [NEW] | 锚点厚度 (mm) |
| anchor_source | TEXT [NEW] | 锚点来源类型（见 14.3） |
| start_date | TEXT | 周期开始日期 |
| status | TEXT | 换辊状态（NORMAL / SUGGEST / HARD_STOP） |

## 14.3 锚点来源枚举 [NEW]

AnchorSource（存储于 anchor_source 字段）：

| 值 | 说明 |
|----|------|
| FROZEN_LAST | 冻结区最后一块材料 |
| LOCKED_LAST | 锁定区最后一块材料 |
| USER_CONFIRMED_LAST | 人工确认队列中最后一块 |
| SEED_S2 | S2 种子策略生成（见章节 16） |
| NONE | 无锚点（新周期起始或无候选） |

枚举定义位置：`src/domain/types.rs`

## 14.4 周期切换行为

当发生换辊时（手动触发或达到 `hard_limit_t`）：

1. **累计量重置**：`cum_weight_t = 0`
2. **锚点重置**：
   - `path_anchor_material_id = NULL`
   - `path_anchor_width_mm = NULL`
   - `path_anchor_thickness_mm = NULL`
   - `anchor_source = NONE`
3. **批次递增**：`campaign_no += 1`
4. **状态重置**：`status = NORMAL`
5. **审计记录**：写入 action_log，action_type = `RollCycleReset`

## 14.5 审计记录格式（RollCycleReset）

```json
{
  "action_type": "RollCycleReset",
  "payload_json": {
    "machine_code": "H032",
    "previous_campaign_no": 5,
    "new_campaign_no": 6,
    "reset_trigger": "HARD_LIMIT_REACHED",  // 或 "MANUAL"
    "previous_cum_weight_t": 2500.0,
    "previous_anchor": {
      "material_id": "M100",
      "width_mm": 1150.0,
      "thickness_mm": 8.5
    }
  },
  "impact_summary_json": {
    "anchor_reset": true,
    "cum_weight_reset": true,
    "affected_plan_dates": ["2026-01-20", "2026-01-21"]
  },
  "actor": "system"
}
```

---

# 15. PathRuleEngine（宽厚路径规则引擎）[v0.4+]

## 15.1 概述

宽厚路径规则是热轧工艺的核心约束：**由宽到窄、由厚到薄**。本引擎负责在 Capacity Filler 填充前对候选材料进行路径可行性判定。

## 15.2 硬约束定义

材料通过路径检查需同时满足：

```
width_mm(candidate) <= path_anchor_width_mm + path_width_tolerance_mm
thickness_mm(candidate) <= path_anchor_thickness_mm + path_thickness_tolerance_mm
```

其中：
- `path_anchor_width_mm` / `path_anchor_thickness_mm`：当前锚点（见章节 14）
- `path_width_tolerance_mm` / `path_thickness_tolerance_mm`：配置阈值（见章节 11.1）

## 15.3 输入

| 字段 | 类型 | 来源 |
|------|------|------|
| candidate.material_id | TEXT | material_master |
| candidate.width_mm | REAL | material_master |
| candidate.thickness_mm | REAL | material_master |
| candidate.urgent_level | TEXT | material_state |
| candidate.user_confirmed | INTEGER [NEW] | material_state |
| anchor_width_mm | REAL | RollCycleState |
| anchor_thickness_mm | REAL | RollCycleState |

## 15.4 输出

PathRuleResult：

| 字段 | 类型 | 说明 |
|------|------|------|
| status | TEXT | OK / HARD_VIOLATION / OVERRIDE_REQUIRED |
| violation_type | TEXT | WIDTH_EXCEEDED / THICKNESS_EXCEEDED / BOTH_EXCEEDED / NULL |
| width_delta_mm | REAL | 宽度超限值（正数表示超限） |
| thickness_delta_mm | REAL | 厚度超限值（正数表示超限） |

## 15.5 PathViolationType 枚举 [NEW]

| 值 | 说明 |
|----|------|
| WIDTH_EXCEEDED | 宽度超限 |
| THICKNESS_EXCEEDED | 厚度超限 |
| BOTH_EXCEEDED | 宽度和厚度均超限 |

枚举定义位置：`src/domain/types.rs`

## 15.6 判定逻辑

```python
def check(candidate, anchor, config):
    width_delta = candidate.width_mm - anchor.width_mm - config.path_width_tolerance_mm
    thickness_delta = candidate.thickness_mm - anchor.thickness_mm - config.path_thickness_tolerance_mm

    width_exceeded = width_delta > 0
    thickness_exceeded = thickness_delta > 0

    if not width_exceeded and not thickness_exceeded:
        return PathRuleResult(status="OK", violation_type=None)

    # 判断违规类型
    if width_exceeded and thickness_exceeded:
        violation_type = "BOTH_EXCEEDED"
    elif width_exceeded:
        violation_type = "WIDTH_EXCEEDED"
    else:
        violation_type = "THICKNESS_EXCEEDED"

    # 判断是否允许人工突破
    allowed_levels = config.path_override_allowed_urgency_levels  # e.g., ["L2", "L3"]
    if candidate.urgent_level in allowed_levels:
        return PathRuleResult(
            status="OVERRIDE_REQUIRED",
            violation_type=violation_type,
            width_delta_mm=max(0, width_delta),
            thickness_delta_mm=max(0, thickness_delta)
        )
    else:
        return PathRuleResult(
            status="HARD_VIOLATION",
            violation_type=violation_type,
            width_delta_mm=max(0, width_delta),
            thickness_delta_mm=max(0, thickness_delta)
        )
```

## 15.7 突破流程

当返回 `OVERRIDE_REQUIRED` 时：

1. **前端展示**：列出待确认材料，显示违规类型、超限量、紧急等级
2. **人工确认**：操作员输入确认原因
3. **状态更新**：设置 `material_state.user_confirmed = 1`，记录确认时间/人/原因
4. **审计记录**：写入 action_log，action_type = `PathOverrideConfirm`
5. **重新填充**：确认后的材料可通过路径检查

## 15.8 人工确认字段 [NEW]

material_state 表新增字段：

| 字段名 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| user_confirmed | INTEGER | 0 | 人工确认突破标志 (1=已确认) |
| user_confirmed_at | TEXT | NULL | 确认时间 (ISO DATETIME) |
| user_confirmed_by | TEXT | NULL | 确认人 |
| user_confirmed_reason | TEXT | NULL | 确认原因（必填） |

## 15.9 审计记录格式（PathOverrideConfirm）

```json
{
  "action_type": "PathOverrideConfirm",
  "payload_json": {
    "material_id": "M001",
    "violation_type": "WIDTH_EXCEEDED",
    "anchor_width_mm": 1200.0,
    "anchor_thickness_mm": 10.0,
    "material_width_mm": 1280.0,
    "material_thickness_mm": 9.5,
    "width_delta_mm": 30.0,
    "thickness_delta_mm": 0,
    "urgent_level": "L3",
    "confirm_reason": "紧急订单，客户要求优先交付"
  },
  "impact_summary_json": {
    "path_anchor_shifted": true,
    "new_anchor_width_mm": 1280.0,
    "new_anchor_thickness_mm": 9.5,
    "affected_downstream_materials": 3,
    "risk_level_change": "YELLOW -> ORANGE"
  },
  "actor": "operator_001",
  "machine_code": "H032"
}
```

## 15.10 与 Capacity Filler 的集成

见章节 5.4 填充伪代码。PathRuleEngine 作为 gate 在每个候选材料入池前调用。

---

# 16. Seed Strategy S2（锚点种子策略）[v0.4+]

## 16.1 概述

当冻结区和锁定区均为空时，需要通过种子策略生成初始锚点。S2 策略使用统计方法确定合理的起始宽度/厚度。

**重要约束**：锚点解析**忽略现场真实数据**（不使用 production/dispatched），仅使用排产系统内的状态。

## 16.2 锚点解析优先级（冻结）

```
1. FROZEN_LAST        → 冻结区最后一块材料的 width_mm / thickness_mm
2. LOCKED_LAST        → 锁定区最后一块材料
3. USER_CONFIRMED_LAST → 人工确认队列中最后一块
4. SEED_S2            → S2 种子策略计算生成
5. NULL               → 无锚点（极端情况，候选池为空）
```

解析顺序严格按优先级，找到第一个非空即返回。

## 16.3 锚点解析伪代码

```python
def resolve_anchor(frozen_items, locked_items, user_confirmed_items, candidates):
    """
    按优先级解析路径锚点
    """
    # 1. 冻结区最后一块
    if frozen_items:
        last = sorted(frozen_items, key=lambda x: x.seq_no)[-1]
        return Anchor(
            source=FROZEN_LAST,
            material_id=last.material_id,
            width_mm=last.width_mm,
            thickness_mm=last.thickness_mm
        )

    # 2. 锁定区最后一块
    if locked_items:
        last = sorted(locked_items, key=lambda x: x.seq_no)[-1]
        return Anchor(
            source=LOCKED_LAST,
            material_id=last.material_id,
            width_mm=last.width_mm,
            thickness_mm=last.thickness_mm
        )

    # 3. 人工确认队列最后一块
    if user_confirmed_items:
        last = sorted(user_confirmed_items, key=lambda x: x.user_confirmed_at)[-1]
        return Anchor(
            source=USER_CONFIRMED_LAST,
            material_id=last.material_id,
            width_mm=last.width_mm,
            thickness_mm=last.thickness_mm
        )

    # 4. S2 种子策略
    if candidates:
        width, thickness = compute_seed_s2(candidates)
        if width is not None and thickness is not None:
            return Anchor(
                source=SEED_S2,
                material_id=None,  # S2 不关联具体材料
                width_mm=width,
                thickness_mm=thickness
            )

    # 5. 无锚点
    return Anchor(source=NONE, width_mm=None, thickness_mm=None)
```

## 16.4 S2 种子策略算法

```python
def compute_seed_s2(candidates, percentile=0.95, small_sample_threshold=10):
    """
    S2 种子策略：上沿分位点 / 小样本用 max

    参数:
        candidates: 当日候选材料列表（已过滤适温）
        percentile: 分位点（配置项 seed_s2_percentile，默认 0.95）
        small_sample_threshold: 小样本阈值（配置项 seed_s2_small_sample_threshold，默认 10）

    返回:
        (anchor_width_mm, anchor_thickness_mm)
    """
    if len(candidates) == 0:
        return (None, None)  # 无候选，返回 NULL

    # 提取有效值
    widths = [m.width_mm for m in candidates if m.width_mm is not None]
    thicknesses = [m.thickness_mm for m in candidates if m.thickness_mm is not None]

    # 计算宽度锚点
    if len(widths) >= small_sample_threshold:
        anchor_width = percentile_calc(widths, percentile)
    elif len(widths) > 0:
        anchor_width = max(widths)  # 小样本用 max
    else:
        anchor_width = None

    # 计算厚度锚点
    if len(thicknesses) >= small_sample_threshold:
        anchor_thickness = percentile_calc(thicknesses, percentile)
    elif len(thicknesses) > 0:
        anchor_thickness = max(thicknesses)  # 小样本用 max
    else:
        anchor_thickness = None

    return (anchor_width, anchor_thickness)
```

## 16.5 配置项

见章节 11.1 宽厚路径规则配置：
- `seed_s2_percentile`：上沿分位点（默认 0.95）
- `seed_s2_small_sample_threshold`：小样本阈值（默认 10）

## 16.6 锚点更新时机

1. **填充成功时**：每成功填充一个材料，锚点更新为该材料的 width_mm / thickness_mm
2. **换辊重置时**：锚点清空，下次填充重新解析（见章节 14.4）
3. **手动刷新时**：支持前端触发重新解析锚点

---

# 17. 数据落地约束（v0.4+ 补充）

## 17.1 新增字段清单 [NEW]

### material_state 表

| 字段名 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| user_confirmed | INTEGER | 0 | 人工确认突破标志 |
| user_confirmed_at | TEXT | NULL | 确认时间 |
| user_confirmed_by | TEXT | NULL | 确认人 |
| user_confirmed_reason | TEXT | NULL | 确认原因 |

### roller_campaign 表

| 字段名 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| path_anchor_material_id | TEXT | NULL | 路径锚点材料ID |
| path_anchor_width_mm | REAL | NULL | 锚点宽度 (mm) |
| path_anchor_thickness_mm | REAL | NULL | 锚点厚度 (mm) |
| anchor_source | TEXT | NULL | 锚点来源类型 |

### plan_item 表（violation_flags JSON 扩展）

```json
{
  "path_violation": {
    "type": "WIDTH_EXCEEDED",
    "anchor_width_mm": 1200.0,
    "material_width_mm": 1280.0,
    "tolerance_mm": 50.0,
    "delta_mm": 30.0,
    "user_confirmed": true,
    "confirmed_by": "operator_001",
    "confirmed_at": "2026-01-20T10:30:00"
  }
}
```

## 17.2 新增枚举清单 [NEW]

| 枚举名 | 位置 | 值 |
|--------|------|-----|
| AnchorSource | src/domain/types.rs | FROZEN_LAST, LOCKED_LAST, USER_CONFIRMED_LAST, SEED_S2, NONE |
| PathViolationType | src/domain/types.rs | WIDTH_EXCEEDED, THICKNESS_EXCEEDED, BOTH_EXCEEDED |

## 17.3 新增 ActionType [NEW]

| ActionType | 说明 |
|------------|------|
| PathOverrideConfirm | 路径突破人工确认 |
| RollCycleReset | 换辊周期重置 |

---

# 18. 一致性检查清单（v0.4+）

## 18.1 字段一致性

- [x] width_mm / thickness_mm：单位 mm，类型 REAL，来源 material_master
- [x] urgent_level：枚举 L0/L1/L2/L3，来源 material_state
- [x] campaign_no / cum_weight_t：来源 roller_campaign
- [ ] user_confirmed 等字段：需新增至 material_state
- [ ] path_anchor_* 字段：需新增至 roller_campaign

## 18.2 枚举一致性

- [ ] AnchorSource：需新增至 src/domain/types.rs
- [ ] PathViolationType：需新增至 src/domain/types.rs
- [ ] ActionType：需新增 PathOverrideConfirm / RollCycleReset

## 18.3 配置一致性

- [ ] 所有配置项存储于 config_kv 表
- [ ] scope_id = 'global'（全局生效）
- [ ] 前端配置管理页面可编辑

## 18.4 审计一致性

- [ ] 人工确认必须记录 actor + reason
- [ ] 周期重置必须记录 payload + impact_summary
- [ ] 所有写入必须通过 action_log

## 18.5 数据库迁移

- [ ] 新增字段设置 NULL 默认值，兼容现有数据
- [ ] 迁移脚本命名：v0.4_path_rule_extension.sql

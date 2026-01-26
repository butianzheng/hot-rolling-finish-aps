# Tauri API Contract（整合版 v0.3）
（Command / Event 契约说明书）

目标：前端（WebView）与 Rust 后端通过 Tauri Command / Event 交互的稳定契约。  
约束：MVP 单机单用户；所有长任务必须支持进度事件；所有写操作必须落 action_log。  

整合来源：Tauri_API_Contract_v0.1 + Tauri_API_Contract_v0.2  
定位：本文件作为 **前后端交互唯一契约（Single Source of Truth）**

---

# 1. 通用约定

## 1.1 时间与单位
- 日期：YYYY-MM-DD（ISO DATE）  
- 时间戳：ISO8601 字符串  
- 重量：吨（t），展示与计算统一保留 3 位小数（Rust 侧建议 Decimal，落库 REAL）  

---

## 1.2 错误响应（统一结构）

```json
{
  "code": "ConstraintViolation",
  "message": "limit_capacity_t exceeded",
  "details": { "machine_code": "H0XX", "plan_date": "2026-01-12" }
}
```

---

## 1.3 长任务进度事件

- event: long_task_progress  
- payload:

```json
{ "task_id":"...", "phase":"recalc", "pct":35, "message":"Filling pools..." }
```

---

# 2. Commands（按业务域）

---

## 2.1 导入域（import）

### import_materials
- 入参：
```json
{ "file_path": "...", "source_batch_id": "...", "mapping_profile_id": "optional" }
```

- 行为：
  - 导入 material_master  
  - 重复 material_id → 写 import_conflict（Option C）  
  - 支持催料组合规则字段导入：  
    contract_nature / weekly_delivery_flag / export_flag  

- 返回：
```json
{ "imported": 100, "updated": 20, "conflicts": 3, "batch_id": "..." }
```

---

### list_import_conflicts
- 入参：
```json
{ "status": "OPEN|RESOLVED|IGNORED", "limit": 50, "offset": 0 }
```

- 返回：冲突列表

---

### resolve_import_conflict
- 入参：
```json
{ "conflict_id": "...", "action": "KEEP_EXISTING|OVERWRITE|MERGE", "note": "..." }
```

- 返回：处理结果（并写 action_log）

---

## 2.2 材料域（materials）

### list_materials
- 入参：筛选条件（machine_code、urgent_level、sched_state、due_date_range）  
- 返回：材料列表（含 material_state 派生字段）

---

### batch_lock_materials
- 入参：
```json
{ "material_ids": ["..."], "lock": true, "reason": "..." }
```

- 返回：影响摘要（Impact Summary）

---

### batch_force_release
- 入参：
```json
{ "material_ids": ["..."], "force": true, "reason": "..." }
```

- 返回：影响摘要

---

## 2.3 产能域（capacity）

### get_capacity_pools
- 入参：
```json
{ "machine_codes": ["H032"], "date_from": "2026-01-01", "date_to": "2026-01-15", "version_id": "optional" }
```

- 返回：产能池列表

---

### update_capacity_pool
- 入参：
```json
{ "machine_code": "H032", "plan_date": "2026-01-10", "target_capacity_t": 1500, "limit_capacity_t": 2500, "reason": "..." }
```

- 返回：更新结果（写 action_log）

---

## 2.4 方案域（plan）

### create_plan
- 入参：
```json
{ "plan_name": "...", "plan_type": "DRAFT|CANDIDATE" }
```

- 返回：plan_id

---

### create_plan_version
- 入参：
```json
{ "plan_id": "...", "recalc_window_days": 7, "frozen_from_date": "2026-01-10", "config_snapshot": {} }
```

- 返回：version_id

---

### recalc_plan_version
- 入参：
```json
{ 
  "plan_id": "...",
  "base_version_id": "optional",
  "recalc_window_days": 7,
  "frozen_from_date": "2026-01-10",
  "machine_scope": ["H032"],
  "mode": "FULL|PARTIAL"
}
```

- 返回：新 version_id（长任务，发 progress 事件）

---

### compare_versions
- 入参：
```json
{ "version_a": "...", "version_b": "..." }
```

- 返回：diff（移动/挤出/风险变化）

---

### rollback_to_version
- 入参：
```json
{ "plan_id": "...", "target_version_id": "..." }
```

- 返回：新 version_id（复制生成）

---

## 2.5 排产操作域（schedule）

### move_items
- 入参：
```json
{
  "version_id": "...",
  "moves": [
    { "material_id": "...", "to_date": "2026-01-12", "to_seq": 3, "to_machine": "H032" }
  ],
  "mode": "AUTO_FIX|STRICT"
}
```

- 返回：影响摘要 + 违规提示

---

### batch_move_items
- 入参：同 move_items（批量）  
- 返回：影响摘要

---

## 2.6 诊断域（diagnostics）

### dry_run_recalc
- 入参：同 recalc_plan_version，但不落 plan_item  
- 返回：风险摘要（按机组 × 日期）

---

### get_risk_snapshot
- 入参：
```json
{ "version_id": "...", "date_from": "2026-01-01", "date_to": "2026-01-15", "machine_codes": ["H032"] }
```

- 返回：risk_snapshot 列表

---

### diagnose_rush_level（v0.2 新增）
- 入参：
```json
{ "material_ids": ["..."] }
```
或
```json
{ "sample": 100, "machine_code": "H032" }
```

- 返回：
  - material_id  
  - rush_level  
  - hit_rule_id / hit_rule_name  

用于现场核验催料组合规则是否正确。

---

## 2.7 配置域（config）（v0.2 新增）

### set_global_config
- 入参：
```json
{ "key": "season_mode", "value": "AUTO" }
```

支持：season_mode / manual_season / winter_months / urgent_n1_days / urgent_n2_days / min_temp_days_* 等。

---

### get_global_config
- 入参：
```json
{ "keys": ["season_mode", "urgent_n1_days"] }
```

- 返回：
```json
{ "season_mode": "AUTO", "urgent_n1_days": 2 }
```

---

# 3. Events（事件推送）

- plan_updated  
- risk_snapshot_updated  
- material_state_changed  
- long_task_progress  

统一结构：
```json
{ "event": "xxx", "payload": { } }
```

---

# 4. 错误码（MVP 冻结）

- ConfigInvalid  
- DataQualityError  
- LockedConflict  
- CapacityOverflow  
- RollHardStop  
- ConstraintViolation  
- NotFound  
- RuleEvalError  

---

# 5. 契约级约束（工程冻结）

1. 所有写操作必须写 action_log  
2. 所有长任务必须支持 progress event  
3. 所有规则类错误必须返回可解释 error code  
4. 所有跨版本操作必须可回滚（基于 plan_version）  

---

# 6. 推荐实现结构（非强制）

- commands/import.rs  
- commands/materials.rs  
- commands/capacity.rs  
- commands/plan.rs  
- commands/schedule.rs  
- commands/diagnostics.rs  
- commands/config.rs  
- events/mod.rs  

---

# 7. 版本演进说明

v0.1：基础 Command/Event 契约框架  
v0.2：补充配置控制 + 催料诊断接口  
v0.3：统一冻结为单一 API Contract 文档

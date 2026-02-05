# 数据字典 v0.1（MVP）
本字典与 `schema_v0.1.sql` 对应，面向 Tauri + Rust + SQLite 实现。

## 1. 关键口径冻结
- 主实体：material（材料/卷）
- 合同不独立建表：合同字段作为 material_master 的影子列
- current_machine_code：当 `精整返修机组` 非空时覆盖（Option A）
- 重量单位：吨（t），保留小数点后 3 位（存储 REAL，展示/计算按 3 位）
- 重复材料号导入：进入 `import_conflict` 冲突队列（Option C）
- 滞留口径：`stock_age_days = 状态时间(天)`
- 适温反推：rolling_output_age_days 基于 output_age_days_raw + 机组偏移规则（见字段映射说明书）

---

## 2. 表清单与用途概览
- **machine_master**：机组主数据与默认产能参数
- **material_master**：材料主数据（含合同影子字段）
- **material_state**：系统排产状态机（与物理状态分离）
- **capacity_pool**：产能池（机组×日期）
- **plan / plan_version / plan_item**：方案、版本、版本内排产明细
- **roller_campaign**：换辊窗口（版本内结构约束）
- **risk_snapshot**：驾驶舱快照表（版本×机组×日期）
- **action_log**：审计与回放
- **import_conflict**：导入冲突队列（重复材料号/字段冲突等）
- **config_scope / config_kv**：配置分层存储

---

## 3. 关键表字段说明（节选）

### 3.1 material_master（材料主表）
| 字段 | 类型 | 含义 | 规则/备注 |
|---|---|---|---|
| material_id | TEXT | 材料号 | 主键，唯一 |
| contract_no | TEXT | 合同号 | 影子列 |
| due_date | TEXT | 合同交货期 | ISO DATE（YYYY-MM-DD）；用于紧急等级 |
| rush_flag | TEXT | 催料标记 | 紧急等级触发因子候选 |
| next_machine_code | TEXT | 下道机组代码 | 源字段 |
| rework_machine_code | TEXT | 精整返修机组 | 源字段 |
| current_machine_code | TEXT | 当前机组代码 | 计算：COALESCE(rework_machine_code, next_machine_code) |
| width_mm / thickness_mm | REAL | 宽/厚 | mm |
| weight_t | REAL | 重量（吨） | 应用层保留 3 位小数 |
| output_age_days_raw | INTEGER | 产出时间(天) | 导入时刻静态快照 |
| rolling_output_date | TEXT | 轧制产出日期 | ISO DATE（YYYY-MM-DD）；v0.7 新增；用于动态计算实际产出天数 |
| stock_age_days | INTEGER | 状态时间(天) | 库存压力主口径 |
| status_updated_at | TEXT | 状态修改时间 | 用于增量导入 |

### 3.2 material_state（系统状态机）
| 字段 | 类型 | 含义 | 规则/备注 |
|---|---|---|---|
| sched_state | TEXT | 排产状态 | PENDING_MATURE / READY / SCHEDULED / LOCKED / FORCE_RELEASE / DONE |
| lock_flag | INTEGER | 锁定 | 1/0 |
| urgent_level | TEXT | 紧急等级 | L0-L3（等级制） |
| urgent_reason | TEXT | 紧急原因 | 建议 JSON，支持可解释 |
| rolling_output_age_days | INTEGER | 等效轧制产出天数 | 按机组偏移规则派生 |
| earliest_sched_date | TEXT | 最早可排日期 | ISO DATE |

### 3.3 capacity_pool（产能池）
| 字段 | 类型 | 含义 | 规则/备注 |
|---|---|---|---|
| machine_code | TEXT | 机组 | FK |
| plan_date | TEXT | 日期 | ISO DATE |
| target_capacity_t | REAL | 目标负荷 | 计划值 |
| limit_capacity_t | REAL | 极限负荷 | 红线值 |
| manual_adjust_reason | TEXT | 调整原因 | 检修/提效等 |
| is_frozen | INTEGER | 冻结 | 冻结日池（可选） |

### 3.4 plan_version / plan_item（排产版本与明细）
- plan_version 保存计算窗口、冻结区、配置快照
- plan_item 保存版本内材料落位（机组×日期×顺序）

---

## 4. 索引建议（MVP）
- material_master(current_machine_code), material_master(due_date), material_master(status_updated_at)
- material_master(rolling_output_date) - v0.7 新增，支持适温查询优化
- plan_item(version_id, machine_code, plan_date, seq_no)
- capacity_pool(machine_code, plan_date)
- risk_snapshot(version_id, plan_date)

---

## 5. 约束与一致性策略（MVP）
- 所有批量导入、重算、批量移动：必须事务化（BEGIN IMMEDIATE）
- 驾驶舱查询优先使用 risk_snapshot 快表，避免在线计算
- 版本回滚粒度：version_id 级别

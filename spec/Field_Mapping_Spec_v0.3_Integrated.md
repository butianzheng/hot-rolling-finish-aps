# 字段映射与口径说明书（整合版 v0.3）

项目：热轧精整机组优先级驱动排产决策支持系统（Tauri + Rust + SQLite）
范围：基于《热卷物料清单示例.xlsx》形成 MVP 字段映射与派生口径，合同暂不独立建表（合同字段作为材料影子列）。

整合来源：Field_Mapping_Spec_v0.1 + Field_Mapping_Spec_v0.2
定位：本文件作为 **导入层 / 数据准备层 / 口径冻结主文档（Single Source of Truth）**

---

# 1. 结论性冻结（当前版本必须遵循）

1. **轧制产出时间采用反推口径（选择 B）**基于 `产出时间(天)` 反推，并按机组代码调整偏移。
2. **在库/滞留指标采用 `状态时间(天)`（选择 A 的调整版）**系统以 `状态时间(天)` 作为库存压力/滞留主口径。
3. **合同不独立建表**`合同号 / 合同交货期 / 催料相关字段` 等以材料表影子字段存储。
4. **新增冻结（v0.2）：催料采用组合规则，不再使用单一 rush_flag 映射。**
   由合同性质 + 按周交货标志 + 出口标记组合计算 rush_level。

---

# 2. 标准字段（Canonical Schema）最小集

MVP 以“材料（卷/板）”为主实体，推荐最小标准字段如下：

- material_id（材料号，主键）
- current_machine_code（当前机组）
- due_date（合同交货期）
- urgent_level（紧急等级 L0–L3，派生）
- lock_flag（锁定标记，系统字段）
- force_release_flag（强制放行，系统字段）
- width_mm / thickness_mm / weight_t（工艺关键维度）
- stock_age_days（滞留/库存压力口径：状态时间(天)）
- output_age_days_raw（产出时间(天)）
- rolling_output_age_days（轧制产出时间(天)，派生）
- ready_in_days（满足适温还需天数，派生）
- earliest_sched_date（最早可排日期，派生）

新增业务影子字段（用于 rush_level）：

- contract_nature
- weekly_delivery_flag
- export_flag

---

# 3. 关键派生字段口径（冻结）

## 3.1 轧制产出时间（天）反推规则

输入：`产出时间(天)` + `current_machine_code`

规则：

- 若 current_machine_code ∉ {H032, H033, H034}→ rolling_output_age_days = output_age_days_raw + 4
- 否则
  → rolling_output_age_days = output_age_days_raw

说明：rolling_output_age_days 为“适温判断用的等效轧制产出时间口径”。

---

## 3.2 适温可排判断与最早可排日期

系统配置：min_temp_days（默认 3/4 天，支持机组/钢种/日期覆写）

- is_temp_ready = (rolling_output_age_days ≥ min_temp_days)
- ready_in_days = max(0, min_temp_days - rolling_output_age_days)
- earliest_sched_date = today + ready_in_days

强制放行：
当 force_release_flag = true 时可绕过适温，但必须记录风险原因与操作人。

---

## 3.3 滞留 / 库存压力口径

- stock_age_days = 状态时间(天)
- 在库时间(天) 仅作为参考字段，不进入主逻辑。

---

# 4. 源字段 → 标准字段映射表（整合版）

| 标准字段                 | 类型     | 源字段           | 转换/清洗        | 说明                                    |
| ------------------------ | -------- | ---------------- | ---------------- | --------------------------------------- |
| material_id              | TEXT     | 材料号           | 原值             | 主键                                    |
| manufacturing_order_id   | TEXT     | 制造命令号       | 原值             | 建议保留                                |
| material_status_code_src | TEXT     | 材料状态码       | 原值             | 状态影子字段                            |
| due_date                 | DATE     | 合同交货期       | YYYYMMDD → DATE | 关键交期                                |
| current_machine_code     | TEXT     | 下道机组代码     | 原值             | 当前计划机组                            |
| rework_machine_code      | TEXT     | 精整返修机组     | 原值             | 是否覆盖 current_machine_code（待确认） |
| width_mm                 | REAL     | 材料实际宽度     | 原值             | mm                                      |
| thickness_mm             | REAL     | 材料实际厚度     | 原值             | mm                                      |
| length_m                 | REAL     | 材料实际长度     | 原值             | m                                       |
| weight_t                 | REAL     | 材料实际重量     | 原值/换算        | 吨（需确认）                            |
| available_width_mm       | REAL     | 可利用宽度       | 原值             | 可选                                    |
| steel_mark               | TEXT     | 出钢记号         | 原值             | 钢种影子字段                            |
| slab_id                  | TEXT     | 板坯号           | 原值             | 追溯                                    |
| stock_age_days           | INTEGER  | 状态时间(天)     | 原值             | 主滞留口径                              |
| output_age_days_raw      | INTEGER  | 产出时间(天)     | 原值             | 派生基础                                |
| status_updated_at        | DATETIME | 物料状态修改时间 | 转 ISO8601       | 增量基准                                |
| contract_nature          | TEXT     | 合同性质代码     | TRIM + UPPER     | rush 规则字段①                         |
| weekly_delivery_flag     | TEXT     | 按周交货标志     | TRIM + UPPER     | rush 规则字段②                         |
| export_flag              | TEXT/INT | 出口标记         | 统一为 '1'/'0'   | rush 规则字段③                         |

---

# 5. 催料组合规则（工程冻结）

新增中间变量：rush_level（L0/L1/L2）

执行顺序：

1. contract_nature 非空 且首字符不为 Y/X 且 weekly_delivery_flag = 'D'→ rush_level = L2
2. 否则若 contract_nature 非空 且首字符不为 Y/X 且 weekly_delivery_flag = 'A' 且 export_flag = '1'→ rush_level = L1
3. 其他
   → rush_level = L0

最终紧急等级：
urgent_level = max(交期/冻结/适温红线等级, rush_level)

---

# 6. 数据质量规则（DQ Rules）

## 6.1 主键与唯一性

- material_id 必须非空唯一。

## 6.2 日期字段

- 合同交货期：非法值视为缺失。
- 物料状态修改时间：作为增量导入基准。

## 6.3 数值字段

- 宽/厚/重量必须 >0，超阈值报警。
- 重量单位必须统一为吨（待确认）。

## 6.4 适温反推字段

- 产出时间 / 状态时间 必须为非负整数，缺失则 BLOCKED。

## 6.5 催料字段

- 任一字段缺失：
  - rush_level = L0
  - 进入数据质量报告提示。

---

# 7. 导入管道建议顺序

1. 原始字段导入 material_master
2. 基础清洗（trim / upper / null）
3. 计算 rolling_output_age_days
4. 计算 rush_level
5. 写 material_state（urgent_reason 记录命中规则）
6. 进入 Urgency Engine 总判定

---

# 8. 建议索引（可选）

```sql
CREATE INDEX idx_material_rush_fields 
ON material_master(contract_nature, weekly_delivery_flag, export_flag);
```

---

备注需注意：空交期材料允许排产，优先级低。

---

# 9. 附：样例字段存在性说明

样例文件已包含并验证的字段：
材料号、合同交货期、产出时间(天)、状态时间(天)、在库时间(天)、下道机组代码、精整返修机组、物料状态修改时间、合同性质代码、按周交货标志、出口标记。

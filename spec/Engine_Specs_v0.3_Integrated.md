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

## 5.2 输出

- plan_item  
- material_state.sched_state = SCHEDULED  

## 5.3 行为规则

- 冻结区材料优先且不改变  
- 计算区填充至 target_capacity_t  
- 允许填充到 limit_capacity_t（需风险标记）  
- 允许结构跳过，但锁定材料不可跳过  

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

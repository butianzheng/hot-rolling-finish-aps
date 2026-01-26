# 热轧精整机组排产系统 · Claude 开发主控文档（Master Spec v1.0）
面向：Claude Code + 个人独立开发  
技术栈：Tauri + Rust + SQLite  
系统定位：决策支持系统（人工最终控制权）

---
# PART A 系统宪法（冻结）

## A1 定位
- 本系统是决策支持系统，不是自动控制系统  
- 所有结果必须允许人工修正  
- 不引入全局 APS / 黑箱优化算法  

## A2 红线
- 冻结区材料不可被系统调整  
- 非适温材料不得进入当日产能池  
- 紧急等级是“等级制”，不是评分制  
- 排产必须可解释（reason 字段）  

## A3 审计增强
- material_state 是唯一事实层  
- plan_item 只是方案快照，不可反向污染  
- 产能池约束优先于材料优先级  
- 未成熟材料必须进入未来风险统计  
- Claude 不得合并引擎、弱化解释字段  

---
# PART B 业务内核冻结

## B1 适温与冷料
- 冬季默认 3 天，夏季默认 4 天（可配置）
- rolling_output_age_days：
  - 非 H032/H033/H034 → output_age_days + 4
  - 否则 → output_age_days
- 未成熟材料不得进入当日产能池

## B2 紧急等级 L0–L3
顺序判定：
1. 人工红线 → L3
2. 冻结区 → ≥L2
3. 超期 → L3
4. 适温阻断红线 → L3
5. 临期：N1→L2；N2→L1
6. 催料抬升：rush_level
7. 默认 → L0

N1：2/3/5 天（可配置）  
N2：7/10/14 天（可配置）  

### 催料组合规则
- 合同性质代码、按周交货标志、出口标记组合：
  - 非空且非 Y/X 且 D → L2
  - 非空且非 Y/X 且 A 且出口1 → L1
  - 否则 → L0

## B3 产能与换辊
- 产能以吨位池管理（target/limit）
- 换辊建议 1500t，强制 2500t（可配置）
- 结构目标为软约束

---
# PART C 数据与状态体系

## 主实体
- material_master（材料 + 合同影子字段）
- material_state（系统排产状态）
- capacity_pool（机组×日期）
- plan / plan_version / plan_item
- roller_campaign
- risk_snapshot
- action_log

## 主口径
- weight_t：吨，3 位小数
- stock_age_days = 状态时间(天)

---
# PART D 引擎体系

1. Eligibility Engine（适温/状态）
2. Urgency Engine（等级制紧急）
3. Priority Sorter（等级内排序）
4. Capacity Filler（吨位填池）
5. Roll Campaign Engine
6. Recalc Engine
7. Risk Engine
8. Impact Summary Engine

引擎铁律：
- Engine 不拼 SQL
- Repository 不含业务
- 所有规则必须输出 reason

---
# PART E Tauri 工程结构

/src  
  /domain  
  /repository  
  /engine  
  /importer  
  /config  
  /api  
  /app  

---
# PART F Claude 工作法

单模块 Prompt：  
“你是本系统的 {模块名} 工程师，请严格遵守 Master Spec。  
只实现单一模块，禁止修改业务红线。  
先给测试，再写代码。”

审计 Prompt：  
“请对照 Master Spec 审计条款，检查状态污染/冷料绕过/产能越权/解释缺失。”

---
# PART G 成功判定

系统必须能回答：
- 哪天最危险
- 哪些紧急单无法完成
- 哪些冷料压库
- 哪个机组最堵
- 换辊是否异常
- 是否存在产能优化空间

---
（本文件为 Claude Code 唯一主控文档，其他文件均为其子规格）

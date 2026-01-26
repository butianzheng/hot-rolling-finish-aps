-- ==========================================
-- Migration v0.4: Decision Layer Read Models
-- ==========================================
-- 目的: 创建决策层读模型表，支持 6 个核心决策用例
-- 依据: 热轧精整排产系统_决策结构重构方案_v1.0.docx - 第 6 章
-- 依据: src/decision/use_cases/*.rs - 用例接口定义
-- 变更:
--   1) 创建 decision_day_summary 表（D1: 哪天最危险）
--   2) 创建 decision_order_failure_set 表（D2: 哪些紧急单无法完成）
--   3) 创建 decision_cold_stock_profile 表（D3: 哪些冷料压库）
--   4) 创建 decision_machine_bottleneck 表（D4: 哪个机组最堵）
--   5) 创建 decision_roll_campaign_alert 表（D5: 换辊是否异常）
--   6) 创建 decision_capacity_opportunity 表（D6: 是否存在产能优化空间）
--   7) 创建 decision_refresh_log 表（刷新日志）
-- ==========================================

PRAGMA foreign_keys = ON;

-- ==========================================
-- D1: 哪天最危险 (Most Risky Day)
-- ==========================================
-- 输入: version_id, date_range
-- 输出: 按 risk_score 降序的日期风险摘要
-- 刷新触发: risk_snapshot_updated, plan_item_changed
-- ==========================================

CREATE TABLE IF NOT EXISTS decision_day_summary (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  plan_date TEXT NOT NULL,

  -- 风险指标
  risk_score REAL NOT NULL,                    -- 风险分数 (0-100)
  risk_level TEXT NOT NULL,                    -- 风险等级 (LOW/MEDIUM/HIGH/CRITICAL)

  -- 产能指标
  capacity_util_pct REAL NOT NULL,             -- 产能利用率 (%)

  -- 风险原因（JSON 数组: [{code, msg, weight, severity}]）
  top_reasons TEXT NOT NULL,                   -- 前 N 个风险原因

  -- 影响范围
  affected_machines INTEGER NOT NULL,          -- 受影响机组数
  bottleneck_machines INTEGER NOT NULL,        -- 堵塞机组数
  has_roll_risk INTEGER NOT NULL DEFAULT 0,    -- 是否存在换辊风险 (0/1)

  -- 建议措施（JSON 数组: [string]）
  suggested_actions TEXT,                      -- 建议措施列表

  -- 元数据
  refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),

  PRIMARY KEY (version_id, plan_date)
);

CREATE INDEX IF NOT EXISTS idx_day_summary_version_risk
  ON decision_day_summary(version_id, risk_score DESC);

CREATE INDEX IF NOT EXISTS idx_day_summary_date_range
  ON decision_day_summary(version_id, plan_date);

-- ==========================================
-- D2: 哪些紧急单无法完成 (Order Failure)
-- ==========================================
-- 输入: version_id, fail_type (可选)
-- 输出: 按 urgency_level 降序的订单失败记录
-- 刷新触发: plan_item_changed, material_state_changed
-- ==========================================

CREATE TABLE IF NOT EXISTS decision_order_failure_set (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  contract_no TEXT NOT NULL,

  -- 订单信息
  due_date TEXT NOT NULL,                      -- 交货日期
  urgency_level TEXT NOT NULL,                 -- 紧急等级 (L0/L1/L2/L3)

  -- 失败类型
  fail_type TEXT NOT NULL,                     -- 失败类型 (Overdue/NearDueImpossible/CapacityShortage/StructureConflict/ColdStockNotReady/Other)

  -- 材料统计
  total_materials INTEGER NOT NULL,            -- 总材料数
  unscheduled_count INTEGER NOT NULL,          -- 未排产数量
  unscheduled_weight_t REAL NOT NULL,          -- 未排产重量 (吨)
  completion_rate REAL NOT NULL,               -- 完成率 (0-1)

  -- 时间指标
  days_to_due INTEGER NOT NULL,                -- 距交货日天数

  -- 失败原因（JSON 数组: [string]）
  failure_reasons TEXT NOT NULL,               -- 失败原因列表

  -- 阻塞因素（JSON 数组: [{code, description, affected_count, affected_weight_t, is_removable}]）
  blocking_factors TEXT NOT NULL,              -- 阻塞因素列表

  -- 建议措施（JSON 数组: [string]）
  suggested_actions TEXT,                      -- 建议措施列表

  -- 元数据
  refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),

  PRIMARY KEY (version_id, contract_no)
);

CREATE INDEX IF NOT EXISTS idx_order_failure_version_urgency
  ON decision_order_failure_set(version_id, urgency_level DESC);

CREATE INDEX IF NOT EXISTS idx_order_failure_fail_type
  ON decision_order_failure_set(version_id, fail_type);

CREATE INDEX IF NOT EXISTS idx_order_failure_due_date
  ON decision_order_failure_set(version_id, due_date);

-- ==========================================
-- D3: 哪些冷料压库 (Cold Stock Profile)
-- ==========================================
-- 输入: version_id, machine_code (可选)
-- 输出: 按 pressure_score 降序的冷料分桶
-- 刷新触发: material_state_changed, plan_item_changed
-- ==========================================

CREATE TABLE IF NOT EXISTS decision_cold_stock_profile (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  age_bin TEXT NOT NULL,                       -- 年龄分桶 (0-7/8-14/15-30/30+)

  -- 年龄范围
  age_min_days INTEGER NOT NULL,               -- 最小天数
  age_max_days INTEGER,                        -- 最大天数 (NULL 表示无上限)

  -- 材料统计
  count INTEGER NOT NULL,                      -- 材料数量
  weight_t REAL NOT NULL,                      -- 总重量 (吨)

  -- 压库指标
  pressure_score REAL NOT NULL,                -- 压库分数 (0-100)
  pressure_level TEXT NOT NULL,                -- 压库等级 (LOW/MEDIUM/HIGH/CRITICAL)

  -- 压库原因（JSON 数组: [string]）
  reasons TEXT NOT NULL,                       -- 压库原因列表

  -- 结构缺口
  structure_gap TEXT,                          -- 结构缺口描述

  -- 预计适温日期
  estimated_ready_date TEXT,                   -- 预计适温日期 (YYYY-MM-DD)

  -- 释放标志
  can_force_release INTEGER NOT NULL DEFAULT 0, -- 是否可强制释放 (0/1)

  -- 建议措施（JSON 数组: [string]）
  suggested_actions TEXT,                      -- 建议措施列表

  -- 元数据
  refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),

  PRIMARY KEY (version_id, machine_code, age_bin)
);

CREATE INDEX IF NOT EXISTS idx_cold_stock_version_pressure
  ON decision_cold_stock_profile(version_id, pressure_score DESC);

CREATE INDEX IF NOT EXISTS idx_cold_stock_machine
  ON decision_cold_stock_profile(version_id, machine_code);

CREATE INDEX IF NOT EXISTS idx_cold_stock_high_pressure
  ON decision_cold_stock_profile(version_id, pressure_level)
  WHERE pressure_level IN ('HIGH', 'CRITICAL');

-- ==========================================
-- D4: 哪个机组最堵 (Machine Bottleneck)
-- ==========================================
-- 输入: version_id, machine_code (可选), date_range
-- 输出: 按 bottleneck_score 降序的机组堵塞概况
-- 刷新触发: capacity_pool_changed, plan_item_changed, material_state_changed
-- ==========================================

CREATE TABLE IF NOT EXISTS decision_machine_bottleneck (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  plan_date TEXT NOT NULL,

  -- 堵塞指标
  bottleneck_score REAL NOT NULL,              -- 堵塞分数 (0-100)
  bottleneck_level TEXT NOT NULL,              -- 堵塞等级 (NONE/LOW/MEDIUM/HIGH/CRITICAL)

  -- 堵塞类型（JSON 数组: [string]）
  bottleneck_types TEXT NOT NULL,              -- 堵塞类型列表 (Capacity/Structure/RollChange/ColdStock/Mixed)

  -- 堵塞原因（JSON 数组: [{code, description, severity, impact_t}]）
  reasons TEXT NOT NULL,                       -- 堵塞原因列表

  -- 产能指标
  remaining_capacity_t REAL NOT NULL,          -- 剩余产能 (吨)
  capacity_utilization REAL NOT NULL,          -- 产能利用率 (0-1)

  -- 换辊标志
  needs_roll_change INTEGER NOT NULL DEFAULT 0, -- 是否需要换辊 (0/1)

  -- 结构违规
  structure_violations INTEGER NOT NULL DEFAULT 0, -- 结构违规数量

  -- 待排材料
  pending_materials INTEGER NOT NULL DEFAULT 0, -- 待排材料数量

  -- 建议措施（JSON 数组: [string]）
  suggested_actions TEXT,                      -- 建议措施列表

  -- 元数据
  refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),

  PRIMARY KEY (version_id, machine_code, plan_date)
);

CREATE INDEX IF NOT EXISTS idx_bottleneck_version_score
  ON decision_machine_bottleneck(version_id, bottleneck_score DESC);

CREATE INDEX IF NOT EXISTS idx_bottleneck_machine_date
  ON decision_machine_bottleneck(version_id, machine_code, plan_date);

CREATE INDEX IF NOT EXISTS idx_bottleneck_high_level
  ON decision_machine_bottleneck(version_id, bottleneck_level)
  WHERE bottleneck_level IN ('HIGH', 'CRITICAL');

-- ==========================================
-- D5: 换辊是否异常 (Roll Campaign Alert)
-- ==========================================
-- 输入: version_id, alert_level (可选)
-- 输出: 按 alert_level 降序的换辊预警
-- 刷新触发: roll_campaign_changed, plan_item_changed
-- ==========================================

CREATE TABLE IF NOT EXISTS decision_roll_campaign_alert (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  campaign_no INTEGER NOT NULL,

  -- 累计重量
  cum_weight_t REAL NOT NULL,                  -- 累计重量 (吨)
  suggest_threshold_t REAL NOT NULL,           -- 建议阈值 (吨)
  hard_limit_t REAL NOT NULL,                  -- 硬限制 (吨)

  -- 预警等级
  alert_level TEXT NOT NULL,                   -- 预警等级 (NONE/WARNING/CRITICAL/EMERGENCY)

  -- 预警原因
  reason TEXT,                                 -- 预警原因

  -- 距离指标
  distance_to_suggest REAL NOT NULL,           -- 距建议阈值 (吨)
  distance_to_hard REAL NOT NULL,              -- 距硬限制 (吨)

  -- 利用率
  utilization_rate REAL NOT NULL,              -- 利用率 (0-1)

  -- 预计换辊日期
  estimated_change_date TEXT,                  -- 预计换辊日期 (YYYY-MM-DD)

  -- 紧急标志
  needs_immediate_change INTEGER NOT NULL DEFAULT 0, -- 是否需要立即换辊 (0/1)

  -- 建议措施（JSON 数组: [string]）
  suggested_actions TEXT,                      -- 建议措施列表

  -- 元数据
  refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),

  PRIMARY KEY (version_id, machine_code, campaign_no)
);

CREATE INDEX IF NOT EXISTS idx_roll_alert_version_level
  ON decision_roll_campaign_alert(version_id, alert_level DESC);

CREATE INDEX IF NOT EXISTS idx_roll_alert_machine
  ON decision_roll_campaign_alert(version_id, machine_code);

CREATE INDEX IF NOT EXISTS idx_roll_alert_emergency
  ON decision_roll_campaign_alert(version_id, alert_level)
  WHERE alert_level IN ('CRITICAL', 'EMERGENCY');

-- ==========================================
-- D6: 是否存在产能优化空间 (Capacity Opportunity)
-- ==========================================
-- 输入: version_id, machine_code (可选), date_range
-- 输出: 按 opportunity_level 降序的产能优化机会
-- 刷新触发: capacity_pool_changed, plan_item_changed
-- ==========================================

CREATE TABLE IF NOT EXISTS decision_capacity_opportunity (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  plan_date TEXT NOT NULL,

  -- 松弛空间
  slack_t REAL NOT NULL,                       -- 松弛空间 (吨)
  soft_adjust_space_t REAL,                    -- 软约束调整空间 (吨)

  -- 利用率
  utilization_rate REAL NOT NULL,              -- 利用率 (0-1)

  -- 绑定约束（JSON 数组: [{constraint_type, description, slack_t}]）
  binding_constraints TEXT NOT NULL,           -- 绑定约束列表

  -- 机会等级
  opportunity_level TEXT NOT NULL,             -- 机会等级 (NONE/LOW/MEDIUM/HIGH)

  -- 敏感性分析（JSON 对象: {soft_constraint_gain_t, target_adjustment_gain_t, structure_optimization_gain_t, total_potential_gain_t, risk_assessment}）
  sensitivity TEXT,                            -- 敏感性分析

  -- 建议优化（JSON 数组: [string]）
  suggested_optimizations TEXT,                -- 建议优化列表

  -- 元数据
  refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),

  PRIMARY KEY (version_id, machine_code, plan_date)
);

CREATE INDEX IF NOT EXISTS idx_capacity_opp_version_level
  ON decision_capacity_opportunity(version_id, opportunity_level DESC);

CREATE INDEX IF NOT EXISTS idx_capacity_opp_machine_date
  ON decision_capacity_opportunity(version_id, machine_code, plan_date);

CREATE INDEX IF NOT EXISTS idx_capacity_opp_high_level
  ON decision_capacity_opportunity(version_id, opportunity_level)
  WHERE opportunity_level IN ('MEDIUM', 'HIGH');

-- ==========================================
-- 决策刷新日志 (Decision Refresh Log)
-- ==========================================
-- 用途: 记录决策视图刷新历史，支持增量刷新和审计
-- ==========================================

CREATE TABLE IF NOT EXISTS decision_refresh_log (
  refresh_id TEXT PRIMARY KEY,
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,

  -- 刷新触发
  trigger_type TEXT NOT NULL,                  -- 触发类型 (PlanItemChanged/RiskSnapshotUpdated/MaterialStateChanged/CapacityPoolChanged/RollCampaignChanged/VersionCreated/ManualRefresh)
  trigger_source TEXT,                         -- 触发源（操作人/系统组件）

  -- 刷新范围
  is_full_refresh INTEGER NOT NULL DEFAULT 0,  -- 是否全量刷新 (0/1)
  affected_machines TEXT,                      -- 受影响机组（JSON 数组）
  affected_date_range TEXT,                    -- 受影响日期范围（JSON 对象: {start, end}）

  -- 刷新结果
  refreshed_tables TEXT NOT NULL,              -- 已刷新表列表（JSON 数组）
  rows_affected INTEGER NOT NULL DEFAULT 0,    -- 影响行数

  -- 时间戳
  started_at TEXT NOT NULL DEFAULT (datetime('now')),
  completed_at TEXT,
  duration_ms INTEGER,

  -- 状态
  status TEXT NOT NULL DEFAULT 'RUNNING',      -- 状态 (RUNNING/SUCCESS/FAILED)
  error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_refresh_log_version
  ON decision_refresh_log(version_id, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_refresh_log_trigger
  ON decision_refresh_log(trigger_type, started_at DESC);

-- ==========================================
-- 更新 schema_version
-- ==========================================

INSERT INTO schema_version (version, applied_at)
  VALUES (4, datetime('now'));

-- ==========================================
-- 验证 migration 成功
-- ==========================================

-- 检查决策表是否已创建
SELECT
  CASE
    WHEN COUNT(*) = 7 THEN 'OK: 决策层表已创建（7个表）'
    ELSE 'ERROR: 决策层表缺失，实际: ' || COUNT(*) || ' 个'
  END AS validation_result
FROM sqlite_master
WHERE type='table'
  AND name IN (
    'decision_day_summary',
    'decision_order_failure_set',
    'decision_cold_stock_profile',
    'decision_machine_bottleneck',
    'decision_roll_campaign_alert',
    'decision_capacity_opportunity',
    'decision_refresh_log'
  );

-- 检查索引是否已创建
SELECT
  CASE
    WHEN COUNT(*) >= 18 THEN 'OK: 决策层索引已创建'
    ELSE 'ERROR: 决策层索引缺失，实际: ' || COUNT(*) || ' 个'
  END AS validation_result
FROM sqlite_master
WHERE type='index'
  AND name LIKE 'idx_%'
  AND (
    name LIKE 'idx_day_summary_%' OR
    name LIKE 'idx_order_failure_%' OR
    name LIKE 'idx_cold_stock_%' OR
    name LIKE 'idx_bottleneck_%' OR
    name LIKE 'idx_roll_alert_%' OR
    name LIKE 'idx_capacity_opp_%' OR
    name LIKE 'idx_refresh_log_%'
  );

-- 检查 schema_version
SELECT
  CASE
    WHEN MAX(version) = 4 THEN 'OK: schema_version = 4'
    ELSE 'ERROR: schema_version 不正确，当前: ' || MAX(version)
  END AS validation_result
FROM schema_version;

-- ==========================================
-- 数据刷新协议说明
-- ==========================================
--
-- 1. 刷新触发器映射:
--    - PlanItemChanged → D1, D2, D3, D4, D5, D6
--    - RiskSnapshotUpdated → D1
--    - MaterialStateChanged → D2, D3, D4
--    - CapacityPoolChanged → D4, D6
--    - RollCampaignChanged → D5
--    - VersionCreated → 全部
--    - ManualRefresh → 全部
--
-- 2. 刷新范围控制:
--    - 全量刷新: 删除 version_id 下所有记录，重新计算
--    - 增量刷新: 仅更新受影响的 machine_code + date_range
--
-- 3. 刷新顺序:
--    - D1, D4, D6 依赖 risk_snapshot 和 capacity_pool
--    - D2, D3 依赖 material_state 和 plan_item
--    - D5 依赖 roller_campaign
--    - 建议顺序: D5 → D2, D3 → D4 → D1, D6
--
-- 4. JSON 字段格式:
--    - top_reasons: [{"code": "R001", "msg": "...", "weight": 0.3, "severity": 0.8}]
--    - failure_reasons: ["原因1", "原因2"]
--    - blocking_factors: [{"code": "B001", "description": "...", "affected_count": 10, "affected_weight_t": 100.0, "is_removable": true}]
--    - suggested_actions: ["建议1", "建议2"]
--    - bottleneck_types: ["Capacity", "Structure"]
--    - reasons: [{"code": "BN001", "description": "...", "severity": 0.8, "impact_t": 50.0}]
--    - binding_constraints: [{"constraint_type": "TargetCapacity", "description": "...", "slack_t": 0.0}]
--    - sensitivity: {"soft_constraint_gain_t": 50.0, "target_adjustment_gain_t": 100.0, "structure_optimization_gain_t": 30.0, "total_potential_gain_t": 180.0, "risk_assessment": "LOW"}
--
-- ==========================================

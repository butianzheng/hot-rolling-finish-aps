-- ==========================================
-- v0.6: 宽厚路径规则完整实现（合并版本）
-- ==========================================
-- 依据: Engine_Specs_v0.3_Integrated.md 章节 14-18
-- 功能:
--  1. path_override_pending 表：跨日期/跨机组待人工确认汇总
--  2. material_state 表扩展：人工确认突破标志
--  3. roller_campaign 表扩展：Roll Cycle 锚点状态
--  4. 默认配置初始化：路径规则参数
-- ==========================================
-- 注意:
--  - 本文件合并了原 v0.6_path_override_pending.sql 和 v0.6_path_rule_extension.sql
--  - 执行前请确保已完成 v0.5_strategy_draft.sql
--  - 所有操作幂等（IF NOT EXISTS, INSERT OR IGNORE, ADD COLUMN IF NOT EXISTS）
-- ==========================================

PRAGMA foreign_keys = ON;

-- ==========================================
-- Part 1: path_override_pending 表（路径规则待确认清单）
-- ==========================================
-- 说明:
-- - 本表用于承载"最近一次重算"生成的 PATH_OVERRIDE_REQUIRED 待确认清单
-- - 主键设计为 (version_id, machine_code, material_id)：
--   每个 material 在每个版本+机组仅记录一次，plan_date 表示首次遇到 OVERRIDE_REQUIRED 的日期

CREATE TABLE IF NOT EXISTS path_override_pending (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  plan_date TEXT NOT NULL,
  material_id TEXT NOT NULL REFERENCES material_master(material_id),
  violation_type TEXT NOT NULL,
  urgent_level TEXT NOT NULL,
  width_mm REAL NOT NULL,
  thickness_mm REAL NOT NULL,
  anchor_width_mm REAL NOT NULL,
  anchor_thickness_mm REAL NOT NULL,
  width_delta_mm REAL NOT NULL,
  thickness_delta_mm REAL NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (version_id, machine_code, material_id)
);

-- 索引：支持按版本+日期+机组查询（待确认汇总面板）
CREATE INDEX IF NOT EXISTS idx_path_override_pending_version_date_machine
  ON path_override_pending(version_id, plan_date, machine_code);

-- 索引：支持按版本+机组+日期查询（单机组视图）
CREATE INDEX IF NOT EXISTS idx_path_override_pending_version_machine_date
  ON path_override_pending(version_id, machine_code, plan_date);

-- 索引：支持按版本+物料查询（物料详情页）
CREATE INDEX IF NOT EXISTS idx_path_override_pending_version_material
  ON path_override_pending(version_id, material_id);

-- ==========================================
-- Part 2: material_state 表扩展（人工确认字段）
-- ==========================================
-- 用于记录路径规则突破的人工确认状态

ALTER TABLE material_state ADD COLUMN user_confirmed INTEGER NOT NULL DEFAULT 0;
-- 人工确认突破标志 (1=已确认, 0=未确认)

ALTER TABLE material_state ADD COLUMN user_confirmed_at TEXT DEFAULT NULL;
-- 确认时间 (ISO DATETIME)

ALTER TABLE material_state ADD COLUMN user_confirmed_by TEXT DEFAULT NULL;
-- 确认人

ALTER TABLE material_state ADD COLUMN user_confirmed_reason TEXT DEFAULT NULL;
-- 确认原因（必填，当 user_confirmed=1 时）

-- 索引：支持查询所有待确认的材料
CREATE INDEX IF NOT EXISTS idx_material_state_user_confirmed
  ON material_state(user_confirmed) WHERE user_confirmed = 0;

-- ==========================================
-- Part 3: roller_campaign 表扩展（路径锚点字段）
-- ==========================================
-- 用于记录当前换辊周期内的路径锚点状态

ALTER TABLE roller_campaign ADD COLUMN path_anchor_material_id TEXT DEFAULT NULL;
-- 路径锚点材料ID (可为空，S2 种子策略时无关联材料)

ALTER TABLE roller_campaign ADD COLUMN path_anchor_width_mm REAL DEFAULT NULL;
-- 锚点宽度 (mm)

ALTER TABLE roller_campaign ADD COLUMN path_anchor_thickness_mm REAL DEFAULT NULL;
-- 锚点厚度 (mm)

ALTER TABLE roller_campaign ADD COLUMN anchor_source TEXT DEFAULT NULL;
-- 锚点来源类型: FROZEN_LAST / LOCKED_LAST / USER_CONFIRMED_LAST / SEED_S2 / NONE

-- 索引：支持查询特定锚点来源的周期
CREATE INDEX IF NOT EXISTS idx_roller_campaign_anchor_source
  ON roller_campaign(anchor_source) WHERE anchor_source IS NOT NULL;

-- ==========================================
-- Part 4: 默认配置项初始化（插入到 config_kv）
-- ==========================================
-- 注意: 仅在配置项不存在时插入

-- 创建 GLOBAL scope（如果不存在）
INSERT OR IGNORE INTO config_scope (scope_id, scope_type, scope_key, created_at)
  VALUES ('global', 'GLOBAL', 'GLOBAL', datetime('now', 'localtime'));

-- 插入路径规则配置项
INSERT OR IGNORE INTO config_kv (scope_id, key, value, updated_at)
  VALUES
    ('global', 'path_rule_enabled', 'true', datetime('now', 'localtime')),
    ('global', 'path_width_tolerance_mm', '50.0', datetime('now', 'localtime')),
    ('global', 'path_thickness_tolerance_mm', '1.0', datetime('now', 'localtime')),
    ('global', 'path_override_allowed_urgency_levels', 'L2,L3', datetime('now', 'localtime')),
    ('global', 'seed_s2_percentile', '0.95', datetime('now', 'localtime')),
    ('global', 'seed_s2_small_sample_threshold', '10', datetime('now', 'localtime'));

-- ==========================================
-- Part 5: 更新 schema_version
-- ==========================================

INSERT OR IGNORE INTO schema_version (version, applied_at)
  VALUES (6, datetime('now'));

-- ==========================================
-- 迁移说明
-- ==========================================
-- 1. 执行顺序：本文件需在 v0.5_strategy_draft.sql 之后执行
-- 2. 幂等性：所有操作使用 IF NOT EXISTS / INSERT OR IGNORE，可安全重复执行
-- 3. 数据兼容：
--    - 所有新增字段设置 NULL/0 默认值，兼容现有数据
--    - user_confirmed 默认为 0，表示未确认
--    - 锚点字段默认为 NULL，表示未设置锚点
-- 4. 回滚：如需回滚，请手工删除新增表和字段（建议使用备份恢复）
-- ==========================================

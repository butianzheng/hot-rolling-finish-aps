-- ==========================================
-- v0.6: path_rule_extension (宽厚路径规则扩展)
-- ==========================================
-- 依据: Engine_Specs_v0.3_Integrated.md 章节 14-18
-- 功能: 宽厚路径规则 + Roll Cycle 锚点 + 人工确认突破
-- ==========================================

PRAGMA foreign_keys = ON;

-- ==========================================
-- 1. material_state 表: 新增人工确认字段
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

-- ==========================================
-- 2. roller_campaign 表: 新增路径锚点字段
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

-- ==========================================
-- 3. 索引优化
-- ==========================================

-- 人工确认待处理查询优化
CREATE INDEX IF NOT EXISTS idx_material_state_user_confirmed
ON material_state(user_confirmed) WHERE user_confirmed = 0;

-- 锚点来源查询优化
CREATE INDEX IF NOT EXISTS idx_roller_campaign_anchor_source
ON roller_campaign(anchor_source) WHERE anchor_source IS NOT NULL;

-- ==========================================
-- 4. 默认配置项初始化 (插入到 config_kv)
-- ==========================================
-- 注意: 仅在配置项不存在时插入

-- 创建 GLOBAL scope（如果不存在）
INSERT OR IGNORE INTO config_scope (scope_id, scope_type, scope_key, created_at)
VALUES ('global', 'GLOBAL', 'GLOBAL', datetime('now', 'localtime'));

INSERT OR IGNORE INTO config_kv (scope_id, key, value, updated_at)
VALUES
  ('global', 'path_rule_enabled', 'true', datetime('now', 'localtime')),
  ('global', 'path_width_tolerance_mm', '50.0', datetime('now', 'localtime')),
  ('global', 'path_thickness_tolerance_mm', '1.0', datetime('now', 'localtime')),
  ('global', 'path_override_allowed_urgency_levels', 'L2,L3', datetime('now', 'localtime')),
  ('global', 'seed_s2_percentile', '0.95', datetime('now', 'localtime')),
  ('global', 'seed_s2_small_sample_threshold', '10', datetime('now', 'localtime'));

-- ==========================================
-- 5. 更新 schema_version
-- ==========================================

INSERT OR IGNORE INTO schema_version (version, applied_at)
  VALUES (6, datetime('now'));

-- ==========================================
-- 迁移说明
-- ==========================================
-- 1. 所有新增字段设置 NULL/0 默认值，兼容现有数据
-- 2. user_confirmed 默认为 0，表示未确认
-- 3. 配置项使用 INSERT OR IGNORE 避免重复插入
-- 4. 索引使用 IF NOT EXISTS 确保幂等性

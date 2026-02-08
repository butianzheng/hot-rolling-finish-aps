-- ==========================================
-- v0.9: 物料管理机组覆盖异常阈值配置
-- ==========================================
-- 目的:
--  1. 将“机组覆盖异常阈值”纳入设置中心可配置项（config_kv）
--  2. 默认值 4
-- ==========================================

PRAGMA foreign_keys = ON;

-- 创建 GLOBAL scope（如果不存在）
INSERT OR IGNORE INTO config_scope (scope_id, scope_type, scope_key, created_at)
  VALUES ('global', 'GLOBAL', 'GLOBAL', datetime('now', 'localtime'));

-- 配置项初始化（不存在时插入）
INSERT OR IGNORE INTO config_kv (scope_id, key, value, updated_at)
  VALUES ('global', 'material_management_coverage_alert_threshold', '4', datetime('now', 'localtime'));

-- 更新 schema_version
INSERT INTO schema_version (version, applied_at)
VALUES (9, datetime('now'));


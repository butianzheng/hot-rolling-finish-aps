-- ==========================================
-- v0.10: 连续排程兜底阈值配置
-- ==========================================
-- 目的:
--  1. 将“空白日自动换周期重试”的触发阈值纳入设置中心可配置项（config_kv）
--  2. 默认值 200 吨
-- ==========================================

PRAGMA foreign_keys = ON;

-- 创建 GLOBAL scope（如果不存在）
INSERT OR IGNORE INTO config_scope (scope_id, scope_type, scope_key, created_at)
  VALUES ('global', 'GLOBAL', 'GLOBAL', datetime('now', 'localtime'));

-- 配置项初始化（不存在时插入）
INSERT OR IGNORE INTO config_kv (scope_id, key, value, updated_at)
  VALUES ('global', 'empty_day_recover_threshold_t', '200', datetime('now', 'localtime'));

-- 更新 schema_version
INSERT INTO schema_version (version, applied_at)
VALUES (10, datetime('now'));

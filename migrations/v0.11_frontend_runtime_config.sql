-- ==========================================
-- v0.11: 前端运行治理参数配置化
-- ==========================================
-- 目的：
--  1) latest_run_ttl_ms：latest run 前端状态 TTL（毫秒）
--  2) stale_plan_rev_toast_cooldown_ms：STALE_PLAN_REV 提示冷却（毫秒）

BEGIN TRANSACTION;

INSERT OR IGNORE INTO config_kv (scope_id, key, value, updated_at)
  VALUES ('global', 'latest_run_ttl_ms', '120000', datetime('now', 'localtime'));

INSERT OR IGNORE INTO config_kv (scope_id, key, value, updated_at)
  VALUES ('global', 'stale_plan_rev_toast_cooldown_ms', '4000', datetime('now', 'localtime'));

INSERT OR IGNORE INTO schema_version (version, applied_at)
  VALUES (11, datetime('now', 'localtime'));

COMMIT;


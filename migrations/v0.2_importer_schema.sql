-- ==========================================
-- Migration v0.2: Importer Schema Enhancements
-- ==========================================
-- 依据: Field_Mapping_Spec_v0.3_Integrated.md - 催料组合规则
-- 依据: data_dictionary_v0.1.md - 审计增强
-- 变更:
--   1) material_master 添加催料字段（contract_nature, weekly_delivery_flag, export_flag）
--   2) 新增 import_batch 表（批次元信息 + DQ 报告）
--   3) import_conflict 表添加 row_number 字段
-- ==========================================

PRAGMA foreign_keys = ON;

-- ==========================================
-- 1. 添加 material_master 催料字段
-- ==========================================
-- 用途: 支持催料组合规则（rush_level 派生）

ALTER TABLE material_master ADD COLUMN contract_nature TEXT;
ALTER TABLE material_master ADD COLUMN weekly_delivery_flag TEXT;
ALTER TABLE material_master ADD COLUMN export_flag TEXT;

-- 催料字段索引（可选，用于查询优化）
CREATE INDEX IF NOT EXISTS idx_material_rush_fields
  ON material_master(contract_nature, weekly_delivery_flag, export_flag);

-- ==========================================
-- 2. 新增 import_batch 表
-- ==========================================
-- 用途: 记录每批导入的元信息与 DQ 报告
-- 红线: 符合 Master Spec A3 审计增强原则

CREATE TABLE IF NOT EXISTS import_batch (
  batch_id TEXT PRIMARY KEY,              -- 批次 ID（UUID v4）
  file_name TEXT NOT NULL,                -- 源文件名（含扩展名）
  file_path TEXT,                         -- 源文件路径（可选）
  total_rows INTEGER NOT NULL,            -- 总行数（不含表头）
  success_rows INTEGER NOT NULL DEFAULT 0, -- 成功导入行数
  blocked_rows INTEGER NOT NULL DEFAULT 0, -- 阻断行数（DQ ERROR）
  warning_rows INTEGER NOT NULL DEFAULT 0, -- 警告行数（DQ WARNING）
  conflict_rows INTEGER NOT NULL DEFAULT 0, -- 冲突行数（进入 import_conflict）
  imported_at TEXT NOT NULL DEFAULT (datetime('now')), -- 导入时间戳
  imported_by TEXT,                       -- 导入人（用户 ID/系统标识）
  elapsed_ms INTEGER,                     -- 导入耗时（毫秒）
  dq_report_json TEXT                     -- DQ 报告完整 JSON（含违规明细）
);

-- 按导入时间索引（用于查询最近导入批次）
CREATE INDEX IF NOT EXISTS idx_batch_imported_at ON import_batch(imported_at DESC);

-- 按文件名索引（用于去重检查）
CREATE INDEX IF NOT EXISTS idx_batch_filename ON import_batch(file_name);

-- ==========================================
-- 3. 增强 import_conflict 表
-- ==========================================
-- 变更: 添加 row_number 字段（用于 DQ 报告定位）

ALTER TABLE import_conflict ADD COLUMN row_number INTEGER;

-- 更新现有记录的 row_number 为 NULL（向下兼容）
-- 新记录必须填写 row_number

-- ==========================================
-- 4. 更新 schema_version
-- ==========================================

INSERT INTO schema_version (version, applied_at)
  VALUES (2, datetime('now'));

-- ==========================================
-- 5. 初始化 config_kv（导入模块配置）
-- ==========================================
-- 依据: Engine_Specs_v0.3_Integrated.md - 11. 配置项全集

-- 创建 GLOBAL scope（如果不存在）
INSERT OR IGNORE INTO config_scope (scope_id, scope_type, scope_key, created_at)
  VALUES ('global', 'GLOBAL', 'GLOBAL', datetime('now'));

-- 季节与适温配置
INSERT OR IGNORE INTO config_kv (scope_id, key, value, updated_at) VALUES
  ('global', 'season_mode', 'MANUAL', datetime('now')),
  ('global', 'manual_season', 'WINTER', datetime('now')),
  ('global', 'winter_months', '11,12,1,2,3', datetime('now')),
  ('global', 'min_temp_days_winter', '3', datetime('now')),
  ('global', 'min_temp_days_summer', '4', datetime('now'));

-- 机组代码配置
INSERT OR IGNORE INTO config_kv (scope_id, key, value, updated_at) VALUES
  ('global', 'standard_finishing_machines', 'H032,H033,H034', datetime('now')),
  ('global', 'machine_offset_days', '4', datetime('now'));

-- 数据质量配置
INSERT OR IGNORE INTO config_kv (scope_id, key, value, updated_at) VALUES
  ('global', 'weight_anomaly_threshold', '100.0', datetime('now')),
  ('global', 'batch_retention_days', '90', datetime('now'));

-- ==========================================
-- 6. 验证 migration 成功
-- ==========================================
-- 检查 material_master 新字段
SELECT
  CASE
    WHEN COUNT(*) = 3 THEN 'OK: material_master 催料字段已添加'
    ELSE 'ERROR: material_master 催料字段缺失'
  END AS validation_result
FROM pragma_table_info('material_master')
WHERE name IN ('contract_nature', 'weekly_delivery_flag', 'export_flag');

-- 检查 import_batch 表
SELECT
  CASE
    WHEN COUNT(*) > 0 THEN 'OK: import_batch 表已创建'
    ELSE 'ERROR: import_batch 表缺失'
  END AS validation_result
FROM sqlite_master
WHERE type='table' AND name='import_batch';

-- 检查 schema_version
SELECT
  CASE
    WHEN MAX(version) = 2 THEN 'OK: schema_version = 2'
    ELSE 'ERROR: schema_version 不正确'
  END AS validation_result
FROM schema_version;

-- ==========================================
-- v0.7: 适温动态计算增强（添加轧制产出日期）
-- ==========================================
-- 依据: Engine_Specs_v0.3_Integrated.md 章节 2.1 适温准入规则
-- 功能:
--  1. material_master 表新增 rolling_output_date TEXT 字段
--  2. 历史数据回填逻辑（推算产出日期）
--  3. 数据完整性验证
-- ==========================================
-- 背景:
--  - 问题: output_age_days_raw 为导入时刻静态快照，无法随时间推进
--  - 示例: 1月14日导入时 output_age_days_raw=1，1月20日重排产时仍为1（错误）
--  - 解决: 新增 rolling_output_date（轧制产出日期），动态计算实际天数
--    → actual_age_days = today - rolling_output_date
-- ==========================================

PRAGMA foreign_keys = ON;

-- ==========================================
-- Part 1: 添加 rolling_output_date 字段
-- ==========================================
-- 字段定义:
--  - rolling_output_date TEXT: 轧制产出日期（ISO DATE，YYYY-MM-DD）
--  - NULL 值含义: 历史数据或 output_age_days_raw 缺失
--  - 派生规则: rolling_output_date = import_date - output_age_days_raw

ALTER TABLE material_master
ADD COLUMN rolling_output_date TEXT;

-- 创建索引（支持适温查询优化）
CREATE INDEX IF NOT EXISTS idx_material_rolling_output_date
  ON material_master(rolling_output_date);

-- ==========================================
-- Part 2: 历史数据回填逻辑
-- ==========================================
-- 策略:
--  1. 使用 created_at 作为导入时间基准
--  2. 回填公式: rolling_output_date = date(created_at) - output_age_days_raw
--  3. 仅回填 output_age_days_raw 有效的记录

UPDATE material_master
SET rolling_output_date = date(
    created_at,
    '-' || CAST(output_age_days_raw AS TEXT) || ' days'
)
WHERE output_age_days_raw IS NOT NULL
  AND output_age_days_raw >= 0
  AND rolling_output_date IS NULL;

-- ==========================================
-- Part 3: 数据完整性验证
-- ==========================================
-- 验证 1: 检查回填覆盖率
-- 预期: 所有有 output_age_days_raw 的记录都应有 rolling_output_date
SELECT
  '验证 1: 回填覆盖率' AS check_name,
  COUNT(*) AS total_materials,
  SUM(CASE WHEN output_age_days_raw IS NOT NULL AND rolling_output_date IS NULL THEN 1 ELSE 0 END) AS missing_dates,
  SUM(CASE WHEN rolling_output_date IS NOT NULL THEN 1 ELSE 0 END) AS backfilled_dates,
  ROUND(100.0 * SUM(CASE WHEN rolling_output_date IS NOT NULL THEN 1 ELSE 0 END) /
    NULLIF(SUM(CASE WHEN output_age_days_raw IS NOT NULL THEN 1 ELSE 0 END), 0), 2) AS coverage_pct
FROM material_master;

-- 验证 2: 检查日期合理性（产出日期不应晚于创建日期）
SELECT
  '验证 2: 日期合理性（产出日期晚于创建日期的异常数据）' AS check_name,
  COUNT(*) AS anomaly_count
FROM material_master
WHERE rolling_output_date IS NOT NULL
  AND rolling_output_date > date(created_at);

-- 如有异常，列出前5条（调试用）
SELECT
  '异常数据示例（前5条）:' AS info,
  material_id,
  rolling_output_date,
  date(created_at) AS created_date,
  output_age_days_raw
FROM material_master
WHERE rolling_output_date IS NOT NULL
  AND rolling_output_date > date(created_at)
LIMIT 5;

-- ==========================================
-- Part 4: 更新 schema_version
-- ==========================================
INSERT INTO schema_version (version, applied_at)
VALUES (7, datetime('now'));

-- ==========================================
-- 迁移完成
-- ==========================================
-- 验证步骤:
--  1. 检查字段已添加: PRAGMA table_info(material_master);
--  2. 检查回填覆盖率: 运行验证 1 查询
--  3. 检查数据合理性: 运行验证 2 查询（应返回 0 异常）
--  4. 检查版本更新: SELECT * FROM schema_version;
-- ==========================================

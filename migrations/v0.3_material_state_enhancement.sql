-- ==========================================
-- Migration v0.3: MaterialState Schema Enhancement
-- ==========================================
-- 目的: 对齐 Domain 层 MaterialState 与数据库 Schema
-- 依据: Claude_Dev_Master_Spec.md - PART C 数据与状态体系
-- 依据: Engine_Specs_v0.3_Integrated.md - material_state 完整字段
-- 变更:
--   1) 添加 rush_level（催料等级，中间变量）
--   2) 添加 stock_age_days（库存压力）
--   3) 添加排产落位字段（scheduled_date, scheduled_machine_code, seq_no）
--   4) 添加人工干预标志（manual_urgent_flag, in_frozen_zone）
--   5) 添加审计字段（updated_by）
--   6) 添加性能优化索引
-- ==========================================

PRAGMA foreign_keys = ON;

-- ==========================================
-- 1. 添加 MaterialState 缺失字段
-- ==========================================

-- 催料等级（中间变量，用于紧急等级计算）
-- 值域: L0/L1/L2
ALTER TABLE material_state ADD COLUMN rush_level TEXT;

-- 库存压力（状态时间，天）
-- 用途: 库存压力主口径，从 material_master.stock_age_days 同步
ALTER TABLE material_state ADD COLUMN stock_age_days INTEGER;

-- 排产落位字段（由 Capacity Filler 写入）
-- 用途: 记录材料已排产的日期、机组、顺序号
ALTER TABLE material_state ADD COLUMN scheduled_date TEXT;
ALTER TABLE material_state ADD COLUMN scheduled_machine_code TEXT;
ALTER TABLE material_state ADD COLUMN seq_no INTEGER;

-- 人工干预标志
-- manual_urgent_flag: 人工红线标志（1=人工强制L3，0=系统计算）
-- in_frozen_zone: 冻结区标志（1=在冻结区，0=不在冻结区）
ALTER TABLE material_state ADD COLUMN manual_urgent_flag INTEGER NOT NULL DEFAULT 0;
ALTER TABLE material_state ADD COLUMN in_frozen_zone INTEGER NOT NULL DEFAULT 0;

-- 操作人审计字段
-- 用途: 记录最后修改人（用户ID/系统标识）
ALTER TABLE material_state ADD COLUMN updated_by TEXT;

-- ==========================================
-- 2. 添加索引（性能优化）
-- ==========================================

-- 按排产日期查询索引
-- 用途: 查询指定日期的排产材料
CREATE INDEX IF NOT EXISTS idx_state_scheduled_date
  ON material_state(scheduled_date);

-- 按机组+日期查询索引
-- 用途: 查询指定机组+日期的排产材料（日计划视图）
CREATE INDEX IF NOT EXISTS idx_state_machine_date
  ON material_state(scheduled_machine_code, scheduled_date);

-- 冻结区材料查询索引（部分索引）
-- 用途: 快速查询冻结区材料（红线保护）
CREATE INDEX IF NOT EXISTS idx_state_frozen
  ON material_state(in_frozen_zone)
  WHERE in_frozen_zone = 1;

-- 人工红线材料查询索引（部分索引）
-- 用途: 快速查询人工红线材料（优先级最高）
CREATE INDEX IF NOT EXISTS idx_state_manual_urgent
  ON material_state(manual_urgent_flag)
  WHERE manual_urgent_flag = 1;

-- ==========================================
-- 3. 更新 schema_version
-- ==========================================

INSERT INTO schema_version (version, applied_at)
  VALUES (3, datetime('now'));

-- ==========================================
-- 4. 验证 migration 成功
-- ==========================================

-- 检查新字段是否已添加
SELECT
  CASE
    WHEN COUNT(*) = 8 THEN 'OK: material_state 新字段已添加（8个字段）'
    ELSE 'ERROR: material_state 新字段缺失，实际: ' || COUNT(*) || ' 个'
  END AS validation_result
FROM pragma_table_info('material_state')
WHERE name IN (
  'rush_level',
  'stock_age_days',
  'scheduled_date',
  'scheduled_machine_code',
  'seq_no',
  'manual_urgent_flag',
  'in_frozen_zone',
  'updated_by'
);

-- 检查索引是否已创建
SELECT
  CASE
    WHEN COUNT(*) >= 4 THEN 'OK: material_state 索引已创建'
    ELSE 'ERROR: material_state 索引缺失'
  END AS validation_result
FROM sqlite_master
WHERE type='index'
  AND tbl_name='material_state'
  AND name IN (
    'idx_state_scheduled_date',
    'idx_state_machine_date',
    'idx_state_frozen',
    'idx_state_manual_urgent'
  );

-- 检查 schema_version
SELECT
  CASE
    WHEN MAX(version) = 3 THEN 'OK: schema_version = 3'
    ELSE 'ERROR: schema_version 不正确，当前: ' || MAX(version)
  END AS validation_result
FROM schema_version;

-- ==========================================
-- 5. 数据迁移说明
-- ==========================================
-- 注意: 现有 material_state 记录的新字段将为 NULL
-- 建议: 执行 migration 后，运行 Recalc Engine 重新计算所有材料状态
-- ==========================================

-- ==========================================
-- Migration 001: capacity_pool 版本化改造
-- ==========================================
-- P1-1: 为 capacity_pool 增加 version_id 字段
-- 主键从 (machine_code, plan_date) 改为 (version_id, machine_code, plan_date)
-- 避免跨版本产能污染问题
-- ==========================================

-- 开启外键约束
PRAGMA foreign_keys = OFF;

-- 1. 创建新表（带 version_id）
CREATE TABLE IF NOT EXISTS capacity_pool_new (
  version_id TEXT NOT NULL,
  machine_code TEXT NOT NULL,
  plan_date TEXT NOT NULL,
  target_capacity_t REAL NOT NULL,
  limit_capacity_t REAL NOT NULL,
  used_capacity_t REAL NOT NULL DEFAULT 0.0,
  overflow_t REAL NOT NULL DEFAULT 0.0,
  frozen_capacity_t REAL NOT NULL DEFAULT 0.0,
  accumulated_tonnage_t REAL NOT NULL DEFAULT 0.0,
  roll_campaign_id TEXT,
  PRIMARY KEY (version_id, machine_code, plan_date)
);

-- 2. 迁移数据：将旧 capacity_pool 数据复制到新表
-- 使用当前激活版本 ID（status = 'ACTIVE' 的最新版本）
-- 如果没有激活版本，则使用最新版本
INSERT INTO capacity_pool_new (
  version_id,
  machine_code,
  plan_date,
  target_capacity_t,
  limit_capacity_t,
  used_capacity_t,
  overflow_t,
  frozen_capacity_t,
  accumulated_tonnage_t,
  roll_campaign_id
)
SELECT
  COALESCE(
    (SELECT version_id FROM plan_version WHERE status = 'ACTIVE' ORDER BY created_at DESC LIMIT 1),
    (SELECT version_id FROM plan_version ORDER BY created_at DESC LIMIT 1),
    'DEFAULT_VERSION'
  ) AS version_id,
  machine_code,
  plan_date,
  target_capacity_t,
  limit_capacity_t,
  used_capacity_t,
  overflow_t,
  frozen_capacity_t,
  accumulated_tonnage_t,
  roll_campaign_id
FROM capacity_pool;

-- 3. 删除旧表
DROP TABLE IF EXISTS capacity_pool;

-- 4. 重命名新表
ALTER TABLE capacity_pool_new RENAME TO capacity_pool;

-- 5. 创建索引
CREATE INDEX IF NOT EXISTS idx_pool_version_machine_date ON capacity_pool(version_id, machine_code, plan_date);
CREATE INDEX IF NOT EXISTS idx_pool_machine_date ON capacity_pool(machine_code, plan_date);

-- 6. 恢复外键约束（注意：SQLite 中外键约束需要在表创建时定义，或通过重建表添加）
-- 由于 SQLite 限制，外键约束需要在新建数据库时通过 schema.sql 定义
PRAGMA foreign_keys = ON;

-- ==========================================
-- 验证迁移
-- ==========================================
-- 查询迁移后的数据量
-- SELECT COUNT(*) AS migrated_rows FROM capacity_pool;
-- 查询迁移后的版本分布
-- SELECT version_id, COUNT(*) AS row_count FROM capacity_pool GROUP BY version_id;

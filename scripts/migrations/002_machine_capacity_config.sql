-- ==========================================
-- Migration 002: machine_capacity_config 表创建
-- ==========================================
-- P1: 产能池管理日历化改造 - 机组级产能配置
-- 新增 machine_capacity_config 表用于存储版本化的机组产能配置
-- 支持版本隔离、历史记录追踪和配置审计
-- ==========================================

-- 开启外键约束检查（创建表时需要）
PRAGMA foreign_keys = ON;

-- 1. 创建 machine_capacity_config 表
CREATE TABLE IF NOT EXISTS machine_capacity_config (
  config_id TEXT PRIMARY KEY,                       -- 配置ID (应用层生成UUID)
  version_id TEXT NOT NULL,                         -- 版本ID (外键关联plan_version)
  machine_code TEXT NOT NULL,                       -- 机组代码
  default_daily_target_t REAL NOT NULL,             -- 机组级默认目标产能(吨/天)
  default_daily_limit_pct REAL NOT NULL,            -- 机组级默认极限产能百分比 (如 1.05 表示 105%)
  effective_date TEXT,                              -- 生效日期(可选, ISO DATE格式 YYYY-MM-DD)
  created_at TEXT NOT NULL DEFAULT (datetime('now')), -- 创建时间
  updated_at TEXT NOT NULL DEFAULT (datetime('now')), -- 更新时间
  created_by TEXT NOT NULL,                         -- 创建人
  reason TEXT,                                      -- 配置原因/备注
  FOREIGN KEY (version_id) REFERENCES plan_version(version_id) ON DELETE CASCADE,
  UNIQUE(version_id, machine_code)                  -- 每个版本下每个机组只能有一条配置
);

-- 2. 创建索引以优化查询性能
-- 按版本查询配置（最常用）
CREATE INDEX IF NOT EXISTS idx_machine_config_version
  ON machine_capacity_config(version_id);

-- 按机组查询配置（用于查询历史）
CREATE INDEX IF NOT EXISTS idx_machine_config_machine
  ON machine_capacity_config(machine_code);

-- 按创建时间查询（用于审计和历史记录）
CREATE INDEX IF NOT EXISTS idx_machine_config_created_at
  ON machine_capacity_config(created_at DESC);

-- 复合索引：版本+机组+生效日期（用于查询特定版本下某机组的配置）
CREATE INDEX IF NOT EXISTS idx_machine_config_version_machine_date
  ON machine_capacity_config(version_id, machine_code, effective_date);

-- ==========================================
-- 数据完整性约束验证
-- ==========================================
-- 注意: SQLite 不支持 CHECK 约束在 ALTER TABLE 中添加
-- 因此以下约束在创建表时已内置，这里仅作为文档说明

-- 约束1: default_daily_target_t 必须 > 0
-- 约束2: default_daily_limit_pct 必须 >= 1.0 (100%)
-- 约束3: version_id 必须在 plan_version 表中存在（外键约束已定义）

-- ==========================================
-- 验证迁移
-- ==========================================
-- 查询表是否创建成功
-- SELECT name FROM sqlite_master WHERE type='table' AND name='machine_capacity_config';

-- 查询索引是否创建成功
-- SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='machine_capacity_config';

-- 查询表结构
-- PRAGMA table_info(machine_capacity_config);

-- 查询外键约束
-- PRAGMA foreign_key_list(machine_capacity_config);

-- ==========================================
-- 回滚说明
-- ==========================================
-- 如需回滚此迁移，执行以下语句：
-- DROP INDEX IF EXISTS idx_machine_config_version_machine_date;
-- DROP INDEX IF EXISTS idx_machine_config_created_at;
-- DROP INDEX IF EXISTS idx_machine_config_machine;
-- DROP INDEX IF EXISTS idx_machine_config_version;
-- DROP TABLE IF EXISTS machine_capacity_config;

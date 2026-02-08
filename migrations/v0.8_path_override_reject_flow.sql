-- ==========================================
-- v0.8: 路径规则“拒绝”闭环
-- ==========================================
-- 功能:
--  1. material_state 扩展拒绝字段（不新增实体表）
--  2. 索引支持拒绝态筛选
-- ==========================================

PRAGMA foreign_keys = ON;

-- ==========================================
-- Part 1: material_state 扩展拒绝字段
-- ==========================================

ALTER TABLE material_state ADD COLUMN path_override_rejected INTEGER NOT NULL DEFAULT 0;
-- 路径规则拒绝标志（1=已拒绝，0=未拒绝）

ALTER TABLE material_state ADD COLUMN path_override_rejected_at TEXT DEFAULT NULL;
-- 拒绝时间（RFC3339）

ALTER TABLE material_state ADD COLUMN path_override_rejected_by TEXT DEFAULT NULL;
-- 拒绝操作人

ALTER TABLE material_state ADD COLUMN path_override_rejected_reason TEXT DEFAULT NULL;
-- 拒绝原因

ALTER TABLE material_state ADD COLUMN path_override_reject_cycle_no INTEGER DEFAULT NULL;
-- 拒绝时所属换辊周期号（用于“至少后移一套换辊周期”）

ALTER TABLE material_state ADD COLUMN path_override_reject_base_sched_state TEXT DEFAULT NULL;
-- 拒绝时原始排程状态（用于“提升一档”判定）

CREATE INDEX IF NOT EXISTS idx_material_state_path_override_rejected
  ON material_state(path_override_rejected) WHERE path_override_rejected = 1;

-- ==========================================
-- Part 2: 更新 schema_version
-- ==========================================

INSERT INTO schema_version (version, applied_at)
VALUES (8, datetime('now'));


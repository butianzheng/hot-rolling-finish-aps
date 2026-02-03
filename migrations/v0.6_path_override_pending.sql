-- v0.6+: 路径规则待人工确认（跨日期/跨机组汇总）
-- 说明:
-- - 本表用于承载“最近一次重算”生成的 PATH_OVERRIDE_REQUIRED 待确认清单
-- - 主键设计为 (version_id, machine_code, material_id)：
--   每个 material 在每个版本+机组仅记录一次，plan_date 表示首次遇到 OVERRIDE_REQUIRED 的日期

CREATE TABLE IF NOT EXISTS path_override_pending (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  plan_date TEXT NOT NULL,
  material_id TEXT NOT NULL REFERENCES material_master(material_id),
  violation_type TEXT NOT NULL,
  urgent_level TEXT NOT NULL,
  width_mm REAL NOT NULL,
  thickness_mm REAL NOT NULL,
  anchor_width_mm REAL NOT NULL,
  anchor_thickness_mm REAL NOT NULL,
  width_delta_mm REAL NOT NULL,
  thickness_delta_mm REAL NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (version_id, machine_code, material_id)
);

CREATE INDEX IF NOT EXISTS idx_path_override_pending_version_date_machine
  ON path_override_pending(version_id, plan_date, machine_code);
CREATE INDEX IF NOT EXISTS idx_path_override_pending_version_machine_date
  ON path_override_pending(version_id, machine_code, plan_date);
CREATE INDEX IF NOT EXISTS idx_path_override_pending_version_material
  ON path_override_pending(version_id, material_id);


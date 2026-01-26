-- SQLite Schema v0.1
-- Project: 热轧精整机组优先级驱动排产决策支持系统
-- Notes:
-- 1) Contract is not a separate entity in MVP; contract fields are stored in material_master.
-- 2) Weight unit: ton (t), keep 3 decimals.
-- 3) current_machine_code: prefer rework machine when 精整返修机组 not null (Option A).
-- 4) Import duplicate strategy: Option C (conflict list, manual resolution).

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_version (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS machine_master (
  machine_code TEXT PRIMARY KEY,
  machine_name TEXT,
  hourly_capacity_t REAL,
  default_daily_target_t REAL,
  default_daily_limit_pct REAL,
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS material_master (
  material_id TEXT PRIMARY KEY,
  manufacturing_order_id TEXT,
  contract_no TEXT,
  due_date TEXT,
  rush_flag TEXT,
  next_machine_code TEXT,
  rework_machine_code TEXT,
  current_machine_code TEXT,
  width_mm REAL,
  thickness_mm REAL,
  length_m REAL,
  weight_t REAL,
  available_width_mm REAL,
  steel_mark TEXT,
  slab_id TEXT,
  material_status_code_src TEXT,
  status_updated_at TEXT,
  output_age_days_raw INTEGER,
  stock_age_days INTEGER,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_material_machine ON material_master(current_machine_code);
CREATE INDEX IF NOT EXISTS idx_material_due ON material_master(due_date);
CREATE INDEX IF NOT EXISTS idx_material_status_updated ON material_master(status_updated_at);

CREATE TABLE IF NOT EXISTS material_state (
  material_id TEXT PRIMARY KEY REFERENCES material_master(material_id) ON DELETE CASCADE,
  sched_state TEXT NOT NULL,
  lock_flag INTEGER NOT NULL DEFAULT 0,
  force_release_flag INTEGER NOT NULL DEFAULT 0,
  urgent_level TEXT,
  urgent_reason TEXT,
  rolling_output_age_days INTEGER,
  ready_in_days INTEGER,
  earliest_sched_date TEXT,
  last_calc_version_id TEXT,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_state_sched_state ON material_state(sched_state);
CREATE INDEX IF NOT EXISTS idx_state_urgent ON material_state(urgent_level);
CREATE INDEX IF NOT EXISTS idx_state_earliest ON material_state(earliest_sched_date);

CREATE TABLE IF NOT EXISTS capacity_pool (
  pool_id TEXT PRIMARY KEY,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  plan_date TEXT NOT NULL,
  design_capacity_t REAL,
  target_capacity_t REAL NOT NULL,
  limit_capacity_t REAL NOT NULL,
  manual_adjust_reason TEXT,
  structure_template_id TEXT,
  is_frozen INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(machine_code, plan_date)
);

CREATE INDEX IF NOT EXISTS idx_pool_machine_date ON capacity_pool(machine_code, plan_date);

CREATE TABLE IF NOT EXISTS plan (
  plan_id TEXT PRIMARY KEY,
  plan_name TEXT NOT NULL,
  plan_type TEXT NOT NULL,
  base_plan_id TEXT,
  created_by TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS plan_version (
  version_id TEXT PRIMARY KEY,
  plan_id TEXT NOT NULL REFERENCES plan(plan_id) ON DELETE CASCADE,
  version_no INTEGER NOT NULL,
  status TEXT NOT NULL,
  frozen_from_date TEXT,
  recalc_window_days INTEGER,
  config_snapshot_json TEXT,
  created_by TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  revision INTEGER NOT NULL DEFAULT 0,  -- 乐观锁：版本修订号
  UNIQUE(plan_id, version_no)
);

CREATE INDEX IF NOT EXISTS idx_version_plan ON plan_version(plan_id, version_no);

CREATE TABLE IF NOT EXISTS plan_item (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  material_id TEXT NOT NULL REFERENCES material_master(material_id),
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  plan_date TEXT NOT NULL,
  seq_no INTEGER NOT NULL,
  weight_t REAL NOT NULL,
  source_type TEXT NOT NULL,
  locked_in_plan INTEGER NOT NULL DEFAULT 0,
  force_release_in_plan INTEGER NOT NULL DEFAULT 0,
  violation_flags TEXT,
  PRIMARY KEY (version_id, material_id)
);

CREATE INDEX IF NOT EXISTS idx_item_version_machine_date ON plan_item(version_id, machine_code, plan_date, seq_no);

CREATE TABLE IF NOT EXISTS roller_campaign (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  campaign_no INTEGER NOT NULL,
  start_date TEXT NOT NULL,
  end_date TEXT,
  cum_weight_t REAL NOT NULL DEFAULT 0,
  suggest_threshold_t REAL NOT NULL,
  hard_limit_t REAL NOT NULL,
  status TEXT NOT NULL,
  PRIMARY KEY (version_id, machine_code, campaign_no)
);

CREATE INDEX IF NOT EXISTS idx_campaign_version_machine ON roller_campaign(version_id, machine_code);

CREATE TABLE IF NOT EXISTS risk_snapshot (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  plan_date TEXT NOT NULL,
  risk_level TEXT NOT NULL,
  risk_reasons TEXT,
  target_capacity_t REAL NOT NULL,
  used_capacity_t REAL NOT NULL,
  limit_capacity_t REAL NOT NULL,
  overflow_t REAL NOT NULL,
  urgent_total_t REAL NOT NULL,
  mature_backlog_t REAL NOT NULL,
  immature_backlog_t REAL NOT NULL,
  campaign_status TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (version_id, machine_code, plan_date)
);

CREATE INDEX IF NOT EXISTS idx_risk_version_date ON risk_snapshot(version_id, plan_date);

CREATE TABLE IF NOT EXISTS action_log (
  action_id TEXT PRIMARY KEY,
  version_id TEXT REFERENCES plan_version(version_id),
  action_type TEXT NOT NULL,
  actor TEXT,
  action_ts TEXT NOT NULL DEFAULT (datetime('now')),
  payload_json TEXT,
  impact_summary_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_action_version_ts ON action_log(version_id, action_ts);

CREATE TABLE IF NOT EXISTS import_conflict (
  conflict_id TEXT PRIMARY KEY,
  source_batch_id TEXT NOT NULL,
  material_id TEXT NOT NULL,
  detected_at TEXT NOT NULL DEFAULT (datetime('now')),
  conflict_type TEXT NOT NULL,
  source_row_json TEXT NOT NULL,
  existing_row_json TEXT,
  resolution_status TEXT NOT NULL DEFAULT 'OPEN',
  resolution_note TEXT
);

CREATE INDEX IF NOT EXISTS idx_conflict_status ON import_conflict(resolution_status);

CREATE TABLE IF NOT EXISTS config_scope (
  scope_id TEXT PRIMARY KEY,
  scope_type TEXT NOT NULL,
  scope_key TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(scope_type, scope_key)
);

CREATE TABLE IF NOT EXISTS config_kv (
  scope_id TEXT NOT NULL REFERENCES config_scope(scope_id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (scope_id, key)
);

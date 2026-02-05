-- Dev DB schema (SQLite)
-- NOTE: This is intended for local/dev testing only.

PRAGMA foreign_keys = ON;

-- ==========================================
-- Meta
-- ==========================================

CREATE TABLE schema_version (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

-- ==========================================
-- Config
-- ==========================================

CREATE TABLE config_scope (
  scope_id TEXT PRIMARY KEY,
  scope_type TEXT NOT NULL,
  scope_key TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(scope_type, scope_key)
);

CREATE TABLE config_kv (
  scope_id TEXT NOT NULL REFERENCES config_scope(scope_id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (scope_id, key)
);

-- ==========================================
-- Master data
-- ==========================================

CREATE TABLE machine_master (
  machine_code TEXT PRIMARY KEY,
  machine_name TEXT,
  hourly_capacity_t REAL,
  default_daily_target_t REAL,
  default_daily_limit_pct REAL,
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ==========================================
-- machine_capacity_config: 机组级产能配置表 (v0.8新增)
-- 用于存储版本化的机组产能配置，支持版本隔离和历史追踪
-- ==========================================
CREATE TABLE machine_capacity_config (
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

CREATE INDEX idx_machine_config_version ON machine_capacity_config(version_id);
CREATE INDEX idx_machine_config_machine ON machine_capacity_config(machine_code);
CREATE INDEX idx_machine_config_created_at ON machine_capacity_config(created_at DESC);
CREATE INDEX idx_machine_config_version_machine_date ON machine_capacity_config(version_id, machine_code, effective_date);

CREATE TABLE material_master (
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
  rolling_output_date TEXT,  -- 轧制产出日期（ISO DATE）- v0.7 新增
  stock_age_days INTEGER,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  contract_nature TEXT,
  weekly_delivery_flag TEXT,
  export_flag TEXT,
  -- 品种大类（用于每日生产节奏管理；可为空，后续可由映射/导入补齐）
  product_category TEXT
);

CREATE INDEX idx_material_machine ON material_master(current_machine_code);
CREATE INDEX idx_material_next_machine ON material_master(next_machine_code);
CREATE INDEX idx_material_rolling_output_date ON material_master(rolling_output_date);  -- v0.7 新增
CREATE INDEX idx_material_due ON material_master(due_date);
CREATE INDEX idx_material_status_updated ON material_master(status_updated_at);
CREATE INDEX idx_material_rush_fields
  ON material_master(contract_nature, weekly_delivery_flag, export_flag);

-- material_state is the "single source of truth" for scheduling state.
-- Some decision/use-case implementations rely on a few denormalized columns;
-- keep them nullable/defaulted so existing writers remain compatible.
CREATE TABLE material_state (
  material_id TEXT PRIMARY KEY REFERENCES material_master(material_id) ON DELETE CASCADE,

  sched_state TEXT NOT NULL,
  lock_flag INTEGER NOT NULL DEFAULT 0,
  force_release_flag INTEGER NOT NULL DEFAULT 0,

  urgent_level TEXT,
  urgent_reason TEXT,
  rush_level TEXT,

  rolling_output_age_days INTEGER,
  ready_in_days INTEGER,
  earliest_sched_date TEXT,

  last_calc_version_id TEXT,
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),

  stock_age_days INTEGER,

  scheduled_date TEXT,
  scheduled_machine_code TEXT,
  seq_no INTEGER,

  manual_urgent_flag INTEGER NOT NULL DEFAULT 0,
  user_confirmed INTEGER NOT NULL DEFAULT 0,
  user_confirmed_at TEXT,
  user_confirmed_by TEXT,
  user_confirmed_reason TEXT,
  in_frozen_zone INTEGER NOT NULL DEFAULT 0,

  updated_by TEXT,

  -- Denormalized columns used by decision read-model refresh (seeded by scripts)
  contract_no TEXT,
  due_date TEXT,
  urgency_level TEXT,
  weight_t REAL NOT NULL DEFAULT 0.0,
  is_mature INTEGER NOT NULL DEFAULT 1,
  machine_code TEXT,
  spec_width_mm REAL,
  spec_thick_mm REAL
);

CREATE INDEX idx_state_sched_state ON material_state(sched_state);
CREATE INDEX idx_state_urgent ON material_state(urgent_level);
CREATE INDEX idx_state_earliest ON material_state(earliest_sched_date);
CREATE INDEX idx_state_scheduled_date ON material_state(scheduled_date);
CREATE INDEX idx_state_machine_date ON material_state(scheduled_machine_code, scheduled_date);
CREATE INDEX idx_state_frozen
  ON material_state(in_frozen_zone)
  WHERE in_frozen_zone = 1;
CREATE INDEX idx_state_manual_urgent
  ON material_state(manual_urgent_flag)
  WHERE manual_urgent_flag = 1;

CREATE INDEX idx_material_state_user_confirmed
  ON material_state(user_confirmed)
  WHERE user_confirmed = 0;

-- ==========================================
-- Plan / version / items
-- ==========================================

CREATE TABLE plan (
  plan_id TEXT PRIMARY KEY,
  plan_name TEXT NOT NULL,
  plan_type TEXT NOT NULL,
  base_plan_id TEXT,
  created_by TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE plan_version (
  version_id TEXT PRIMARY KEY,
  plan_id TEXT NOT NULL REFERENCES plan(plan_id) ON DELETE CASCADE,
  version_no INTEGER NOT NULL,
  status TEXT NOT NULL,
  frozen_from_date TEXT,
  recalc_window_days INTEGER,
  config_snapshot_json TEXT,
  created_by TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  revision INTEGER NOT NULL DEFAULT 0,
  UNIQUE(plan_id, version_no)
);

CREATE INDEX idx_version_plan ON plan_version(plan_id, version_no);

CREATE TABLE plan_item (
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

CREATE INDEX idx_item_version_machine_date ON plan_item(version_id, machine_code, plan_date, seq_no);

-- ==========================================
-- Plan rhythm (daily production rhythm targets)
-- ==========================================

-- 预设节奏模板（多套可选比例）
CREATE TABLE plan_rhythm_preset (
  preset_id TEXT PRIMARY KEY,
  preset_name TEXT NOT NULL,
  dimension TEXT NOT NULL, -- e.g. PRODUCT_CATEGORY
  target_json TEXT NOT NULL, -- JSON object: {category: ratio}
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_by TEXT
);

-- 每日节奏目标（按版本 + 机组 + 日期，一天一套）
CREATE TABLE plan_rhythm_target (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  plan_date TEXT NOT NULL, -- YYYY-MM-DD
  dimension TEXT NOT NULL, -- e.g. PRODUCT_CATEGORY
  target_json TEXT NOT NULL, -- JSON object: {category: ratio}
  preset_id TEXT REFERENCES plan_rhythm_preset(preset_id),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_by TEXT,
  PRIMARY KEY (version_id, machine_code, plan_date, dimension)
);

CREATE INDEX idx_rhythm_target_version_machine_date
  ON plan_rhythm_target(version_id, machine_code, plan_date);

-- ==========================================
-- Capacity / risk / roll
-- ==========================================

-- P1-1: capacity_pool 版本化改造
-- 将 version_id 纳入主键，避免跨版本产能污染
CREATE TABLE capacity_pool (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
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

CREATE INDEX idx_pool_version_machine_date ON capacity_pool(version_id, machine_code, plan_date);
CREATE INDEX idx_pool_machine_date ON capacity_pool(machine_code, plan_date);

CREATE TABLE risk_snapshot (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  snapshot_date TEXT NOT NULL,
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
  PRIMARY KEY (version_id, machine_code, snapshot_date)
);

CREATE INDEX idx_risk_version_date ON risk_snapshot(version_id, snapshot_date);

CREATE TABLE roller_campaign (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  campaign_no INTEGER NOT NULL,
  start_date TEXT NOT NULL,
  end_date TEXT,
  cum_weight_t REAL NOT NULL DEFAULT 0,
  suggest_threshold_t REAL NOT NULL,
  hard_limit_t REAL NOT NULL,
  status TEXT NOT NULL,
  path_anchor_material_id TEXT,
  path_anchor_width_mm REAL,
  path_anchor_thickness_mm REAL,
  anchor_source TEXT,
  PRIMARY KEY (version_id, machine_code, campaign_no)
);

CREATE INDEX idx_campaign_version_machine ON roller_campaign(version_id, machine_code);

CREATE INDEX idx_roller_campaign_anchor_source
  ON roller_campaign(anchor_source)
  WHERE anchor_source IS NOT NULL;

-- 路径规则待人工确认（由重算生成；每个 material 在每个版本+机组仅记录一次，plan_date 表示首次遇到 OVERRIDE_REQUIRED 的日期）
CREATE TABLE path_override_pending (
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

CREATE INDEX idx_path_override_pending_version_date_machine
  ON path_override_pending(version_id, plan_date, machine_code);
CREATE INDEX idx_path_override_pending_version_machine_date
  ON path_override_pending(version_id, machine_code, plan_date);
CREATE INDEX idx_path_override_pending_version_material
  ON path_override_pending(version_id, material_id);

-- roll_campaign_plan: 换辊时间监控/微调（按版本+机组）
CREATE TABLE roll_campaign_plan (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  initial_start_at TEXT NOT NULL, -- 换辊周期起点 (YYYY-MM-DD HH:MM:SS)
  next_change_at TEXT, -- 计划换辊时刻 (可选)
  downtime_minutes INTEGER, -- 计划停机时长（分钟）
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_by TEXT,
  PRIMARY KEY (version_id, machine_code)
);

CREATE INDEX idx_roll_campaign_plan_version_machine
  ON roll_campaign_plan(version_id, machine_code);

-- ==========================================
-- Action log (audit)
-- ==========================================

CREATE TABLE action_log (
  action_id TEXT PRIMARY KEY,
  version_id TEXT REFERENCES plan_version(version_id),
  action_type TEXT NOT NULL,
  action_ts TEXT NOT NULL DEFAULT (datetime('now')),
  actor TEXT,
  payload_json TEXT,
  impact_summary_json TEXT,
  machine_code TEXT,
  date_range_start TEXT,
  date_range_end TEXT,
  detail TEXT
);

CREATE INDEX idx_action_version_ts ON action_log(version_id, action_ts);
CREATE INDEX idx_action_ts ON action_log(action_ts);
CREATE INDEX idx_action_type_ts ON action_log(action_type, action_ts);
CREATE INDEX idx_action_actor_ts ON action_log(actor, action_ts);
CREATE INDEX idx_action_machine_ts ON action_log(machine_code, action_ts);
CREATE INDEX idx_action_date_range ON action_log(date_range_start, date_range_end);

-- ==========================================
-- Decision: Strategy drafts (persistent)
-- ==========================================

CREATE TABLE decision_strategy_draft (
  draft_id TEXT PRIMARY KEY,
  base_version_id TEXT NOT NULL REFERENCES plan_version(version_id),
  plan_date_from TEXT NOT NULL,
  plan_date_to TEXT NOT NULL,

  -- strategy profile (supports preset + custom:xxx)
  strategy_key TEXT NOT NULL,
  strategy_base TEXT NOT NULL,
  strategy_title_cn TEXT NOT NULL,
  strategy_params_json TEXT,

  -- lifecycle
  status TEXT NOT NULL CHECK(status IN ('DRAFT', 'PUBLISHED', 'EXPIRED')),
  created_by TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
  expires_at TEXT NOT NULL,
  published_as_version_id TEXT REFERENCES plan_version(version_id),
  published_by TEXT,
  published_at TEXT,

  -- soft lock for concurrency (best-effort)
  locked_by TEXT,
  locked_at TEXT,

  -- payload
  summary_json TEXT NOT NULL,
  diff_items_json TEXT NOT NULL,
  diff_items_total INTEGER NOT NULL DEFAULT 0,
  diff_items_truncated INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_strategy_draft_base_version ON decision_strategy_draft(base_version_id);
CREATE INDEX idx_strategy_draft_status ON decision_strategy_draft(status);
CREATE INDEX idx_strategy_draft_expires_at ON decision_strategy_draft(expires_at);
CREATE INDEX idx_strategy_draft_created_at ON decision_strategy_draft(created_at DESC);

-- ==========================================
-- Importer
-- ==========================================

CREATE TABLE import_batch (
  batch_id TEXT PRIMARY KEY,
  file_name TEXT NOT NULL,
  file_path TEXT,
  total_rows INTEGER NOT NULL,
  success_rows INTEGER NOT NULL DEFAULT 0,
  blocked_rows INTEGER NOT NULL DEFAULT 0,
  warning_rows INTEGER NOT NULL DEFAULT 0,
  conflict_rows INTEGER NOT NULL DEFAULT 0,
  imported_at TEXT NOT NULL DEFAULT (datetime('now')),
  imported_by TEXT,
  elapsed_ms INTEGER,
  dq_report_json TEXT
);

CREATE INDEX idx_batch_imported_at ON import_batch(imported_at DESC);
CREATE INDEX idx_batch_filename ON import_batch(file_name);

CREATE TABLE import_conflict (
  conflict_id TEXT PRIMARY KEY,
  source_batch_id TEXT NOT NULL,
  material_id TEXT NOT NULL,
  detected_at TEXT NOT NULL DEFAULT (datetime('now')),
  conflict_type TEXT NOT NULL,
  source_row_json TEXT NOT NULL,
  existing_row_json TEXT,
  resolution_status TEXT NOT NULL DEFAULT 'OPEN',
  resolution_action TEXT,
  resolution_note TEXT,
  resolved_at TEXT,
  row_number INTEGER
);

CREATE INDEX idx_conflict_status ON import_conflict(resolution_status);

-- ==========================================
-- Decision refresh queue (event bridge)
-- ==========================================

CREATE TABLE decision_refresh_queue (
  task_id TEXT PRIMARY KEY,
  version_id TEXT NOT NULL,
  trigger_type TEXT NOT NULL,
  trigger_source TEXT,
  is_full_refresh INTEGER NOT NULL DEFAULT 0,
  affected_machines TEXT,
  affected_date_range TEXT,
  status TEXT NOT NULL DEFAULT 'PENDING',
  retry_count INTEGER NOT NULL DEFAULT 0,
  max_retries INTEGER NOT NULL DEFAULT 3,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  started_at TEXT,
  completed_at TEXT,
  error_message TEXT,
  refresh_id TEXT
);

CREATE INDEX idx_refresh_queue_status
  ON decision_refresh_queue(status, created_at);
CREATE INDEX idx_refresh_queue_version
  ON decision_refresh_queue(version_id, status);

-- ==========================================
-- Decision read models (v0.4)
-- ==========================================

CREATE TABLE decision_day_summary (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  plan_date TEXT NOT NULL,
  risk_score REAL NOT NULL,
  risk_level TEXT NOT NULL,
  capacity_util_pct REAL NOT NULL,
  top_reasons TEXT NOT NULL,
  affected_machines INTEGER NOT NULL,
  bottleneck_machines INTEGER NOT NULL,
  has_roll_risk INTEGER NOT NULL DEFAULT 0,
  suggested_actions TEXT,
  refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (version_id, plan_date)
);

CREATE INDEX idx_day_summary_version_risk
  ON decision_day_summary(version_id, risk_score DESC);
CREATE INDEX idx_day_summary_date_range
  ON decision_day_summary(version_id, plan_date);

CREATE TABLE decision_order_failure_set (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  contract_no TEXT NOT NULL,
  due_date TEXT NOT NULL,
  urgency_level TEXT NOT NULL,
  fail_type TEXT NOT NULL,
  total_materials INTEGER NOT NULL,
  unscheduled_count INTEGER NOT NULL,
  unscheduled_weight_t REAL NOT NULL,
  completion_rate REAL NOT NULL,
  days_to_due INTEGER NOT NULL,
  failure_reasons TEXT NOT NULL,
  blocking_factors TEXT NOT NULL,
  suggested_actions TEXT,
  refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (version_id, contract_no)
);

CREATE INDEX idx_order_failure_version_urgency
  ON decision_order_failure_set(version_id, urgency_level DESC);
CREATE INDEX idx_order_failure_fail_type
  ON decision_order_failure_set(version_id, fail_type);
CREATE INDEX idx_order_failure_due_date
  ON decision_order_failure_set(version_id, due_date);

CREATE TABLE decision_cold_stock_profile (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  age_bin TEXT NOT NULL,
  age_min_days INTEGER NOT NULL,
  age_max_days INTEGER,
  count INTEGER NOT NULL,
  weight_t REAL NOT NULL,
  avg_age_days REAL NOT NULL DEFAULT 0.0,
  pressure_score REAL NOT NULL,
  pressure_level TEXT NOT NULL,
  reasons TEXT NOT NULL,
  structure_gap TEXT,
  estimated_ready_date TEXT,
  can_force_release INTEGER NOT NULL DEFAULT 0,
  suggested_actions TEXT,
  refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (version_id, machine_code, age_bin)
);

CREATE INDEX idx_cold_stock_version_pressure
  ON decision_cold_stock_profile(version_id, pressure_score DESC);
CREATE INDEX idx_cold_stock_machine
  ON decision_cold_stock_profile(version_id, machine_code);
CREATE INDEX idx_cold_stock_high_pressure
  ON decision_cold_stock_profile(version_id, pressure_level)
  WHERE pressure_level IN ('HIGH', 'CRITICAL');

CREATE TABLE decision_machine_bottleneck (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  plan_date TEXT NOT NULL,
  bottleneck_score REAL NOT NULL,
  bottleneck_level TEXT NOT NULL,
  bottleneck_types TEXT NOT NULL,
  reasons TEXT NOT NULL,
  remaining_capacity_t REAL NOT NULL,
  capacity_utilization REAL NOT NULL,
  needs_roll_change INTEGER NOT NULL DEFAULT 0,
  structure_violations INTEGER NOT NULL DEFAULT 0,
  pending_materials INTEGER NOT NULL DEFAULT 0,
  suggested_actions TEXT,
  refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (version_id, machine_code, plan_date)
);

CREATE INDEX idx_bottleneck_version_score
  ON decision_machine_bottleneck(version_id, bottleneck_score DESC);
CREATE INDEX idx_bottleneck_machine_date
  ON decision_machine_bottleneck(version_id, machine_code, plan_date);
CREATE INDEX idx_bottleneck_high_level
  ON decision_machine_bottleneck(version_id, bottleneck_level)
  WHERE bottleneck_level IN ('HIGH', 'CRITICAL');

CREATE TABLE decision_roll_campaign_alert (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  campaign_no INTEGER NOT NULL,
  cum_weight_t REAL NOT NULL,
  suggest_threshold_t REAL NOT NULL,
  hard_limit_t REAL NOT NULL,
  alert_level TEXT NOT NULL,
  reason TEXT,
  distance_to_suggest REAL NOT NULL,
  distance_to_hard REAL NOT NULL,
  utilization_rate REAL NOT NULL,
  estimated_change_date TEXT,
  needs_immediate_change INTEGER NOT NULL DEFAULT 0,
  suggested_actions TEXT,
  campaign_start_at TEXT,
  planned_change_at TEXT,
  planned_downtime_minutes INTEGER,
  estimated_soft_reach_at TEXT,
  estimated_hard_reach_at TEXT,
  refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (version_id, machine_code, campaign_no)
);

CREATE INDEX idx_roll_alert_version_level
  ON decision_roll_campaign_alert(version_id, alert_level DESC);
CREATE INDEX idx_roll_alert_machine
  ON decision_roll_campaign_alert(version_id, machine_code);
CREATE INDEX idx_roll_alert_emergency
  ON decision_roll_campaign_alert(version_id, alert_level)
  WHERE alert_level IN ('CRITICAL', 'EMERGENCY');

CREATE TABLE decision_capacity_opportunity (
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  plan_date TEXT NOT NULL,
  slack_t REAL NOT NULL,
  soft_adjust_space_t REAL,
  utilization_rate REAL NOT NULL,
  binding_constraints TEXT,
  opportunity_level TEXT NOT NULL,
  sensitivity TEXT,
  suggested_optimizations TEXT,
  refreshed_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (version_id, machine_code, plan_date)
);

CREATE INDEX idx_capacity_opp_version_level
  ON decision_capacity_opportunity(version_id, opportunity_level DESC);
CREATE INDEX idx_capacity_opp_machine_date
  ON decision_capacity_opportunity(version_id, machine_code, plan_date);
CREATE INDEX idx_capacity_opp_high_level
  ON decision_capacity_opportunity(version_id, opportunity_level)
  WHERE opportunity_level IN ('MEDIUM', 'HIGH');

CREATE TABLE decision_refresh_log (
  refresh_id TEXT PRIMARY KEY,
  version_id TEXT NOT NULL REFERENCES plan_version(version_id) ON DELETE CASCADE,
  trigger_type TEXT NOT NULL,
  trigger_source TEXT,
  is_full_refresh INTEGER NOT NULL DEFAULT 0,
  affected_machines TEXT,
  affected_date_range TEXT,
  refreshed_tables TEXT NOT NULL,
  rows_affected INTEGER NOT NULL DEFAULT 0,
  started_at TEXT NOT NULL DEFAULT (datetime('now')),
  completed_at TEXT,
  duration_ms INTEGER,
  status TEXT NOT NULL DEFAULT 'RUNNING',
  error_message TEXT
);

CREATE INDEX idx_refresh_log_version
  ON decision_refresh_log(version_id, started_at DESC);
CREATE INDEX idx_refresh_log_trigger
  ON decision_refresh_log(trigger_type, started_at DESC);

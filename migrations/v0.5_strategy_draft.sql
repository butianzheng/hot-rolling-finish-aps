-- ==========================================
-- v0.5: decision_strategy_draft (策略草案持久化)
-- ==========================================

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS decision_strategy_draft (
  draft_id TEXT PRIMARY KEY,
  base_version_id TEXT NOT NULL REFERENCES plan_version(version_id),
  plan_date_from TEXT NOT NULL,
  plan_date_to TEXT NOT NULL,

  strategy_key TEXT NOT NULL,
  strategy_base TEXT NOT NULL,
  strategy_title_cn TEXT NOT NULL,
  strategy_params_json TEXT,

  status TEXT NOT NULL CHECK(status IN ('DRAFT', 'PUBLISHED', 'EXPIRED')),
  created_by TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
  expires_at TEXT NOT NULL,
  published_as_version_id TEXT REFERENCES plan_version(version_id),
  published_by TEXT,
  published_at TEXT,

  locked_by TEXT,
  locked_at TEXT,

  summary_json TEXT NOT NULL,
  diff_items_json TEXT NOT NULL,
  diff_items_total INTEGER NOT NULL DEFAULT 0,
  diff_items_truncated INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_strategy_draft_base_version ON decision_strategy_draft(base_version_id);
CREATE INDEX IF NOT EXISTS idx_strategy_draft_status ON decision_strategy_draft(status);
CREATE INDEX IF NOT EXISTS idx_strategy_draft_expires_at ON decision_strategy_draft(expires_at);
CREATE INDEX IF NOT EXISTS idx_strategy_draft_created_at ON decision_strategy_draft(created_at DESC);

-- 更新 schema_version
INSERT OR IGNORE INTO schema_version (version, applied_at)
  VALUES (5, datetime('now'));

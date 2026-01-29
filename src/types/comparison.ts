import type { StrategyType } from './preferences';

export interface PlanItemSnapshot {
  material_id: string;
  machine_code: string;
  plan_date: string;
  seq_no: number;
  weight_t?: number;
  urgent_level?: string;
  locked_in_plan?: boolean;
  force_release_in_plan?: boolean;
  sched_state?: string;
  assign_reason?: string;
}

export type VersionDiffChangeType = 'ADDED' | 'REMOVED' | 'MODIFIED' | 'MOVED';

export interface VersionDiff {
  materialId: string;
  changeType: VersionDiffChangeType;
  previousState: PlanItemSnapshot | null;
  currentState: PlanItemSnapshot | null;
  reason?: string;
}

export type ComparisonStrategyType = Exclude<StrategyType, 'manual'>;

export type StrategyDraftStatus = 'DRAFT' | 'PUBLISHED';

export type StrategyParameters = Record<string, unknown>;

export interface StrategyDraft {
  draftId: string;
  sourceVersionId: string;
  status: StrategyDraftStatus;
  strategyType: ComparisonStrategyType;
  parameters: StrategyParameters;
  changes: VersionDiff[];
}

export interface VersionComparisonResult {
  versionIdA: string;
  versionIdB: string;
  diffs: VersionDiff[];
  summary: {
    totalChanges: number;
    addedCount: number;
    removedCount: number;
    modifiedCount: number;
    movedCount: number;
  };
}

// ==========================================
// Backend comparison (current Rust API)
// ==========================================

export interface BackendRiskDelta {
  date: string;
  risk_score_a: number | null;
  risk_score_b: number | null;
  risk_score_delta: number;
}

export interface BackendCapacityDelta {
  machine_code: string;
  date: string;
  used_capacity_a: number | null;
  used_capacity_b: number | null;
  capacity_delta: number;
}

export interface BackendConfigChange {
  key: string;
  value_a: string | null;
  value_b: string | null;
}

export interface BackendVersionComparisonResult {
  version_id_a: string;
  version_id_b: string;
  moved_count: number;
  added_count: number;
  removed_count: number;
  squeezed_out_count: number;
  risk_delta: BackendRiskDelta[] | null;
  capacity_delta: BackendCapacityDelta[] | null;
  config_changes: BackendConfigChange[] | null;
  message: string;
}

export interface BackendVersionDiffCounts {
  moved_count: number;
  added_count: number;
  removed_count: number;
  squeezed_out_count: number;
}

export interface BackendVersionKpiSummary {
  plan_items_count: number;
  total_weight_t: number;
  locked_in_plan_count: number;
  force_release_in_plan_count: number;
  plan_date_from: string | null;
  plan_date_to: string | null;

  overflow_days: number | null;
  overflow_t: number | null;
  capacity_used_t: number | null;
  capacity_target_t: number | null;
  capacity_limit_t: number | null;
  capacity_util_pct: number | null;
  mature_backlog_t: number | null;
  immature_backlog_t: number | null;
  urgent_total_t: number | null;
  snapshot_date_from: string | null;
  snapshot_date_to: string | null;
}

export interface BackendVersionComparisonKpiResult {
  version_id_a: string;
  version_id_b: string;
  kpi_a: BackendVersionKpiSummary;
  kpi_b: BackendVersionKpiSummary;
  diff_counts: BackendVersionDiffCounts;
  message: string;
}

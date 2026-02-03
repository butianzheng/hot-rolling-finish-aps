import { z } from 'zod';

import { DateString } from './_shared';

// ==========================================================
// P1-1: version comparison KPI aggregation
// ==========================================================

export const VersionDiffCountsSchema = z
  .object({
    moved_count: z.number(),
    added_count: z.number(),
    removed_count: z.number(),
    squeezed_out_count: z.number(),
  })
  .passthrough();

export const VersionKpiSummarySchema = z
  .object({
    plan_items_count: z.number(),
    total_weight_t: z.number(),
    locked_in_plan_count: z.number(),
    force_release_in_plan_count: z.number(),
    plan_date_from: DateString.nullable(),
    plan_date_to: DateString.nullable(),

    overflow_days: z.number().nullable(),
    overflow_t: z.number().nullable(),
    capacity_used_t: z.number().nullable(),
    capacity_target_t: z.number().nullable(),
    capacity_limit_t: z.number().nullable(),
    capacity_util_pct: z.number().nullable(),
    mature_backlog_t: z.number().nullable(),
    immature_backlog_t: z.number().nullable(),
    urgent_total_t: z.number().nullable(),
    snapshot_date_from: DateString.nullable(),
    snapshot_date_to: DateString.nullable(),
  })
  .passthrough();

export const VersionComparisonKpiResultSchema = z
  .object({
    version_id_a: z.string(),
    version_id_b: z.string(),
    kpi_a: VersionKpiSummarySchema,
    kpi_b: VersionKpiSummarySchema,
    diff_counts: VersionDiffCountsSchema,
    message: z.string(),
  })
  .passthrough();

export const RiskDeltaSchema = z
  .object({
    date: DateString,
    risk_score_a: z.number().nullable().optional(),
    risk_score_b: z.number().nullable().optional(),
    risk_score_delta: z.number(),
  })
  .passthrough();

export const CapacityDeltaSchema = z
  .object({
    machine_code: z.string(),
    date: DateString,
    used_capacity_a: z.number().nullable().optional(),
    used_capacity_b: z.number().nullable().optional(),
    capacity_delta: z.number(),
  })
  .passthrough();

export const ConfigChangeSchema = z
  .object({
    key: z.string(),
    value_a: z.string().nullable().optional(),
    value_b: z.string().nullable().optional(),
  })
  .passthrough();

export const VersionComparisonResultSchema = z
  .object({
    version_id_a: z.string(),
    version_id_b: z.string(),
    moved_count: z.number(),
    added_count: z.number(),
    removed_count: z.number(),
    squeezed_out_count: z.number(),
    risk_delta: z.array(RiskDeltaSchema).nullable().optional(),
    capacity_delta: z.array(CapacityDeltaSchema).nullable().optional(),
    config_changes: z.array(ConfigChangeSchema).nullable().optional(),
    message: z.string(),
  })
  .passthrough();


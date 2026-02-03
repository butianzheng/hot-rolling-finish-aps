import { z } from 'zod';

import { DateString } from './_shared';

// ==========================================================
// 宽厚路径规则 (v0.6) 响应 Schema
// ==========================================================

export const PathRuleConfigSchema = z
  .object({
    enabled: z.boolean(),
    width_tolerance_mm: z.number(),
    thickness_tolerance_mm: z.number(),
    override_allowed_urgency_levels: z.array(z.string()),
    seed_s2_percentile: z.number(),
    seed_s2_small_sample_threshold: z.number(),
  })
  .passthrough();

export const PathOverridePendingSchema = z
  .object({
    material_id: z.string(),
    material_no: z.string(),
    width_mm: z.number(),
    thickness_mm: z.number(),
    urgent_level: z.string(),
    violation_type: z.string(),
    anchor_width_mm: z.number(),
    anchor_thickness_mm: z.number(),
    width_delta_mm: z.number(),
    thickness_delta_mm: z.number(),
  })
  .passthrough();

export const PathOverridePendingSummarySchema = z
  .object({
    machine_code: z.string(),
    plan_date: DateString,
    pending_count: z.number(),
  })
  .passthrough();

export const RollCycleAnchorSchema = z
  .object({
    version_id: z.string(),
    machine_code: z.string(),
    campaign_no: z.number(),
    cum_weight_t: z.number(),
    anchor_source: z.string(),
    anchor_material_id: z.string().nullable().optional(),
    anchor_width_mm: z.number().nullable().optional(),
    anchor_thickness_mm: z.number().nullable().optional(),
    status: z.string(),
  })
  .passthrough();

export const BatchConfirmPathOverrideResultSchema = z
  .object({
    success_count: z.number(),
    fail_count: z.number(),
    failed_material_ids: z.array(z.string()),
  })
  .passthrough();


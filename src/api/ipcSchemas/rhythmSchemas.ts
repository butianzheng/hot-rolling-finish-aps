import { z } from 'zod';

import { DateString } from './_shared';

// ==========================================================
// 每日生产节奏（品种大类）Schema
// ==========================================================

export const PlanRhythmPresetSchema = z
  .object({
    preset_id: z.string(),
    preset_name: z.string(),
    dimension: z.string(),
    target_json: z.string(),
    is_active: z.boolean(),
    created_at: z.string(),
    updated_at: z.string(),
    updated_by: z.string().nullable().optional(),
  })
  .passthrough();

export const PlanRhythmPresetsResponseSchema = z.array(PlanRhythmPresetSchema);

export const PlanRhythmTargetSchema = z
  .object({
    version_id: z.string(),
    machine_code: z.string(),
    plan_date: DateString,
    dimension: z.string(),
    target_json: z.string(),
    preset_id: z.string().nullable().optional(),
    updated_at: z.string(),
    updated_by: z.string().nullable().optional(),
  })
  .passthrough();

export const PlanRhythmTargetsResponseSchema = z.array(PlanRhythmTargetSchema);

export const ApplyRhythmPresetResponseSchema = z
  .object({
    applied: z.number(),
  })
  .passthrough();

export const DailyRhythmCategoryRowSchema = z
  .object({
    category: z.string(),
    scheduled_weight_t: z.number(),
    actual_ratio: z.number(),
    target_ratio: z.number().nullable().optional(),
    diff_ratio: z.number().nullable().optional(),
  })
  .passthrough();

export const DailyRhythmProfileSchema = z
  .object({
    version_id: z.string(),
    machine_code: z.string(),
    plan_date: DateString,
    dimension: z.string(),
    total_scheduled_weight_t: z.number(),
    deviation_threshold: z.number(),
    max_deviation: z.number(),
    is_violated: z.boolean(),
    target_preset_id: z.string().nullable().optional(),
    target_updated_at: z.string().nullable().optional(),
    target_updated_by: z.string().nullable().optional(),
    categories: z.array(DailyRhythmCategoryRowSchema),
  })
  .passthrough();


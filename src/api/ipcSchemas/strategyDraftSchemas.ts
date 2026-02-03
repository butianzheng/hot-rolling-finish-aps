import { z } from 'zod';

import { DateString } from './_shared';

// ==========================================================
// P0-1: strategy draft (draft persistence)
// ==========================================================

export const StrategyDraftSummarySchema = z
  .object({
    draft_id: z.string(),
    base_version_id: z.string(),
    strategy: z.string(),
    plan_items_count: z.number(),
    frozen_items_count: z.number(),
    calc_items_count: z.number(),
    mature_count: z.number(),
    immature_count: z.number(),
    total_capacity_used_t: z.number(),
    overflow_days: z.number(),
    moved_count: z.number(),
    added_count: z.number(),
    removed_count: z.number(),
    squeezed_out_count: z.number(),
    message: z.string(),
  })
  .passthrough();

export const GenerateStrategyDraftsResponseSchema = z
  .object({
    base_version_id: z.string(),
    plan_date_from: DateString,
    plan_date_to: DateString,
    drafts: z.array(StrategyDraftSummarySchema),
    message: z.string(),
  })
  .passthrough();

export const ListStrategyDraftsResponseSchema = GenerateStrategyDraftsResponseSchema;

export const ApplyStrategyDraftResponseSchema = z
  .object({
    version_id: z.string(),
    success: z.boolean(),
    message: z.string(),
  })
  .passthrough();

export const StrategyDraftDiffItemSchema = z
  .object({
    material_id: z.string(),
    change_type: z.string(),

    from_plan_date: DateString.nullable().optional(),
    from_machine_code: z.string().nullable().optional(),
    from_seq_no: z.number().nullable().optional(),

    to_plan_date: DateString.nullable().optional(),
    to_machine_code: z.string().nullable().optional(),
    to_seq_no: z.number().nullable().optional(),

    to_assign_reason: z.string().nullable().optional(),
    to_urgent_level: z.string().nullable().optional(),
    to_sched_state: z.string().nullable().optional(),

    material_state_snapshot: z
      .object({
        sched_state: z.string().nullable().optional(),
        urgent_level: z.string().nullable().optional(),
        rush_level: z.string().nullable().optional(),
        lock_flag: z.boolean().nullable().optional(),
        force_release_flag: z.boolean().nullable().optional(),
        manual_urgent_flag: z.boolean().nullable().optional(),
        in_frozen_zone: z.boolean().nullable().optional(),
        ready_in_days: z.number().nullable().optional(),
        earliest_sched_date: DateString.nullable().optional(),
        scheduled_date: DateString.nullable().optional(),
        scheduled_machine_code: z.string().nullable().optional(),
        seq_no: z.number().nullable().optional(),
      })
      .passthrough()
      .nullable()
      .optional(),
  })
  .passthrough();

export const GetStrategyDraftDetailResponseSchema = z
  .object({
    draft_id: z.string(),
    base_version_id: z.string(),
    plan_date_from: DateString,
    plan_date_to: DateString,
    strategy: z.string(),
    diff_items: z.array(StrategyDraftDiffItemSchema),
    diff_items_total: z.number(),
    diff_items_truncated: z.boolean(),
    message: z.string(),
  })
  .passthrough();

export const CleanupStrategyDraftsResponseSchema = z
  .object({
    deleted_count: z.number(),
    message: z.string(),
  })
  .passthrough();


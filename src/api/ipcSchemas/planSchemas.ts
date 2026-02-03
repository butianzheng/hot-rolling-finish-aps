import { z } from 'zod';

import { DateString } from './_shared';

// ==========================================================
// Plan API 响应 Schema（优先级2）
// ==========================================================

export const PlanSchema = z
  .object({
    plan_id: z.string(),
    plan_name: z.string(),
    plan_type: z.string(),
    base_plan_id: z.string().nullable().optional(),
    created_by: z.string(),
    created_at: z.string(),
    updated_at: z.string(),
  })
  .passthrough();

export const PlanVersionSchema = z
  .object({
    version_id: z.string(),
    plan_id: z.string(),
    version_no: z.number(),
    status: z.string(),
    frozen_from_date: DateString.nullable().optional(),
    recalc_window_days: z.number().nullable().optional(),
    config_snapshot_json: z.string().nullable().optional(),
    created_by: z.string().nullable().optional(),
    created_at: z.string(),
    revision: z.number(),
  })
  .passthrough();

export const PlanItemSchema = z
  .object({
    version_id: z.string(),
    material_id: z.string(),
    machine_code: z.string(),
    plan_date: DateString,
    seq_no: z.number(),
    weight_t: z.number(),
    source_type: z.string(),
    locked_in_plan: z.boolean(),
    force_release_in_plan: z.boolean(),
    violation_flags: z.string().nullable().optional(),
    urgent_level: z.string().nullable().optional(),
    sched_state: z.string().nullable().optional(),
    assign_reason: z.string().nullable().optional(),
    steel_grade: z.string().nullable().optional(),
  })
  .passthrough();

export const StrategyPresetSchema = z
  .object({
    strategy: z.string(),
    title: z.string(),
    description: z.string(),
    default_parameters: z.unknown(),
  })
  .passthrough();

export const ManualRefreshDecisionResponseSchema = z
  .object({
    version_id: z.string(),
    task_id: z.string().nullable().optional(),
    success: z.boolean(),
    message: z.string(),
  })
  .passthrough();

export const RollbackVersionResponseSchema = z
  .object({
    plan_id: z.string(),
    from_version_id: z.string().nullable().optional(),
    to_version_id: z.string(),
    restored_config_count: z.number().nullable().optional(),
    config_restore_skipped: z.string().nullable().optional(),
    message: z.string(),
  })
  .passthrough();

export const MoveItemResultSchema = z
  .object({
    material_id: z.string(),
    success: z.boolean(),
    from_date: DateString.nullable().optional(),
    from_machine: z.string().nullable().optional(),
    to_date: DateString,
    to_machine: z.string(),
    error: z.string().nullable().optional(),
    violation_type: z.string().nullable().optional(),
  })
  .passthrough();

export const MoveItemsResponseSchema = z
  .object({
    version_id: z.string(),
    results: z.array(MoveItemResultSchema),
    success_count: z.number(),
    failed_count: z.number(),
    has_violations: z.boolean(),
    message: z.string(),
  })
  .passthrough();

// ==========================================================
// 一键重算/试算 响应 Schema
// ==========================================================

export const RecalcResponseSchema = z
  .object({
    version_id: z.string(),
    plan_items_count: z.number(),
    frozen_items_count: z.number(),
    success: z.boolean(),
    message: z.string(),
  })
  .passthrough();


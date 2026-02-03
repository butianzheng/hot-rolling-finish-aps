import { z } from 'zod';

// ==========================================================
// Config API 响应 Schema（优先级3）
// ==========================================================

export const ConfigItemSchema = z
  .object({
    scope_id: z.string(),
    scope_type: z.string(),
    key: z.string(),
    value: z.string(),
    updated_at: z.string().optional(),
  })
  .passthrough();

export const ImpactSummarySchema = z
  .object({
    success_count: z.number(),
    fail_count: z.number(),
    message: z.string(),
    details: z.record(z.unknown()).nullable().optional(),
  })
  .passthrough();

export const BatchUpdateConfigsResponseSchema = z
  .object({
    updated_count: z.number(),
  })
  .passthrough();

export const RestoreConfigFromSnapshotResponseSchema = z
  .object({
    restored_count: z.number(),
  })
  .passthrough();

export const ConfigSnapshotSchema = z.record(z.unknown());

export const CustomStrategyParametersSchema = z
  .object({
    urgent_weight: z.number().nullable().optional(),
    capacity_weight: z.number().nullable().optional(),
    cold_stock_weight: z.number().nullable().optional(),
    due_date_weight: z.number().nullable().optional(),
    rolling_output_age_weight: z.number().nullable().optional(),
    cold_stock_age_threshold_days: z.number().nullable().optional(),
    overflow_tolerance_pct: z.number().nullable().optional(),
  })
  .passthrough();

export const CustomStrategyProfileSchema = z
  .object({
    strategy_id: z.string(),
    title: z.string(),
    description: z.string().nullable().optional(),
    base_strategy: z.string(),
    parameters: CustomStrategyParametersSchema.nullable().optional(),
  })
  .passthrough();

export const SaveCustomStrategyResponseSchema = z
  .object({
    strategy_id: z.string(),
    stored_key: z.string(),
    existed: z.boolean(),
    message: z.string(),
  })
  .passthrough();


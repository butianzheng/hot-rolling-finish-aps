import { z } from 'zod';

// ==========================================================
// P0-2: decision refresh status
// ==========================================================

export const DecisionRefreshQueueCountsSchema = z
  .object({
    pending: z.number(),
    running: z.number(),
    failed: z.number(),
    completed: z.number(),
    cancelled: z.number(),
  })
  .passthrough();

export const DecisionRefreshTaskSchema = z
  .object({
    task_id: z.string(),
    version_id: z.string(),
    trigger_type: z.string(),
    trigger_source: z.string().nullable().optional(),
    is_full_refresh: z.boolean(),
    status: z.string(),
    retry_count: z.number(),
    max_retries: z.number(),
    created_at: z.string(),
    started_at: z.string().nullable().optional(),
    completed_at: z.string().nullable().optional(),
    error_message: z.string().nullable().optional(),
    refresh_id: z.string().nullable().optional(),
  })
  .passthrough();

export const DecisionRefreshLogSchema = z
  .object({
    refresh_id: z.string(),
    version_id: z.string(),
    trigger_type: z.string(),
    trigger_source: z.string().nullable().optional(),
    is_full_refresh: z.boolean(),
    refreshed_tables_json: z.string(),
    rows_affected: z.number(),
    started_at: z.string(),
    completed_at: z.string().nullable().optional(),
    duration_ms: z.number().nullable().optional(),
    status: z.string(),
    error_message: z.string().nullable().optional(),
  })
  .passthrough();

export const DecisionRefreshStatusResponseSchema = z
  .object({
    version_id: z.string(),
    status: z.string(),
    is_refreshing: z.boolean(),
    queue_counts: DecisionRefreshQueueCountsSchema,
    latest_task: DecisionRefreshTaskSchema.nullable().optional(),
    latest_log: DecisionRefreshLogSchema.nullable().optional(),
    last_error: z.string().nullable().optional(),
    message: z.string(),
  })
  .passthrough();


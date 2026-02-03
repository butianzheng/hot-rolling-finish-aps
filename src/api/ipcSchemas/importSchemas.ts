import { z } from 'zod';

import { DateTimeString } from './_shared';

// ==========================================================
// Import API 响应 Schema（优先级2）
// ==========================================================

export const DqSummarySchema = z
  .object({
    total_rows: z.number(),
    success: z.number(),
    blocked: z.number(),
    warning: z.number(),
    conflict: z.number(),
  })
  .passthrough();

export const DqViolationSchema = z
  .object({
    row_number: z.number(),
    material_id: z.string().nullable().optional(),
    level: z.string(),
    field: z.string(),
    message: z.string(),
  })
  .passthrough();

export const ImportApiResponseSchema = z
  .object({
    imported: z.number(),
    updated: z.number(),
    conflicts: z.number(),
    batch_id: z.string(),
    import_batch_id: z.string(),
    dq_summary: DqSummarySchema,
    dq_violations: z.array(DqViolationSchema),
    elapsed_ms: z.number(),
  })
  .passthrough();

export const ImportConflictSchema = z
  .object({
    conflict_id: z.string(),
    batch_id: z.string(),
    row_number: z.number(),
    material_id: z.string().nullable().optional(),
    conflict_type: z.string(),
    raw_data: z.string(),
    reason: z.string(),
    resolved: z.boolean(),
    created_at: DateTimeString,
  })
  .passthrough();

export const ImportConflictListResponseSchema = z
  .object({
    conflicts: z.array(ImportConflictSchema),
    total: z.number(),
    limit: z.number(),
    offset: z.number(),
  })
  .passthrough();

// ==========================================================
// 批量处理导入冲突 响应 Schema
// ==========================================================

export const BatchResolveConflictsResponseSchema = z
  .object({
    success_count: z.number(),
    fail_count: z.number(),
    message: z.string(),
    all_resolved: z.boolean(),
    failed_ids: z.array(z.string()).optional(),
    details: z.record(z.unknown()).nullable().optional(),
  })
  .passthrough();

export type BatchResolveConflictsResponse = z.infer<typeof BatchResolveConflictsResponseSchema>;

// ==========================================================
// 取消导入批次 响应 Schema
// ==========================================================

export const CancelImportBatchResponseSchema = z
  .object({
    deleted_materials: z.number(),
    deleted_conflicts: z.number(),
    message: z.string(),
  })
  .passthrough();

export type CancelImportBatchResponse = z.infer<typeof CancelImportBatchResponseSchema>;


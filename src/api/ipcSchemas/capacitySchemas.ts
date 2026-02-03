import { z } from 'zod';

import { DateString } from './_shared';
import { ManualRefreshDecisionResponseSchema } from './planSchemas';

// ==========================================================
// Capacity API 响应 Schema（优先级3）
// ==========================================================

export const CapacityPoolSchema = z
  .object({
    version_id: z.string(),
    machine_code: z.string(),
    plan_date: DateString,
    target_capacity_t: z.number(),
    limit_capacity_t: z.number(),
    used_capacity_t: z.number(),
    overflow_t: z.number(),
    frozen_capacity_t: z.number(),
    accumulated_tonnage_t: z.number(),
    roll_campaign_id: z.string().nullable().optional(),
  })
  .passthrough();

export const BatchUpdateCapacityPoolsResponseSchema = z
  .object({
    requested: z.number(),
    updated: z.number(),
    skipped: z.number(),
    upserted_rows: z.number(),
    refresh: ManualRefreshDecisionResponseSchema.nullable().optional(),
    message: z.string(),
  })
  .passthrough();


import { z } from 'zod';

import { DateString, DateTimeString } from './_shared';

// ==========================================================
// Roller / Roll Campaign API 响应 Schema（优先级3）
// ==========================================================

export const RollCampaignPlanInfoSchema = z
  .object({
    version_id: z.string(),
    machine_code: z.string(),
    initial_start_at: DateTimeString,
    next_change_at: DateTimeString.nullable().optional(),
    downtime_minutes: z.number().nullable().optional(),
    updated_at: DateTimeString,
    updated_by: z.string().nullable().optional(),
  })
  .passthrough();

export const RollerCampaignInfoSchema = z
  .object({
    version_id: z.string(),
    machine_code: z.string(),
    campaign_no: z.number(),
    start_date: DateString,
    end_date: DateString.nullable().optional(),
    cum_weight_t: z.number(),
    suggest_threshold_t: z.number(),
    hard_limit_t: z.number(),
    status: z.string(),
    is_active: z.boolean(),
    remaining_tonnage_t: z.number(),
    utilization_ratio: z.number(),
    should_change_roll: z.boolean(),
    is_hard_stop: z.boolean(),
  })
  .passthrough();


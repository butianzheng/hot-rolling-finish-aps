import { z } from 'zod';

import { DateString } from './_shared';

// ==========================================================
// Action Log 响应 Schema
// ==========================================================

export const ActionLogSchema = z
  .object({
    action_id: z.string(),
    version_id: z.string().nullable(),
    action_type: z.string(),
    action_ts: z.string(),
    actor: z.string(),
    payload_json: z.record(z.unknown()).nullable().optional(),
    impact_summary_json: z.record(z.unknown()).nullable().optional(),
    machine_code: z.string().nullable().optional(),
    date_range_start: DateString.nullable().optional(),
    date_range_end: DateString.nullable().optional(),
    detail: z.string().nullable().optional(),
  })
  .passthrough();


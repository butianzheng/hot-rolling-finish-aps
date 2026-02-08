import { z } from 'zod';

import { DateString } from './_shared';

// ==========================================================
// Material API 响应 Schema（优先级2）
// ==========================================================

export const MaterialWithStateSchema = z
  .object({
    material_id: z.string(),
    machine_code: z.string().nullable().optional(),
    weight_t: z.number().nullable().optional(),
    width_mm: z.number().nullable().optional(),
    thickness_mm: z.number().nullable().optional(),
    steel_mark: z.string().nullable().optional(),
    contract_no: z.string().nullable().optional(),
    due_date: DateString.nullable().optional(),
    sched_state: z.string(),
    urgent_level: z.string(),
    lock_flag: z.boolean(),
    manual_urgent_flag: z.boolean(),
    scheduled_date: DateString.nullable().optional(),
    scheduled_machine_code: z.string().nullable().optional(),
    seq_no: z.number().nullable().optional(),
    rolling_output_age_days: z.number().nullable().optional(),
    stock_age_days: z.number().nullable().optional(),
  })
  .passthrough();

export const MaterialMasterSchema = z
  .object({
    material_id: z.string(),
    manufacturing_order_id: z.string().nullable().optional(),
    material_status_code_src: z.string().nullable().optional(),
    steel_mark: z.string().nullable().optional(),
    slab_id: z.string().nullable().optional(),
    next_machine_code: z.string().nullable().optional(),
    rework_machine_code: z.string().nullable().optional(),
    current_machine_code: z.string().nullable().optional(),
    width_mm: z.number().nullable().optional(),
    thickness_mm: z.number().nullable().optional(),
    length_m: z.number().nullable().optional(),
    weight_t: z.number().nullable().optional(),
    available_width_mm: z.number().nullable().optional(),
    due_date: DateString.nullable().optional(),
    stock_age_days: z.number().nullable().optional(),
    output_age_days_raw: z.number().nullable().optional(),
    rolling_output_date: DateString.nullable().optional(), // v0.7: 轧制产出日期（动态计算基准）
    status_updated_at: z.string().nullable().optional(),
    contract_no: z.string().nullable().optional(),
    contract_nature: z.string().nullable().optional(),
    weekly_delivery_flag: z.string().nullable().optional(),
    export_flag: z.string().nullable().optional(),
    created_at: z.string(),
    updated_at: z.string(),
  })
  .passthrough();

export const MaterialStateSchema = z
  .object({
    material_id: z.string(),
    sched_state: z.string(),
    lock_flag: z.boolean(),
    force_release_flag: z.boolean(),
    urgent_level: z.string(),
    urgent_reason: z.string().nullable().optional(),
    rush_level: z.string(),
    rolling_output_age_days: z.number(),
    ready_in_days: z.number(),
    earliest_sched_date: DateString.nullable().optional(),
    stock_age_days: z.number(),
    scheduled_date: DateString.nullable().optional(),
    scheduled_machine_code: z.string().nullable().optional(),
    seq_no: z.number().nullable().optional(),
    manual_urgent_flag: z.boolean(),
    in_frozen_zone: z.boolean(),
    last_calc_version_id: z.string().nullable().optional(),
    updated_at: z.string(),
    updated_by: z.string().nullable().optional(),
  })
  .passthrough();

export const MaterialDetailResponseSchema = z
  .object({
    master: MaterialMasterSchema.nullable().optional(),
    state: MaterialStateSchema.nullable().optional(),
  })
  .passthrough();

export const MaterialPoolStateSummarySchema = z
  .object({
    sched_state: z.string(),
    count: z.number(),
  })
  .passthrough();

export const MaterialPoolMachineSummarySchema = z
  .object({
    machine_code: z.string(),
    total_count: z.number(),
    states: z.array(MaterialPoolStateSummarySchema),
  })
  .passthrough();

export const MaterialPoolSummaryResponseSchema = z
  .object({
    total_count: z.number(),
    machines: z.array(MaterialPoolMachineSummarySchema),
  })
  .passthrough();

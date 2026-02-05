import { z } from 'zod';

import { DateString } from './_shared';

// ==========================================================
// Machine Capacity Config API Schema
// ==========================================================

// 机组产能配置 DTO
export const MachineConfigSchema = z
  .object({
    config_id: z.string(),
    version_id: z.string(),
    machine_code: z.string(),
    default_daily_target_t: z.number(),
    default_daily_limit_pct: z.number(),
    effective_date: DateString.nullable().optional(),
    created_at: z.string(),
    updated_at: z.string(),
    created_by: z.string(),
    reason: z.string().nullable().optional(),
  })
  .passthrough();

// 创建或更新机组配置请求
export const CreateOrUpdateMachineConfigRequestSchema = z
  .object({
    version_id: z.string().min(1),
    machine_code: z.string().min(1),
    default_daily_target_t: z.number().positive(),
    default_daily_limit_pct: z.number().min(1.0),
    effective_date: DateString.nullable().optional(),
    reason: z.string().min(1),
    operator: z.string().min(1),
  })
  .passthrough();

// 创建或更新机组配置响应
export const CreateOrUpdateMachineConfigResponseSchema = z
  .object({
    success: z.boolean(),
    config_id: z.string(),
    message: z.string(),
  })
  .passthrough();

// 应用配置到日期范围请求
export const ApplyConfigToDateRangeRequestSchema = z
  .object({
    version_id: z.string().min(1),
    machine_code: z.string().min(1),
    date_from: DateString,
    date_to: DateString,
    reason: z.string().min(1),
    operator: z.string().min(1),
  })
  .passthrough();

// 应用配置到日期范围响应
export const ApplyConfigToDateRangeResponseSchema = z
  .object({
    success: z.boolean(),
    updated_count: z.number(),
    skipped_count: z.number(),
    message: z.string(),
  })
  .passthrough();

// ==========================================================
// 产能池日历相关 Schema
// ==========================================================

// 产能池日历数据（单个日期）
export const CapacityPoolCalendarDataSchema = z
  .object({
    plan_date: DateString,
    machine_code: z.string(),
    target_capacity_t: z.number(),
    used_capacity_t: z.number(),
    limit_capacity_t: z.number(),
    utilization_pct: z.number(), // 利用率（used/target）
    color: z.string(), // 颜色映射（绿-蓝-橙-红）
  })
  .passthrough();

// ==========================================================
// 导出类型定义（供组件使用）
// ==========================================================

export type MachineConfig = z.infer<typeof MachineConfigSchema>;
export type CreateOrUpdateMachineConfigRequest = z.infer<
  typeof CreateOrUpdateMachineConfigRequestSchema
>;
export type CreateOrUpdateMachineConfigResponse = z.infer<
  typeof CreateOrUpdateMachineConfigResponseSchema
>;
export type ApplyConfigToDateRangeRequest = z.infer<typeof ApplyConfigToDateRangeRequestSchema>;
export type ApplyConfigToDateRangeResponse = z.infer<typeof ApplyConfigToDateRangeResponseSchema>;
export type CapacityPoolCalendarData = z.infer<typeof CapacityPoolCalendarDataSchema>;

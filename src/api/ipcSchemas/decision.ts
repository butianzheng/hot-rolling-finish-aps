// ==========================================
// 决策层 Zod Schema 验证模式（IPC 契约）
// ==========================================
// 提供运行时类型验证，确保后端响应与TypeScript类型匹配
// 注意：后端返回 snake_case，此 schema 验证原始响应
// 转换为 camelCase 在 decision-service.ts 中进行
// ==========================================

import { z } from 'zod';

// ==========================================
// 通用 Schema（匹配后端 snake_case 响应）
// ==========================================

/**
 * 原因项 Schema
 */
export const ReasonItemSchema = z.object({
  code: z.string(),
  msg: z.string(),
  weight: z.number().min(0).max(1),
  affected_count: z.number().int().nonnegative().optional(),
});

/**
 * 类型统计 Schema
 */
export const TypeCountSchema = z.object({
  type_name: z.string(),
  count: z.number().int().nonnegative(),
  weight_t: z.number().nonnegative(),
});

// ==========================================
// D1: 日期风险摘要 Schema
// ==========================================

/**
 * 风险等级 Schema
 */
export const RiskLevelSchema = z.enum(['LOW', 'MEDIUM', 'HIGH', 'CRITICAL']);

/**
 * 日期摘要 Schema
 */
export const DaySummarySchema = z.object({
  plan_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  risk_score: z.number().min(0).max(100),
  risk_level: RiskLevelSchema,
  capacity_util_pct: z.number().nonnegative(),
  overload_weight_t: z.number().nonnegative(),
  urgent_failure_count: z.number().int().nonnegative(),
  top_reasons: z.array(ReasonItemSchema),
  involved_machines: z.array(z.string()),
});

/**
 * D1 响应 Schema
 */
export const DecisionDaySummaryResponseSchema = z.object({
  version_id: z.string(),
  as_of: z.string(),
  items: z.array(DaySummarySchema),
  total_count: z.number().int().nonnegative(),
});

// ==========================================
// D4: 机组堵塞概况 Schema
// ==========================================

/**
 * 堵塞等级 Schema
 */
export const BottleneckLevelSchema = z.enum(['NONE', 'LOW', 'MEDIUM', 'HIGH', 'CRITICAL']);

/**
 * 堵塞类型 Schema
 */
export const BottleneckTypeSchema = z.enum([
  'Capacity',
  'Structure',
  'RollChange',
  'ColdStock',
  'Mixed',
]);

/**
 * 堵塞点 Schema
 */
export const BottleneckPointSchema = z.object({
  machine_code: z.string(),
  plan_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  bottleneck_score: z.number().min(0).max(100),
  bottleneck_level: BottleneckLevelSchema,
  bottleneck_types: z.array(BottleneckTypeSchema),
  capacity_util_pct: z.number().nonnegative(),
  pending_material_count: z.number().int().nonnegative(),
  pending_weight_t: z.number().nonnegative(),
  scheduled_material_count: z.number().int().nonnegative(),
  scheduled_weight_t: z.number().nonnegative(),
  reasons: z.array(ReasonItemSchema),
  recommended_actions: z.array(z.string()).optional(),
});

/**
 * 机组统计 Schema
 */
export const MachineStatsSchema = z.object({
  machine_code: z.string(),
  avg_score: z.number().nonnegative(),
  max_score: z.number().nonnegative(),
  bottleneck_days: z.number().int().nonnegative(),
});

/**
 * 热力图统计 Schema
 */
export const HeatmapStatsSchema = z.object({
  avg_score: z.number().nonnegative(),
  max_score: z.number().nonnegative(),
  bottleneck_days_count: z.number().int().nonnegative(),
  by_machine: z.array(MachineStatsSchema),
});

/**
 * D4 响应 Schema
 */
export const MachineBottleneckProfileResponseSchema = z.object({
  version_id: z.string(),
  as_of: z.string(),
  items: z.array(BottleneckPointSchema),
  total_count: z.number().int().nonnegative(),
  heatmap_stats: HeatmapStatsSchema.optional(),
});

// ==========================================
// D2: 订单失败集合 Schema
// ==========================================

/**
 * 失败类型 Schema
 */
export const FailTypeSchema = z.string().min(1);

/**
 * 紧急等级 Schema
 */
export const UrgencyLevelSchema = z.enum(['L0', 'L1', 'L2', 'L3']);

/**
 * 阻塞因素 Schema
 */
export const BlockingFactorSchema = z.object({
  factor_type: z.string(),
  description: z.string(),
  impact: z.number().min(0).max(1),
  affected_material_count: z.number().int().nonnegative(),
});

/**
 * 订单失败 Schema
 */
export const OrderFailureSchema = z.object({
  contract_no: z.string(),
  material_id: z.string().optional(),
  due_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  days_to_due: z.number().int(),
  urgency_level: UrgencyLevelSchema,
  fail_type: FailTypeSchema,
  // 后端 DecisionApiImpl 返回的是百分比（0-100）。
  completion_rate: z.number().min(0).max(100),
  total_weight_t: z.number().nonnegative(),
  scheduled_weight_t: z.number().nonnegative(),
  unscheduled_weight_t: z.number().nonnegative(),
  machine_code: z.string(),
  blocking_factors: z.array(BlockingFactorSchema),
  failure_reasons: z.array(z.string()),
  recommended_actions: z.array(z.string()).optional(),
});

/**
 * 材料失败 Schema（材料维度）
 */
export const MaterialFailureSchema = z.object({
  material_id: z.string(),
  contract_no: z.string(),
  due_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  days_to_due: z.number().int(),
  urgency_level: UrgencyLevelSchema,
  fail_type: FailTypeSchema,
  completion_rate: z.number().min(0).max(100),
  weight_t: z.number().nonnegative(),
  unscheduled_weight_t: z.number().nonnegative(),
  machine_code: z.string(),
  is_scheduled: z.boolean(),
  blocking_factors: z.array(BlockingFactorSchema),
  failure_reasons: z.array(z.string()),
  recommended_actions: z.array(z.string()).optional(),
});

/**
 * 订单失败摘要 Schema
 */
export const OrderFailureSummarySchema = z.object({
  total_failures: z.number().int().nonnegative(),
  by_fail_type: z.array(TypeCountSchema),
  by_urgency: z.array(TypeCountSchema),
  total_unscheduled_weight_t: z.number().nonnegative(),
});

/**
 * 材料失败摘要 Schema
 */
export const MaterialFailureSummarySchema = z.object({
  total_failed_materials: z.number().int().nonnegative(),
  total_failed_contracts: z.number().int().nonnegative(),
  overdue_materials: z.number().int().nonnegative(),
  unscheduled_materials: z.number().int().nonnegative(),
  total_unscheduled_weight_t: z.number().nonnegative(),
  by_fail_type: z.array(TypeCountSchema),
  by_urgency: z.array(TypeCountSchema),
});

/**
 * 材料失败合同聚合 Schema
 */
export const MaterialFailureContractAggregateSchema = z.object({
  contract_no: z.string(),
  material_count: z.number().int().nonnegative(),
  unscheduled_count: z.number().int().nonnegative(),
  overdue_count: z.number().int().nonnegative(),
  earliest_due_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  max_urgency_level: UrgencyLevelSchema,
  representative_material_id: z.string(),
});

/**
 * D2 响应 Schema
 */
export const OrderFailureSetResponseSchema = z.object({
  version_id: z.string(),
  as_of: z.string(),
  items: z.array(OrderFailureSchema),
  total_count: z.number().int().nonnegative(),
  summary: OrderFailureSummarySchema,
});

/**
 * D2M 响应 Schema（材料维度）
 */
export const MaterialFailureSetResponseSchema = z.object({
  version_id: z.string(),
  as_of: z.string(),
  items: z.array(MaterialFailureSchema),
  total_count: z.number().int().nonnegative(),
  summary: MaterialFailureSummarySchema,
  contract_aggregates: z.array(MaterialFailureContractAggregateSchema),
});

// ==========================================
// D3: 冷料压库概况 Schema
// ==========================================

/**
 * 压力等级 Schema
 */
export const PressureLevelSchema = z.enum(['LOW', 'MEDIUM', 'HIGH', 'CRITICAL']);

/**
 * 年龄分桶 Schema
 */
export const AgeBinSchema = z.enum(['0-7', '8-14', '15-30', '30+']);

/**
 * 趋势方向 Schema
 */
export const TrendDirectionSchema = z.enum(['RISING', 'STABLE', 'FALLING']);

/**
 * 结构性缺口 Schema
 */
export const StructureGapSchema = z.string();

/**
 * 冷料趋势 Schema
 */
export const ColdStockTrendSchema = z.object({
  direction: TrendDirectionSchema,
  change_rate_pct: z.number(),
  baseline_days: z.number().int().nonnegative(),
});

/**
 * 冷料分桶 Schema
 */
export const ColdStockBucketSchema = z.object({
  machine_code: z.string(),
  age_bin: AgeBinSchema,
  count: z.number().int().nonnegative(),
  weight_t: z.number().nonnegative(),
  pressure_score: z.number().min(0).max(100),
  pressure_level: PressureLevelSchema,
  avg_age_days: z.number().nonnegative(),
  max_age_days: z.number().int().nonnegative(),
  structure_gap: StructureGapSchema,
  reasons: z.array(ReasonItemSchema),
  trend: ColdStockTrendSchema.optional(),
});

/**
 * 机组库存统计 Schema
 */
export const MachineStockStatsSchema = z.object({
  machine_code: z.string(),
  count: z.number().int().nonnegative(),
  weight_t: z.number().nonnegative(),
  avg_pressure_score: z.number().nonnegative(),
});

/**
 * 年龄分桶统计 Schema
 */
export const AgeBinStatsSchema = z.object({
  age_bin: AgeBinSchema,
  count: z.number().int().nonnegative(),
  weight_t: z.number().nonnegative(),
});

/**
 * 冷料摘要 Schema
 */
export const ColdStockSummarySchema = z.object({
  total_cold_stock_count: z.number().int().nonnegative(),
  total_cold_stock_weight_t: z.number().nonnegative(),
  avg_age_days: z.number().nonnegative(),
  high_pressure_count: z.number().int().nonnegative(),
  by_machine: z.array(MachineStockStatsSchema),
  by_age_bin: z.array(AgeBinStatsSchema),
});

/**
 * D3 响应 Schema
 */
export const ColdStockProfileResponseSchema = z.object({
  version_id: z.string(),
  as_of: z.string(),
  items: z.array(ColdStockBucketSchema),
  total_count: z.number().int().nonnegative(),
  summary: ColdStockSummarySchema,
});

// ==========================================
// 错误响应 Schema
// ==========================================

/**
 * 错误响应 Schema
 */
export const ErrorResponseSchema = z.object({
  code: z.string(),
  message: z.string(),
  // H13修复：使用z.unknown()替代z.any()以保持类型安全
  details: z.unknown().optional(),
});

// ==========================================
// D5: 轧制活动警报 Schema (对齐 Rust DTO)
// ==========================================

/**
 * 轧制活动警报 Schema
 * 对应 Rust: RollAlertDto (src/decision/api/dto.rs L417-433)
 */
export const RollCampaignAlertSchema = z.object({
  machine_code: z.string(),
  campaign_id: z.string(),
  campaign_start_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  current_tonnage_t: z.number().nonnegative(),
  soft_limit_t: z.number().nonnegative(),
  hard_limit_t: z.number().nonnegative(),
  remaining_tonnage_t: z.number(),
  alert_level: z.string(),
  alert_type: z.string(),
  // Rust DTO uses `#[serde(skip_serializing_if = "Option::is_none")]`,
  // so this field may be omitted entirely when no estimated date is available.
  estimated_hard_stop_date: z
    .string()
    .regex(/^\d{4}-\d{2}-\d{2}$/)
    .nullable()
    .optional(),
  // New monitoring fields (optional): datetime strings in SQLite format.
  campaign_start_at: z
    .string()
    .regex(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/)
    .optional(),
  planned_change_at: z
    .string()
    .regex(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/)
    .nullable()
    .optional(),
  planned_downtime_minutes: z.number().int().positive().optional(),
  estimated_soft_reach_at: z
    .string()
    .regex(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/)
    .nullable()
    .optional(),
  estimated_hard_reach_at: z
    .string()
    .regex(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/)
    .nullable()
    .optional(),
  alert_message: z.string(),
  impact_description: z.string(),
  recommended_actions: z.array(z.string()),
});

/**
 * 换辊警报摘要 Schema
 * 对应 Rust: RollAlertSummaryDto (src/decision/api/dto.rs L436-442)
 */
export const RollAlertSummarySchema = z.object({
  total_alerts: z.number().int().nonnegative(),
  by_level: z.array(TypeCountSchema),
  by_type: z.array(TypeCountSchema),
  near_hard_stop_count: z.number().int().nonnegative(),
});

/**
 * D5 响应 Schema
 * 对应 Rust: RollCampaignAlertsResponse (src/decision/api/dto.rs L406-414)
 */
export const RollCampaignAlertResponseSchema = z.object({
  version_id: z.string(),
  as_of: z.string(),
  items: z.array(RollCampaignAlertSchema),
  total_count: z.number().int().nonnegative(),
  summary: RollAlertSummarySchema,
});

// ==========================================
// D6: 容量优化机会 Schema (对齐 Rust DTO)
// ==========================================

/**
 * 场景 Schema
 * 对应 Rust: ScenarioDto
 */
export const ScenarioSchema = z.object({
  name: z.string(),
  adjustment: z.string(),
  util_pct: z.number(),
  risk_score: z.number(),
  affected_material_count: z.number().int().nonnegative(),
});

/**
 * 敏感性分析 Schema
 * 对应 Rust: SensitivityAnalysisDto
 */
export const SensitivityAnalysisSchema = z.object({
  scenarios: z.array(ScenarioSchema),
  best_scenario_index: z.number().int().nonnegative(),
});

/**
 * 容量优化机会 Schema
 * 对应 Rust: CapacityOpportunityDto (L477-492)
 */
export const CapacityOpportunitySchema = z.object({
  machine_code: z.string(),
  plan_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  opportunity_type: z.string(),
  current_util_pct: z.number(),
  target_capacity_t: z.number().nonnegative(),
  used_capacity_t: z.number().nonnegative(),
  opportunity_space_t: z.number(),
  optimized_util_pct: z.number(),
  sensitivity: SensitivityAnalysisSchema.optional(),
  description: z.string(),
  recommended_actions: z.array(z.string()),
  potential_benefits: z.array(z.string()),
});

/**
 * 容量优化机会摘要 Schema
 * 对应 Rust: CapacityOpportunitySummaryDto
 */
export const CapacityOpportunitySummarySchema = z.object({
  total_opportunities: z.number().int().nonnegative(),
  total_opportunity_space_t: z.number(),
  by_type: z.array(TypeCountSchema),
  avg_current_util_pct: z.number(),
  avg_optimized_util_pct: z.number(),
});

/**
 * D6 响应 Schema
 * 对应 Rust: CapacityOpportunityResponse
 */
export const CapacityOpportunityResponseSchema = z.object({
  version_id: z.string(),
  as_of: z.string(),
  items: z.array(CapacityOpportunitySchema),
  total_count: z.number().int().nonnegative(),
  summary: CapacityOpportunitySummarySchema,
});

// 兼容旧名称
export const DailyCapacityOpportunitySchema = CapacityOpportunitySchema;

// ==========================================
// 类型推断（保持 snake_case，供验证使用）
// 前端使用时应通过 decision-service 获取 camelCase 版本
// ==========================================

export type DecisionDaySummaryResponseRaw = z.infer<typeof DecisionDaySummaryResponseSchema>;
export type MachineBottleneckProfileResponseRaw = z.infer<
  typeof MachineBottleneckProfileResponseSchema
>;
export type OrderFailureSetResponseRaw = z.infer<typeof OrderFailureSetResponseSchema>;
export type MaterialFailureSetResponseRaw = z.infer<typeof MaterialFailureSetResponseSchema>;
export type ColdStockProfileResponseRaw = z.infer<typeof ColdStockProfileResponseSchema>;
export type RollCampaignAlertResponseRaw = z.infer<typeof RollCampaignAlertResponseSchema>;
export type CapacityOpportunityResponseRaw = z.infer<typeof CapacityOpportunityResponseSchema>;
export type ErrorResponseType = z.infer<typeof ErrorResponseSchema>;

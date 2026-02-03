import { z } from 'zod';

// ==========================================
// P2-1: 统一 IPC Schema 源
// ==========================================
// 从 decision-schema.ts 导入 D1-D6 决策看板 Schema，避免重复定义
// decision-schema.ts 是决策层的单一真实源（Single Source of Truth）
import {
  DecisionDaySummaryResponseSchema as DecisionDaySummaryResponseSchemaStrict,
  MachineBottleneckProfileResponseSchema as MachineBottleneckProfileResponseSchemaStrict,
  OrderFailureSetResponseSchema as OrderFailureSetResponseSchemaStrict,
  ColdStockProfileResponseSchema as ColdStockProfileResponseSchemaStrict,
  RollCampaignAlertResponseSchema as RollCampaignAlertResponseSchemaStrict,
  CapacityOpportunityResponseSchema as CapacityOpportunityResponseSchemaStrict,
} from '../types/schemas/decision-schema';

export { z };

// ==========================================================
// IPC response schemas (Zod)
//
// Goal:
// - Catch backend/frontend contract drift early (before UI logic runs).
// - Be strict on required fields, permissive on unknown extra fields.
// ==========================================================

export function zodValidator<T extends z.ZodTypeAny>(schema: T, name: string) {
  return (value: unknown): z.infer<T> => {
    const parsed = schema.safeParse(value);
    if (parsed.success) return parsed.data;

    throw {
      code: 'IPC_SCHEMA_MISMATCH',
      message: `IPC 响应契约校验失败: ${name}`,
      details: {
        issues: parsed.error.issues,
      },
    };
  };
}

const DateString = z.string().min(1);
const DateTimeString = z.string().min(1);

// ==========================================================
// Common / shared response schemas
// ==========================================================

// Common success response for commands that return `{}`.
export const EmptyOkResponseSchema = z.object({}).passthrough();

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
// P0-1: strategy draft (draft persistence)
// ==========================================================

export const StrategyDraftSummarySchema = z
  .object({
    draft_id: z.string(),
    base_version_id: z.string(),
    strategy: z.string(),
    plan_items_count: z.number(),
    frozen_items_count: z.number(),
    calc_items_count: z.number(),
    mature_count: z.number(),
    immature_count: z.number(),
    total_capacity_used_t: z.number(),
    overflow_days: z.number(),
    moved_count: z.number(),
    added_count: z.number(),
    removed_count: z.number(),
    squeezed_out_count: z.number(),
    message: z.string(),
  })
  .passthrough();

export const GenerateStrategyDraftsResponseSchema = z
  .object({
    base_version_id: z.string(),
    plan_date_from: DateString,
    plan_date_to: DateString,
    drafts: z.array(StrategyDraftSummarySchema),
    message: z.string(),
  })
  .passthrough();

export const ListStrategyDraftsResponseSchema = GenerateStrategyDraftsResponseSchema;

export const ApplyStrategyDraftResponseSchema = z
  .object({
    version_id: z.string(),
    success: z.boolean(),
    message: z.string(),
  })
  .passthrough();

export const StrategyDraftDiffItemSchema = z
  .object({
    material_id: z.string(),
    change_type: z.string(),

    from_plan_date: DateString.nullable().optional(),
    from_machine_code: z.string().nullable().optional(),
    from_seq_no: z.number().nullable().optional(),

    to_plan_date: DateString.nullable().optional(),
    to_machine_code: z.string().nullable().optional(),
    to_seq_no: z.number().nullable().optional(),

    to_assign_reason: z.string().nullable().optional(),
    to_urgent_level: z.string().nullable().optional(),
    to_sched_state: z.string().nullable().optional(),

    material_state_snapshot: z
      .object({
        sched_state: z.string().nullable().optional(),
        urgent_level: z.string().nullable().optional(),
        rush_level: z.string().nullable().optional(),
        lock_flag: z.boolean().nullable().optional(),
        force_release_flag: z.boolean().nullable().optional(),
        manual_urgent_flag: z.boolean().nullable().optional(),
        in_frozen_zone: z.boolean().nullable().optional(),
        ready_in_days: z.number().nullable().optional(),
        earliest_sched_date: DateString.nullable().optional(),
        scheduled_date: DateString.nullable().optional(),
        scheduled_machine_code: z.string().nullable().optional(),
        seq_no: z.number().nullable().optional(),
      })
      .passthrough()
      .nullable()
      .optional(),
  })
  .passthrough();

export const GetStrategyDraftDetailResponseSchema = z
  .object({
    draft_id: z.string(),
    base_version_id: z.string(),
    plan_date_from: DateString,
    plan_date_to: DateString,
    strategy: z.string(),
    diff_items: z.array(StrategyDraftDiffItemSchema),
    diff_items_total: z.number(),
    diff_items_truncated: z.boolean(),
    message: z.string(),
  })
  .passthrough();

export const CleanupStrategyDraftsResponseSchema = z
  .object({
    deleted_count: z.number(),
    message: z.string(),
  })
  .passthrough();

// ==========================================================
// P1-1: version comparison KPI aggregation
// ==========================================================

export const VersionDiffCountsSchema = z
  .object({
    moved_count: z.number(),
    added_count: z.number(),
    removed_count: z.number(),
    squeezed_out_count: z.number(),
  })
  .passthrough();

export const VersionKpiSummarySchema = z
  .object({
    plan_items_count: z.number(),
    total_weight_t: z.number(),
    locked_in_plan_count: z.number(),
    force_release_in_plan_count: z.number(),
    plan_date_from: DateString.nullable(),
    plan_date_to: DateString.nullable(),

    overflow_days: z.number().nullable(),
    overflow_t: z.number().nullable(),
    capacity_used_t: z.number().nullable(),
    capacity_target_t: z.number().nullable(),
    capacity_limit_t: z.number().nullable(),
    capacity_util_pct: z.number().nullable(),
    mature_backlog_t: z.number().nullable(),
    immature_backlog_t: z.number().nullable(),
    urgent_total_t: z.number().nullable(),
    snapshot_date_from: DateString.nullable(),
    snapshot_date_to: DateString.nullable(),
  })
  .passthrough();

export const VersionComparisonKpiResultSchema = z
  .object({
    version_id_a: z.string(),
    version_id_b: z.string(),
    kpi_a: VersionKpiSummarySchema,
    kpi_b: VersionKpiSummarySchema,
    diff_counts: VersionDiffCountsSchema,
    message: z.string(),
  })
  .passthrough();

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

// ==========================================================
// Dashboard API 响应 Schema（优先级1 - D1-D6决策看板）
// ==========================================================
// P2-1 修复：从 decision-schema.ts 导入统一 schema，避免双重定义
// 保留 .passthrough() 以向后兼容（宽松解析）
//
// 注意：decision-schema.ts 是真实源，这里只是re-export并添加 .passthrough()

// D1: 哪天最危险
export const DecisionDaySummaryResponseSchema = DecisionDaySummaryResponseSchemaStrict.passthrough();

// D2: 哪些紧急单无法完成
export const OrderFailureSetResponseSchema = OrderFailureSetResponseSchemaStrict.passthrough();

// D3: 哪些冷料压库
export const ColdStockProfileResponseSchema = ColdStockProfileResponseSchemaStrict.passthrough();

// D4: 哪个机组最堵
export const MachineBottleneckProfileResponseSchema = MachineBottleneckProfileResponseSchemaStrict.passthrough();

// D5: 换辊是否异常 (保留 plural 命名以兼容旧代码)
export const RollCampaignAlertsResponseSchema = RollCampaignAlertResponseSchemaStrict.passthrough();

// D6: 是否存在产能优化空间
export const CapacityOpportunityResponseSchema = CapacityOpportunityResponseSchemaStrict.passthrough();

// ==========================================================
// Material API 响应 Schema（优先级2）
// ==========================================================

export const MaterialWithStateSchema = z
  .object({
    material_id: z.string(),
    machine_code: z.string().nullable().optional(),
    weight_t: z.number().nullable().optional(),
    steel_mark: z.string().nullable().optional(),
    sched_state: z.string(),
    urgent_level: z.string(),
    lock_flag: z.boolean(),
    manual_urgent_flag: z.boolean(),
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

export const RiskDeltaSchema = z
  .object({
    date: DateString,
    risk_score_a: z.number().nullable().optional(),
    risk_score_b: z.number().nullable().optional(),
    risk_score_delta: z.number(),
  })
  .passthrough();

export const CapacityDeltaSchema = z
  .object({
    machine_code: z.string(),
    date: DateString,
    used_capacity_a: z.number().nullable().optional(),
    used_capacity_b: z.number().nullable().optional(),
    capacity_delta: z.number(),
  })
  .passthrough();

export const ConfigChangeSchema = z
  .object({
    key: z.string(),
    value_a: z.string().nullable().optional(),
    value_b: z.string().nullable().optional(),
  })
  .passthrough();

export const VersionComparisonResultSchema = z
  .object({
    version_id_a: z.string(),
    version_id_b: z.string(),
    moved_count: z.number(),
    added_count: z.number(),
    removed_count: z.number(),
    squeezed_out_count: z.number(),
    risk_delta: z.array(RiskDeltaSchema).nullable().optional(),
    capacity_delta: z.array(CapacityDeltaSchema).nullable().optional(),
    config_changes: z.array(ConfigChangeSchema).nullable().optional(),
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

// ==========================================================
// 宽厚路径规则 (v0.6) 响应 Schema
// ==========================================================

export const PathRuleConfigSchema = z
  .object({
    enabled: z.boolean(),
    width_tolerance_mm: z.number(),
    thickness_tolerance_mm: z.number(),
    override_allowed_urgency_levels: z.array(z.string()),
    seed_s2_percentile: z.number(),
    seed_s2_small_sample_threshold: z.number(),
  })
  .passthrough();

export const PathOverridePendingSchema = z
  .object({
    material_id: z.string(),
    material_no: z.string(),
    width_mm: z.number(),
    thickness_mm: z.number(),
    urgent_level: z.string(),
    violation_type: z.string(),
    anchor_width_mm: z.number(),
    anchor_thickness_mm: z.number(),
    width_delta_mm: z.number(),
    thickness_delta_mm: z.number(),
  })
  .passthrough();

export const PathOverridePendingSummarySchema = z
  .object({
    machine_code: z.string(),
    plan_date: DateString,
    pending_count: z.number(),
  })
  .passthrough();

export const RollCycleAnchorSchema = z
  .object({
    version_id: z.string(),
    machine_code: z.string(),
    campaign_no: z.number(),
    cum_weight_t: z.number(),
    anchor_source: z.string(),
    anchor_material_id: z.string().nullable().optional(),
    anchor_width_mm: z.number().nullable().optional(),
    anchor_thickness_mm: z.number().nullable().optional(),
    status: z.string(),
  })
  .passthrough();

export const BatchConfirmPathOverrideResultSchema = z
  .object({
    success_count: z.number(),
    fail_count: z.number(),
    failed_material_ids: z.array(z.string()),
  })
  .passthrough();

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

// ==========================================================
// 每日生产节奏（品种大类）Schema
// ==========================================================

export const PlanRhythmPresetSchema = z
  .object({
    preset_id: z.string(),
    preset_name: z.string(),
    dimension: z.string(),
    target_json: z.string(),
    is_active: z.boolean(),
    created_at: z.string(),
    updated_at: z.string(),
    updated_by: z.string().nullable().optional(),
  })
  .passthrough();

export const PlanRhythmPresetsResponseSchema = z.array(PlanRhythmPresetSchema);

export const PlanRhythmTargetSchema = z
  .object({
    version_id: z.string(),
    machine_code: z.string(),
    plan_date: DateString,
    dimension: z.string(),
    target_json: z.string(),
    preset_id: z.string().nullable().optional(),
    updated_at: z.string(),
    updated_by: z.string().nullable().optional(),
  })
  .passthrough();

export const PlanRhythmTargetsResponseSchema = z.array(PlanRhythmTargetSchema);

export const ApplyRhythmPresetResponseSchema = z
  .object({
    applied: z.number(),
  })
  .passthrough();

export const DailyRhythmCategoryRowSchema = z
  .object({
    category: z.string(),
    scheduled_weight_t: z.number(),
    actual_ratio: z.number(),
    target_ratio: z.number().nullable().optional(),
    diff_ratio: z.number().nullable().optional(),
  })
  .passthrough();

export const DailyRhythmProfileSchema = z
  .object({
    version_id: z.string(),
    machine_code: z.string(),
    plan_date: DateString,
    dimension: z.string(),
    total_scheduled_weight_t: z.number(),
    deviation_threshold: z.number(),
    max_deviation: z.number(),
    is_violated: z.boolean(),
    target_preset_id: z.string().nullable().optional(),
    target_updated_at: z.string().nullable().optional(),
    target_updated_by: z.string().nullable().optional(),
    categories: z.array(DailyRhythmCategoryRowSchema),
  })
  .passthrough();

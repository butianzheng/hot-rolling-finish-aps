// ==========================================
// Tauri API 调用封装层
// ==========================================
// 职责: 封装所有Tauri命令调用，提供类型安全的API
// 红线: 不包含业务逻辑，只负责API调用和错误处理
// 重要: Rust 后端配置了 #[tauri::command(rename_all = "snake_case")]
//       前端必须使用 snake_case 参数名与 Rust 后端对齐
// ==========================================

import { IpcClient } from './ipcClient';
import {
  z,
  zodValidator,
  EmptyOkResponseSchema,
  ImportApiResponseSchema,
  ImportConflictListResponseSchema,
  GenerateStrategyDraftsResponseSchema,
  ListStrategyDraftsResponseSchema,
  ApplyStrategyDraftResponseSchema,
  GetStrategyDraftDetailResponseSchema,
  CleanupStrategyDraftsResponseSchema,
  StrategyPresetSchema,
  ManualRefreshDecisionResponseSchema,
  RollbackVersionResponseSchema,
  VersionComparisonResultSchema,
  MoveItemsResponseSchema,
  VersionComparisonKpiResultSchema,
  DecisionRefreshStatusResponseSchema,
  DecisionDaySummaryResponseSchema,
  MaterialWithStateSchema,
  MaterialDetailResponseSchema,
  PlanSchema,
  PlanVersionSchema,
  PlanItemSchema,
  ConfigItemSchema,
  ActionLogSchema,
  ImpactSummarySchema,
  BatchUpdateConfigsResponseSchema,
  RestoreConfigFromSnapshotResponseSchema,
  ConfigSnapshotSchema,
  CustomStrategyProfileSchema,
  SaveCustomStrategyResponseSchema,
  RecalcResponseSchema,
  CapacityPoolSchema,
  BatchUpdateCapacityPoolsResponseSchema,
  PathRuleConfigSchema,
  PathOverridePendingSchema,
  PathOverridePendingSummarySchema,
  RollCycleAnchorSchema,
  BatchConfirmPathOverrideResultSchema,
  RollCampaignPlanInfoSchema,
  RollerCampaignInfoSchema,
  BatchResolveConflictsResponseSchema,
  type BatchResolveConflictsResponse,
  CancelImportBatchResponseSchema,
  type CancelImportBatchResponse,
  PlanRhythmPresetSchema,
  PlanRhythmPresetsResponseSchema,
  PlanRhythmTargetsResponseSchema,
  ApplyRhythmPresetResponseSchema,
  DailyRhythmProfileSchema,
} from './ipcSchemas';

// ==========================================
// 错误响应类型
// ==========================================

export interface ErrorResponse {
  code: string;
  message: string;
  details?: any;
}

// ==========================================
// Import API
// ==========================================

export const importApi = {
  async importMaterials(
    filePath: string,
    sourceBatchId: string,
    mappingProfileId?: string
  ): Promise<z.infer<typeof ImportApiResponseSchema>> {
    // 使用 snake_case 参数名（后端配置 rename_all = "snake_case"）
    return IpcClient.call('import_materials', {
      file_path: filePath,
      source_batch_id: sourceBatchId,
      mapping_profile_id: mappingProfileId,
    }, {
      validate: zodValidator(ImportApiResponseSchema, 'import_materials'),
    });
  },

  async listImportConflicts(
    status?: string,
    limit: number = 50,
    offset: number = 0,
    batchId?: string
  ): Promise<z.infer<typeof ImportConflictListResponseSchema>> {
    return IpcClient.call('list_import_conflicts', { status, limit, offset, batch_id: batchId }, {
      validate: zodValidator(ImportConflictListResponseSchema, 'list_import_conflicts'),
    });
  },

  async resolveImportConflict(
    conflictId: string,
    action: 'KEEP_EXISTING' | 'OVERWRITE' | 'MERGE',
    note?: string,
    operator: string = 'system'
  ): Promise<void> {
    await IpcClient.call('resolve_import_conflict', {
      conflict_id: conflictId,
      action,
      note,
      operator,
    }, {
      validate: zodValidator(EmptyOkResponseSchema, 'resolve_import_conflict'),
    });
  },

  /**
   * 批量处理导入冲突
   * @param conflictIds 冲突ID列表
   * @param action 处理动作 (KEEP_EXISTING | OVERWRITE | MERGE)
   * @param note 处理备注（可选）
   * @param operator 操作人（可选，默认为 'system'）
   */
  async batchResolveImportConflicts(
    conflictIds: string[],
    action: 'KEEP_EXISTING' | 'OVERWRITE' | 'MERGE',
    note?: string,
    operator: string = 'system'
  ): Promise<BatchResolveConflictsResponse> {
    return IpcClient.call('batch_resolve_import_conflicts', {
      conflict_ids: conflictIds,
      action,
      note,
      operator,
    }, {
      validate: zodValidator(BatchResolveConflictsResponseSchema, 'batch_resolve_import_conflicts'),
    });
  },

  /**
   * 取消导入批次
   * @param batchId 批次ID
   * @param operator 操作人（可选，默认为 'system'）
   */
  async cancelImportBatch(
    batchId: string,
    operator: string = 'system'
  ): Promise<CancelImportBatchResponse> {
    return IpcClient.call('cancel_import_batch', {
      batch_id: batchId,
      operator,
    }, {
      validate: zodValidator(CancelImportBatchResponseSchema, 'cancel_import_batch'),
    });
  },
};

// ==========================================
// Material API
// ==========================================

export const materialApi = {
  async listMaterials(params: {
    // 仅支持 snake_case 风格（与 Rust 后端对齐）
    machine_code?: string;
    steel_grade?: string;
    limit: number;
    offset: number;
  }): Promise<Array<z.infer<typeof MaterialWithStateSchema>>> {
    return IpcClient.call('list_materials', {
      machine_code: params.machine_code,
      steel_grade: params.steel_grade,
      limit: params.limit,
      offset: params.offset,
    }, {
      validate: zodValidator(z.array(MaterialWithStateSchema), 'list_materials'),
    });
  },

  async getMaterialDetail(materialId: string): Promise<z.infer<typeof MaterialDetailResponseSchema>> {
    return IpcClient.call('get_material_detail', { material_id: materialId }, {
      validate: zodValidator(MaterialDetailResponseSchema, 'get_material_detail'),
    });
  },

  async listReadyMaterials(machineCode?: string): Promise<Array<z.infer<typeof MaterialWithStateSchema>>> {
    return IpcClient.call('list_ready_materials', { machine_code: machineCode }, {
      validate: zodValidator(z.array(MaterialWithStateSchema), 'list_ready_materials'),
    });
  },

  async batchLockMaterials(
    materialIds: string[],
    lockFlag: boolean,
    operator: string,
    reason: string,
    mode?: 'Strict' | 'AutoFix'
  ): Promise<z.infer<typeof ImpactSummarySchema>> {
    return IpcClient.call('batch_lock_materials', {
      material_ids: materialIds,
      lock_flag: lockFlag,
      operator,
      reason,
      mode,
    }, {
      validate: zodValidator(ImpactSummarySchema, 'batch_lock_materials'),
    });
  },

  async batchForceRelease(
    materialIds: string[],
    operator: string,
    reason: string,
    mode?: 'Strict' | 'AutoFix'
  ): Promise<z.infer<typeof ImpactSummarySchema>> {
    return IpcClient.call('batch_force_release', {
      material_ids: materialIds,
      operator,
      reason,
      mode,
    }, {
      validate: zodValidator(ImpactSummarySchema, 'batch_force_release'),
    });
  },

  async batchSetUrgent(
    materialIds: string[],
    manualUrgentFlag: boolean,
    operator: string,
    reason: string
  ): Promise<z.infer<typeof ImpactSummarySchema>> {
    return IpcClient.call('batch_set_urgent', {
      material_ids: materialIds,
      manual_urgent_flag: manualUrgentFlag,
      operator,
      reason,
    }, {
      validate: zodValidator(ImpactSummarySchema, 'batch_set_urgent'),
    });
  },

  async listMaterialsByUrgentLevel(
    urgentLevel: string,
    machineCode?: string
  ): Promise<Array<z.infer<typeof MaterialWithStateSchema>>> {
    return IpcClient.call('list_materials_by_urgent_level', {
      urgent_level: urgentLevel,
      machine_code: machineCode,
    }, {
      validate: zodValidator(z.array(MaterialWithStateSchema), 'list_materials_by_urgent_level'),
    });
  },
};

// ==========================================
// Plan API
// ==========================================

export const planApi = {
  async createPlan(planName: string, createdBy: string): Promise<string> {
    return IpcClient.call('create_plan', {
      plan_name: planName,
      created_by: createdBy,
    });
  },

  async listPlans(): Promise<Array<z.infer<typeof PlanSchema>>> {
    return IpcClient.call('list_plans', {}, {
      validate: zodValidator(z.array(PlanSchema), 'list_plans'),
    });
  },

  async getPlanDetail(planId: string): Promise<z.infer<typeof PlanSchema> | null | undefined> {
    return IpcClient.call('get_plan_detail', { plan_id: planId }, {
      validate: zodValidator(PlanSchema.nullable().optional(), 'get_plan_detail'),
    });
  },

  async getLatestActiveVersionId(): Promise<string | null> {
    return IpcClient.call('get_latest_active_version_id');
  },

  async deletePlan(planId: string, operator: string): Promise<void> {
    return IpcClient.call('delete_plan', {
      plan_id: planId,
      operator,
    });
  },

  async deleteVersion(versionId: string, operator: string): Promise<void> {
    return IpcClient.call('delete_version', {
      version_id: versionId,
      operator,
    });
  },

  async createVersion(
    planId: string,
    windowDays: number,
    frozenFromDate?: string,
    note?: string,
    createdBy: string = 'admin'
  ): Promise<string> {
    return IpcClient.call('create_version', {
      plan_id: planId,
      window_days: windowDays,
      frozen_from_date: frozenFromDate,
      note,
      created_by: createdBy,
    });
  },

  async listVersions(planId: string): Promise<Array<z.infer<typeof PlanVersionSchema>>> {
    return IpcClient.call('list_versions', { plan_id: planId }, {
      validate: zodValidator(z.array(PlanVersionSchema), 'list_versions'),
    });
  },

  async activateVersion(versionId: string, operator: string): Promise<void> {
    return IpcClient.call('activate_version', {
      version_id: versionId,
      operator,
    });
  },

  async rollbackVersion(
    planId: string,
    targetVersionId: string,
    operator: string,
    reason: string
  ): Promise<z.infer<typeof RollbackVersionResponseSchema>> {
    return IpcClient.call('rollback_version', {
      plan_id: planId,
      target_version_id: targetVersionId,
      operator,
      reason,
    }, {
      validate: zodValidator(RollbackVersionResponseSchema, 'rollback_version'),
    });
  },

  async simulateRecalc(
    versionId: string,
    baseDate: string,
    frozenDate?: string,
    operator: string = 'admin',
    strategy?: string
  ): Promise<z.infer<typeof RecalcResponseSchema>> {
    return IpcClient.call('simulate_recalc', {
      version_id: versionId,
      base_date: baseDate,
      frozen_date: frozenDate,
      operator,
      strategy,
    }, {
      validate: zodValidator(RecalcResponseSchema, 'simulate_recalc'),
    });
  },

  async recalcFull(
    versionId: string,
    baseDate: string,
    frozenDate?: string,
    operator: string = 'admin',
    strategy?: string
  ): Promise<z.infer<typeof RecalcResponseSchema>> {
    return IpcClient.call('recalc_full', {
      version_id: versionId,
      base_date: baseDate,
      frozen_date: frozenDate,
      operator,
      strategy,
    }, {
      validate: zodValidator(RecalcResponseSchema, 'recalc_full'),
    });
  },

  async getStrategyPresets(): Promise<Array<z.infer<typeof StrategyPresetSchema>>> {
    return IpcClient.call('get_strategy_presets', {}, {
      validate: zodValidator(z.array(StrategyPresetSchema), 'get_strategy_presets'),
    });
  },

  async generateStrategyDrafts(params: {
    base_version_id: string;
    plan_date_from: string;
    plan_date_to: string;
    strategies: string[];
    operator: string;
  }): Promise<z.infer<typeof GenerateStrategyDraftsResponseSchema>> {
    return IpcClient.call('generate_strategy_drafts', {
      base_version_id: params.base_version_id,
      plan_date_from: params.plan_date_from,
      plan_date_to: params.plan_date_to,
      strategies: params.strategies,
      operator: params.operator,
    }, {
      validate: zodValidator(GenerateStrategyDraftsResponseSchema, 'generate_strategy_drafts'),
    });
  },

  async applyStrategyDraft(draftId: string, operator: string): Promise<z.infer<typeof ApplyStrategyDraftResponseSchema>> {
    return IpcClient.call('apply_strategy_draft', {
      draft_id: draftId,
      operator,
    }, {
      validate: zodValidator(ApplyStrategyDraftResponseSchema, 'apply_strategy_draft'),
    });
  },

  async getStrategyDraftDetail(draftId: string): Promise<z.infer<typeof GetStrategyDraftDetailResponseSchema>> {
    return IpcClient.call('get_strategy_draft_detail', {
      draft_id: draftId,
    }, {
      validate: zodValidator(GetStrategyDraftDetailResponseSchema, 'get_strategy_draft_detail'),
    });
  },

  async listStrategyDrafts(params: {
    base_version_id: string;
    plan_date_from: string;
    plan_date_to: string;
    status_filter?: string;
    limit?: number;
  }): Promise<z.infer<typeof ListStrategyDraftsResponseSchema>> {
    return IpcClient.call('list_strategy_drafts', {
      base_version_id: params.base_version_id,
      plan_date_from: params.plan_date_from,
      plan_date_to: params.plan_date_to,
      status_filter: params.status_filter,
      limit: params.limit,
    }, {
      validate: zodValidator(ListStrategyDraftsResponseSchema, 'list_strategy_drafts'),
    });
  },

  async cleanupExpiredStrategyDrafts(keepDays: number): Promise<z.infer<typeof CleanupStrategyDraftsResponseSchema>> {
    return IpcClient.call('cleanup_expired_strategy_drafts', {
      keep_days: keepDays,
    }, {
      validate: zodValidator(CleanupStrategyDraftsResponseSchema, 'cleanup_expired_strategy_drafts'),
    });
  },

  async listPlanItems(
    versionId: string,
    opts?: {
      plan_date_from?: string;
      plan_date_to?: string;
      machine_code?: string;
      limit?: number;
      offset?: number;
    }
  ): Promise<Array<z.infer<typeof PlanItemSchema>>> {
    return IpcClient.call('list_plan_items', {
      version_id: versionId,
      plan_date_from: opts?.plan_date_from,
      plan_date_to: opts?.plan_date_to,
      machine_code: opts?.machine_code,
      limit: opts?.limit,
      offset: opts?.offset,
    }, {
      validate: zodValidator(z.array(PlanItemSchema), 'list_plan_items'),
    });
  },

  async listItemsByDate(versionId: string, planDate: string): Promise<Array<z.infer<typeof PlanItemSchema>>> {
    return IpcClient.call('list_items_by_date', {
      version_id: versionId,
      plan_date: planDate,
    }, {
      validate: zodValidator(z.array(PlanItemSchema), 'list_items_by_date'),
    });
  },

  async compareVersions(versionIdA: string, versionIdB: string): Promise<z.infer<typeof VersionComparisonResultSchema>> {
    return IpcClient.call('compare_versions', {
      version_id_a: versionIdA,
      version_id_b: versionIdB,
    }, {
      validate: zodValidator(VersionComparisonResultSchema, 'compare_versions'),
    });
  },

  async compareVersionsKpi(versionIdA: string, versionIdB: string): Promise<z.infer<typeof VersionComparisonKpiResultSchema>> {
    return IpcClient.call('compare_versions_kpi', {
      version_id_a: versionIdA,
      version_id_b: versionIdB,
    }, {
      validate: zodValidator(VersionComparisonKpiResultSchema, 'compare_versions_kpi'),
    });
  },

  async moveItems(
    versionId: string,
    moves: Array<{
      material_id: string;
      to_date: string;
      to_seq: number;
      to_machine: string;
    }>,
    mode: 'AUTO_FIX' | 'STRICT' = 'AUTO_FIX',
    operator: string = 'system',
    reason?: string
  ): Promise<z.infer<typeof MoveItemsResponseSchema>> {
    return IpcClient.call('move_items', {
      version_id: versionId,
      moves: JSON.stringify(moves),
      mode,
      operator,
      reason,
    }, {
      validate: zodValidator(MoveItemsResponseSchema, 'move_items'),
    });
  },
};

// ==========================================
// Capacity API (产能池管理)
// ==========================================

export const capacityApi = {
  async getCapacityPools(
    machineCodes: string[],
    dateFrom: string,
    dateTo: string,
    versionId?: string
  ): Promise<Array<z.infer<typeof CapacityPoolSchema>>> {
    return IpcClient.call('get_capacity_pools', {
      machine_codes: JSON.stringify(machineCodes),  // 后端期望 JSON 字符串
      date_from: dateFrom,
      date_to: dateTo,
      version_id: versionId,
    }, {
      validate: zodValidator(z.array(CapacityPoolSchema), 'get_capacity_pools'),
    });
  },

  async updateCapacityPool(
    machineCode: string,
    planDate: string,
    targetCapacityT: number,
    limitCapacityT: number,
    reason: string,
    operator: string = 'system',
    versionId?: string
  ): Promise<void> {
    await IpcClient.call('update_capacity_pool', {
      machine_code: machineCode,
      plan_date: planDate,
      target_capacity_t: targetCapacityT,
      limit_capacity_t: limitCapacityT,
      reason,
      operator,
      version_id: versionId,
    }, {
      validate: zodValidator(EmptyOkResponseSchema, 'update_capacity_pool'),
    });
  },

  async batchUpdateCapacityPools(
    updates: Array<{
      machine_code: string;
      plan_date: string;
      target_capacity_t: number;
      limit_capacity_t: number;
    }>,
    reason: string,
    operator: string = 'system',
    versionId?: string
  ): Promise<z.infer<typeof BatchUpdateCapacityPoolsResponseSchema>> {
    return IpcClient.call('batch_update_capacity_pools', {
      updates: JSON.stringify(updates),
      reason,
      operator,
      version_id: versionId,
    }, {
      validate: zodValidator(BatchUpdateCapacityPoolsResponseSchema, 'batch_update_capacity_pools'),
    });
  },
};

// ==========================================
// Dashboard API
// ==========================================

export const dashboardApi = {
  async listRiskSnapshots(versionId: string): Promise<z.infer<typeof DecisionDaySummaryResponseSchema>> {
    return IpcClient.call('list_risk_snapshots', { version_id: versionId }, {
      validate: zodValidator(DecisionDaySummaryResponseSchema, 'list_risk_snapshots'),
    });
  },

  async getRiskSnapshot(versionId: string, snapshotDate: string): Promise<z.infer<typeof DecisionDaySummaryResponseSchema>> {
    return IpcClient.call('get_risk_snapshot', {
      version_id: versionId,
      snapshot_date: snapshotDate,
    }, {
      validate: zodValidator(DecisionDaySummaryResponseSchema, 'get_risk_snapshot'),
    });
  },

  async getRefreshStatus(versionId: string): Promise<z.infer<typeof DecisionRefreshStatusResponseSchema>> {
    return IpcClient.call('get_refresh_status', {
      version_id: versionId,
    }, {
      validate: zodValidator(DecisionRefreshStatusResponseSchema, 'get_refresh_status'),
    });
  },

  async manualRefreshDecision(versionId: string, operator: string = 'admin'): Promise<z.infer<typeof ManualRefreshDecisionResponseSchema>> {
    return IpcClient.call('manual_refresh_decision', {
      version_id: versionId,
      operator,
    }, {
      validate: zodValidator(ManualRefreshDecisionResponseSchema, 'manual_refresh_decision'),
    });
  },

  async listActionLogs(startTime: string, endTime: string): Promise<Array<z.infer<typeof ActionLogSchema>>> {
    return IpcClient.call('list_action_logs', {
      start_time: startTime,
      end_time: endTime,
    }, {
      validate: zodValidator(z.array(ActionLogSchema), 'list_action_logs'),
    });
  },

  async listActionLogsByMaterial(
    materialId: string,
    startTime: string,
    endTime: string,
    limit?: number
  ): Promise<Array<z.infer<typeof ActionLogSchema>>> {
    return IpcClient.call('list_action_logs_by_material', {
      material_id: materialId,
      start_time: startTime,
      end_time: endTime,
      limit,
    }, {
      validate: zodValidator(z.array(ActionLogSchema), 'list_action_logs_by_material'),
    });
  },

  async listActionLogsByVersion(versionId: string): Promise<Array<z.infer<typeof ActionLogSchema>>> {
    return IpcClient.call('list_action_logs_by_version', {
      version_id: versionId,
    }, {
      validate: zodValidator(z.array(ActionLogSchema), 'list_action_logs_by_version'),
    });
  },

  async getRecentActions(
    limit: number,
    opts?: {
      offset?: number;
      start_time?: string;
      end_time?: string;
    }
  ): Promise<Array<z.infer<typeof ActionLogSchema>>> {
    return IpcClient.call('get_recent_actions', {
      limit,
      offset: opts?.offset,
      start_time: opts?.start_time,
      end_time: opts?.end_time,
    }, {
      validate: zodValidator(z.array(ActionLogSchema), 'get_recent_actions'),
    });
  },
};

// ==========================================
// Config API
// ==========================================

export const configApi = {
  async listConfigs(): Promise<Array<z.infer<typeof ConfigItemSchema>>> {
    return IpcClient.call('list_configs', {}, {
      validate: zodValidator(z.array(ConfigItemSchema), 'list_configs'),
    });
  },

  async getConfig(scopeId: string, key: string): Promise<z.infer<typeof ConfigItemSchema> | null | undefined> {
    return IpcClient.call('get_config', {
      scope_id: scopeId,
      key,
    }, {
      validate: zodValidator(ConfigItemSchema.nullable().optional(), 'get_config'),
    });
  },

  async updateConfig(
    scopeId: string,
    key: string,
    value: string,
    operator: string,
    reason: string
  ): Promise<void> {
    await IpcClient.call('update_config', {
      scope_id: scopeId,
      key,
      value,
      operator,
      reason,
    }, {
      validate: zodValidator(EmptyOkResponseSchema, 'update_config'),
    });
  },

  async batchUpdateConfigs(
    configs: Array<{ scope_id: string; key: string; value: string }>,
    operator: string,
    reason: string
  ): Promise<z.infer<typeof BatchUpdateConfigsResponseSchema>> {
    // 后端期望 configs 为 JSON 字符串
    return IpcClient.call('batch_update_configs', {
      configs: JSON.stringify(configs),
      operator,
      reason,
    }, {
      validate: zodValidator(BatchUpdateConfigsResponseSchema, 'batch_update_configs'),
    });
  },

  async getConfigSnapshot(): Promise<z.infer<typeof ConfigSnapshotSchema>> {
    return IpcClient.call('get_config_snapshot', {}, {
      validate: zodValidator(ConfigSnapshotSchema, 'get_config_snapshot'),
    });
  },

  async restoreFromSnapshot(
    snapshotJson: string,
    operator: string,
    reason: string
  ): Promise<z.infer<typeof RestoreConfigFromSnapshotResponseSchema>> {
    // 注意：后端命令名为 restore_config_from_snapshot
    return IpcClient.call('restore_config_from_snapshot', {
      snapshot_json: snapshotJson,
      operator,
      reason,
    }, {
      validate: zodValidator(RestoreConfigFromSnapshotResponseSchema, 'restore_config_from_snapshot'),
    });
  },

  // ==========================================
  // Custom Strategy (P2)
  // ==========================================

  async saveCustomStrategy(params: {
    strategy: z.infer<typeof CustomStrategyProfileSchema>;
    operator: string;
    reason: string;
  }): Promise<z.infer<typeof SaveCustomStrategyResponseSchema>> {
    return IpcClient.call('save_custom_strategy', {
      strategy_json: JSON.stringify(params.strategy),
      operator: params.operator,
      reason: params.reason,
    }, {
      validate: zodValidator(SaveCustomStrategyResponseSchema, 'save_custom_strategy'),
    });
  },

  async listCustomStrategies(): Promise<Array<z.infer<typeof CustomStrategyProfileSchema>>> {
    return IpcClient.call('list_custom_strategies', {}, {
      validate: zodValidator(z.array(CustomStrategyProfileSchema), 'list_custom_strategies'),
    });
  },
};

// ==========================================
// Path Rule API (宽厚路径规则 v0.6)
// ==========================================

export const pathRuleApi = {
  async getPathRuleConfig(): Promise<z.infer<typeof PathRuleConfigSchema>> {
    return IpcClient.call('get_path_rule_config', {}, {
      validate: zodValidator(PathRuleConfigSchema, 'get_path_rule_config'),
    });
  },

  async updatePathRuleConfig(params: {
    config: z.infer<typeof PathRuleConfigSchema>;
    operator: string;
    reason: string;
  }): Promise<void> {
    await IpcClient.call('update_path_rule_config', {
      config_json: JSON.stringify(params.config),
      operator: params.operator,
      reason: params.reason,
    }, {
      validate: zodValidator(EmptyOkResponseSchema, 'update_path_rule_config'),
    });
  },

  async listPathOverridePending(params: {
    versionId: string;
    machineCode: string;
    planDate: string; // YYYY-MM-DD
  }): Promise<Array<z.infer<typeof PathOverridePendingSchema>>> {
    return IpcClient.call('list_path_override_pending', {
      version_id: params.versionId,
      machine_code: params.machineCode,
      plan_date: params.planDate,
    }, {
      validate: zodValidator(z.array(PathOverridePendingSchema), 'list_path_override_pending'),
    });
  },

  async listPathOverridePendingSummary(params: {
    versionId: string;
    planDateFrom: string; // YYYY-MM-DD
    planDateTo: string; // YYYY-MM-DD
    machineCodes?: string[];
  }): Promise<Array<z.infer<typeof PathOverridePendingSummarySchema>>> {
    return IpcClient.call('list_path_override_pending_summary', {
      version_id: params.versionId,
      plan_date_from: params.planDateFrom,
      plan_date_to: params.planDateTo,
      machine_codes: params.machineCodes ? JSON.stringify(params.machineCodes) : undefined,
    }, {
      validate: zodValidator(z.array(PathOverridePendingSummarySchema), 'list_path_override_pending_summary'),
    });
  },

  async confirmPathOverride(params: {
    versionId: string;
    materialId: string;
    confirmedBy: string;
    reason: string;
  }): Promise<void> {
    await IpcClient.call('confirm_path_override', {
      version_id: params.versionId,
      material_id: params.materialId,
      confirmed_by: params.confirmedBy,
      reason: params.reason,
    }, {
      validate: zodValidator(EmptyOkResponseSchema, 'confirm_path_override'),
    });
  },

  async batchConfirmPathOverride(params: {
    versionId: string;
    materialIds: string[];
    confirmedBy: string;
    reason: string;
  }): Promise<z.infer<typeof BatchConfirmPathOverrideResultSchema>> {
    return IpcClient.call('batch_confirm_path_override', {
      version_id: params.versionId,
      material_ids: JSON.stringify(params.materialIds),
      confirmed_by: params.confirmedBy,
      reason: params.reason,
    }, {
      validate: zodValidator(BatchConfirmPathOverrideResultSchema, 'batch_confirm_path_override'),
    });
  },

  async batchConfirmPathOverrideByRange(params: {
    versionId: string;
    planDateFrom: string; // YYYY-MM-DD
    planDateTo: string; // YYYY-MM-DD
    machineCodes?: string[];
    confirmedBy: string;
    reason: string;
  }): Promise<z.infer<typeof BatchConfirmPathOverrideResultSchema>> {
    return IpcClient.call('batch_confirm_path_override_by_range', {
      version_id: params.versionId,
      plan_date_from: params.planDateFrom,
      plan_date_to: params.planDateTo,
      machine_codes: params.machineCodes ? JSON.stringify(params.machineCodes) : undefined,
      confirmed_by: params.confirmedBy,
      reason: params.reason,
    }, {
      validate: zodValidator(BatchConfirmPathOverrideResultSchema, 'batch_confirm_path_override_by_range'),
    });
  },

  async getRollCycleAnchor(params: {
    versionId: string;
    machineCode: string;
  }): Promise<z.infer<typeof RollCycleAnchorSchema> | null> {
    return IpcClient.call('get_roll_cycle_anchor', {
      version_id: params.versionId,
      machine_code: params.machineCode,
    }, {
      validate: zodValidator(RollCycleAnchorSchema.nullable(), 'get_roll_cycle_anchor'),
    });
  },

  async resetRollCycle(params: {
    versionId: string;
    machineCode: string;
    actor: string;
    reason: string;
  }): Promise<void> {
    await IpcClient.call('reset_roll_cycle', {
      version_id: params.versionId,
      machine_code: params.machineCode,
      actor: params.actor,
      reason: params.reason,
    }, {
      validate: zodValidator(EmptyOkResponseSchema, 'reset_roll_cycle'),
    });
  },
};

// ==========================================
// Roll Campaign API (换辊管理)
// ==========================================

export const rollApi = {
  async listRollCampaigns(versionId: string): Promise<Array<z.infer<typeof RollerCampaignInfoSchema>>> {
    return IpcClient.call('list_roll_campaigns', {
      version_id: versionId,
    }, {
      validate: zodValidator(z.array(RollerCampaignInfoSchema), 'list_roll_campaigns'),
    });
  },

  async listRollCampaignPlans(versionId: string): Promise<Array<z.infer<typeof RollCampaignPlanInfoSchema>>> {
    return IpcClient.call('list_roll_campaign_plans', {
      version_id: versionId,
    }, {
      validate: zodValidator(z.array(RollCampaignPlanInfoSchema), 'list_roll_campaign_plans'),
    });
  },

  async upsertRollCampaignPlan(params: {
    versionId: string;
    machineCode: string;
    initialStartAt: string; // YYYY-MM-DD HH:MM[:SS] or ISO
    nextChangeAt?: string;
    downtimeMinutes?: number;
    operator: string;
    reason: string;
  }): Promise<void> {
    await IpcClient.call('upsert_roll_campaign_plan', {
      version_id: params.versionId,
      machine_code: params.machineCode,
      initial_start_at: params.initialStartAt,
      next_change_at: params.nextChangeAt,
      downtime_minutes: params.downtimeMinutes,
      operator: params.operator,
      reason: params.reason,
    }, {
      validate: zodValidator(EmptyOkResponseSchema, 'upsert_roll_campaign_plan'),
    });
  },

  async getActiveRollCampaign(
    versionId: string,
    machineCode: string
  ): Promise<z.infer<typeof RollerCampaignInfoSchema> | null> {
    return IpcClient.call('get_active_roll_campaign', {
      version_id: versionId,
      machine_code: machineCode,
    }, {
      validate: zodValidator(RollerCampaignInfoSchema.nullable(), 'get_active_roll_campaign'),
    });
  },

  async listNeedsRollChange(versionId: string): Promise<Array<z.infer<typeof RollerCampaignInfoSchema>>> {
    return IpcClient.call('list_needs_roll_change', {
      version_id: versionId,
    }, {
      validate: zodValidator(z.array(RollerCampaignInfoSchema), 'list_needs_roll_change'),
    });
  },

  /**
   * 创建换辊计划
   *
   * 注意: TypeScript 函数签名按照 TS 最佳实践（必需参数在前，可选参数在后）
   * IPC 调用参数顺序已与后端 Rust 签名对齐:
   * version_id, machine_code, campaign_no, start_date, suggest_threshold_t, hard_limit_t, operator, reason
   */
  async createRollCampaign(
    versionId: string,
    machineCode: string,
    campaignNo: number,
    startDate: string,
    operator: string,
    reason: string,
    suggestThresholdT?: number,
    hardLimitT?: number
  ): Promise<void> {
    await IpcClient.call('create_roll_campaign', {
      version_id: versionId,
      machine_code: machineCode,
      campaign_no: campaignNo,
      start_date: startDate,
      suggest_threshold_t: suggestThresholdT,
      hard_limit_t: hardLimitT,
      operator,
      reason,
    }, {
      validate: zodValidator(EmptyOkResponseSchema, 'create_roll_campaign'),
    });
  },

  async closeRollCampaign(
    versionId: string,
    machineCode: string,
    campaignNo: number,
    endDate: string,
    operator: string,
    reason: string
  ): Promise<void> {
    await IpcClient.call('close_roll_campaign', {
      version_id: versionId,
      machine_code: machineCode,
      campaign_no: campaignNo,
      end_date: endDate,
      operator,
      reason,
    }, {
      validate: zodValidator(EmptyOkResponseSchema, 'close_roll_campaign'),
    });
  },
};

// ==========================================
// Plan Rhythm API (每日生产节奏管理)
// ==========================================

export const rhythmApi = {
  async listRhythmPresets(
    dimension: string = 'PRODUCT_CATEGORY',
    activeOnly: boolean = true
  ): Promise<z.infer<typeof PlanRhythmPresetsResponseSchema>> {
    return IpcClient.call('list_rhythm_presets', { dimension, active_only: activeOnly }, {
      validate: zodValidator(PlanRhythmPresetsResponseSchema, 'list_rhythm_presets'),
    });
  },

  async upsertRhythmPreset(params: {
    presetId?: string;
    presetName: string;
    dimension?: string;
    targetJson: string;
    isActive?: boolean;
    operator: string;
    reason: string;
  }): Promise<z.infer<typeof PlanRhythmPresetSchema>> {
    return IpcClient.call('upsert_rhythm_preset', {
      preset_id: params.presetId,
      preset_name: params.presetName,
      dimension: params.dimension || 'PRODUCT_CATEGORY',
      target_json: params.targetJson,
      is_active: params.isActive,
      operator: params.operator,
      reason: params.reason,
    }, {
      validate: zodValidator(PlanRhythmPresetSchema, 'upsert_rhythm_preset'),
    });
  },

  async setRhythmPresetActive(
    presetId: string,
    isActive: boolean,
    operator: string,
    reason: string
  ): Promise<z.infer<typeof PlanRhythmPresetSchema>> {
    return IpcClient.call('set_rhythm_preset_active', {
      preset_id: presetId,
      is_active: isActive,
      operator,
      reason,
    }, {
      validate: zodValidator(PlanRhythmPresetSchema, 'set_rhythm_preset_active'),
    });
  },

  async listRhythmTargets(params: {
    versionId: string;
    dimension?: string;
    machineCodes?: string[];
    dateFrom?: string;
    dateTo?: string;
  }): Promise<z.infer<typeof PlanRhythmTargetsResponseSchema>> {
    return IpcClient.call('list_rhythm_targets', {
      version_id: params.versionId,
      dimension: params.dimension || 'PRODUCT_CATEGORY',
      machine_codes: params.machineCodes ? JSON.stringify(params.machineCodes) : undefined,
      date_from: params.dateFrom,
      date_to: params.dateTo,
    }, {
      validate: zodValidator(PlanRhythmTargetsResponseSchema, 'list_rhythm_targets'),
    });
  },

  async upsertRhythmTarget(params: {
    versionId: string;
    machineCode: string;
    planDate: string; // YYYY-MM-DD
    dimension?: string;
    targetJson: string; // JSON object {category: ratio}
    presetId?: string;
    operator: string;
    reason: string;
  }): Promise<void> {
    await IpcClient.call('upsert_rhythm_target', {
      version_id: params.versionId,
      machine_code: params.machineCode,
      plan_date: params.planDate,
      dimension: params.dimension || 'PRODUCT_CATEGORY',
      target_json: params.targetJson,
      preset_id: params.presetId,
      operator: params.operator,
      reason: params.reason,
    }, {
      validate: zodValidator(EmptyOkResponseSchema, 'upsert_rhythm_target'),
    });
  },

  async applyRhythmPreset(params: {
    versionId: string;
    dimension?: string;
    presetId: string;
    machineCodes: string[];
    dateFrom: string; // YYYY-MM-DD
    dateTo: string; // YYYY-MM-DD
    overwrite?: boolean;
    operator: string;
    reason: string;
  }): Promise<z.infer<typeof ApplyRhythmPresetResponseSchema>> {
    return IpcClient.call('apply_rhythm_preset', {
      version_id: params.versionId,
      dimension: params.dimension || 'PRODUCT_CATEGORY',
      preset_id: params.presetId,
      machine_codes: JSON.stringify(params.machineCodes),
      date_from: params.dateFrom,
      date_to: params.dateTo,
      overwrite: params.overwrite,
      operator: params.operator,
      reason: params.reason,
    }, {
      validate: zodValidator(ApplyRhythmPresetResponseSchema, 'apply_rhythm_preset'),
    });
  },

  async getDailyRhythmProfile(versionId: string, machineCode: string, planDate: string): Promise<z.infer<typeof DailyRhythmProfileSchema>> {
    return IpcClient.call('get_daily_rhythm_profile', {
      version_id: versionId,
      machine_code: machineCode,
      plan_date: planDate,
    }, {
      validate: zodValidator(DailyRhythmProfileSchema, 'get_daily_rhythm_profile'),
    });
  },
};

// ==========================================
// Decision Service (D1-D6)
// ==========================================
// 统一对外出口：推荐在业务/Hook 中通过 `api/tauri.ts` 引用，避免绕过统一封装
export * from '../services/decision-service';

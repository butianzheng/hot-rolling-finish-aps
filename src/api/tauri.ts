// ==========================================
// Tauri API 调用封装层
// ==========================================
// 职责: 封装所有Tauri命令调用，提供类型安全的API
// 红线: 不包含业务逻辑，只负责API调用和错误处理
// 重要: Rust 后端配置了 #[tauri::command(rename_all = "snake_case")]
//       前端必须使用 snake_case 参数名与 Rust 后端对齐
// ==========================================

import { IpcClient } from './ipcClient';

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
  ): Promise<any> {
    // 使用 snake_case 参数名（后端配置 rename_all = "snake_case"）
    return IpcClient.call('import_materials', {
      file_path: filePath,
      source_batch_id: sourceBatchId,
      mapping_profile_id: mappingProfileId,
    });
  },

  async listImportConflicts(
    status?: string,
    limit: number = 50,
    offset: number = 0,
    batchId?: string
  ): Promise<any> {
    return IpcClient.call('list_import_conflicts', { status, limit, offset, batch_id: batchId });
  },

  async resolveImportConflict(
    conflictId: string,
    action: 'KEEP_EXISTING' | 'OVERWRITE' | 'MERGE',
    note?: string,
    operator: string = 'system'
  ): Promise<any> {
    return IpcClient.call('resolve_import_conflict', {
      conflict_id: conflictId,
      action,
      note,
      operator,
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
  }): Promise<any> {
    return IpcClient.call('list_materials', {
      machine_code: params.machine_code,
      steel_grade: params.steel_grade,
      limit: params.limit,
      offset: params.offset,
    });
  },

  async getMaterialDetail(materialId: string): Promise<any> {
    return IpcClient.call('get_material_detail', { material_id: materialId });
  },

  async listReadyMaterials(machineCode?: string): Promise<any> {
    return IpcClient.call('list_ready_materials', { machine_code: machineCode });
  },

  async batchLockMaterials(
    materialIds: string[],
    lockFlag: boolean,
    operator: string,
    reason: string,
    mode?: 'Strict' | 'AutoFix'
  ): Promise<any> {
    return IpcClient.call('batch_lock_materials', {
      material_ids: materialIds,
      lock_flag: lockFlag,
      operator,
      reason,
      mode,
    });
  },

  async batchForceRelease(
    materialIds: string[],
    operator: string,
    reason: string,
    mode?: 'Strict' | 'AutoFix'
  ): Promise<any> {
    return IpcClient.call('batch_force_release', {
      material_ids: materialIds,
      operator,
      reason,
      mode,
    });
  },

  async batchSetUrgent(
    materialIds: string[],
    manualUrgentFlag: boolean,
    operator: string,
    reason: string
  ): Promise<any> {
    return IpcClient.call('batch_set_urgent', {
      material_ids: materialIds,
      manual_urgent_flag: manualUrgentFlag,
      operator,
      reason,
    });
  },

  async listMaterialsByUrgentLevel(
    urgentLevel: string,
    machineCode?: string
  ): Promise<any> {
    return IpcClient.call('list_materials_by_urgent_level', {
      urgent_level: urgentLevel,
      machine_code: machineCode,
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

  async listPlans(): Promise<any> {
    return IpcClient.call('list_plans');
  },

  async getPlanDetail(planId: string): Promise<any> {
    return IpcClient.call('get_plan_detail', { plan_id: planId });
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

  async listVersions(planId: string): Promise<any> {
    return IpcClient.call('list_versions', { plan_id: planId });
  },

  async activateVersion(versionId: string, operator: string): Promise<void> {
    return IpcClient.call('activate_version', {
      version_id: versionId,
      operator,
    });
  },

  async simulateRecalc(
    versionId: string,
    baseDate: string,
    frozenDate?: string,
    operator: string = 'admin'
  ): Promise<any> {
    return IpcClient.call('simulate_recalc', {
      version_id: versionId,
      base_date: baseDate,
      frozen_date: frozenDate,
      operator,
    });
  },

  async recalcFull(
    versionId: string,
    baseDate: string,
    frozenDate?: string,
    operator: string = 'admin'
  ): Promise<any> {
    return IpcClient.call('recalc_full', {
      version_id: versionId,
      base_date: baseDate,
      frozen_date: frozenDate,
      operator,
    });
  },

  async listPlanItems(versionId: string): Promise<any> {
    return IpcClient.call('list_plan_items', { version_id: versionId });
  },

  async listItemsByDate(versionId: string, planDate: string): Promise<any> {
    return IpcClient.call('list_items_by_date', {
      version_id: versionId,
      plan_date: planDate,
    });
  },

  async compareVersions(versionIdA: string, versionIdB: string): Promise<any> {
    return IpcClient.call('compare_versions', {
      version_id_a: versionIdA,
      version_id_b: versionIdB,
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
    mode: 'AUTO_FIX' | 'STRICT' = 'AUTO_FIX'
  ): Promise<any> {
    return IpcClient.call('move_items', {
      version_id: versionId,
      moves: JSON.stringify(moves),
      mode,
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
  ): Promise<any> {
    return IpcClient.call('get_capacity_pools', {
      machine_codes: JSON.stringify(machineCodes),  // 后端期望 JSON 字符串
      date_from: dateFrom,
      date_to: dateTo,
      version_id: versionId,
    });
  },

  async updateCapacityPool(
    machineCode: string,
    planDate: string,
    targetCapacityT: number,
    limitCapacityT: number,
    reason: string,
    operator: string = 'system'
  ): Promise<any> {
    return IpcClient.call('update_capacity_pool', {
      machine_code: machineCode,
      plan_date: planDate,
      target_capacity_t: targetCapacityT,
      limit_capacity_t: limitCapacityT,
      reason,
      operator,
    });
  },
};

// ==========================================
// Dashboard API
// ==========================================

export const dashboardApi = {
  async listRiskSnapshots(versionId: string): Promise<any> {
    return IpcClient.call('list_risk_snapshots', { version_id: versionId });
  },

  async getRiskSnapshot(versionId: string, snapshotDate: string): Promise<any> {
    return IpcClient.call('get_risk_snapshot', {
      version_id: versionId,
      snapshot_date: snapshotDate,
    });
  },

  async getMostRiskyDate(versionId: string): Promise<any> {
    return IpcClient.call('get_most_risky_date', { version_id: versionId });
  },

  async getUnsatisfiedUrgentMaterials(versionId?: string): Promise<any> {
    return IpcClient.call('get_unsatisfied_urgent_materials', {
      version_id: versionId,
    });
  },

  async getColdStockMaterials(versionId: string, thresholdDays?: number): Promise<any> {
    return IpcClient.call('get_cold_stock_materials', {
      version_id: versionId,
      threshold_days: thresholdDays,
    });
  },

  async getMostCongestedMachine(versionId: string): Promise<any> {
    return IpcClient.call('get_most_congested_machine', {
      version_id: versionId,
    });
  },

  async listActionLogs(startTime: string, endTime: string): Promise<any> {
    return IpcClient.call('list_action_logs', {
      start_time: startTime,
      end_time: endTime,
    });
  },

  async listActionLogsByVersion(versionId: string): Promise<any> {
    return IpcClient.call('list_action_logs_by_version', {
      version_id: versionId,
    });
  },

  async getRecentActions(limit: number): Promise<any> {
    return IpcClient.call('get_recent_actions', { limit });
  },
};

// ==========================================
// Config API
// ==========================================

export const configApi = {
  async listConfigs(): Promise<any> {
    return IpcClient.call('list_configs');
  },

  async getConfig(scopeId: string, key: string): Promise<any> {
    return IpcClient.call('get_config', {
      scope_id: scopeId,
      key,
    });
  },

  async updateConfig(
    scopeId: string,
    key: string,
    value: string,
    operator: string,
    reason: string
  ): Promise<any> {
    return IpcClient.call('update_config', {
      scope_id: scopeId,
      key,
      value,
      operator,
      reason,
    });
  },

  async batchUpdateConfigs(
    configs: Array<{ scope_id: string; key: string; value: string }>,
    operator: string,
    reason: string
  ): Promise<any> {
    // 后端期望 configs 为 JSON 字符串
    return IpcClient.call('batch_update_configs', {
      configs: JSON.stringify(configs),
      operator,
      reason,
    });
  },

  async getConfigSnapshot(): Promise<any> {
    return IpcClient.call('get_config_snapshot');
  },

  async restoreFromSnapshot(
    snapshotJson: string,
    operator: string,
    reason: string
  ): Promise<any> {
    // 注意：后端命令名为 restore_config_from_snapshot
    return IpcClient.call('restore_config_from_snapshot', {
      snapshot_json: snapshotJson,
      operator,
      reason,
    });
  },
};

// ==========================================
// Decision API (D1-D6)
// ==========================================
// 注意: 推荐使用 services/decision-service.ts 中的封装
//       decision-service.ts 提供了 Zod 验证和更完整的类型支持
//       此处的 decisionApi 提供低层 API 访问，供特殊场景使用

export const decisionApi = {
  // D1: 哪天最危险
  async getDecisionDaySummary(params: {
    versionId: string;
    dateFrom: string;
    dateTo: string;
    riskLevelFilter?: string[];
    limit?: number;
    sortBy?: string;
  }): Promise<any> {
    return IpcClient.call('get_decision_day_summary', {
      version_id: params.versionId,
      date_from: params.dateFrom,
      date_to: params.dateTo,
      risk_level_filter: params.riskLevelFilter
        ? JSON.stringify(params.riskLevelFilter)
        : undefined,
      limit: params.limit,
      sort_by: params.sortBy,
    });
  },

  // D2: 哪些紧急单无法完成
  async listOrderFailureSet(params: {
    versionId: string;
    failTypeFilter?: string[];
    urgencyLevelFilter?: string[];
    machineCodes?: string[];
    dueDateFrom?: string;
    dueDateTo?: string;
    completionRateThreshold?: number;
    limit?: number;
    offset?: number;
  }): Promise<any> {
    return IpcClient.call('list_order_failure_set', {
      version_id: params.versionId,
      fail_type_filter: params.failTypeFilter
        ? JSON.stringify(params.failTypeFilter)
        : undefined,
      urgency_level_filter: params.urgencyLevelFilter
        ? JSON.stringify(params.urgencyLevelFilter)
        : undefined,
      machine_codes: params.machineCodes
        ? JSON.stringify(params.machineCodes)
        : undefined,
      due_date_from: params.dueDateFrom,
      due_date_to: params.dueDateTo,
      completion_rate_threshold: params.completionRateThreshold,
      limit: params.limit,
      offset: params.offset,
    });
  },

  // D3: 哪些冷料压库
  async getColdStockProfile(params: {
    versionId: string;
    machineCodes?: string[];
    pressureLevelFilter?: string[];
    ageBinFilter?: string[];
    limit?: number;
  }): Promise<any> {
    return IpcClient.call('get_cold_stock_profile', {
      version_id: params.versionId,
      machine_codes: params.machineCodes
        ? JSON.stringify(params.machineCodes)
        : undefined,
      pressure_level_filter: params.pressureLevelFilter
        ? JSON.stringify(params.pressureLevelFilter)
        : undefined,
      age_bin_filter: params.ageBinFilter
        ? JSON.stringify(params.ageBinFilter)
        : undefined,
      limit: params.limit,
    });
  },

  // D4: 哪个机组最堵
  async getMachineBottleneckProfile(params: {
    versionId: string;
    dateFrom: string;
    dateTo: string;
    machineCodes?: string[];
    bottleneckLevelFilter?: string[];
    bottleneckTypeFilter?: string[];
    limit?: number;
  }): Promise<any> {
    return IpcClient.call('get_machine_bottleneck_profile', {
      version_id: params.versionId,
      date_from: params.dateFrom,
      date_to: params.dateTo,
      machine_codes: params.machineCodes
        ? JSON.stringify(params.machineCodes)
        : undefined,
      bottleneck_level_filter: params.bottleneckLevelFilter
        ? JSON.stringify(params.bottleneckLevelFilter)
        : undefined,
      bottleneck_type_filter: params.bottleneckTypeFilter
        ? JSON.stringify(params.bottleneckTypeFilter)
        : undefined,
      limit: params.limit,
    });
  },

  // D5: 换辊是否异常
  async getRollCampaignAlert(params: {
    versionId: string;
    machineCodes?: string[];
    alertLevelFilter?: string[];
    alertTypeFilter?: string[];
    dateFrom?: string;
    dateTo?: string;
    limit?: number;
  }): Promise<any> {
    return IpcClient.call('get_roll_campaign_alert', {
      version_id: params.versionId,
      machine_codes: params.machineCodes
        ? JSON.stringify(params.machineCodes)
        : undefined,
      alert_level_filter: params.alertLevelFilter
        ? JSON.stringify(params.alertLevelFilter)
        : undefined,
      alert_type_filter: params.alertTypeFilter
        ? JSON.stringify(params.alertTypeFilter)
        : undefined,
      date_from: params.dateFrom,
      date_to: params.dateTo,
      limit: params.limit,
    });
  },

  // D6: 是否存在产能优化空间
  async getCapacityOpportunity(params: {
    versionId: string;
    dateFrom?: string;
    dateTo?: string;
    machineCodes?: string[];
    opportunityTypeFilter?: string[];
    minOpportunityT?: number;
    limit?: number;
  }): Promise<any> {
    return IpcClient.call('get_capacity_opportunity', {
      version_id: params.versionId,
      date_from: params.dateFrom,
      date_to: params.dateTo,
      machine_codes: params.machineCodes
        ? JSON.stringify(params.machineCodes)
        : undefined,
      opportunity_type_filter: params.opportunityTypeFilter
        ? JSON.stringify(params.opportunityTypeFilter)
        : undefined,
      min_opportunity_t: params.minOpportunityT,
      limit: params.limit,
    });
  },
};

// ==========================================
// Roll Campaign API (换辊管理)
// ==========================================

export const rollApi = {
  async listRollCampaigns(versionId: string): Promise<any> {
    return IpcClient.call('list_roll_campaigns', {
      version_id: versionId,
    });
  },

  async getActiveRollCampaign(
    versionId: string,
    machineCode: string
  ): Promise<any> {
    return IpcClient.call('get_active_roll_campaign', {
      version_id: versionId,
      machine_code: machineCode,
    });
  },

  async listNeedsRollChange(versionId: string): Promise<any> {
    return IpcClient.call('list_needs_roll_change', {
      version_id: versionId,
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
  ): Promise<any> {
    return IpcClient.call('create_roll_campaign', {
      version_id: versionId,
      machine_code: machineCode,
      campaign_no: campaignNo,
      start_date: startDate,
      suggest_threshold_t: suggestThresholdT,
      hard_limit_t: hardLimitT,
      operator,
      reason,
    });
  },

  async closeRollCampaign(
    versionId: string,
    machineCode: string,
    campaignNo: number,
    endDate: string,
    operator: string,
    reason: string
  ): Promise<any> {
    return IpcClient.call('close_roll_campaign', {
      version_id: versionId,
      machine_code: machineCode,
      campaign_no: campaignNo,
      end_date: endDate,
      operator,
      reason,
    });
  },
};

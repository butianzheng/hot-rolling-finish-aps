import { IpcClient } from '../ipcClient';
import {
  z,
  zodValidator,
  PlanSchema,
  PlanVersionSchema,
  StrategyPresetSchema,
  GenerateStrategyDraftsResponseSchema,
  ApplyStrategyDraftResponseSchema,
  GetStrategyDraftDetailResponseSchema,
  ListStrategyDraftsResponseSchema,
  CleanupStrategyDraftsResponseSchema,
  PlanItemSchema,
  PlanItemDateBoundsResponseSchema,
  VersionComparisonResultSchema,
  VersionComparisonKpiResultSchema,
  MoveItemsResponseSchema,
  RollbackVersionResponseSchema,
  RecalcResponseSchema,
} from '../ipcSchemas';

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
    return IpcClient.call(
      'activate_version',
      {
        version_id: versionId,
        operator,
      },
      {
        timeout: 300000,
      }
    );
  },

  async rollbackVersion(
    planId: string,
    targetVersionId: string,
    operator: string,
    reason: string
  ): Promise<z.infer<typeof RollbackVersionResponseSchema>> {
    return IpcClient.call(
      'rollback_version',
      {
        plan_id: planId,
        target_version_id: targetVersionId,
        operator,
        reason,
      },
      {
        validate: zodValidator(RollbackVersionResponseSchema, 'rollback_version'),
      }
    );
  },

  async simulateRecalc(
    versionId: string,
    baseDate: string,
    frozenDate?: string,
    operator: string = 'admin',
    strategy?: string,
    windowDaysOverride?: number
  ): Promise<z.infer<typeof RecalcResponseSchema>> {
    return IpcClient.call(
      'simulate_recalc',
      {
        version_id: versionId,
        base_date: baseDate,
        frozen_date: frozenDate,
        operator,
        strategy,
        window_days_override: windowDaysOverride,
      },
      {
        timeout: 300000,
        validate: zodValidator(RecalcResponseSchema, 'simulate_recalc'),
      }
    );
  },

  async recalcFull(
    versionId: string,
    baseDate: string,
    frozenDate?: string,
    operator: string = 'admin',
    strategy?: string,
    windowDaysOverride?: number
  ): Promise<z.infer<typeof RecalcResponseSchema>> {
    return IpcClient.call(
      'recalc_full',
      {
        version_id: versionId,
        base_date: baseDate,
        frozen_date: frozenDate,
        operator,
        strategy,
        window_days_override: windowDaysOverride,
      },
      {
        timeout: 300000,
        validate: zodValidator(RecalcResponseSchema, 'recalc_full'),
      }
    );
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
    return IpcClient.call(
      'generate_strategy_drafts',
      {
        base_version_id: params.base_version_id,
        plan_date_from: params.plan_date_from,
        plan_date_to: params.plan_date_to,
        strategies: params.strategies,
        operator: params.operator,
      },
      {
        timeout: 300000,
        validate: zodValidator(GenerateStrategyDraftsResponseSchema, 'generate_strategy_drafts'),
      }
    );
  },

  async applyStrategyDraft(
    draftId: string,
    operator: string
  ): Promise<z.infer<typeof ApplyStrategyDraftResponseSchema>> {
    return IpcClient.call(
      'apply_strategy_draft',
      {
        draft_id: draftId,
        operator,
      },
      {
        timeout: 300000,
        validate: zodValidator(ApplyStrategyDraftResponseSchema, 'apply_strategy_draft'),
      }
    );
  },

  async getStrategyDraftDetail(draftId: string): Promise<z.infer<typeof GetStrategyDraftDetailResponseSchema>> {
    return IpcClient.call(
      'get_strategy_draft_detail',
      {
        draft_id: draftId,
      },
      {
        validate: zodValidator(GetStrategyDraftDetailResponseSchema, 'get_strategy_draft_detail'),
      }
    );
  },

  async listStrategyDrafts(params: {
    base_version_id: string;
    plan_date_from: string;
    plan_date_to: string;
    status_filter?: string;
    limit?: number;
  }): Promise<z.infer<typeof ListStrategyDraftsResponseSchema>> {
    return IpcClient.call(
      'list_strategy_drafts',
      {
        base_version_id: params.base_version_id,
        plan_date_from: params.plan_date_from,
        plan_date_to: params.plan_date_to,
        status_filter: params.status_filter,
        limit: params.limit,
      },
      {
        validate: zodValidator(ListStrategyDraftsResponseSchema, 'list_strategy_drafts'),
      }
    );
  },

  async cleanupExpiredStrategyDrafts(keepDays: number): Promise<z.infer<typeof CleanupStrategyDraftsResponseSchema>> {
    return IpcClient.call(
      'cleanup_expired_strategy_drafts',
      {
        keep_days: keepDays,
      },
      {
        validate: zodValidator(CleanupStrategyDraftsResponseSchema, 'cleanup_expired_strategy_drafts'),
      }
    );
  },

  async getPlanItemDateBounds(
    versionId: string,
    machineCode?: string
  ): Promise<z.infer<typeof PlanItemDateBoundsResponseSchema>> {
    return IpcClient.call(
      'get_plan_item_date_bounds',
      {
        version_id: versionId,
        machine_code: machineCode,
      },
      {
        validate: zodValidator(PlanItemDateBoundsResponseSchema, 'get_plan_item_date_bounds'),
        timeout: 60000,
      }
    );
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
    return IpcClient.call(
      'list_plan_items',
      {
        version_id: versionId,
        plan_date_from: opts?.plan_date_from,
        plan_date_to: opts?.plan_date_to,
        machine_code: opts?.machine_code,
        limit: opts?.limit,
        offset: opts?.offset,
      },
      {
        validate: zodValidator(z.array(PlanItemSchema), 'list_plan_items'),
        // Plan items can be large (10k~50k+) depending on window/versions; avoid false timeouts.
        timeout: 120000,
      }
    );
  },

  async listItemsByDate(versionId: string, planDate: string): Promise<Array<z.infer<typeof PlanItemSchema>>> {
    return IpcClient.call(
      'list_items_by_date',
      {
        version_id: versionId,
        plan_date: planDate,
      },
      {
        validate: zodValidator(z.array(PlanItemSchema), 'list_items_by_date'),
        timeout: 60000,
      }
    );
  },

  async compareVersions(versionIdA: string, versionIdB: string): Promise<z.infer<typeof VersionComparisonResultSchema>> {
    return IpcClient.call(
      'compare_versions',
      {
        version_id_a: versionIdA,
        version_id_b: versionIdB,
      },
      {
        validate: zodValidator(VersionComparisonResultSchema, 'compare_versions'),
      }
    );
  },

  async compareVersionsKpi(
    versionIdA: string,
    versionIdB: string
  ): Promise<z.infer<typeof VersionComparisonKpiResultSchema>> {
    return IpcClient.call(
      'compare_versions_kpi',
      {
        version_id_a: versionIdA,
        version_id_b: versionIdB,
      },
      {
        validate: zodValidator(VersionComparisonKpiResultSchema, 'compare_versions_kpi'),
      }
    );
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
    return IpcClient.call(
      'move_items',
      {
        version_id: versionId,
        moves: JSON.stringify(moves),
        mode,
        operator,
        reason,
      },
      {
        validate: zodValidator(MoveItemsResponseSchema, 'move_items'),
      }
    );
  },
};

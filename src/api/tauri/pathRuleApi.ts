import { IpcClient } from '../ipcClient';
import {
  z,
  zodValidator,
  EmptyOkResponseSchema,
  PathRuleConfigSchema,
  PathOverridePendingSchema,
  PathOverridePendingSummarySchema,
  RollCycleAnchorSchema,
  BatchConfirmPathOverrideResultSchema,
} from '../ipcSchemas';

// Path Rule API (宽厚路径规则 v0.6)
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
    await IpcClient.call(
      'update_path_rule_config',
      {
        config_json: JSON.stringify(params.config),
        operator: params.operator,
        reason: params.reason,
      },
      {
        validate: zodValidator(EmptyOkResponseSchema, 'update_path_rule_config'),
      }
    );
  },

  async listPathOverridePending(params: {
    versionId: string;
    machineCode: string;
    planDate: string; // YYYY-MM-DD
  }): Promise<Array<z.infer<typeof PathOverridePendingSchema>>> {
    return IpcClient.call(
      'list_path_override_pending',
      {
        version_id: params.versionId,
        machine_code: params.machineCode,
        plan_date: params.planDate,
      },
      {
        validate: zodValidator(z.array(PathOverridePendingSchema), 'list_path_override_pending'),
      }
    );
  },

  async listPathOverridePendingSummary(params: {
    versionId: string;
    planDateFrom: string; // YYYY-MM-DD
    planDateTo: string; // YYYY-MM-DD
    machineCodes?: string[];
  }): Promise<Array<z.infer<typeof PathOverridePendingSummarySchema>>> {
    return IpcClient.call(
      'list_path_override_pending_summary',
      {
        version_id: params.versionId,
        plan_date_from: params.planDateFrom,
        plan_date_to: params.planDateTo,
        machine_codes: params.machineCodes ? JSON.stringify(params.machineCodes) : undefined,
      },
      {
        validate: zodValidator(z.array(PathOverridePendingSummarySchema), 'list_path_override_pending_summary'),
      }
    );
  },

  async confirmPathOverride(params: {
    versionId: string;
    materialId: string;
    confirmedBy: string;
    reason: string;
  }): Promise<void> {
    await IpcClient.call(
      'confirm_path_override',
      {
        version_id: params.versionId,
        material_id: params.materialId,
        confirmed_by: params.confirmedBy,
        reason: params.reason,
      },
      {
        validate: zodValidator(EmptyOkResponseSchema, 'confirm_path_override'),
      }
    );
  },

  async batchConfirmPathOverride(params: {
    versionId: string;
    materialIds: string[];
    confirmedBy: string;
    reason: string;
  }): Promise<z.infer<typeof BatchConfirmPathOverrideResultSchema>> {
    return IpcClient.call(
      'batch_confirm_path_override',
      {
        version_id: params.versionId,
        material_ids: JSON.stringify(params.materialIds),
        confirmed_by: params.confirmedBy,
        reason: params.reason,
      },
      {
        validate: zodValidator(BatchConfirmPathOverrideResultSchema, 'batch_confirm_path_override'),
      }
    );
  },

  async batchConfirmPathOverrideByRange(params: {
    versionId: string;
    planDateFrom: string; // YYYY-MM-DD
    planDateTo: string; // YYYY-MM-DD
    machineCodes?: string[];
    confirmedBy: string;
    reason: string;
  }): Promise<z.infer<typeof BatchConfirmPathOverrideResultSchema>> {
    return IpcClient.call(
      'batch_confirm_path_override_by_range',
      {
        version_id: params.versionId,
        plan_date_from: params.planDateFrom,
        plan_date_to: params.planDateTo,
        machine_codes: params.machineCodes ? JSON.stringify(params.machineCodes) : undefined,
        confirmed_by: params.confirmedBy,
        reason: params.reason,
      },
      {
        validate: zodValidator(BatchConfirmPathOverrideResultSchema, 'batch_confirm_path_override_by_range'),
      }
    );
  },

  async getRollCycleAnchor(params: {
    versionId: string;
    machineCode: string;
  }): Promise<z.infer<typeof RollCycleAnchorSchema> | null> {
    return IpcClient.call(
      'get_roll_cycle_anchor',
      {
        version_id: params.versionId,
        machine_code: params.machineCode,
      },
      {
        validate: zodValidator(RollCycleAnchorSchema.nullable(), 'get_roll_cycle_anchor'),
      }
    );
  },

  async resetRollCycle(params: {
    versionId: string;
    machineCode: string;
    actor: string;
    reason: string;
  }): Promise<void> {
    await IpcClient.call(
      'reset_roll_cycle',
      {
        version_id: params.versionId,
        machine_code: params.machineCode,
        actor: params.actor,
        reason: params.reason,
      },
      {
        validate: zodValidator(EmptyOkResponseSchema, 'reset_roll_cycle'),
      }
    );
  },
};


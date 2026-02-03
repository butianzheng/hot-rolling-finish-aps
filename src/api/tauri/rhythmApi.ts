import { IpcClient } from '../ipcClient';
import {
  z,
  zodValidator,
  EmptyOkResponseSchema,
  PlanRhythmPresetSchema,
  PlanRhythmPresetsResponseSchema,
  PlanRhythmTargetsResponseSchema,
  ApplyRhythmPresetResponseSchema,
  DailyRhythmProfileSchema,
} from '../ipcSchemas';

// Plan Rhythm API (每日生产节奏管理)
export const rhythmApi = {
  async listRhythmPresets(
    dimension: string = 'PRODUCT_CATEGORY',
    activeOnly: boolean = true
  ): Promise<z.infer<typeof PlanRhythmPresetsResponseSchema>> {
    return IpcClient.call(
      'list_rhythm_presets',
      { dimension, active_only: activeOnly },
      {
        validate: zodValidator(PlanRhythmPresetsResponseSchema, 'list_rhythm_presets'),
      }
    );
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
    return IpcClient.call(
      'upsert_rhythm_preset',
      {
        preset_id: params.presetId,
        preset_name: params.presetName,
        dimension: params.dimension || 'PRODUCT_CATEGORY',
        target_json: params.targetJson,
        is_active: params.isActive,
        operator: params.operator,
        reason: params.reason,
      },
      {
        validate: zodValidator(PlanRhythmPresetSchema, 'upsert_rhythm_preset'),
      }
    );
  },

  async setRhythmPresetActive(
    presetId: string,
    isActive: boolean,
    operator: string,
    reason: string
  ): Promise<z.infer<typeof PlanRhythmPresetSchema>> {
    return IpcClient.call(
      'set_rhythm_preset_active',
      {
        preset_id: presetId,
        is_active: isActive,
        operator,
        reason,
      },
      {
        validate: zodValidator(PlanRhythmPresetSchema, 'set_rhythm_preset_active'),
      }
    );
  },

  async listRhythmTargets(params: {
    versionId: string;
    dimension?: string;
    machineCodes?: string[];
    dateFrom?: string;
    dateTo?: string;
  }): Promise<z.infer<typeof PlanRhythmTargetsResponseSchema>> {
    return IpcClient.call(
      'list_rhythm_targets',
      {
        version_id: params.versionId,
        dimension: params.dimension || 'PRODUCT_CATEGORY',
        machine_codes: params.machineCodes ? JSON.stringify(params.machineCodes) : undefined,
        date_from: params.dateFrom,
        date_to: params.dateTo,
      },
      {
        validate: zodValidator(PlanRhythmTargetsResponseSchema, 'list_rhythm_targets'),
      }
    );
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
    await IpcClient.call(
      'upsert_rhythm_target',
      {
        version_id: params.versionId,
        machine_code: params.machineCode,
        plan_date: params.planDate,
        dimension: params.dimension || 'PRODUCT_CATEGORY',
        target_json: params.targetJson,
        preset_id: params.presetId,
        operator: params.operator,
        reason: params.reason,
      },
      {
        validate: zodValidator(EmptyOkResponseSchema, 'upsert_rhythm_target'),
      }
    );
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
    return IpcClient.call(
      'apply_rhythm_preset',
      {
        version_id: params.versionId,
        dimension: params.dimension || 'PRODUCT_CATEGORY',
        preset_id: params.presetId,
        machine_codes: JSON.stringify(params.machineCodes),
        date_from: params.dateFrom,
        date_to: params.dateTo,
        overwrite: params.overwrite,
        operator: params.operator,
        reason: params.reason,
      },
      {
        validate: zodValidator(ApplyRhythmPresetResponseSchema, 'apply_rhythm_preset'),
      }
    );
  },

  async getDailyRhythmProfile(
    versionId: string,
    machineCode: string,
    planDate: string
  ): Promise<z.infer<typeof DailyRhythmProfileSchema>> {
    return IpcClient.call(
      'get_daily_rhythm_profile',
      {
        version_id: versionId,
        machine_code: machineCode,
        plan_date: planDate,
      },
      {
        validate: zodValidator(DailyRhythmProfileSchema, 'get_daily_rhythm_profile'),
      }
    );
  },
};


import { IpcClient } from '../ipcClient';
import {
  z,
  zodValidator,
  ImpactSummarySchema,
  MaterialWithStateSchema,
  MaterialDetailResponseSchema,
  MaterialPoolSummaryResponseSchema,
} from '../ipcSchemas';

export const materialApi = {
  async listMaterials(params: {
    // 仅支持 snake_case 风格（与 Rust 后端对齐）
    machine_code?: string;
    steel_grade?: string;
    sched_state?: string;
    urgent_level?: string;
    lock_status?: string;
    query_text?: string;
    limit: number;
    offset: number;
  }): Promise<Array<z.infer<typeof MaterialWithStateSchema>>> {
    return IpcClient.call(
      'list_materials',
      {
        machine_code: params.machine_code,
        steel_grade: params.steel_grade,
        sched_state: params.sched_state,
        urgent_level: params.urgent_level,
        lock_status: params.lock_status,
        query_text: params.query_text,
        limit: params.limit,
        offset: params.offset,
      },
      {
        validate: zodValidator(z.array(MaterialWithStateSchema), 'list_materials'),
      }
    );
  },

  async getMaterialPoolSummary(): Promise<z.infer<typeof MaterialPoolSummaryResponseSchema>> {
    return IpcClient.call('get_material_pool_summary', {}, {
      validate: zodValidator(MaterialPoolSummaryResponseSchema, 'get_material_pool_summary'),
      timeout: 60000,
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
    return IpcClient.call(
      'batch_lock_materials',
      {
        material_ids: materialIds,
        lock_flag: lockFlag,
        operator,
        reason,
        mode,
      },
      {
        validate: zodValidator(ImpactSummarySchema, 'batch_lock_materials'),
      }
    );
  },

  async batchForceRelease(
    materialIds: string[],
    operator: string,
    reason: string,
    mode?: 'Strict' | 'AutoFix'
  ): Promise<z.infer<typeof ImpactSummarySchema>> {
    return IpcClient.call(
      'batch_force_release',
      {
        material_ids: materialIds,
        operator,
        reason,
        mode,
      },
      {
        validate: zodValidator(ImpactSummarySchema, 'batch_force_release'),
      }
    );
  },

  async batchSetUrgent(
    materialIds: string[],
    manualUrgentFlag: boolean,
    operator: string,
    reason: string
  ): Promise<z.infer<typeof ImpactSummarySchema>> {
    return IpcClient.call(
      'batch_set_urgent',
      {
        material_ids: materialIds,
        manual_urgent_flag: manualUrgentFlag,
        operator,
        reason,
      },
      {
        validate: zodValidator(ImpactSummarySchema, 'batch_set_urgent'),
      }
    );
  },

  async listMaterialsByUrgentLevel(
    urgentLevel: string,
    machineCode?: string
  ): Promise<Array<z.infer<typeof MaterialWithStateSchema>>> {
    return IpcClient.call(
      'list_materials_by_urgent_level',
      {
        urgent_level: urgentLevel,
        machine_code: machineCode,
      },
      {
        validate: zodValidator(z.array(MaterialWithStateSchema), 'list_materials_by_urgent_level'),
      }
    );
  },
};

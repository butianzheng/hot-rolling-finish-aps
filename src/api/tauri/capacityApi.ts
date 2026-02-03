import { IpcClient } from '../ipcClient';
import {
  z,
  zodValidator,
  EmptyOkResponseSchema,
  CapacityPoolSchema,
  BatchUpdateCapacityPoolsResponseSchema,
} from '../ipcSchemas';

// Capacity API (产能池管理)
export const capacityApi = {
  async getCapacityPools(
    machineCodes: string[],
    dateFrom: string,
    dateTo: string,
    versionId?: string
  ): Promise<Array<z.infer<typeof CapacityPoolSchema>>> {
    return IpcClient.call(
      'get_capacity_pools',
      {
        machine_codes: JSON.stringify(machineCodes), // 后端期望 JSON 字符串
        date_from: dateFrom,
        date_to: dateTo,
        version_id: versionId,
      },
      {
        validate: zodValidator(z.array(CapacityPoolSchema), 'get_capacity_pools'),
      }
    );
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
    await IpcClient.call(
      'update_capacity_pool',
      {
        machine_code: machineCode,
        plan_date: planDate,
        target_capacity_t: targetCapacityT,
        limit_capacity_t: limitCapacityT,
        reason,
        operator,
        version_id: versionId,
      },
      {
        validate: zodValidator(EmptyOkResponseSchema, 'update_capacity_pool'),
      }
    );
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
    return IpcClient.call(
      'batch_update_capacity_pools',
      {
        updates: JSON.stringify(updates),
        reason,
        operator,
        version_id: versionId,
      },
      {
        validate: zodValidator(BatchUpdateCapacityPoolsResponseSchema, 'batch_update_capacity_pools'),
      }
    );
  },
};


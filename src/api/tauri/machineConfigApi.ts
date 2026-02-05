import { IpcClient } from '../ipcClient';
import { z, zodValidator } from '../ipcSchemas';
import {
  MachineConfigSchema,
  CreateOrUpdateMachineConfigRequestSchema,
  CreateOrUpdateMachineConfigResponseSchema,
  ApplyConfigToDateRangeRequestSchema,
  ApplyConfigToDateRangeResponseSchema,
} from '../ipcSchemas/machineConfigSchemas';

/**
 * 机组产能配置 API
 *
 * 用于"产能池管理日历化"功能，支持机组级配置和版本化管理
 */
export const machineConfigApi = {
  /**
   * 查询机组产能配置
   * @param versionId 版本ID
   * @param machineCodes 可选的机组代码列表（为空返回所有）
   * @returns 配置列表
   */
  async getMachineCapacityConfigs(
    versionId: string,
    machineCodes?: string[]
  ): Promise<Array<z.infer<typeof MachineConfigSchema>>> {
    return IpcClient.call(
      'get_machine_capacity_configs',
      {
        version_id: versionId,
        machine_codes: machineCodes ? JSON.stringify(machineCodes) : undefined,
      },
      {
        validate: zodValidator(z.array(MachineConfigSchema), 'get_machine_capacity_configs'),
      }
    );
  },

  /**
   * 创建或更新机组配置
   * @param request 配置请求
   * @returns 响应
   */
  async createOrUpdateMachineConfig(
    request: z.infer<typeof CreateOrUpdateMachineConfigRequestSchema>
  ): Promise<z.infer<typeof CreateOrUpdateMachineConfigResponseSchema>> {
    // 验证请求
    CreateOrUpdateMachineConfigRequestSchema.parse(request);

    return IpcClient.call(
      'create_or_update_machine_config',
      {
        request_json: JSON.stringify(request),
      },
      {
        validate: zodValidator(
          CreateOrUpdateMachineConfigResponseSchema,
          'create_or_update_machine_config'
        ),
      }
    );
  },

  /**
   * 批量应用机组配置到产能池日期范围
   * @param request 应用请求
   * @returns 响应
   */
  async applyMachineConfigToDates(
    request: z.infer<typeof ApplyConfigToDateRangeRequestSchema>
  ): Promise<z.infer<typeof ApplyConfigToDateRangeResponseSchema>> {
    // 验证请求
    ApplyConfigToDateRangeRequestSchema.parse(request);

    return IpcClient.call(
      'apply_machine_config_to_dates',
      {
        request_json: JSON.stringify(request),
      },
      {
        validate: zodValidator(
          ApplyConfigToDateRangeResponseSchema,
          'apply_machine_config_to_dates'
        ),
      }
    );
  },

  /**
   * 查询机组配置历史（跨版本）
   * @param machineCode 机组代码
   * @param limit 可选的限制条数
   * @returns 历史配置列表（按创建时间倒序）
   */
  async getMachineConfigHistory(
    machineCode: string,
    limit?: number
  ): Promise<Array<z.infer<typeof MachineConfigSchema>>> {
    return IpcClient.call(
      'get_machine_config_history',
      {
        machine_code: machineCode,
        limit,
      },
      {
        validate: zodValidator(z.array(MachineConfigSchema), 'get_machine_config_history'),
      }
    );
  },
};

import { IpcClient } from '../ipcClient';
import {
  z,
  zodValidator,
  EmptyOkResponseSchema,
  ConfigItemSchema,
  BatchUpdateConfigsResponseSchema,
  RestoreConfigFromSnapshotResponseSchema,
  ConfigSnapshotSchema,
  CustomStrategyProfileSchema,
  SaveCustomStrategyResponseSchema,
} from '../ipcSchemas';

export const configApi = {
  async listConfigs(): Promise<Array<z.infer<typeof ConfigItemSchema>>> {
    return IpcClient.call('list_configs', {}, {
      validate: zodValidator(z.array(ConfigItemSchema), 'list_configs'),
    });
  },

  async getConfig(scopeId: string, key: string): Promise<z.infer<typeof ConfigItemSchema> | null | undefined> {
    return IpcClient.call(
      'get_config',
      {
        scope_id: scopeId,
        key,
      },
      {
        validate: zodValidator(ConfigItemSchema.nullable().optional(), 'get_config'),
      }
    );
  },

  async updateConfig(
    scopeId: string,
    key: string,
    value: string,
    operator: string,
    reason: string
  ): Promise<void> {
    await IpcClient.call(
      'update_config',
      {
        scope_id: scopeId,
        key,
        value,
        operator,
        reason,
      },
      {
        validate: zodValidator(EmptyOkResponseSchema, 'update_config'),
      }
    );
  },

  async batchUpdateConfigs(
    configs: Array<{ scope_id: string; key: string; value: string }>,
    operator: string,
    reason: string
  ): Promise<z.infer<typeof BatchUpdateConfigsResponseSchema>> {
    // 后端期望 configs 为 JSON 字符串
    return IpcClient.call(
      'batch_update_configs',
      {
        configs: JSON.stringify(configs),
        operator,
        reason,
      },
      {
        validate: zodValidator(BatchUpdateConfigsResponseSchema, 'batch_update_configs'),
      }
    );
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
    return IpcClient.call(
      'restore_config_from_snapshot',
      {
        snapshot_json: snapshotJson,
        operator,
        reason,
      },
      {
        validate: zodValidator(RestoreConfigFromSnapshotResponseSchema, 'restore_config_from_snapshot'),
      }
    );
  },

  // ==========================================
  // Custom Strategy (P2)
  // ==========================================

  async saveCustomStrategy(params: {
    strategy: z.infer<typeof CustomStrategyProfileSchema>;
    operator: string;
    reason: string;
  }): Promise<z.infer<typeof SaveCustomStrategyResponseSchema>> {
    return IpcClient.call(
      'save_custom_strategy',
      {
        strategy_json: JSON.stringify(params.strategy),
        operator: params.operator,
        reason: params.reason,
      },
      {
        validate: zodValidator(SaveCustomStrategyResponseSchema, 'save_custom_strategy'),
      }
    );
  },

  async listCustomStrategies(): Promise<Array<z.infer<typeof CustomStrategyProfileSchema>>> {
    return IpcClient.call('list_custom_strategies', {}, {
      validate: zodValidator(z.array(CustomStrategyProfileSchema), 'list_custom_strategies'),
    });
  },
};


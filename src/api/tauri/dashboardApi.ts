import { IpcClient } from '../ipcClient';
import {
  z,
  zodValidator,
  DecisionDaySummaryResponseSchema,
  DecisionRefreshStatusResponseSchema,
  ManualRefreshDecisionResponseSchema,
  ActionLogSchema,
} from '../ipcSchemas';

export const dashboardApi = {
  async listRiskSnapshots(versionId: string): Promise<z.infer<typeof DecisionDaySummaryResponseSchema>> {
    return IpcClient.call('list_risk_snapshots', { version_id: versionId }, {
      validate: zodValidator(DecisionDaySummaryResponseSchema, 'list_risk_snapshots'),
    });
  },

  async getRiskSnapshot(
    versionId: string,
    snapshotDate: string
  ): Promise<z.infer<typeof DecisionDaySummaryResponseSchema>> {
    return IpcClient.call(
      'get_risk_snapshot',
      {
        version_id: versionId,
        snapshot_date: snapshotDate,
      },
      {
        validate: zodValidator(DecisionDaySummaryResponseSchema, 'get_risk_snapshot'),
      }
    );
  },

  async getRefreshStatus(versionId: string): Promise<z.infer<typeof DecisionRefreshStatusResponseSchema>> {
    return IpcClient.call(
      'get_refresh_status',
      {
        version_id: versionId,
      },
      {
        validate: zodValidator(DecisionRefreshStatusResponseSchema, 'get_refresh_status'),
      }
    );
  },

  async manualRefreshDecision(
    versionId: string,
    operator: string = 'admin'
  ): Promise<z.infer<typeof ManualRefreshDecisionResponseSchema>> {
    return IpcClient.call(
      'manual_refresh_decision',
      {
        version_id: versionId,
        operator,
      },
      {
        validate: zodValidator(ManualRefreshDecisionResponseSchema, 'manual_refresh_decision'),
      }
    );
  },

  async listActionLogs(startTime: string, endTime: string): Promise<Array<z.infer<typeof ActionLogSchema>>> {
    return IpcClient.call(
      'list_action_logs',
      {
        start_time: startTime,
        end_time: endTime,
      },
      {
        validate: zodValidator(z.array(ActionLogSchema), 'list_action_logs'),
      }
    );
  },

  async listActionLogsByMaterial(
    materialId: string,
    startTime: string,
    endTime: string,
    limit?: number
  ): Promise<Array<z.infer<typeof ActionLogSchema>>> {
    return IpcClient.call(
      'list_action_logs_by_material',
      {
        material_id: materialId,
        start_time: startTime,
        end_time: endTime,
        limit,
      },
      {
        validate: zodValidator(z.array(ActionLogSchema), 'list_action_logs_by_material'),
      }
    );
  },

  async listActionLogsByVersion(versionId: string): Promise<Array<z.infer<typeof ActionLogSchema>>> {
    return IpcClient.call(
      'list_action_logs_by_version',
      {
        version_id: versionId,
      },
      {
        validate: zodValidator(z.array(ActionLogSchema), 'list_action_logs_by_version'),
      }
    );
  },

  async getRecentActions(
    limit: number,
    opts?: {
      offset?: number;
      start_time?: string;
      end_time?: string;
    }
  ): Promise<Array<z.infer<typeof ActionLogSchema>>> {
    return IpcClient.call(
      'get_recent_actions',
      {
        limit,
        offset: opts?.offset,
        start_time: opts?.start_time,
        end_time: opts?.end_time,
      },
      {
        validate: zodValidator(z.array(ActionLogSchema), 'get_recent_actions'),
      }
    );
  },
};


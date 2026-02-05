// ==========================================
// Dashboard API（决策刷新管理 + 操作日志）
// ==========================================
// 职责分工：
// - 决策刷新状态管理（getRefreshStatus, manualRefreshDecision）
// - 操作日志查询（listActionLogs 系列）
//
// 注意：D1-D6 决策支持查询请使用 decisionService.ts
// ==========================================

import { IpcClient } from '../ipcClient';
import {
  z,
  zodValidator,
  DecisionRefreshStatusResponseSchema,
  ManualRefreshDecisionResponseSchema,
  ActionLogSchema,
} from '../ipcSchemas';

export const dashboardApi = {
  /**
   * 获取决策数据刷新状态
   */
  async getRefreshStatus(versionId: string): Promise<z.infer<typeof DecisionRefreshStatusResponseSchema>> {
    return IpcClient.call(
      'get_refresh_status',
      {
        version_id: versionId,
      },
      {
        validate: zodValidator(DecisionRefreshStatusResponseSchema, 'get_refresh_status'),
        timeout: 60000,
      }
    );
  },

  /**
   * 手动触发决策数据刷新
   */
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
        timeout: 300000,
      }
    );
  },

  /**
   * 查询操作日志（按时间范围）
   */
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

  /**
   * 查询操作日志（按物料 ID + 时间范围）
   */
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

  /**
   * 查询操作日志（按版本）
   */
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

  /**
   * 查询最近操作（支持分页和时间过滤）
   */
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

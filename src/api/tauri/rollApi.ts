import { IpcClient } from '../ipcClient';
import { z, zodValidator, EmptyOkResponseSchema, RollCampaignPlanInfoSchema, RollerCampaignInfoSchema } from '../ipcSchemas';

// Roll Campaign API (换辊管理)
export const rollApi = {
  async listRollCampaigns(versionId: string): Promise<Array<z.infer<typeof RollerCampaignInfoSchema>>> {
    return IpcClient.call(
      'list_roll_campaigns',
      {
        version_id: versionId,
      },
      {
        validate: zodValidator(z.array(RollerCampaignInfoSchema), 'list_roll_campaigns'),
      }
    );
  },

  async listRollCampaignPlans(versionId: string): Promise<Array<z.infer<typeof RollCampaignPlanInfoSchema>>> {
    return IpcClient.call(
      'list_roll_campaign_plans',
      {
        version_id: versionId,
      },
      {
        validate: zodValidator(z.array(RollCampaignPlanInfoSchema), 'list_roll_campaign_plans'),
      }
    );
  },

  async upsertRollCampaignPlan(params: {
    versionId: string;
    machineCode: string;
    initialStartAt: string; // YYYY-MM-DD HH:MM[:SS] or ISO
    nextChangeAt?: string;
    downtimeMinutes?: number;
    operator: string;
    reason: string;
  }): Promise<void> {
    await IpcClient.call(
      'upsert_roll_campaign_plan',
      {
        version_id: params.versionId,
        machine_code: params.machineCode,
        initial_start_at: params.initialStartAt,
        next_change_at: params.nextChangeAt,
        downtime_minutes: params.downtimeMinutes,
        operator: params.operator,
        reason: params.reason,
      },
      {
        validate: zodValidator(EmptyOkResponseSchema, 'upsert_roll_campaign_plan'),
      }
    );
  },

  async getActiveRollCampaign(
    versionId: string,
    machineCode: string
  ): Promise<z.infer<typeof RollerCampaignInfoSchema> | null> {
    return IpcClient.call(
      'get_active_roll_campaign',
      {
        version_id: versionId,
        machine_code: machineCode,
      },
      {
        validate: zodValidator(RollerCampaignInfoSchema.nullable(), 'get_active_roll_campaign'),
      }
    );
  },

  async listNeedsRollChange(versionId: string): Promise<Array<z.infer<typeof RollerCampaignInfoSchema>>> {
    return IpcClient.call(
      'list_needs_roll_change',
      {
        version_id: versionId,
      },
      {
        validate: zodValidator(z.array(RollerCampaignInfoSchema), 'list_needs_roll_change'),
      }
    );
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
  ): Promise<void> {
    await IpcClient.call(
      'create_roll_campaign',
      {
        version_id: versionId,
        machine_code: machineCode,
        campaign_no: campaignNo,
        start_date: startDate,
        suggest_threshold_t: suggestThresholdT,
        hard_limit_t: hardLimitT,
        operator,
        reason,
      },
      {
        validate: zodValidator(EmptyOkResponseSchema, 'create_roll_campaign'),
      }
    );
  },

  async closeRollCampaign(
    versionId: string,
    machineCode: string,
    campaignNo: number,
    endDate: string,
    operator: string,
    reason: string
  ): Promise<void> {
    await IpcClient.call(
      'close_roll_campaign',
      {
        version_id: versionId,
        machine_code: machineCode,
        campaign_no: campaignNo,
        end_date: endDate,
        operator,
        reason,
      },
      {
        validate: zodValidator(EmptyOkResponseSchema, 'close_roll_campaign'),
      }
    );
  },
};


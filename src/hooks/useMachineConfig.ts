/**
 * 机组产能配置状态管理 Hook
 * 管理机组配置的加载、创建、更新和历史查询
 */

import { useCallback, useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { message } from 'antd';
import { machineConfigApi } from '../api/tauri';
import type {
  MachineConfig,
  CreateOrUpdateMachineConfigRequest,
  CreateOrUpdateMachineConfigResponse,
  ApplyConfigToDateRangeRequest,
  ApplyConfigToDateRangeResponse,
} from '../api/ipcSchemas/machineConfigSchemas';

/**
 * useMachineConfig Hook 返回接口
 */
export interface UseMachineConfigReturn {
  // 当前版本的机组配置
  configs: MachineConfig[];
  configsLoading: boolean;
  configsError: Error | null;
  refetchConfigs: () => Promise<void>;

  // 单个机组配置
  getConfigByMachineCode: (machineCode: string) => MachineConfig | undefined;

  // 创建或更新配置
  updateConfig: (request: CreateOrUpdateMachineConfigRequest) => Promise<CreateOrUpdateMachineConfigResponse>;
  updateConfigLoading: boolean;

  // 应用配置到日期范围
  applyConfigToDates: (request: ApplyConfigToDateRangeRequest) => Promise<ApplyConfigToDateRangeResponse>;
  applyConfigLoading: boolean;

  // 查询配置历史
  configHistory: MachineConfig[];
  configHistoryLoading: boolean;
  getConfigHistory: (machineCode: string) => Promise<void>;
  clearConfigHistory: () => void;
}

/**
 * 机组产能配置 Hook
 * 支持版本绑定和历史查询
 */
export function useMachineConfig(versionId: string): UseMachineConfigReturn {
  const queryClient = useQueryClient();

  // ========== 状态 ==========
  const [selectedMachineCodeForHistory, setSelectedMachineCodeForHistory] = useState<string>('');
  const [configHistory, setConfigHistory] = useState<MachineConfig[]>([]);

  // ========== 查询 - 当前版本的机组配置 ==========
  const {
    data: configs = [],
    isLoading: configsLoading,
    error: configsError,
    refetch: refetchConfigsQuery,
  } = useQuery({
    queryKey: ['machineConfigs', versionId],
    queryFn: async () => {
      try {
        const data = await machineConfigApi.getMachineCapacityConfigs(versionId);
        return data || [];
      } catch (e: any) {
        console.error('【机组配置】查询失败：', e);
        message.error(e?.message || '加载机组配置失败');
        throw e;
      }
    },
    enabled: !!versionId,
    staleTime: 30 * 1000, // 30秒内不重新查询
  });

  // ========== 业务逻辑 - 快速查询 ==========
  const getConfigByMachineCode = useCallback(
    (machineCode: string): MachineConfig | undefined => {
      return configs.find((c) => c.machine_code === machineCode);
    },
    [configs]
  );

  const refetchConfigs = useCallback(async () => {
    await refetchConfigsQuery();
  }, [refetchConfigsQuery]);

  // ========== 变更 - 创建或更新配置 ==========
  const updateConfigMutation = useMutation({
    mutationFn: async (request: CreateOrUpdateMachineConfigRequest) => {
      try {
        const response = await machineConfigApi.createOrUpdateMachineConfig(request);
        message.success(response.message);
        return response;
      } catch (e: any) {
        console.error('【机组配置】更新失败：', e);
        message.error(e?.message || '更新机组配置失败');
        throw e;
      }
    },
    onSuccess: () => {
      // 更新成功后，重新查询机组配置
      queryClient.invalidateQueries({ queryKey: ['machineConfigs', versionId] });
    },
  });

  // ========== 变更 - 应用配置到日期范围 ==========
  const applyConfigMutation = useMutation({
    mutationFn: async (request: ApplyConfigToDateRangeRequest) => {
      try {
        const response = await machineConfigApi.applyMachineConfigToDates(request);
        message.success(
          `成功应用配置到 ${response.updated_count} 条记录，跳过 ${response.skipped_count} 条已有数据的记录`
        );
        return response;
      } catch (e: any) {
        console.error('【机组配置】应用失败：', e);
        message.error(e?.message || '应用配置失败');
        throw e;
      }
    },
  });

  // ========== 业务逻辑 - 查询历史 ==========
  const getConfigHistory = useCallback(
    async (machineCode: string) => {
      setSelectedMachineCodeForHistory(machineCode);
      try {
        const history = await machineConfigApi.getMachineConfigHistory(machineCode);
        setConfigHistory(history || []);
      } catch (e: any) {
        console.error('【机组配置】历史查询失败：', e);
        message.error(e?.message || '查询配置历史失败');
        setConfigHistory([]);
      }
    },
    []
  );

  const clearConfigHistory = useCallback(() => {
    setConfigHistory([]);
    setSelectedMachineCodeForHistory('');
  }, []);

  return {
    // 当前版本的机组配置
    configs,
    configsLoading,
    configsError: configsError instanceof Error ? configsError : null,
    refetchConfigs,
    getConfigByMachineCode,

    // 创建或更新配置
    updateConfig: updateConfigMutation.mutateAsync,
    updateConfigLoading: updateConfigMutation.isPending,

    // 应用配置到日期范围
    applyConfigToDates: applyConfigMutation.mutateAsync,
    applyConfigLoading: applyConfigMutation.isPending,

    // 查询配置历史
    configHistory,
    configHistoryLoading: selectedMachineCodeForHistory !== '' && configHistory.length === 0,
    getConfigHistory,
    clearConfigHistory,
  };
}

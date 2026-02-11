import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';

import { workbenchQueryKeys } from '../queryKeys';
import { decisionQueryKeys } from '../../../hooks/queries/use-decision-queries';

/**
 * Workbench 统一刷新协调器
 *
 * 使用 React Query 的 invalidateQueries 统一管理刷新策略
 * 替代之前的 refreshSignal + direct refetch 双轨制
 */
export function useWorkbenchRefreshActions(): {
  refreshAll: () => Promise<void>;
  refreshPlanItems: () => Promise<void>;
  refreshMaterials: () => Promise<void>;
  refreshPathOverride: () => Promise<void>;
  refreshRollCycleAnchor: () => Promise<void>;
} {
  const queryClient = useQueryClient();

  // 刷新所有 Workbench + 风险概览依赖数据（操作完成后的全量刷新）
  const refreshAll = useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: workbenchQueryKeys.all }),
      // 遗留 queryKeys：ScheduleCardView 等组件使用旧 key 格式
      // TODO(M1): 统一所有组件到 workbenchQueryKeys 后可移除
      queryClient.invalidateQueries({ queryKey: ['planItems'] }),
      queryClient.invalidateQueries({ queryKey: ['materials'] }),
      queryClient.invalidateQueries({ queryKey: decisionQueryKeys.all }),
      queryClient.invalidateQueries({ queryKey: ['globalKpi'] }),
      queryClient.invalidateQueries({ queryKey: ['decisionRefreshStatus'] }),
    ]);
  }, [queryClient]);

  // 刷新 planItems（仅刷新排程数据）
  const refreshPlanItems = useCallback(async () => {
    await queryClient.invalidateQueries({
      queryKey: workbenchQueryKeys.planItems.all,
    });
  }, [queryClient]);

  // 刷新 materials（仅刷新物料数据）
  const refreshMaterials = useCallback(async () => {
    await queryClient.invalidateQueries({
      queryKey: workbenchQueryKeys.materials.all,
    });
  }, [queryClient]);

  // 刷新 pathOverride（路径规则相关）
  const refreshPathOverride = useCallback(async () => {
    await queryClient.invalidateQueries({
      queryKey: workbenchQueryKeys.pathOverride.all,
    });
  }, [queryClient]);

  // 刷新 rollCycleAnchor（换辊周期锚点）
  const refreshRollCycleAnchor = useCallback(async () => {
    await queryClient.invalidateQueries({
      queryKey: workbenchQueryKeys.rollCycleAnchor.all,
    });
  }, [queryClient]);

  return { refreshAll, refreshPlanItems, refreshMaterials, refreshPathOverride, refreshRollCycleAnchor };
}

import type { UseQueryResult } from '@tanstack/react-query';
import { useQuery } from '@tanstack/react-query';

import { planApi } from '../../../api/tauri';
import { workbenchQueryKeys } from '../queryKeys';

type IpcPlanItem = Awaited<ReturnType<typeof planApi.listPlanItems>>[number];

/**
 * Workbench planItems 数据查询
 *
 * 使用统一的 queryKey，通过 invalidateQueries 触发刷新
 * 移除 refreshSignal 依赖，统一刷新策略
 */
export function useWorkbenchPlanItems(params: {
  activeVersionId: string | null;
}): {
  planItemsQuery: UseQueryResult<IpcPlanItem[], unknown>;
  planItems: IpcPlanItem[];
} {
  const { activeVersionId } = params;

  const planItemsQuery = useQuery({
    queryKey: workbenchQueryKeys.planItems.byVersion(activeVersionId),
    enabled: !!activeVersionId,
    queryFn: async () => {
      if (!activeVersionId) return [];
      return planApi.listPlanItems(activeVersionId);
    },
    staleTime: 30 * 1000,
  });

  return {
    planItemsQuery,
    planItems: planItemsQuery.data ?? [],
  };
}


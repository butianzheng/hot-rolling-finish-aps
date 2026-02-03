import { useEffect } from 'react';
import type { UseQueryResult } from '@tanstack/react-query';
import { useQuery } from '@tanstack/react-query';

import { planApi } from '../../../api/tauri';

type IpcPlanItem = Awaited<ReturnType<typeof planApi.listPlanItems>>[number];

export function useWorkbenchPlanItems(params: {
  activeVersionId: string | null;
  refreshSignal: number;
}): {
  planItemsQuery: UseQueryResult<IpcPlanItem[], unknown>;
  planItems: IpcPlanItem[];
} {
  const { activeVersionId, refreshSignal } = params;

  const planItemsQuery = useQuery({
    queryKey: ['planItems', activeVersionId],
    enabled: !!activeVersionId,
    queryFn: async () => {
      if (!activeVersionId) return [];
      return planApi.listPlanItems(activeVersionId);
    },
    staleTime: 30 * 1000,
  });

  useEffect(() => {
    if (!activeVersionId) return;
    if (refreshSignal == null) return;
    planItemsQuery.refetch();
  }, [activeVersionId, refreshSignal, planItemsQuery.refetch]);

  return {
    planItemsQuery,
    planItems: planItemsQuery.data ?? [],
  };
}


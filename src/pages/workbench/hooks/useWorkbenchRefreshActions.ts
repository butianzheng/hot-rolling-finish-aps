import { useCallback } from 'react';

export function useWorkbenchRefreshActions(params: {
  bumpRefreshSignal: () => void;
  materialsRefetch: () => void;
  planItemsRefetch: () => void;
}): {
  refreshAll: () => void;
  retryMaterials: () => void;
  retryPlanItems: () => void;
} {
  const { bumpRefreshSignal, materialsRefetch, planItemsRefetch } = params;

  const retryMaterials = useCallback(() => void materialsRefetch(), [materialsRefetch]);
  const retryPlanItems = useCallback(() => void planItemsRefetch(), [planItemsRefetch]);

  const refreshAll = useCallback(() => {
    bumpRefreshSignal();
    void materialsRefetch();
  }, [bumpRefreshSignal, materialsRefetch]);

  return { refreshAll, retryMaterials, retryPlanItems };
}


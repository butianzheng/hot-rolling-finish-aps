import { useMemo } from 'react';
import dayjs from 'dayjs';
import { useQuery } from '@tanstack/react-query';

import { capacityApi } from '../../../api/tauri';
import type { MoveImpactPreview, MoveValidationMode } from '../types';
import type { IpcPlanItem } from '../move/planItems';
import type { MoveImpactBase } from '../move/impact';
import { computeMoveImpactBase } from '../move/impact';
import { buildCapacityPoolMap } from '../move/recommend';
import { makeMachineDateKey } from '../move/key';

type IpcCapacityPool = Awaited<ReturnType<typeof capacityApi.getCapacityPools>>[number];

export function useWorkbenchMoveImpactPreview(params: {
  activeVersionId: string | null;
  moveModalOpen: boolean;
  moveTargetMachine: string | null;
  moveTargetDate: dayjs.Dayjs | null;
  moveValidationMode: MoveValidationMode;
  planItems: IpcPlanItem[];
  selectedMaterialIds: string[];
}): MoveImpactPreview | null {
  const { activeVersionId, moveModalOpen, moveTargetMachine, moveTargetDate, moveValidationMode, planItems, selectedMaterialIds } = params;

  const moveImpactBase = useMemo<MoveImpactBase | null>(() => {
    return computeMoveImpactBase({
      moveModalOpen,
      moveTargetMachine,
      moveTargetDate,
      moveValidationMode,
      planItems,
      selectedMaterialIds,
    });
  }, [moveModalOpen, moveTargetDate, moveTargetMachine, moveValidationMode, planItems, selectedMaterialIds]);

  const moveImpactCapacityQuery = useQuery({
    queryKey: [
      'moveImpactCapacity',
      activeVersionId,
      moveImpactBase?.affectedMachines.join(',') || '',
      moveImpactBase?.dateFrom || '',
      moveImpactBase?.dateTo || '',
    ],
    enabled:
      !!activeVersionId &&
      !!moveModalOpen &&
      !!moveImpactBase &&
      moveImpactBase.affectedMachines.length > 0 &&
      !!moveImpactBase.dateFrom &&
      !!moveImpactBase.dateTo,
    queryFn: async () => {
      if (!activeVersionId || !moveImpactBase) return [];
      return capacityApi.getCapacityPools(
        moveImpactBase.affectedMachines,
        moveImpactBase.dateFrom,
        moveImpactBase.dateTo,
        activeVersionId
      );
    },
    staleTime: 30 * 1000,
  });

  return useMemo<MoveImpactPreview | null>(() => {
    if (!moveImpactBase) return null;
    const pools: IpcCapacityPool[] = moveImpactCapacityQuery.data ?? [];
    const poolMap = buildCapacityPoolMap(pools);

    const rows = moveImpactBase.rows.map((r) => {
      const cap = poolMap.get(makeMachineDateKey(r.machine_code, r.date));
      return {
        ...r,
        target_capacity_t: cap?.target ?? null,
        limit_capacity_t: cap?.limit ?? cap?.target ?? null,
      };
    });

    const overflowRows = rows.filter((r) => {
      if (r.limit_capacity_t == null) return false;
      return r.after_t > r.limit_capacity_t + 1e-9;
    });

    return { rows, overflowRows, loading: moveImpactCapacityQuery.isLoading };
  }, [moveImpactBase, moveImpactCapacityQuery.data, moveImpactCapacityQuery.isLoading]);
}

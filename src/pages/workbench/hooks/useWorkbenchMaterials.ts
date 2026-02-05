import { useMemo } from 'react';
import type { InfiniteData, UseInfiniteQueryResult, UseQueryResult } from '@tanstack/react-query';
import { useInfiniteQuery, useQuery } from '@tanstack/react-query';
import type { DataNode } from 'antd/es/tree';

import { materialApi } from '../../../api/tauri';
import type { MaterialPoolMaterial } from '../../../components/workbench/MaterialPool';
import type { MaterialPoolSelection } from '../../../components/workbench/MaterialPool';
import type { WorkbenchLockStatusFilter } from '../../../stores/use-global-store';
import { normalizeSchedState } from '../../../utils/schedState';
import { workbenchQueryKeys } from '../queryKeys';
import { buildTreeDataFromSummary } from '../../../components/material-pool/utils';

type IpcMaterialWithState = Awaited<ReturnType<typeof materialApi.listMaterials>>[number];
type IpcMaterialPoolSummary = Awaited<ReturnType<typeof materialApi.getMaterialPoolSummary>>;

const MATERIAL_PAGE_SIZE = 1000;

/**
 * Workbench materials 数据查询
 *
 * 使用统一的 queryKey，通过 invalidateQueries 触发刷新
 */
export function useWorkbenchMaterials(params: {
  selection: MaterialPoolSelection;
  urgencyLevel: string | null;
  lockStatus: WorkbenchLockStatusFilter;
}): {
  materialsQuery: UseInfiniteQueryResult<InfiniteData<IpcMaterialWithState[], unknown>, Error>;
  materials: MaterialPoolMaterial[];
  poolSummaryQuery: UseQueryResult<IpcMaterialPoolSummary, Error>;
  poolTreeData: DataNode[];
} {
  const { selection, urgencyLevel, lockStatus } = params;

  const baseParams = useMemo(() => {
    const machineCode = selection.machineCode && selection.machineCode !== 'all'
      ? String(selection.machineCode).trim()
      : undefined;
    const schedState = selection.schedState ? String(selection.schedState).trim() : undefined;
    const urgent = urgencyLevel ? String(urgencyLevel).trim() : undefined;
    const lock = lockStatus && lockStatus !== 'ALL' ? lockStatus : undefined;

    return {
      machine_code: machineCode || undefined,
      sched_state: schedState || undefined,
      urgent_level: urgent || undefined,
      lock_status: lock || undefined,
      limit: MATERIAL_PAGE_SIZE,
    };
  }, [lockStatus, selection.machineCode, selection.schedState, urgencyLevel]);

  const materialsQuery = useInfiniteQuery({
    queryKey: workbenchQueryKeys.materials.infiniteList(baseParams),
    initialPageParam: 0,
    queryFn: async ({ pageParam }) => {
      const offset = typeof pageParam === 'number' ? pageParam : 0;
      return materialApi.listMaterials({ ...baseParams, offset });
    },
    getNextPageParam: (lastPage, allPages) => {
      if (!Array.isArray(lastPage) || lastPage.length < MATERIAL_PAGE_SIZE) return undefined;
      return allPages.length * MATERIAL_PAGE_SIZE;
    },
    staleTime: 30 * 1000,
  });

  const poolSummaryQuery = useQuery({
    queryKey: workbenchQueryKeys.materials.poolSummary(),
    queryFn: async () => materialApi.getMaterialPoolSummary(),
    staleTime: 30 * 1000,
  });

  const poolTreeData = useMemo(() => {
    return buildTreeDataFromSummary(poolSummaryQuery.data);
  }, [poolSummaryQuery.data]);

  const materials = useMemo<MaterialPoolMaterial[]>(() => {
    const pages = materialsQuery.data?.pages ?? [];
    const raw: IpcMaterialWithState[] = pages.flatMap((p) => (Array.isArray(p) ? p : []));
    return raw.map((m) => {
      const sched = normalizeSchedState(m.sched_state);
      const is_mature =
        sched === 'PENDING_MATURE'
          ? false
          : sched === 'READY' || sched === 'FORCE_RELEASE' || sched === 'SCHEDULED'
            ? true
            : undefined;

      return {
        material_id: String(m.material_id ?? '').trim(),
        machine_code: String(m.machine_code ?? '').trim(),
        weight_t: Number(m.weight_t ?? 0),
        steel_mark: String(m.steel_mark ?? '').trim(),
        sched_state: String(m.sched_state ?? '').trim(),
        urgent_level: String(m.urgent_level ?? '').trim(),
        lock_flag: Boolean(m.lock_flag),
        manual_urgent_flag: Boolean(m.manual_urgent_flag),
        is_mature,
      };
    });
  }, [materialsQuery.data?.pages]);

  return { materialsQuery, materials, poolSummaryQuery, poolTreeData };
}

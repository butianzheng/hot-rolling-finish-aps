import { useMemo } from 'react';
import type { UseQueryResult } from '@tanstack/react-query';
import { useQuery } from '@tanstack/react-query';

import { materialApi } from '../../../api/tauri';
import type { MaterialPoolMaterial } from '../../../components/workbench/MaterialPool';
import { normalizeSchedState } from '../../../utils/schedState';

type IpcMaterialWithState = Awaited<ReturnType<typeof materialApi.listMaterials>>[number];

type MaterialQueryParams = {
  machine_code?: string;
  limit: number;
  offset: number;
};

export function useWorkbenchMaterials(params: { machineCode: string | null }): {
  materialQueryParams: MaterialQueryParams;
  materialsQuery: UseQueryResult<IpcMaterialWithState[], unknown>;
  materials: MaterialPoolMaterial[];
} {
  const { machineCode } = params;

  // P2-2 修复：queryKey 包含筛选参数，避免缓存污染
  // 注意：暂保留 limit=1000 硬编码，待后续实施 useInfiniteQuery 分页优化
  const materialQueryParams = useMemo<MaterialQueryParams>(() => {
    return {
      machine_code: machineCode && machineCode !== 'all' ? machineCode : undefined,
      limit: 1000,
      offset: 0,
    };
  }, [machineCode]);

  const materialsQuery = useQuery({
    queryKey: ['materials', materialQueryParams],
    queryFn: async () => {
      return materialApi.listMaterials(materialQueryParams);
    },
    staleTime: 30 * 1000,
  });

  const materials = useMemo<MaterialPoolMaterial[]>(() => {
    const raw: IpcMaterialWithState[] = materialsQuery.data ?? [];
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
  }, [materialsQuery.data]);

  return { materialQueryParams, materialsQuery, materials };
}


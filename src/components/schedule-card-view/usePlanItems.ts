import { useQuery } from '@tanstack/react-query';
import { planApi } from '../../api/tauri';
import { useActiveVersionId } from '../../stores/use-global-store';
import { workbenchQueryKeys } from '../../pages/workbench/queryKeys';
import type { PlanItemRow } from './types';

export const usePlanItems = (machineCode?: string | null) => {
  const activeVersionId = useActiveVersionId();

  const query = useQuery({
    queryKey: workbenchQueryKeys.planItems.byVersion(activeVersionId),
    enabled: !!activeVersionId,
    queryFn: async () => {
      if (!activeVersionId) return [];
      const res = await planApi.listPlanItems(activeVersionId, {
        machine_code: machineCode && machineCode !== 'all' ? machineCode : undefined,
      });
      return Array.isArray(res) ? res : [];
    },
    staleTime: 30 * 1000,
  });

  return query;
};

export const normalizePlanItems = (data: unknown): PlanItemRow[] => {
  const raw = Array.isArray(data) ? data : [];
  return raw.map((it: unknown) => {
    const r = (it && typeof it === 'object' ? it : {}) as Record<string, unknown>;
    return {
      material_id: String(r.material_id ?? ''),
      machine_code: String(r.machine_code ?? ''),
      plan_date: String(r.plan_date ?? ''),
      seq_no: Number(r.seq_no ?? 0),
      weight_t: Number(r.weight_t ?? 0),
      urgent_level: r.urgent_level ? String(r.urgent_level) : undefined,
      locked_in_plan: !!r.locked_in_plan,
      force_release_in_plan: !!r.force_release_in_plan,
    };
  });
};

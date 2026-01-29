import { useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { planApi } from '../../api/tauri';
import { useActiveVersionId } from '../../stores/use-global-store';
import type { PlanItemRow } from './types';

export const usePlanItems = (machineCode?: string | null, refreshSignal?: number) => {
  const activeVersionId = useActiveVersionId();

  const query = useQuery({
    queryKey: ['planItems', activeVersionId, machineCode || 'all'],
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

  useEffect(() => {
    if (!activeVersionId) return;
    if (refreshSignal == null) return;
    query.refetch();
  }, [activeVersionId, refreshSignal, query.refetch]);

  return query;
};

export const normalizePlanItems = (data: any[] | undefined): PlanItemRow[] => {
  const raw = Array.isArray(data) ? data : [];
  return raw.map((it: any) => ({
    material_id: String(it?.material_id ?? ''),
    machine_code: String(it?.machine_code ?? ''),
    plan_date: String(it?.plan_date ?? ''),
    seq_no: Number(it?.seq_no ?? 0),
    weight_t: Number(it?.weight_t ?? 0),
    urgent_level: it?.urgent_level ? String(it.urgent_level) : undefined,
    locked_in_plan: !!it?.locked_in_plan,
    force_release_in_plan: !!it?.force_release_in_plan,
  }));
};

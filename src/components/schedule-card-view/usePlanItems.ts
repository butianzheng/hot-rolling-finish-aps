import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import type { Dayjs } from 'dayjs';
import { planApi } from '../../api/tauri';
import { useActiveVersionId } from '../../stores/use-global-store';
import { workbenchQueryKeys } from '../../pages/workbench/queryKeys';
import { formatDate } from '../../utils/formatters';
import type { PlanItemRow } from './types';

export const usePlanItems = (machineCode: string | null | undefined, dateRange: [Dayjs, Dayjs]) => {
  const activeVersionId = useActiveVersionId();

  const planDateFrom = useMemo(() => formatDate(dateRange[0]), [dateRange]);
  const planDateTo = useMemo(() => formatDate(dateRange[1]), [dateRange]);
  const normalizedMachineCode = useMemo(() => {
    const code = String(machineCode || '').trim();
    return code && code !== 'all' ? code : undefined;
  }, [machineCode]);

  const queryParams = useMemo(
    () => ({
      version_id: activeVersionId,
      machine_code: normalizedMachineCode,
      plan_date_from: planDateFrom,
      plan_date_to: planDateTo,
    }),
    [activeVersionId, normalizedMachineCode, planDateFrom, planDateTo]
  );

  const query = useQuery({
    queryKey: workbenchQueryKeys.planItems.list(queryParams),
    enabled: !!activeVersionId,
    queryFn: async ({ signal }) => {
      if (!activeVersionId) return [];
      const pageSize = 5000;
      const maxItems = 200_000;
      let offset = 0;
      const all: any[] = [];

      while (true) {
        if (signal?.aborted) {
          throw new DOMException('Aborted', 'AbortError');
        }

        const page = await planApi.listPlanItems(activeVersionId, {
          machine_code: normalizedMachineCode,
          plan_date_from: planDateFrom,
          plan_date_to: planDateTo,
          limit: pageSize,
          offset,
        });

        all.push(...page);
        if (page.length < pageSize) break;
        offset += pageSize;
        if (offset >= maxItems) break;
      }

      return all;
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

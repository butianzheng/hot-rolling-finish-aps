import type { UseQueryResult } from '@tanstack/react-query';
import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import type { Dayjs } from 'dayjs';

import { planApi } from '../../../api/tauri';
import { formatDate } from '../../../utils/formatters';
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
  machineCode: string | null;
  dateRange: [Dayjs, Dayjs];
}): {
  planItemsQuery: UseQueryResult<IpcPlanItem[], unknown>;
  planItems: IpcPlanItem[];
} {
  const { activeVersionId, machineCode, dateRange } = params;

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

  const planItemsQuery = useQuery({
    queryKey: workbenchQueryKeys.planItems.list(queryParams),
    enabled: !!activeVersionId,
    queryFn: async ({ signal }) => {
      if (!activeVersionId) return [];

      // 分页拉取，避免一次性返回超大 JSON 导致 IPC 超时/内存峰值过高
      const pageSize = 5000;
      const maxItems = 200_000;
      let offset = 0;
      const all: IpcPlanItem[] = [];

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

  return {
    planItemsQuery,
    planItems: planItemsQuery.data ?? [],
  };
}

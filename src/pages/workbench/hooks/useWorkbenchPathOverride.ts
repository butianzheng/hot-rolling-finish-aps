import { useCallback, useMemo } from 'react';
import dayjs from 'dayjs';
import { message } from 'antd';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { pathRuleApi, planApi } from '../../../api/tauri';
import { formatDate } from '../../../utils/formatters';
import { getErrorMessage } from '../../../utils/errorUtils';
import type { WorkbenchPathOverrideState, WorkbenchScheduleFocus } from '../types';
import { workbenchQueryKeys } from '../queryKeys';

type SummaryRow = Awaited<ReturnType<typeof pathRuleApi.listPathOverridePendingSummary>>[number];

/**
 * Workbench 路径规则覆盖状态
 *
 * 使用统一的 queryKey，通过 invalidateQueries 触发刷新
 * 移除 refreshSignal 依赖
 */
export function useWorkbenchPathOverride(params: {
  activeVersionId: string | null;
  scheduleFocus: WorkbenchScheduleFocus | null;
  poolMachineCode: string | null;
  autoDateRange: [dayjs.Dayjs, dayjs.Dayjs];
  currentUser: string | null;
  defaultStrategy: string | null | undefined;
  setRecalculating: (flag: boolean) => void;
  setActiveVersion: (versionId: string | null) => void;
}): WorkbenchPathOverrideState {
  const {
    activeVersionId,
    scheduleFocus,
    poolMachineCode,
    autoDateRange,
    currentUser,
    defaultStrategy,
    setRecalculating,
    setActiveVersion,
  } = params;

  const queryClient = useQueryClient();

  const defaultPlanDate = useMemo(() => formatDate(dayjs()), []);

  const context = useMemo(() => {
    const machine = String(scheduleFocus?.machine || poolMachineCode || '').trim();
    const date = String(scheduleFocus?.date || defaultPlanDate).trim();
    return {
      machineCode: machine || null,
      planDate: date || null,
    };
  }, [defaultPlanDate, poolMachineCode, scheduleFocus?.date, scheduleFocus?.machine]);

  const pendingQuery = useQuery({
    queryKey: workbenchQueryKeys.pathOverride.pending(
      activeVersionId,
      context.machineCode,
      context.planDate || ''
    ),
    enabled: !!activeVersionId && !!context.machineCode && !!context.planDate,
    queryFn: async () => {
      if (!activeVersionId || !context.machineCode || !context.planDate) return [];
      return pathRuleApi.listPathOverridePending({
        versionId: activeVersionId,
        machineCode: context.machineCode,
        planDate: context.planDate,
      });
    },
    staleTime: 15 * 1000,
  });

  const pendingCount = useMemo(() => pendingQuery.data?.length ?? 0, [pendingQuery.data]);

  const recalcAfterPathOverride = useCallback(
    async (baseDate: string) => {
      if (!activeVersionId) return;
      const base = String(baseDate || '').trim() || defaultPlanDate;
      setRecalculating(true);
      try {
        const res = await planApi.recalcFull(
          activeVersionId,
          base,
          undefined,
          currentUser || 'admin',
          defaultStrategy || 'balanced'
        );
        const nextVersionId = String(res?.version_id ?? '').trim();
        if (nextVersionId) {
          setActiveVersion(nextVersionId);
          message.success(`已重算并切换到新版本：${nextVersionId}`);
        } else {
          message.success(String(res?.message || '重算完成'));
        }
        // 使用统一的 queryKey 刷新
        await queryClient.invalidateQueries({ queryKey: workbenchQueryKeys.all });
      } catch (e: unknown) {
        console.error('【工作台】路径放行后重算失败：', e);
        message.error(getErrorMessage(e) || '重算失败');
      } finally {
        setRecalculating(false);
      }
    },
    [
      activeVersionId,
      currentUser,
      defaultPlanDate,
      defaultStrategy,
      queryClient,
      setActiveVersion,
      setRecalculating,
    ]
  );

  const summaryRange = useMemo(() => {
    return {
      from: formatDate(autoDateRange[0]),
      to: formatDate(autoDateRange[1]),
    };
  }, [autoDateRange]);

  const summaryQuery = useQuery({
    queryKey: workbenchQueryKeys.pathOverride.summary(activeVersionId, summaryRange.from),
    enabled: !!activeVersionId,
    queryFn: async () => {
      if (!activeVersionId) return [];
      return pathRuleApi.listPathOverridePendingSummary({
        versionId: activeVersionId,
        planDateFrom: summaryRange.from,
        planDateTo: summaryRange.to,
      });
    },
    staleTime: 15 * 1000,
  });

  const pendingTotalCount = useMemo(() => {
    const list: SummaryRow[] = summaryQuery.data ?? [];
    return list.reduce((sum, r) => sum + Number(r.pending_count ?? 0), 0);
  }, [summaryQuery.data]);

  const pendingRefetch = useCallback(() => void pendingQuery.refetch(), [pendingQuery.refetch]);
  const summaryRefetch = useCallback(() => void summaryQuery.refetch(), [summaryQuery.refetch]);

  return useMemo(
    () => ({
      context,
      pendingCount,
      pendingIsFetching: pendingQuery.isFetching,
      pendingRefetch,
      summaryRange,
      pendingTotalCount,
      summaryIsFetching: summaryQuery.isFetching,
      summaryRefetch,
      recalcAfterPathOverride,
    }),
    [
      context,
      pendingCount,
      pendingQuery.isFetching,
      pendingRefetch,
      pendingTotalCount,
      recalcAfterPathOverride,
      summaryQuery.isFetching,
      summaryRange,
      summaryRefetch,
    ]
  );
}

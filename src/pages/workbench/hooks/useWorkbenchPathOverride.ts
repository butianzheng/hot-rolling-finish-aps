import { useMemo } from 'react';
import dayjs from 'dayjs';
import { message } from 'antd';
import { useQuery } from '@tanstack/react-query';

import { pathRuleApi, planApi } from '../../../api/tauri';
import { formatDate } from '../../../utils/formatters';
import { getErrorMessage } from '../../../utils/errorUtils';

type ScheduleFocus = {
  machine?: string;
  date: string;
  source?: string;
};

type SummaryRow = Awaited<ReturnType<typeof pathRuleApi.listPathOverridePendingSummary>>[number];

export function useWorkbenchPathOverride(params: {
  activeVersionId: string | null;
  scheduleFocus: ScheduleFocus | null;
  poolMachineCode: string | null;
  autoDateRange: [dayjs.Dayjs, dayjs.Dayjs];
  refreshSignal: number;
  currentUser: string | null;
  defaultStrategy: string | null | undefined;
  setRecalculating: (flag: boolean) => void;
  setActiveVersion: (versionId: string | null) => void;
  bumpRefreshSignal: () => void;
  materialsRefetch: () => void;
}): {
  context: { machineCode: string | null; planDate: string | null };
  pendingCount: number;
  pendingIsFetching: boolean;
  pendingRefetch: () => void;
  summaryRange: { from: string; to: string };
  pendingTotalCount: number;
  summaryIsFetching: boolean;
  summaryRefetch: () => void;
  recalcAfterPathOverride: (baseDate: string) => Promise<void>;
} {
  const {
    activeVersionId,
    scheduleFocus,
    poolMachineCode,
    autoDateRange,
    refreshSignal,
    currentUser,
    defaultStrategy,
    setRecalculating,
    setActiveVersion,
    bumpRefreshSignal,
    materialsRefetch,
  } = params;

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
    queryKey: ['pathOverridePending', activeVersionId, context.machineCode, context.planDate, refreshSignal],
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

  const recalcAfterPathOverride = async (baseDate: string) => {
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
      bumpRefreshSignal();
      materialsRefetch();
    } catch (e: unknown) {
      console.error('[Workbench] recalcAfterPathOverride failed:', e);
      message.error(getErrorMessage(e) || '重算失败');
    } finally {
      setRecalculating(false);
    }
  };

  const summaryRange = useMemo(() => {
    return {
      from: formatDate(autoDateRange[0]),
      to: formatDate(autoDateRange[1]),
    };
  }, [autoDateRange]);

  const summaryQuery = useQuery({
    queryKey: ['pathOverridePendingSummary', activeVersionId, summaryRange.from, summaryRange.to, refreshSignal],
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

  return {
    context,
    pendingCount,
    pendingIsFetching: pendingQuery.isFetching,
    pendingRefetch: () => void pendingQuery.refetch(),
    summaryRange,
    pendingTotalCount,
    summaryIsFetching: summaryQuery.isFetching,
    summaryRefetch: () => void summaryQuery.refetch(),
    recalcAfterPathOverride,
  };
}


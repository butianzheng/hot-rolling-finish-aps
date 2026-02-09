import { useCallback, useMemo } from 'react';
import dayjs from 'dayjs';
import { message } from 'antd';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { pathRuleApi, planApi } from '../../../api/tauri';
import { getLatestRunTtlMs } from '../../../stores/latestRun';
import { useGlobalStore } from '../../../stores/use-global-store';
import { createRunId } from '../../../utils/runId';
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
  beginLatestRun: (input: { runId: string; triggeredAt?: number; ttlMs?: number; versionId?: string | null }) => {
    accepted: boolean;
    reason?: 'OLDER_TRIGGER' | 'EXPIRED_PREVIOUS';
  };
  markLatestRunRunning: (runId: string) => void;
  markLatestRunDone: (runId: string, payload?: { versionId?: string | null; planRev?: number | null }) => void;
  markLatestRunFailed: (runId: string, error?: string | null) => void;
  expireLatestRunIfNeeded: (now?: number) => void;
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
    beginLatestRun,
    markLatestRunRunning,
    markLatestRunDone,
    markLatestRunFailed,
    expireLatestRunIfNeeded,
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

      expireLatestRunIfNeeded();
      const localRunId = createRunId('recalc');
      const beginResult = beginLatestRun({
        runId: localRunId,
        versionId: activeVersionId,
        ttlMs: getLatestRunTtlMs(),
      });

      if (!beginResult.accepted) {
        message.info('已存在更新的重算触发，本次请求已忽略');
        return;
      }

      setRecalculating(true);
      markLatestRunRunning(localRunId);
      try {
        const res = await planApi.recalcFull(
          activeVersionId,
          base,
          undefined,
          currentUser || 'admin',
          defaultStrategy || 'balanced',
          undefined,
          localRunId,
        );

        const responseRunId = String(res?.run_id ?? localRunId).trim() || localRunId;
        const responsePlanRev = Number(res?.plan_rev);
        const nextVersionId = String(res?.version_id ?? '').trim();

        markLatestRunDone(responseRunId, {
          versionId: nextVersionId || activeVersionId,
          planRev: Number.isFinite(responsePlanRev) ? responsePlanRev : undefined,
        });

        const latestRunId = useGlobalStore.getState().latestRun.runId;
        if (latestRunId !== responseRunId) {
          return;
        }

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
        markLatestRunFailed(localRunId, getErrorMessage(e) || '重算失败');
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
      beginLatestRun,
      markLatestRunRunning,
      markLatestRunDone,
      markLatestRunFailed,
      expireLatestRunIfNeeded,
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

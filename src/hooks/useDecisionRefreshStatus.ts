import { useEffect, useRef } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { dashboardApi } from '../api/tauri';
import { decisionQueryKeys } from './queries/use-decision-queries';
import { useOnlineStatus } from './useOnlineStatus';

export type DecisionRefreshStatus = {
  version_id: string;
  status: 'IDLE' | 'REFRESHING' | 'FAILED' | string;
  is_refreshing: boolean;
  queue_counts: {
    pending: number;
    running: number;
    failed: number;
    completed: number;
    cancelled: number;
  };
  latest_task?: {
    task_id: string;
    version_id: string;
    trigger_type: string;
    trigger_source?: string | null;
    is_full_refresh: boolean;
    status: string;
    retry_count: number;
    max_retries: number;
    created_at: string;
    started_at?: string | null;
    completed_at?: string | null;
    error_message?: string | null;
    refresh_id?: string | null;
  } | null;
  latest_log?: {
    refresh_id: string;
    version_id: string;
    trigger_type: string;
    trigger_source?: string | null;
    is_full_refresh: boolean;
    refreshed_tables_json: string;
    rows_affected: number;
    started_at: string;
    completed_at?: string | null;
    duration_ms?: number | null;
    status: string;
    error_message?: string | null;
  } | null;
  last_error?: string | null;
  message: string;
};

export function useDecisionRefreshStatus(versionId: string | null) {
  const queryClient = useQueryClient();
  const isOnline = useOnlineStatus();

  const lastSeen = useRef<{
    isRefreshing?: boolean;
    refreshId?: string | null;
    completedAt?: string | null;
    hadError?: boolean;
  }>({});

  const query = useQuery({
    queryKey: ['decisionRefreshStatus', versionId],
    enabled: Boolean(versionId) && isOnline,
    staleTime: 0,
    gcTime: 5 * 60 * 1000,
    queryFn: async (): Promise<DecisionRefreshStatus> => {
      if (!versionId) {
        throw new Error('MISSING_VERSION_ID');
      }
      return dashboardApi.getRefreshStatus(versionId);
    },
    refetchInterval: (q) => {
      if (!versionId) return false;
      if (!isOnline) return false;
      if (typeof document !== 'undefined' && document.hidden) return false;

      const data = q.state.data as DecisionRefreshStatus | undefined;
      if (data?.is_refreshing) return 1000;
      return 8000;
    },
    refetchOnWindowFocus: true,
    refetchOnReconnect: true,
  });

  useEffect(() => {
    const data = query.data;
    if (!versionId || !data) return;

    const isRefreshing = Boolean(data.is_refreshing);
    const refreshId = data.latest_log?.refresh_id ?? data.latest_task?.refresh_id ?? null;
    const completedAt = data.latest_log?.completed_at ?? data.latest_task?.completed_at ?? null;
    const hadError = Boolean(data.last_error);

    const prev = lastSeen.current;
    const justFinished =
      prev.isRefreshing === true &&
      isRefreshing === false &&
      !hadError &&
      Boolean(completedAt);

    const completedAdvanced =
      !isRefreshing &&
      !hadError &&
      Boolean(completedAt) &&
      completedAt !== prev.completedAt &&
      Boolean(prev.completedAt);

    if (justFinished || completedAdvanced) {
      // 决策读模型刷新完成后，无效化相关查询，让页面自动重新取数。
      queryClient.invalidateQueries({ queryKey: decisionQueryKeys.all });
      queryClient.invalidateQueries({ queryKey: ['globalKpi', versionId] });
    }

    lastSeen.current = {
      isRefreshing,
      refreshId,
      completedAt,
      hadError,
    };
  }, [query.data, queryClient, versionId]);

  return query;
}


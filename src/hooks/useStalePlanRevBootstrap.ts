import { useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { message } from 'antd';

import { planApi } from '../api/tauri';
import { decisionQueryKeys } from './queries/use-decision-queries';
import { workbenchQueryKeys } from '../pages/workbench/queryKeys';
import { useGlobalActions } from '../stores/use-global-store';
import {
  registerStalePlanRevRefreshHandler,
  type StalePlanRevRefreshContext,
} from '../services/stalePlanRev';

function isDeepLinkScene(): boolean {
  if (typeof window === 'undefined') return false;
  const params = new URLSearchParams(window.location.search);
  return (
    params.has('machine') ||
    params.has('date') ||
    params.has('context') ||
    params.has('focus') ||
    params.has('openCell') ||
    params.has('material_id')
  );
}

async function fetchLatestPlanContext(): Promise<{ versionId: string | null; planRev: number | null }> {
  const latestVersionId = await planApi.getLatestActiveVersionId();
  if (!latestVersionId) {
    return { versionId: null, planRev: null };
  }

  const detail = await planApi.getVersionDetail(latestVersionId);
  return {
    versionId: latestVersionId,
    planRev: typeof detail?.revision === 'number' ? detail.revision : null,
  };
}

export function useStalePlanRevBootstrap() {
  const queryClient = useQueryClient();
  const { setPlanContext } = useGlobalActions();

  useEffect(() => {
    registerStalePlanRevRefreshHandler(async (_ctx: StalePlanRevRefreshContext) => {
      const latest = await fetchLatestPlanContext();
      setPlanContext(latest);

      await Promise.all([
        queryClient.invalidateQueries({ queryKey: workbenchQueryKeys.all }),
        queryClient.invalidateQueries({ queryKey: decisionQueryKeys.all }),
        queryClient.invalidateQueries({ queryKey: ['globalKpi'] }),
        queryClient.invalidateQueries({ queryKey: ['decisionRefreshStatus'] }),
      ]);

      if (isDeepLinkScene()) {
        message.info({
          key: 'stale-plan-rev-deeplink',
          content: '深链接对应计划已过期，已切换到最新计划',
          duration: 2,
        });
      }
    });

    return () => {
      registerStalePlanRevRefreshHandler(null);
    };
  }, [queryClient, setPlanContext]);
}

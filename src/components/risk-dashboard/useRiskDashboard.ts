/**
 * 风险仪表盘状态管理 Hook
 */

import { useEffect, useState } from 'react';
import { dashboardApi } from '../../api/tauri';
import { decisionService } from '../../services/decision-service';
import { parseAlertLevel } from '../../types/decision';
import { useActiveVersionId } from '../../stores/use-global-store';
import type { DangerDayData, RollCampaignHealth } from '../../types/dashboard';
import type { BlockedUrgentOrderRow, ColdStockBucketRow } from './types';

export interface UseRiskDashboardReturn {
  activeVersionId: string | null;
  loading: boolean;
  loadError: string | null;
  dangerDay: DangerDayData | null;
  blockedOrders: BlockedUrgentOrderRow[];
  coldStockBuckets: ColdStockBucketRow[];
  rollHealth: RollCampaignHealth[];
}

export function useRiskDashboard(): UseRiskDashboardReturn {
  const activeVersionId = useActiveVersionId();
  const [dangerDay, setDangerDay] = useState<DangerDayData | null>(null);
  const [blockedOrders, setBlockedOrders] = useState<BlockedUrgentOrderRow[]>([]);
  const [coldStockBuckets, setColdStockBuckets] = useState<ColdStockBucketRow[]>([]);
  const [rollHealth, setRollHealth] = useState<RollCampaignHealth[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    if (!activeVersionId) return;

    setLoading(true);
    setLoadError(null);

    (async () => {
      try {
        const [mostRiskyRes, urgentRes, coldRes, rollRes] = await Promise.allSettled([
          dashboardApi.getMostRiskyDate(activeVersionId),
          dashboardApi.getUnsatisfiedUrgentMaterials(activeVersionId),
          dashboardApi.getColdStockMaterials(activeVersionId, 30),
          decisionService.getAllRollCampaignAlerts(activeVersionId),
        ]);

        const errors: string[] = [];

        if (mostRiskyRes.status === 'fulfilled') {
          const most = mostRiskyRes.value?.items?.[0];
          if (most) {
            const riskLevelRaw = String(most.risk_level || '').toUpperCase();
            const riskLevel: DangerDayData['riskLevel'] =
              riskLevelRaw === 'CRITICAL'
                ? 'critical'
                : riskLevelRaw === 'HIGH'
                ? 'high'
                : riskLevelRaw === 'MEDIUM'
                ? 'medium'
                : 'low';

            setDangerDay({
              date: most.plan_date,
              riskLevel,
              capacityOverflow: Number(most.overload_weight_t || 0),
              urgentBacklog: Number(most.urgent_failure_count || 0),
              reasons: (most.top_reasons || []).map((r: any) => r?.msg || '').filter(Boolean),
            });
          } else {
            setDangerDay(null);
          }
        } else {
          errors.push('危险日期加载失败');
          setDangerDay(null);
        }

        if (urgentRes.status === 'fulfilled') {
          setBlockedOrders((urgentRes.value?.items || []) as BlockedUrgentOrderRow[]);
        } else {
          errors.push('紧急阻塞订单加载失败');
          setBlockedOrders([]);
        }

        if (coldRes.status === 'fulfilled') {
          setColdStockBuckets((coldRes.value?.items || []) as ColdStockBucketRow[]);
        } else {
          errors.push('库龄/冷料压库加载失败');
          setColdStockBuckets([]);
        }

        if (rollRes.status === 'fulfilled') {
          const rollItems = rollRes.value?.items || [];
          const mappedRollHealth: RollCampaignHealth[] = rollItems.map((r) => {
            const status = parseAlertLevel(String(r.alertLevel || ''));
            const mappedStatus: RollCampaignHealth['status'] =
              status === 'HARD_STOP'
                ? 'critical'
                : status === 'WARNING' || status === 'SUGGEST'
                ? 'warning'
                : 'healthy';

            return {
              machineCode: r.machineCode,
              currentTonnage: r.currentTonnageT,
              threshold: r.hardLimitT,
              status: mappedStatus,
              estimatedRollsRemaining: 0,
            };
          });
          setRollHealth(mappedRollHealth);
        } else {
          errors.push('换辊警报加载失败');
          setRollHealth([]);
        }

        setLoadError(errors.length ? errors.join('；') : null);
      } catch (e: any) {
        console.error('[RiskDashboard] load failed:', e);
        setLoadError(e?.message || '数据加载失败');
      } finally {
        setLoading(false);
      }
    })();
  }, [activeVersionId]);

  return {
    activeVersionId,
    loading,
    loadError,
    dangerDay,
    blockedOrders,
    coldStockBuckets,
    rollHealth,
  };
}

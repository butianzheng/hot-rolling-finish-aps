import { useQuery } from '@tanstack/react-query';
import { decisionService, materialApi } from '../api/tauri';
import { parseAlertLevel } from '../types/decision';
import type { RollCampaignAlert } from '../types/decision';
import type { GlobalKPI } from '../types/kpi';
import { isScheduled } from '../utils/schedState';

function normalizeRiskLevel(level: unknown): GlobalKPI['riskLevel'] {
  const upper = String(level || '').toUpperCase();
  if (upper === 'CRITICAL') return 'critical';
  if (upper === 'HIGH') return 'high';
  if (upper === 'MEDIUM') return 'medium';
  return 'low';
}

export function useGlobalKPI(versionId: string | null) {
  return useQuery({
    queryKey: ['globalKpi', versionId],
    enabled: !!versionId,
    staleTime: 60 * 1000,
    queryFn: async (): Promise<GlobalKPI> => {
      if (!versionId) {
        throw new Error('MISSING_VERSION_ID');
      }

      const [mostRiskyRes, coldStockRes, urgentL2, urgentL3, rollAlertsRes] = await Promise.all([
        decisionService.getMostRiskyDate(versionId),
        decisionService.getColdStockMaterials(versionId, 30),
        materialApi.listMaterialsByUrgentLevel('L2'),
        materialApi.listMaterialsByUrgentLevel('L3'),
        decisionService.getAllRollCampaignAlerts(versionId),
      ]);

      const most = mostRiskyRes?.items?.[0];
      const riskLevel = normalizeRiskLevel(most?.riskLevel);

      const urgentL2Items = urgentL2;
      const urgentL3Items = urgentL3;
      const urgentMaterials = [...urgentL2Items, ...urgentL3Items];
      const blockedL2 = urgentL2Items.filter((m) => !isScheduled(m.sched_state)).length;
      const blockedL3 = urgentL3Items.filter((m) => !isScheduled(m.sched_state)).length;
      const blockedUrgentCount = blockedL2 + blockedL3;

      const rollItems = rollAlertsRes?.items || [];
      const rollItem = rollItems.reduce<RollCampaignAlert | null>((best, cur) => {
        if (!best) return cur;
        const bestStatus = parseAlertLevel(String(best.alertLevel || ''));
        const curStatus = parseAlertLevel(String(cur.alertLevel || ''));
        const severityOrder: Record<string, number> = {
          HARD_STOP: 3,
          WARNING: 2,
          SUGGEST: 1,
          NORMAL: 0,
        };
        const bestScore = severityOrder[bestStatus] ?? 0;
        const curScore = severityOrder[curStatus] ?? 0;
        if (curScore !== bestScore) return curScore > bestScore ? cur : best;

        const bestUtil = best.hardLimitT > 0 ? best.currentTonnageT / best.hardLimitT : 0;
        const curUtil = cur.hardLimitT > 0 ? cur.currentTonnageT / cur.hardLimitT : 0;
        return curUtil > bestUtil ? cur : best;
      }, null);

      const rollStatusRaw = rollItem ? parseAlertLevel(String(rollItem.alertLevel || '')) : 'NORMAL';
      const rollStatus =
        rollStatusRaw === 'HARD_STOP'
          ? 'critical'
          : rollStatusRaw === 'WARNING' || rollStatusRaw === 'SUGGEST'
          ? 'warning'
          : 'healthy';

      const coldStockCount =
        typeof coldStockRes?.summary?.totalColdStockCount === 'number'
          ? coldStockRes.summary.totalColdStockCount
          : (coldStockRes?.items || []).reduce((sum, b) => sum + (Number(b?.count) || 0), 0);

      return {
        mostRiskyDate: most?.planDate,
        riskLevel,
        urgentOrdersCount: urgentMaterials.length,
        blockedUrgentCount,
        urgentBreakdown: {
          L2: { total: urgentL2Items.length, blocked: blockedL2 },
          L3: { total: urgentL3Items.length, blocked: blockedL3 },
        },
        capacityUtilization: typeof most?.capacityUtilPct === 'number' ? most.capacityUtilPct : 0,
        coldStockCount,
        rollCampaignProgress: rollItem?.currentTonnageT ?? 0,
        rollChangeThreshold: rollItem?.hardLimitT ?? 1500,
        rollStatus,
      };
    },
  });
}

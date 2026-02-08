import { useMemo } from 'react';
import {
  useAllFailedOrders,
  useAllRollCampaignAlerts,
  useColdStockProfile,
  useRecentDaysBottleneck,
  useRecentDaysCapacityOpportunity,
  useRecentDaysRisk,
} from './queries/use-decision-queries';
import { useActivePlanRev } from '../stores/use-global-store';
import { useGlobalKPI } from './useGlobalKPI';
import { parseAlertLevel } from '../types/decision';
import type { UrgencyLevel } from '../types/decision';
import type { GlobalKPI } from '../types/kpi';
import { formatNumber } from '../utils/formatters';
import type {
  BottleneckPoint,
  CapacityOpportunity,
  AgeBin,
  ColdStockBucket,
  DaySummary,
  OrderFailure,
  PressureLevel,
  RollCampaignAlert,
} from '../types/decision';

export type ProblemSeverity = 'P0' | 'P1' | 'P2' | 'P3';

export type DrilldownSpec =
  | { kind: 'orders'; urgency?: UrgencyLevel }
  | { kind: 'coldStock'; machineCode?: string; ageBin?: AgeBin; pressureLevel?: PressureLevel }
  | { kind: 'bottleneck'; machineCode?: string; planDate?: string }
  | { kind: 'roll'; machineCode?: string }
  | { kind: 'risk'; planDate?: string }
  | { kind: 'capacityOpportunity'; machineCode?: string; planDate?: string };

export type WorkbenchTabKey = 'visualization' | 'materials' | 'capacity';

export interface RiskProblem {
  id: string;
  severity: ProblemSeverity;
  title: string;
  detail?: string;
  count?: number;
  impact?: string;
  timeHint?: string;
  drilldown: DrilldownSpec;
  workbenchTab?: WorkbenchTabKey;
  workbenchMachineCode?: string | null;
  workbenchPlanDate?: string | null;
  workbenchContext?: string | null;
  workbenchContractNo?: string | null;
}

export interface RiskOverviewData {
  kpi: GlobalKPI | null;
  isLoading: boolean;
  errors: Array<unknown>;
  loadingByKind: Record<
    'kpi' | 'orders' | 'coldStock' | 'bottleneck' | 'roll' | 'risk' | 'capacityOpportunity',
    boolean
  >;
  errorByKind: Partial<
    Record<'kpi' | 'orders' | 'coldStock' | 'bottleneck' | 'roll' | 'risk' | 'capacityOpportunity', unknown>
  >;
  refetchAll: () => void;

  riskDays: DaySummary[];
  bottlenecks: BottleneckPoint[];
  orderFailures: OrderFailure[];
  coldStockBuckets: ColdStockBucket[];
  rollAlerts: RollCampaignAlert[];
  capacityOpportunities: CapacityOpportunity[];

  problems: RiskProblem[];
}

const severityOrder: Record<ProblemSeverity, number> = { P0: 0, P1: 1, P2: 2, P3: 3 };

function formatTonnage(t: number): string {
  const v = Number.isFinite(t) ? t : 0;
  if (Math.abs(v) >= 1000) return `${formatNumber(v / 1000, 3)}千吨`;
  return `${formatNumber(v, 3)}吨`;
}

function getWorstRiskDay(days: DaySummary[]): DaySummary | null {
  if (!days.length) return null;
  const riskRank: Record<DaySummary['riskLevel'], number> = {
    CRITICAL: 3,
    HIGH: 2,
    MEDIUM: 1,
    LOW: 0,
  };

  return days.reduce<DaySummary | null>((best, cur) => {
    if (!best) return cur;
    const bestRank = riskRank[best.riskLevel] ?? 0;
    const curRank = riskRank[cur.riskLevel] ?? 0;
    if (curRank !== bestRank) return curRank > bestRank ? cur : best;
    return cur.riskScore > best.riskScore ? cur : best;
  }, null);
}

function getBottleneckSeverity(points: BottleneckPoint[]): ProblemSeverity | null {
  const hasCritical = points.some((p) => p.bottleneckLevel === 'CRITICAL');
  const hasHigh = points.some((p) => p.bottleneckLevel === 'HIGH');
  if (hasCritical) return 'P0';
  if (hasHigh) return 'P1';
  return null;
}

function getRollSeverity(alerts: RollCampaignAlert[]): ProblemSeverity | null {
  const hasHardStop = alerts.some((a) => parseAlertLevel(String(a.alertLevel || '')) === 'HARD_STOP');
  const hasWarning = alerts.some((a) => parseAlertLevel(String(a.alertLevel || '')) === 'WARNING');
  const hasSuggest = alerts.some((a) => parseAlertLevel(String(a.alertLevel || '')) === 'SUGGEST');
  if (hasHardStop) return 'P0';
  if (hasWarning) return 'P1';
  if (hasSuggest) return 'P2';
  return null;
}

function getRiskSeverity(level: DaySummary['riskLevel'] | null): ProblemSeverity | null {
  if (!level) return null;
  if (level === 'CRITICAL') return 'P0';
  if (level === 'HIGH') return 'P1';
  if (level === 'MEDIUM') return 'P2';
  return null;
}

export function useRiskOverviewData(versionId: string | null): RiskOverviewData {
  const activePlanRev = useActivePlanRev();
  const kpiQuery = useGlobalKPI(versionId);
  const riskQuery = useRecentDaysRisk(versionId, 30);
  const bottleneckQuery = useRecentDaysBottleneck(versionId, 30);
  const ordersQuery = useAllFailedOrders(versionId);
  const coldStockQuery = useColdStockProfile(
    { versionId: versionId || '', expectedPlanRev: activePlanRev ?? undefined },
    { enabled: !!versionId },
  );
  const rollQuery = useAllRollCampaignAlerts(versionId);
  const capacityOpportunityQuery = useRecentDaysCapacityOpportunity(versionId, 30);

  const kpi = kpiQuery.data ?? null;
  const riskDays = riskQuery.data?.items ?? [];
  const bottlenecks = bottleneckQuery.data?.items ?? [];
  const orderFailures = ordersQuery.data?.items ?? [];
  const coldStockBuckets = coldStockQuery.data?.items ?? [];
  const rollAlerts = rollQuery.data?.items ?? [];
  const capacityOpportunities = capacityOpportunityQuery.data?.items ?? [];

  const problems = useMemo<RiskProblem[]>(() => {
    const out: RiskProblem[] = [];

    const l3Failures = orderFailures.filter((o) => o.urgencyLevel === 'L3');
    if (l3Failures.length > 0) {
      const overdue = l3Failures.filter((o) => o.failType === 'Overdue').length;
      const unscheduledT = l3Failures.reduce((sum, o) => sum + Number(o.unscheduledWeightT || 0), 0);
      const minDaysToDue = l3Failures.reduce((min, o) => Math.min(min, Number(o.daysToDue)), Number.POSITIVE_INFINITY);
      const earliestDueDate = [...l3Failures]
        .map((o) => String(o.dueDate || ''))
        .filter(Boolean)
        .sort()[0];
      out.push({
        id: 'l3-order-failures',
        severity: 'P0',
        title: '三级紧急订单未满足',
        count: l3Failures.length,
        detail: earliestDueDate ? `最早交期 ${earliestDueDate}` : undefined,
        impact: [
          overdue > 0 ? `${overdue} 个逾期` : null,
          unscheduledT > 0 ? `未排 ${formatTonnage(unscheduledT)}` : null,
        ].filter(Boolean).join(' · ') || undefined,
        timeHint:
          Number.isFinite(minDaysToDue) && minDaysToDue !== Number.POSITIVE_INFINITY
            ? (minDaysToDue < 0 ? `已逾期 ${Math.abs(minDaysToDue)} 天` : `距交期 ${minDaysToDue} 天`)
            : undefined,
        drilldown: { kind: 'orders', urgency: 'L3' },
        workbenchTab: 'materials',
        workbenchMachineCode: l3Failures[0]?.machineCode ?? null,
        workbenchContext: 'orders',
        workbenchContractNo: l3Failures[0]?.contractNo ?? null,
      });
    }

    const worstRiskDay = getWorstRiskDay(riskDays);
    const worstSeverity = getRiskSeverity(worstRiskDay?.riskLevel ?? null);
    if (worstRiskDay && (worstRiskDay.riskLevel === 'HIGH' || worstRiskDay.riskLevel === 'CRITICAL')) {
      const topReason = worstRiskDay.topReasons?.[0]?.msg;
      const machines = Array.isArray(worstRiskDay.involvedMachines) ? worstRiskDay.involvedMachines : [];
      out.push({
        id: 'worst-risk-day',
        severity: worstSeverity ?? 'P1',
        title: `最高风险日 ${worstRiskDay.planDate}`,
        detail: `风险 ${formatNumber(worstRiskDay.riskScore, 1)} | 利用率 ${formatNumber(worstRiskDay.capacityUtilPct, 1)}% | 超载 ${formatTonnage(worstRiskDay.overloadWeightT)}`,
        impact: topReason ? `首因：${topReason}` : undefined,
        timeHint: machines.length > 0 ? `涉及机组：${machines.slice(0, 3).join(', ')}${machines.length > 3 ? '…' : ''}` : undefined,
        drilldown: { kind: 'risk', planDate: worstRiskDay.planDate },
        workbenchTab: 'capacity',
        workbenchMachineCode: machines.length > 0 ? machines[0] : null,
        workbenchPlanDate: worstRiskDay.planDate,
        workbenchContext: 'risk',
      });
    }

    const severeBottlenecks = bottlenecks.filter(
      (p) => p.bottleneckLevel === 'HIGH' || p.bottleneckLevel === 'CRITICAL'
    );
    const bottleneckSeverity = getBottleneckSeverity(severeBottlenecks);
    if (severeBottlenecks.length > 0 && bottleneckSeverity) {
      const top = [...severeBottlenecks].sort((a, b) => b.bottleneckScore - a.bottleneckScore)[0];
      const topReason = top?.reasons?.[0]?.msg;
      out.push({
        id: 'bottleneck-severe',
        severity: bottleneckSeverity,
        title: '产能/结构堵塞预警',
        count: severeBottlenecks.length,
        detail: top
          ? `${top.machineCode} ${top.planDate} 分数 ${formatNumber(top.bottleneckScore, 0)} | 利用率 ${formatNumber(top.capacityUtilPct, 1)}%`
          : undefined,
        impact: top ? `未排材料 ${top.pendingMaterialCount} 件 · ${formatTonnage(top.pendingWeightT)}` : undefined,
        timeHint: topReason ? `首因：${topReason}` : undefined,
        drilldown: top ? { kind: 'bottleneck', machineCode: top.machineCode, planDate: top.planDate } : { kind: 'bottleneck' },
        workbenchTab: 'capacity',
        workbenchMachineCode: top?.machineCode ?? null,
        workbenchPlanDate: top?.planDate ?? null,
        workbenchContext: 'bottleneck',
      });
    }

    const highPressureCount = coldStockQuery.data?.summary?.highPressureCount ?? 0;
    if (highPressureCount > 0) {
      const severeBuckets = coldStockBuckets.filter(
        (b) => b.pressureLevel === 'HIGH' || b.pressureLevel === 'CRITICAL'
      );
      const top = [...severeBuckets].sort((a, b) => b.pressureScore - a.pressureScore)[0];
      out.push({
        id: 'cold-stock-high-pressure',
        severity: 'P2',
        title: '冷坨高压力积压',
        count: highPressureCount,
        detail: top ? `${top.machineCode} ${top.ageBin} 压力 ${formatNumber(top.pressureScore, 0)} | ${top.count} 件` : undefined,
        impact: top ? `平均库龄 ${formatNumber(top.avgAgeDays, 1)} 天 · 最大 ${top.maxAgeDays} 天` : undefined,
        timeHint: top?.structureGap && top.structureGap !== '无' ? `结构缺口：${top.structureGap}` : undefined,
        drilldown: { kind: 'coldStock' },
        workbenchTab: 'materials',
        workbenchMachineCode: top?.machineCode ?? null,
        workbenchContext: 'coldStock',
      });
    }

    const nearHardStopCount = rollQuery.data?.summary?.nearHardStopCount ?? 0;
    const rollSeverity = nearHardStopCount > 0 ? 'P0' : getRollSeverity(rollAlerts);
    if (rollSeverity) {
      const activeAlerts = rollAlerts.filter((a) => parseAlertLevel(String(a.alertLevel || '')) !== 'NORMAL');
      const top = [...activeAlerts].sort((a, b) => Number(a.remainingTonnageT) - Number(b.remainingTonnageT))[0];
      out.push({
        id: 'roll-campaign-alerts',
        severity: rollSeverity,
        title: '设备监控：换辊',
        count: nearHardStopCount > 0 ? nearHardStopCount : activeAlerts.length,
        detail: top ? `最紧急：${top.machineCode} 剩余 ${formatTonnage(top.remainingTonnageT)}` : undefined,
        impact: top ? `当前 ${formatTonnage(top.currentTonnageT)} / 硬限 ${formatTonnage(top.hardLimitT)}` : undefined,
        timeHint:
          top?.estimatedHardReachAt || top?.estimatedHardStopDate
            ? `预计触达硬限制：${top.estimatedHardReachAt || top.estimatedHardStopDate}`
            : undefined,
        drilldown: { kind: 'roll' },
        workbenchTab: 'visualization',
        workbenchMachineCode: top?.machineCode ?? null,
        workbenchContext: 'roll',
      });
    }

    if (kpi?.blockedUrgentCount && kpi.blockedUrgentCount > 0) {
      out.push({
        id: 'blocked-urgent-materials',
        severity: 'P1',
        title: '紧急物料阻塞',
        count: kpi.blockedUrgentCount,
        detail: '存在紧急物料未排产或被锁定阻塞',
        drilldown: { kind: 'orders' },
        workbenchTab: 'materials',
        workbenchContext: 'orders',
      });
    }

    out.sort((a, b) => severityOrder[a.severity] - severityOrder[b.severity]);
    return out;
  }, [
    orderFailures,
    riskDays,
    bottlenecks,
    coldStockQuery.data?.summary?.highPressureCount,
    rollAlerts,
    rollQuery.data?.summary?.nearHardStopCount,
    kpi?.blockedUrgentCount,
  ]);

  const isLoading =
    kpiQuery.isLoading ||
    riskQuery.isLoading ||
    bottleneckQuery.isLoading ||
    ordersQuery.isLoading ||
    coldStockQuery.isLoading ||
    rollQuery.isLoading ||
    capacityOpportunityQuery.isLoading;

  const errors = [
    kpiQuery.error,
    riskQuery.error,
    bottleneckQuery.error,
    ordersQuery.error,
    coldStockQuery.error,
    rollQuery.error,
    capacityOpportunityQuery.error,
  ].filter(Boolean);

  const loadingByKind = {
    kpi: kpiQuery.isLoading,
    orders: ordersQuery.isLoading,
    coldStock: coldStockQuery.isLoading,
    bottleneck: bottleneckQuery.isLoading,
    roll: rollQuery.isLoading,
    risk: riskQuery.isLoading,
    capacityOpportunity: capacityOpportunityQuery.isLoading,
  } as const;

  const errorByKind: RiskOverviewData['errorByKind'] = {
    kpi: kpiQuery.error,
    orders: ordersQuery.error,
    coldStock: coldStockQuery.error,
    bottleneck: bottleneckQuery.error,
    roll: rollQuery.error,
    risk: riskQuery.error,
    capacityOpportunity: capacityOpportunityQuery.error,
  };

  const refetchAll = () => {
    void kpiQuery.refetch();
    void riskQuery.refetch();
    void bottleneckQuery.refetch();
    void ordersQuery.refetch();
    void coldStockQuery.refetch();
    void rollQuery.refetch();
    void capacityOpportunityQuery.refetch();
  };

  return {
    kpi,
    isLoading,
    errors,
    loadingByKind,
    errorByKind,
    refetchAll,
    riskDays,
    bottlenecks,
    orderFailures,
    coldStockBuckets,
    rollAlerts,
    capacityOpportunities,
    problems,
  };
}

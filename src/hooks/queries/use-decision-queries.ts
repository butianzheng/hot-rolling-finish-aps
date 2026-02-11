// ==========================================
// 决策API TanStack Query Hooks
// ==========================================
// 职责: 封装D1-D6决策API调用为React Query Hooks
// 特性: 自动缓存、后台刷新、错误处理、加载状态
// ==========================================

import { useQuery, UseQueryOptions, UseQueryResult } from '@tanstack/react-query';
import {
  getDecisionDaySummary,
  getRiskSummaryForRecentDays,
  getMachineBottleneckProfile,
  getBottleneckForRecentDays,
  listOrderFailureSet,
  listMaterialFailureSet,
  getAllFailedOrders,
  getAllFailedMaterials,
  getColdStockProfile,
  getHighPressureColdStock,
  getRollCampaignAlert,
  getAllRollCampaignAlerts,
  getCapacityOpportunity,
  getCapacityOpportunityForRecentDays,
  DecisionApiError,
  ValidationError,
} from '../../api/tauri';
import type {
  GetDecisionDaySummaryRequest,
  DecisionDaySummaryResponse,
  GetMachineBottleneckProfileRequest,
  MachineBottleneckProfileResponse,
  ListOrderFailureSetRequest,
  OrderFailureSetResponse,
  ListMaterialFailureSetRequest,
  MaterialFailureSetResponse,
  GetColdStockProfileRequest,
  ColdStockProfileResponse,
  GetRollCampaignAlertRequest,
  RollCampaignAlertResponse,
  GetCapacityOpportunityRequest,
  CapacityOpportunityResponse,
} from '../../types/decision';
import { useActivePlanRev } from '../../stores/use-global-store';

function withExpectedPlanRev<T extends { expectedPlanRev?: number }>(
  request: T,
  expectedPlanRev: number | null,
): T {
  if (!Number.isFinite(Number(expectedPlanRev))) {
    return request;
  }
  return {
    ...request,
    expectedPlanRev: Number(expectedPlanRev),
  };
}

// ==========================================
// Query Keys（用于缓存管理）
// ==========================================

/**
 * 决策API Query Keys
 */
export const decisionQueryKeys = {
  all: ['decision'] as const,

  // D1: 日期风险摘要
  daySummaries: () => [...decisionQueryKeys.all, 'day-summaries'] as const,
  daySummary: (request: GetDecisionDaySummaryRequest) =>
    [...decisionQueryKeys.daySummaries(), request] as const,
  recentDaysRisk: (versionId: string, days: number) =>
    [...decisionQueryKeys.daySummaries(), 'recent', versionId, days] as const,

  // D4: 机组堵塞概况
  bottlenecks: () => [...decisionQueryKeys.all, 'bottlenecks'] as const,
  bottleneck: (request: GetMachineBottleneckProfileRequest) =>
    [...decisionQueryKeys.bottlenecks(), request] as const,
  recentDaysBottleneck: (versionId: string, days: number) =>
    [...decisionQueryKeys.bottlenecks(), 'recent', versionId, days] as const,

  // D2: 订单失败集合
  orderFailures: () => [...decisionQueryKeys.all, 'order-failures'] as const,
  orderFailureSet: (request: ListOrderFailureSetRequest) =>
    [...decisionQueryKeys.orderFailures(), request] as const,
  materialFailureSet: (request: ListMaterialFailureSetRequest) =>
    [...decisionQueryKeys.orderFailures(), 'materials', request] as const,
  allFailedOrders: (versionId: string) =>
    [...decisionQueryKeys.orderFailures(), 'all', versionId] as const,
  allFailedMaterials: (versionId: string) =>
    [...decisionQueryKeys.orderFailures(), 'materials-all', versionId] as const,

  // D3: 冷料压库概况
  coldStocks: () => [...decisionQueryKeys.all, 'cold-stocks'] as const,
  coldStockProfile: (request: GetColdStockProfileRequest) =>
    [...decisionQueryKeys.coldStocks(), request] as const,
  highPressureColdStock: (versionId: string) =>
    [...decisionQueryKeys.coldStocks(), 'high-pressure', versionId] as const,

  // D5: 轧制活动警报
  rollCampaigns: () => [...decisionQueryKeys.all, 'roll-campaigns'] as const,
  rollCampaignAlert: (request: GetRollCampaignAlertRequest) =>
    [...decisionQueryKeys.rollCampaigns(), request] as const,
  allRollCampaignAlerts: (versionId: string) =>
    [...decisionQueryKeys.rollCampaigns(), 'all', versionId] as const,

  // D6: 容量优化机会
  capacityOpportunities: () => [...decisionQueryKeys.all, 'capacity-opportunities'] as const,
  capacityOpportunity: (request: GetCapacityOpportunityRequest) =>
    [...decisionQueryKeys.capacityOpportunities(), request] as const,
  recentDaysCapacityOpportunity: (versionId: string, days: number) =>
    [...decisionQueryKeys.capacityOpportunities(), 'recent', versionId, days] as const,
} as const;

// ==========================================
// 默认查询配置
// ==========================================

/**
 * 决策查询默认配置
 */
const DEFAULT_DECISION_QUERY_OPTIONS = {
  // 数据保持新鲜时间：5分钟（5分钟内不重新请求）
  staleTime: 5 * 60 * 1000,

  // 缓存时间：10分钟（10分钟后清除缓存）
  gcTime: 10 * 60 * 1000,

  // 窗口重新获得焦点时自动刷新
  refetchOnWindowFocus: true,

  // 网络重连时自动刷新
  refetchOnReconnect: true,

  // 失败后重试次数
  retry: 2,

  // 重试延迟（指数退避）
  retryDelay: (attemptIndex: number) => Math.min(1000 * 2 ** attemptIndex, 30000),
} as const;

// ==========================================
// D1: 日期风险摘要 Hooks
// ==========================================

/**
 * 获取日期风险摘要（D1）
 */
export function useDecisionDaySummary(
  request: GetDecisionDaySummaryRequest,
  options?: Omit<
    UseQueryOptions<DecisionDaySummaryResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<DecisionDaySummaryResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();
  const requestWithRev = withExpectedPlanRev(request, activePlanRev);

  return useQuery({
    queryKey: decisionQueryKeys.daySummary(requestWithRev),
    queryFn: () => getDecisionDaySummary(requestWithRev),
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

/**
 * 获取最近N天的风险摘要（简化版）
 */
export function useRecentDaysRisk(
  versionId: string | null,
  days: number = 30,
  options?: Omit<
    UseQueryOptions<DecisionDaySummaryResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<DecisionDaySummaryResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();

  return useQuery({
    queryKey: [...decisionQueryKeys.recentDaysRisk(versionId || '', days), activePlanRev] as const,
    queryFn: () => {
      if (!versionId) {
        throw new DecisionApiError('MISSING_VERSION_ID', '未选择排产版本');
      }
      return getRiskSummaryForRecentDays(versionId, days, activePlanRev ?? undefined);
    },
    enabled: !!versionId, // 只有当versionId存在时才启用查询
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

// ==========================================
// D4: 机组堵塞概况 Hooks
// ==========================================

/**
 * 获取机组堵塞概况（D4）
 */
export function useMachineBottleneckProfile(
  request: GetMachineBottleneckProfileRequest,
  options?: Omit<
    UseQueryOptions<MachineBottleneckProfileResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<MachineBottleneckProfileResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();
  const requestWithRev = withExpectedPlanRev(request, activePlanRev);

  return useQuery({
    queryKey: decisionQueryKeys.bottleneck(requestWithRev),
    queryFn: () => getMachineBottleneckProfile(requestWithRev),
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

/**
 * 获取最近N天的堵塞概况（简化版）
 */
export function useRecentDaysBottleneck(
  versionId: string | null,
  days: number = 30,
  options?: Omit<
    UseQueryOptions<MachineBottleneckProfileResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<MachineBottleneckProfileResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();

  return useQuery({
    queryKey: [...decisionQueryKeys.recentDaysBottleneck(versionId || '', days), activePlanRev] as const,
    queryFn: () => {
      if (!versionId) {
        throw new DecisionApiError('MISSING_VERSION_ID', '未选择排产版本');
      }
      return getBottleneckForRecentDays(versionId, days, activePlanRev ?? undefined);
    },
    enabled: !!versionId,
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

// ==========================================
// D2: 订单失败集合 Hooks
// ==========================================

/**
 * 获取订单失败集合（D2）
 */
export function useOrderFailureSet(
  request: ListOrderFailureSetRequest,
  options?: Omit<
    UseQueryOptions<OrderFailureSetResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<OrderFailureSetResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();
  const requestWithRev = withExpectedPlanRev(request, activePlanRev);

  return useQuery({
    queryKey: decisionQueryKeys.orderFailureSet(requestWithRev),
    queryFn: () => listOrderFailureSet(requestWithRev),
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

/**
 * 获取所有失败订单（简化版）
 */
export function useAllFailedOrders(
  versionId: string | null,
  options?: Omit<
    UseQueryOptions<OrderFailureSetResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<OrderFailureSetResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();

  return useQuery({
    queryKey: [...decisionQueryKeys.allFailedOrders(versionId || ''), activePlanRev] as const,
    queryFn: () => {
      if (!versionId) {
        throw new DecisionApiError('MISSING_VERSION_ID', '未选择排产版本');
      }
      return getAllFailedOrders(versionId, activePlanRev ?? undefined);
    },
    enabled: !!versionId,
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

/**
 * 获取材料失败集合（D2M）
 */
export function useMaterialFailureSet(
  request: ListMaterialFailureSetRequest,
  options?: Omit<
    UseQueryOptions<MaterialFailureSetResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<MaterialFailureSetResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();
  const requestWithRev = withExpectedPlanRev(request, activePlanRev);

  return useQuery({
    queryKey: decisionQueryKeys.materialFailureSet(requestWithRev),
    queryFn: () => listMaterialFailureSet(requestWithRev),
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

/**
 * 获取所有失败材料（简化版）
 */
export function useAllFailedMaterials(
  versionId: string | null,
  options?: Omit<
    UseQueryOptions<MaterialFailureSetResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<MaterialFailureSetResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();

  return useQuery({
    queryKey: [...decisionQueryKeys.allFailedMaterials(versionId || ''), activePlanRev] as const,
    queryFn: () => {
      if (!versionId) {
        throw new DecisionApiError('MISSING_VERSION_ID', '未选择排产版本');
      }
      return getAllFailedMaterials(versionId, activePlanRev ?? undefined);
    },
    enabled: !!versionId,
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

// ==========================================
// D3: 冷料压库概况 Hooks
// ==========================================

/**
 * 获取冷料压库概况（D3）
 */
export function useColdStockProfile(
  request: GetColdStockProfileRequest,
  options?: Omit<
    UseQueryOptions<ColdStockProfileResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<ColdStockProfileResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();
  const requestWithRev = withExpectedPlanRev(request, activePlanRev);

  return useQuery({
    queryKey: decisionQueryKeys.coldStockProfile(requestWithRev),
    queryFn: () => getColdStockProfile(requestWithRev),
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

/**
 * 获取高压力冷料（简化版）
 */
export function useHighPressureColdStock(
  versionId: string | null,
  options?: Omit<
    UseQueryOptions<ColdStockProfileResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<ColdStockProfileResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();

  return useQuery({
    queryKey: [...decisionQueryKeys.highPressureColdStock(versionId || ''), activePlanRev] as const,
    queryFn: () => {
      if (!versionId) {
        throw new DecisionApiError('MISSING_VERSION_ID', '未选择排产版本');
      }
      return getHighPressureColdStock(versionId, activePlanRev ?? undefined);
    },
    enabled: !!versionId,
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

// ==========================================
// D5: 轧制活动警报 Hooks
// ==========================================

/**
 * 获取轧制活动警报（D5）
 */
export function useRollCampaignAlert(
  request: GetRollCampaignAlertRequest,
  options?: Omit<
    UseQueryOptions<RollCampaignAlertResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<RollCampaignAlertResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();
  const requestWithRev = withExpectedPlanRev(request, activePlanRev);

  return useQuery({
    queryKey: decisionQueryKeys.rollCampaignAlert(requestWithRev),
    queryFn: () => getRollCampaignAlert(requestWithRev),
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

/**
 * 获取所有机组的换辊警报（简化版）
 */
export function useAllRollCampaignAlerts(
  versionId: string | null,
  options?: Omit<
    UseQueryOptions<RollCampaignAlertResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<RollCampaignAlertResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();

  return useQuery({
    queryKey: [...decisionQueryKeys.allRollCampaignAlerts(versionId || ''), activePlanRev] as const,
    queryFn: () => {
      if (!versionId) {
        throw new DecisionApiError('MISSING_VERSION_ID', '未选择排产版本');
      }
      return getAllRollCampaignAlerts(versionId, activePlanRev ?? undefined);
    },
    enabled: !!versionId,
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

// ==========================================
// D6: 容量优化机会 Hooks
// ==========================================

/**
 * 获取容量优化机会（D6）
 */
export function useCapacityOpportunity(
  request: GetCapacityOpportunityRequest,
  options?: Omit<
    UseQueryOptions<CapacityOpportunityResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<CapacityOpportunityResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();
  const requestWithRev = withExpectedPlanRev(request, activePlanRev);

  return useQuery({
    queryKey: decisionQueryKeys.capacityOpportunity(requestWithRev),
    queryFn: () => getCapacityOpportunity(requestWithRev),
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

/**
 * 获取最近N天的容量优化机会（简化版）
 */
export function useRecentDaysCapacityOpportunity(
  versionId: string | null,
  days: number = 30,
  options?: Omit<
    UseQueryOptions<CapacityOpportunityResponse, DecisionApiError | ValidationError>,
    'queryKey' | 'queryFn'
  >
): UseQueryResult<CapacityOpportunityResponse, DecisionApiError | ValidationError> {
  const activePlanRev = useActivePlanRev();

  return useQuery({
    queryKey: [...decisionQueryKeys.recentDaysCapacityOpportunity(versionId || '', days), activePlanRev] as const,
    queryFn: () => {
      if (!versionId) {
        throw new DecisionApiError('MISSING_VERSION_ID', '未选择排产版本');
      }
      return getCapacityOpportunityForRecentDays(versionId, days, activePlanRev ?? undefined);
    },
    enabled: !!versionId,
    ...DEFAULT_DECISION_QUERY_OPTIONS,
    ...options,
  });
}

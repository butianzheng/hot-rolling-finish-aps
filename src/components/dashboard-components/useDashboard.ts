/**
 * Dashboard 状态管理 Hook
 */

import { useState, useCallback } from 'react';
import { message } from 'antd';
import { decisionService } from '../../api/tauri';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import { useActiveVersionId } from '../../stores/use-global-store';
import type {
  OrderFailureRow,
  OrderFailureSetResponse,
  ColdStockBucketRow,
  ColdStockProfileResponse,
  BottleneckPointRow,
  MachineBottleneckProfileResponse,
} from './types';

export interface UseDashboardReturn {
  // 版本
  activeVersionId: string | null;

  // 加载状态
  loading: boolean;

  // 订单失败数据
  orderFailures: OrderFailureRow[];
  orderFailureSummary: OrderFailureSetResponse['summary'];

  // 冷料数据
  coldStockBuckets: ColdStockBucketRow[];
  coldStockSummary: ColdStockProfileResponse['summary'];

  // 瓶颈点
  mostCongestedPoint: BottleneckPointRow | null;

  // 自动刷新配置
  autoRefreshEnabled: boolean;
  setAutoRefreshEnabled: (enabled: boolean) => void;
  refreshInterval: number;
  setRefreshInterval: (interval: number) => void;

  // 刷新状态
  lastRefreshTime: Date | null;
  nextRefreshCountdown: number;
  manualRefresh: () => void;

  // 数据加载
  loadDashboardData: () => Promise<void>;
}

export function useDashboard(): UseDashboardReturn {
  const activeVersionId = useActiveVersionId();
  const [loading, setLoading] = useState(false);
  const [orderFailures, setOrderFailures] = useState<OrderFailureRow[]>([]);
  const [orderFailureSummary, setOrderFailureSummary] = useState<OrderFailureSetResponse['summary']>(undefined);
  const [coldStockBuckets, setColdStockBuckets] = useState<ColdStockBucketRow[]>([]);
  const [coldStockSummary, setColdStockSummary] = useState<ColdStockProfileResponse['summary']>(undefined);
  const [mostCongestedPoint, setMostCongestedPoint] = useState<BottleneckPointRow | null>(null);

  // 自动刷新配置
  const [autoRefreshEnabled, setAutoRefreshEnabled] = useState(true);
  const [refreshInterval, setRefreshInterval] = useState(30000); // 默认 30 秒

  // 加载Dashboard数据
  const loadDashboardData = useCallback(async () => {
    if (!activeVersionId) {
      return; // 没有活动版本时不加载数据
    }
    setLoading(true);
    try {
      // 加载未满足的紧急单
      const orderFailureSet = (await decisionService.getUnsatisfiedUrgentMaterials(
        activeVersionId
      )) as OrderFailureSetResponse;
      setOrderFailures(orderFailureSet?.items || []);
      setOrderFailureSummary(orderFailureSet?.summary);

      // 加载冷料（库存超过30天）
      const coldStockProfile = (await decisionService.getColdStockMaterials(
        activeVersionId,
        30
      )) as ColdStockProfileResponse;
      setColdStockBuckets(coldStockProfile?.items || []);
      setColdStockSummary(coldStockProfile?.summary);

      // 加载最拥堵机组
      const bottleneckProfile = (await decisionService.getMostCongestedMachine(
        activeVersionId
      )) as MachineBottleneckProfileResponse;
      const points = bottleneckProfile?.items || [];
      const most = points.reduce<BottleneckPointRow | null>((max, p) => {
        if (!max) return p;
        return p.bottleneckScore > max.bottleneckScore ? p : max;
      }, null);
      setMostCongestedPoint(most);
    } catch (error: any) {
      message.error(`加载失败: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  }, [activeVersionId]);

  // 使用自动刷新 Hook
  const { lastRefreshTime, nextRefreshCountdown, refresh: manualRefresh } = useAutoRefresh(
    loadDashboardData,
    refreshInterval,
    autoRefreshEnabled
  );

  return {
    activeVersionId,
    loading,
    orderFailures,
    orderFailureSummary,
    coldStockBuckets,
    coldStockSummary,
    mostCongestedPoint,
    autoRefreshEnabled,
    setAutoRefreshEnabled,
    refreshInterval,
    setRefreshInterval,
    lastRefreshTime,
    nextRefreshCountdown,
    manualRefresh,
    loadDashboardData,
  };
}

export default useDashboard;

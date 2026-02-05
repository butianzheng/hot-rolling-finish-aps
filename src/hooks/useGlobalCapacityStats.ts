/**
 * 全局产能统计 Hook
 * 聚合多个机组的产能数据，计算总体统计信息
 */

import { useMemo } from 'react';
import { useQueries } from '@tanstack/react-query';
import { capacityApi } from '../api/tauri';
import type { CapacityPool } from '../api/ipcSchemas';

export interface GlobalStats {
  totalTarget: number;
  totalUsed: number;
  totalRemaining: number;
  avgUtilization: number;
  overLimitCount: number;
}

/**
 * 全局产能统计 Hook
 * 聚合多个机组的产能池数据
 */
export function useGlobalCapacityStats(
  versionId: string,
  machineCodes: string[],
  dateFrom: string,
  dateTo: string
): {
  globalStats: GlobalStats;
  loading: boolean;
  error: boolean;
} {
  // 为每个机组并行查询产能池数据
  const queries = useQueries({
    queries: machineCodes.map((machineCode) => ({
      queryKey: ['capacityPool', versionId, machineCode, dateFrom, dateTo],
      queryFn: async () => {
        try {
          const data = await capacityApi.getCapacityPools(
            [machineCode],
            dateFrom,
            dateTo,
            versionId
          );
          return data || [];
        } catch (e) {
          console.error(`[GlobalStats] Failed to load data for ${machineCode}:`, e);
          return [];
        }
      },
      enabled: !!versionId && !!machineCode && !!dateFrom && !!dateTo,
      staleTime: 60 * 1000, // 60秒缓存
    })),
  });

  // 判断加载状态
  const loading = queries.some((q) => q.isLoading);
  const error = queries.some((q) => q.isError);

  // 聚合所有机组的数据
  const allPools = useMemo(() => {
    const pools: CapacityPool[] = [];
    queries.forEach((query) => {
      if (query.data) {
        pools.push(...query.data);
      }
    });
    return pools;
  }, [queries]);

  // 计算全局统计
  const globalStats = useMemo((): GlobalStats => {
    if (allPools.length === 0) {
      return {
        totalTarget: 0,
        totalUsed: 0,
        totalRemaining: 0,
        avgUtilization: 0,
        overLimitCount: 0,
      };
    }

    const totalTarget = allPools.reduce((sum, p) => sum + p.target_capacity_t, 0);
    const totalUsed = allPools.reduce((sum, p) => sum + p.used_capacity_t, 0);
    const totalRemaining = Math.max(0, totalTarget - totalUsed);
    const avgUtilization = totalTarget > 0 ? totalUsed / totalTarget : 0;

    // 统计超限天数
    const overLimitCount = allPools.filter((p) => p.used_capacity_t > p.limit_capacity_t).length;

    return {
      totalTarget,
      totalUsed,
      totalRemaining,
      avgUtilization,
      overLimitCount,
    };
  }, [allPools]);

  return {
    globalStats,
    loading,
    error,
  };
}

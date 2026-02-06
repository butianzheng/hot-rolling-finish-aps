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

// C7修复：添加错误信息返回类型
export interface GlobalStatsError {
  machineCode: string;
  error: string;
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
  failedMachines: GlobalStatsError[]; // C7修复：添加失败机组列表
} {
  // 为每个机组并行查询产能池数据
  const queries = useQueries({
    queries: machineCodes.map((machineCode) => ({
      queryKey: ['globalCapacityStats', versionId, machineCode, dateFrom, dateTo],
      queryFn: async () => {
        // C7修复：移除try-catch让错误正常传播，React Query会正确设置isError状态
        const data = await capacityApi.getCapacityPools(
          [machineCode],
          dateFrom,
          dateTo,
          versionId
        );
        return data || [];
      },
      enabled: !!versionId && !!machineCode && !!dateFrom && !!dateTo,
      staleTime: 60 * 1000, // 60秒缓存
      retry: 1, // C7修复：失败后重试1次
    })),
  });

  // 判断加载状态
  const loading = queries.some((q) => q.isLoading);
  const error = queries.some((q) => q.isError);

  // C7修复：收集失败的机组信息
  const failedMachines = useMemo(() => {
    const failed: GlobalStatsError[] = [];
    queries.forEach((query, index) => {
      if (query.isError && query.error) {
        const machineCode = machineCodes[index];
        const errorMsg = query.error instanceof Error
          ? query.error.message
          : String(query.error);
        failed.push({
          machineCode,
          error: errorMsg,
        });
        // C7修复：在控制台输出详细错误信息
        console.error(`[GlobalStats] Failed to load data for ${machineCode}:`, query.error);
      }
    });
    return failed;
  }, [queries, machineCodes]);

  // 聚合所有机组的数据（C7修复：仅包含成功加载的数据）
  const allPools = useMemo(() => {
    const pools: CapacityPool[] = [];
    queries.forEach((query) => {
      if (query.data && !query.isError) {
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
    failedMachines, // C7修复：返回失败的机组列表
  };
}

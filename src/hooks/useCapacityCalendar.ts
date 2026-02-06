/**
 * 产能池日历数据管理 Hook
 * 支持分批加载、颜色映射和统计信息计算
 */

import { useCallback, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { message } from 'antd';
import { capacityApi } from '../api/tauri';
import type { CapacityPoolCalendarData } from '../api/ipcSchemas/machineConfigSchemas';
import type { CapacityPool } from '../api/ipcSchemas';

/**
 * 颜色映射：基于产能利用率（参考扁平化设计）
 * - 绿色 (0-70%): 充裕
 * - 蓝色 (70-85%): 适中
 * - 橙色 (85-100%): 紧张
 * - 红色 (>100%): 超限
 * - 灰色: 无数据
 */
const CAPACITY_COLORS = {
  充裕: '#27ae60',   // emerald green
  适中: '#3498db',   // peter river blue
  紧张: '#f39c12',   // orange
  超限: '#e74c3c',   // alizarin red
  无数据: '#ecf0f1', // clouds (light gray)
} as const;

/**
 * 根据利用率计算颜色
 */
function getColorByUtilization(utilizationPct: number): string {
  if (utilizationPct === 0) return CAPACITY_COLORS.无数据;
  if (utilizationPct < 0.7) return CAPACITY_COLORS.充裕;
  if (utilizationPct < 0.85) return CAPACITY_COLORS.适中;
  if (utilizationPct <= 1.0) return CAPACITY_COLORS.紧张;
  return CAPACITY_COLORS.超限;
}

/**
 * 将 CapacityPool 转换为 CapacityPoolCalendarData
 */
function transformToCalendarData(pool: CapacityPool): CapacityPoolCalendarData {
  const utilization =
    pool.target_capacity_t > 0 ? pool.used_capacity_t / pool.target_capacity_t : 0;

  return {
    plan_date: pool.plan_date,
    machine_code: pool.machine_code,
    target_capacity_t: pool.target_capacity_t,
    used_capacity_t: pool.used_capacity_t,
    limit_capacity_t: pool.limit_capacity_t,
    utilization_pct: utilization,
    color: getColorByUtilization(utilization),
  };
}

/**
 * 计算日期范围的天数
 */
function calculateDaysDiff(dateFrom: string, dateTo: string): number {
  const from = new Date(dateFrom);
  const to = new Date(dateTo);
  return Math.ceil((to.getTime() - from.getTime()) / (1000 * 60 * 60 * 24)) + 1;
}

/**
 * 分割日期范围为多个批次（每批最多90天）
 */
function splitDateRange(
  dateFrom: string,
  dateTo: string,
  maxDays: number = 90
): Array<{ dateFrom: string; dateTo: string }> {
  const from = new Date(dateFrom);
  const to = new Date(dateTo);
  const batches: Array<{ dateFrom: string; dateTo: string }> = [];

  let current = from;
  while (current <= to) {
    const batchEnd = new Date(current);
    batchEnd.setDate(batchEnd.getDate() + maxDays - 1);

    const actualEnd = batchEnd > to ? to : batchEnd;

    batches.push({
      dateFrom: current.toISOString().split('T')[0],
      dateTo: actualEnd.toISOString().split('T')[0],
    });

    current = new Date(actualEnd);
    current.setDate(current.getDate() + 1);
  }

  return batches;
}

/**
 * useCapacityCalendar Hook 返回接口
 */
export interface UseCapacityCalendarReturn {
  // 日历数据
  calendarData: CapacityPoolCalendarData[];
  calendarLoading: boolean;
  calendarError: Error | null;
  refetchCalendar: () => Promise<void>;

  // 统计信息
  statistics: {
    totalTarget: number;
    totalUsed: number;
    totalRemaining: number;
    avgUtilization: number;
    overLimitCount: number;
    充裕Count: number;
    适中Count: number;
    紧张Count: number;
    超限Count: number;
  };

  // 按日期查询
  getDataByDate: (date: string) => CapacityPoolCalendarData | undefined;

  // 按日期范围查询
  getDataByDateRange: (dateFrom: string, dateTo: string) => CapacityPoolCalendarData[];
}

/**
 * 产能池日历 Hook
 * 支持分批加载和性能优化
 */
export function useCapacityCalendar(
  versionId: string,
  machineCode: string,
  dateFrom: string,
  dateTo: string
): UseCapacityCalendarReturn {
  // ========== 查询 - 产能池数据 ==========
  const {
    data: rawCapacityPools = [],
    isLoading: calendarLoading,
    error: calendarError,
    refetch: refetchCalendarQuery,
  } = useQuery({
    queryKey: ['capacityCalendar', versionId, machineCode, dateFrom, dateTo],
    queryFn: async () => {
      try {
        // 计算日期范围
        const days = calculateDaysDiff(dateFrom, dateTo);

        // 如果日期范围超过90天，分批加载
        if (days > 90) {
          const batches = splitDateRange(dateFrom, dateTo, 90);
          const allData: CapacityPool[] = [];

          for (const batch of batches) {
            const batchData = await capacityApi.getCapacityPools(
              [machineCode],
              batch.dateFrom,
              batch.dateTo,
              versionId
            );
            allData.push(...batchData);
          }

          return allData;
        }

        // 单次加载（≤90天）
        const data = await capacityApi.getCapacityPools(
          [machineCode],
          dateFrom,
          dateTo,
          versionId
        );
        return data || [];
      } catch (e: any) {
        console.error('【产能日历】查询失败：', e);
        message.error(e?.message || '加载产能日历数据失败');
        throw e;
      }
    },
    enabled: !!versionId && !!machineCode && !!dateFrom && !!dateTo,
    staleTime: 60 * 1000, // 60秒内不重新查询
  });

  // ========== 数据转换 - 转换为日历数据 ==========
  const calendarData = useMemo(() => {
    return rawCapacityPools.map(transformToCalendarData);
  }, [rawCapacityPools]);

  // ========== 统计信息计算 ==========
  const statistics = useMemo(() => {
    const totalTarget = calendarData.reduce((sum, d) => sum + d.target_capacity_t, 0);
    const totalUsed = calendarData.reduce((sum, d) => sum + d.used_capacity_t, 0);
    const totalRemaining = Math.max(0, totalTarget - totalUsed);
    const avgUtilization = totalTarget > 0 ? totalUsed / totalTarget : 0;

    // 按颜色分类统计
    const 充裕Count = calendarData.filter((d) => d.color === CAPACITY_COLORS.充裕).length;
    const 适中Count = calendarData.filter((d) => d.color === CAPACITY_COLORS.适中).length;
    const 紧张Count = calendarData.filter((d) => d.color === CAPACITY_COLORS.紧张).length;
    const 超限Count = calendarData.filter((d) => d.color === CAPACITY_COLORS.超限).length;
    const overLimitCount = calendarData.filter((d) => d.used_capacity_t > d.limit_capacity_t).length;

    return {
      totalTarget,
      totalUsed,
      totalRemaining,
      avgUtilization,
      overLimitCount,
      充裕Count,
      适中Count,
      紧张Count,
      超限Count,
    };
  }, [calendarData]);

  // ========== 业务逻辑 ==========
  const getDataByDate = useCallback(
    (date: string): CapacityPoolCalendarData | undefined => {
      return calendarData.find((d) => d.plan_date === date);
    },
    [calendarData]
  );

  const getDataByDateRange = useCallback(
    (from: string, to: string): CapacityPoolCalendarData[] => {
      return calendarData.filter((d) => d.plan_date >= from && d.plan_date <= to);
    },
    [calendarData]
  );

  const refetchCalendar = useCallback(async () => {
    await refetchCalendarQuery();
  }, [refetchCalendarQuery]);

  return {
    // 日历数据
    calendarData,
    calendarLoading,
    calendarError: calendarError instanceof Error ? calendarError : null,
    refetchCalendar,

    // 统计信息
    statistics,

    // 查询方法
    getDataByDate,
    getDataByDateRange,
  };
}

// 导出颜色常量供组件使用
export { CAPACITY_COLORS };

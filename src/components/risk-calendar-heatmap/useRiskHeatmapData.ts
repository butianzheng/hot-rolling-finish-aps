/**
 * 风险热力图数据处理 Hook
 */

import { useMemo } from 'react';
import type { DaySummary } from '../../types/decision';
import type { HeatmapDataResult } from './types';

export function useRiskHeatmapData(data: DaySummary[]): HeatmapDataResult {
  return useMemo(() => {
    if (!data || data.length === 0) {
      return {
        heatmapData: [],
        dateRange: ['', ''] as [string, string],
        maxRiskScore: 100,
      };
    }

    // 转换为ECharts热力图数据格式: [日期, 风险分数]
    const heatmapData = data.map((item) => [item.planDate, item.riskScore] as [string, number]);

    // 计算日期范围
    const dates = data.map((item) => item.planDate).sort();
    const dateRange: [string, string] = [dates[0], dates[dates.length - 1]];

    // 计算最大风险分数（用于颜色映射）
    const maxRiskScore = Math.max(...data.map((item) => item.riskScore), 100);

    return { heatmapData, dateRange, maxRiskScore };
  }, [data]);
}

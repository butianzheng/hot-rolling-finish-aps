/**
 * 热力图数据处理 Hook
 */

import { useMemo } from 'react';
import type { BottleneckPoint } from '../../types/decision';
import type { HeatmapDataResult } from './types';

export function useHeatmapData(data: BottleneckPoint[]): HeatmapDataResult {
  return useMemo(() => {
    if (!data || data.length === 0) {
      return {
        heatmapData: [],
        machines: [],
        dates: [],
        maxBottleneckScore: 100,
      };
    }

    // 提取唯一的机组和日期
    const machineSet = new Set<string>();
    const dateSet = new Set<string>();

    data.forEach((item) => {
      machineSet.add(item.machineCode);
      dateSet.add(item.planDate);
    });

    const machines = Array.from(machineSet).sort();
    const dates = Array.from(dateSet).sort();

    // 转换为ECharts热力图数据格式: [日期索引, 机组索引, 堵塞分数]
    const heatmapData = data.map((item) => {
      const dateIndex = dates.indexOf(item.planDate);
      const machineIndex = machines.indexOf(item.machineCode);
      return [dateIndex, machineIndex, item.bottleneckScore] as [number, number, number];
    });

    // 计算最大堵塞分数（用于颜色映射）
    const maxBottleneckScore = Math.max(...data.map((item) => item.bottleneckScore), 100);

    return { heatmapData, machines, dates, maxBottleneckScore };
  }, [data]);
}

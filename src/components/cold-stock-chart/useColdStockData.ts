/**
 * 冷料库龄数据处理 Hook
 */

import { useMemo } from 'react';
import type { ColdStockBucket, AgeBin } from '../../types/decision';
import type { ChartDataResult } from './types';
import { AGE_BIN_ORDER } from './types';

export function useColdStockData(
  data: ColdStockBucket[],
  displayMode: 'count' | 'weight'
): ChartDataResult {
  return useMemo(() => {
    if (!data || data.length === 0) {
      return {
        machines: [],
        seriesData: {
          '0-7': [],
          '8-14': [],
          '15-30': [],
          '30+': [],
        },
      };
    }

    // 提取唯一机组并排序
    const machineSet = new Set<string>();
    data.forEach((bucket) => machineSet.add(bucket.machineCode));
    const machines = Array.from(machineSet).sort();

    // 为每个库龄区间创建系列数据
    const seriesData: Record<AgeBin, number[]> = {
      '0-7': [],
      '8-14': [],
      '15-30': [],
      '30+': [],
    };

    // 填充数据
    machines.forEach((machine) => {
      AGE_BIN_ORDER.forEach((ageBin) => {
        const bucket = data.find(
          (b) => b.machineCode === machine && b.ageBin === ageBin
        );
        const value = bucket
          ? displayMode === 'count'
            ? bucket.count
            : bucket.weightT
          : 0;
        seriesData[ageBin].push(value);
      });
    });

    return { machines, seriesData };
  }, [data, displayMode]);
}

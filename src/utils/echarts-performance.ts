// ==========================================
// ECharts 性能优化工具
// ==========================================
// 职责: 提供ECharts性能优化配置和工具函数
// ==========================================

import type { EChartsOption } from 'echarts';

// ==========================================
// 性能优化配置常量
// ==========================================

/**
 * 大数据集阈值
 * 超过此阈值时启用性能优化策略
 */
export const PERFORMANCE_THRESHOLDS = {
  /** 数据点数量阈值（超过此值启用采样） */
  DATA_POINTS: 1000,
  /** 时间序列阈值（超过此值启用渐进式渲染） */
  TIME_SERIES: 500,
  /** 热力图单元格阈值 */
  HEATMAP_CELLS: 500,
} as const;

/**
 * 性能优化配置
 */
export const PERFORMANCE_CONFIG = {
  /** 采样率（保留数据比例） */
  SAMPLING_RATE: 0.5,
  /** 渐进式渲染延迟（毫秒） */
  PROGRESSIVE_DELAY: 300,
  /** 动画禁用阈值 */
  ANIMATION_THRESHOLD: 1000,
} as const;

// ==========================================
// 性能优化工具函数
// ==========================================

/**
 * 获取优化的ECharts基础配置
 *
 * @param dataSize - 数据集大小
 * @returns 优化后的基础配置
 */
export function getOptimizedBaseConfig(dataSize: number): Partial<EChartsOption> {
  const shouldOptimize = dataSize > PERFORMANCE_THRESHOLDS.DATA_POINTS;

  return {
    // 动画配置
    animation: !shouldOptimize || dataSize < PERFORMANCE_CONFIG.ANIMATION_THRESHOLD,
    animationDuration: shouldOptimize ? 0 : 1000,
    animationEasing: 'cubicOut',

    // 渐进式渲染（大数据集时启用）
    ...(shouldOptimize && {
      progressive: Math.floor(dataSize / 10),
      progressiveThreshold: PERFORMANCE_THRESHOLDS.DATA_POINTS,
      progressiveChunkMode: 'sequential' as const,
    }),

    // 懒加载和按需渲染
    lazyUpdate: true,
    silent: shouldOptimize, // 大数据集时禁用交互事件
  };
}

/**
 * 数据采样（用于大数据集降采样）
 *
 * @param data - 原始数据数组
 * @param maxPoints - 最大保留点数
 * @returns 采样后的数据
 */
export function sampleData<T>(data: T[], maxPoints: number): T[] {
  if (data.length <= maxPoints) {
    return data;
  }

  const step = Math.ceil(data.length / maxPoints);
  const sampled: T[] = [];

  for (let i = 0; i < data.length; i += step) {
    sampled.push(data[i]);
  }

  // 确保保留最后一个数据点
  if (sampled[sampled.length - 1] !== data[data.length - 1]) {
    sampled.push(data[data.length - 1]);
  }

  return sampled;
}

/**
 * 智能数据采样（保留重要点）
 *
 * 使用Largest-Triangle-Three-Buckets (LTTB) 算法的简化版本
 * 优先保留数据的极值点和转折点
 *
 * @param data - 原始数据 [x, y][]
 * @param maxPoints - 最大保留点数
 * @returns 采样后的数据
 */
export function smartSampleData(
  data: [number, number][],
  maxPoints: number
): [number, number][] {
  if (data.length <= maxPoints) {
    return data;
  }

  const bucketSize = (data.length - 2) / (maxPoints - 2);
  const sampled: [number, number][] = [data[0]]; // 始终保留第一个点

  for (let i = 0; i < maxPoints - 2; i++) {
    const bucketStart = Math.floor(i * bucketSize) + 1;
    const bucketEnd = Math.floor((i + 1) * bucketSize) + 1;

    let maxValue = -Infinity;
    let maxIndex = bucketStart;

    // 在桶内找到最大值点
    for (let j = bucketStart; j < bucketEnd && j < data.length; j++) {
      if (data[j][1] > maxValue) {
        maxValue = data[j][1];
        maxIndex = j;
      }
    }

    sampled.push(data[maxIndex]);
  }

  sampled.push(data[data.length - 1]); // 始终保留最后一个点
  return sampled;
}

/**
 * 获取优化的热力图配置
 *
 * @param cellCount - 单元格数量
 * @returns 优化后的热力图配置
 */
export function getOptimizedHeatmapConfig(cellCount: number): Record<string, any> {
  const shouldOptimize = cellCount > PERFORMANCE_THRESHOLDS.HEATMAP_CELLS;

  return {
    ...getOptimizedBaseConfig(cellCount),
    // 返回简化配置，由调用方合并到series中
    _optimizationHints: {
      shouldOptimize,
      showLabels: !shouldOptimize,
    },
  };
}

/**
 * 获取优化的折线图配置
 *
 * @param pointCount - 数据点数量
 * @returns 优化后的折线图配置
 */
export function getOptimizedLineConfig(pointCount: number): Record<string, any> {
  const shouldOptimize = pointCount > PERFORMANCE_THRESHOLDS.TIME_SERIES;

  return {
    ...getOptimizedBaseConfig(pointCount),
    _optimizationHints: {
      shouldOptimize,
      sampling: shouldOptimize ? 'lttb' : undefined,
      smooth: !shouldOptimize,
      symbol: shouldOptimize ? 'none' : 'circle',
      symbolSize: shouldOptimize ? 0 : 4,
    },
  };
}

/**
 * 获取优化的柱状图配置
 *
 * @param barCount - 柱子数量
 * @returns 优化后的柱状图配置
 */
export function getOptimizedBarConfig(barCount: number): Record<string, any> {
  const shouldOptimize = barCount > PERFORMANCE_THRESHOLDS.DATA_POINTS;

  return {
    ...getOptimizedBaseConfig(barCount),
    _optimizationHints: {
      shouldOptimize,
      showLabels: !shouldOptimize,
    },
  };
}

// ==========================================
// 性能监控工具
// ==========================================

/**
 * ECharts性能监控器
 */
export class EChartsPerformanceMonitor {
  private renderTimes: number[] = [];
  private maxSamples = 10;

  /**
   * 记录渲染时间
   */
  recordRenderTime(time: number): void {
    this.renderTimes.push(time);
    if (this.renderTimes.length > this.maxSamples) {
      this.renderTimes.shift();
    }
  }

  /**
   * 获取平均渲染时间
   */
  getAverageRenderTime(): number {
    if (this.renderTimes.length === 0) return 0;
    const sum = this.renderTimes.reduce((a, b) => a + b, 0);
    return sum / this.renderTimes.length;
  }

  /**
   * 获取最大渲染时间
   */
  getMaxRenderTime(): number {
    if (this.renderTimes.length === 0) return 0;
    return Math.max(...this.renderTimes);
  }

  /**
   * 重置统计
   */
  reset(): void {
    this.renderTimes = [];
  }

  /**
   * 生成性能报告
   */
  getReport(): {
    avgRenderTime: number;
    maxRenderTime: number;
    sampleCount: number;
  } {
    return {
      avgRenderTime: this.getAverageRenderTime(),
      maxRenderTime: this.getMaxRenderTime(),
      sampleCount: this.renderTimes.length,
    };
  }
}

/**
 * 创建性能监控实例
 */
export function createPerformanceMonitor(): EChartsPerformanceMonitor {
  return new EChartsPerformanceMonitor();
}

// ==========================================
// 数据优化工具
// ==========================================

/**
 * 合并ECharts配置（深度合并）
 *
 * @param base - 基础配置
 * @param override - 覆盖配置
 * @returns 合并后的配置
 */
export function mergeEChartsConfig(
  base: Record<string, any>,
  override: Record<string, any>
): Record<string, any> {
  return {
    ...base,
    ...override,
    // 深度合并series（如果都是数组）
    series: Array.isArray(base.series) && Array.isArray(override.series)
      ? base.series.map((baseSeries: any, index: number) => ({
          ...baseSeries,
          ...(override.series[index] || {}),
        }))
      : override.series || base.series,
  };
}

/**
 * 数据预处理：移除重复点
 *
 * @param data - 原始数据 [x, y][]
 * @param tolerance - 容差（x轴距离小于此值视为重复）
 * @returns 去重后的数据
 */
export function removeDuplicatePoints(
  data: [number, number][],
  tolerance: number = 0.001
): [number, number][] {
  if (data.length <= 1) return data;

  const deduplicated: [number, number][] = [data[0]];

  for (let i = 1; i < data.length; i++) {
    const prev = deduplicated[deduplicated.length - 1];
    const curr = data[i];

    if (Math.abs(curr[0] - prev[0]) > tolerance) {
      deduplicated.push(curr);
    }
  }

  return deduplicated;
}

// ==========================================
// 默认导出
// ==========================================

export default {
  getOptimizedBaseConfig,
  getOptimizedHeatmapConfig,
  getOptimizedLineConfig,
  getOptimizedBarConfig,
  sampleData,
  smartSampleData,
  removeDuplicatePoints,
  mergeEChartsConfig,
  createPerformanceMonitor,
  PERFORMANCE_THRESHOLDS,
  PERFORMANCE_CONFIG,
};

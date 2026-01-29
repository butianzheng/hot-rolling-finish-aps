/**
 * BottleneckHeatmap 类型和常量定义
 */

import type { BottleneckLevel, BottleneckPoint } from '../../types/decision';

// ==========================================
// Props 定义
// ==========================================

export interface BottleneckHeatmapProps {
  data: BottleneckPoint[];
  onPointClick?: (machine: string, date: string) => void;
  selectedPoint?: { machine: string; date: string } | null;
  height?: number;
}

// ==========================================
// 内部类型
// ==========================================

export interface HeatmapDataResult {
  heatmapData: [number, number, number][];
  machines: string[];
  dates: string[];
  maxBottleneckScore: number;
}

// ==========================================
// 常量
// ==========================================

export const BOTTLENECK_LEVEL_COLORS: Record<BottleneckLevel, string> = {
  NONE: '#d9d9d9',
  LOW: '#52c41a',
  MEDIUM: '#1677ff',
  HIGH: '#faad14',
  CRITICAL: '#ff4d4f',
};

/**
 * 根据堵塞分数获取颜色
 */
export function getBottleneckColor(score: number): string {
  if (score < 25) return BOTTLENECK_LEVEL_COLORS.LOW;
  if (score < 50) return BOTTLENECK_LEVEL_COLORS.MEDIUM;
  if (score < 75) return BOTTLENECK_LEVEL_COLORS.HIGH;
  return BOTTLENECK_LEVEL_COLORS.CRITICAL;
}

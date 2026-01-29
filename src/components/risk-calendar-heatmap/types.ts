/**
 * RiskCalendarHeatmap 类型和常量
 */

import type { DaySummary } from '../../types/decision';
import { RISK_LEVEL_COLORS } from '../../types/decision/d1-day-summary';

// ==========================================
// Props 定义
// ==========================================

export interface RiskCalendarHeatmapProps {
  data: DaySummary[];
  onDateClick?: (date: string) => void;
  selectedDate?: string | null;
  height?: number;
}

// ==========================================
// 内部类型
// ==========================================

export interface HeatmapDataResult {
  heatmapData: [string, number][];
  dateRange: [string, string];
  maxRiskScore: number;
}

// ==========================================
// 工具函数
// ==========================================

/**
 * 根据风险分数获取颜色
 */
export function getRiskColor(score: number): string {
  if (score < 25) return RISK_LEVEL_COLORS.LOW;
  if (score < 50) return '#1677ff';
  if (score < 75) return RISK_LEVEL_COLORS.HIGH;
  return RISK_LEVEL_COLORS.CRITICAL;
}

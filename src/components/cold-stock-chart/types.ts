/**
 * ColdStockChart 类型和常量
 */

import type { AgeBin } from '../../types/decision';

// ==========================================
// Props 定义
// ==========================================

export interface ColdStockChartProps {
  data: import('../../types/decision').ColdStockBucket[];
  onMachineClick?: (machine: string) => void;
  selectedMachine?: string | null;
  height?: number;
  displayMode?: 'count' | 'weight';
}

// ==========================================
// 常量
// ==========================================

export const AGE_BIN_COLORS: Record<AgeBin, string> = {
  '0-7': '#52c41a',
  '8-14': '#1677ff',
  '15-30': '#faad14',
  '30+': '#ff4d4f',
};

export const AGE_BIN_LABELS: Record<AgeBin, string> = {
  '0-7': '0-7天',
  '8-14': '8-14天',
  '15-30': '15-30天',
  '30+': '30天以上',
};

export const AGE_BIN_ORDER: AgeBin[] = ['0-7', '8-14', '15-30', '30+'];

// ==========================================
// 内部类型
// ==========================================

export interface ChartDataResult {
  machines: string[];
  seriesData: Record<AgeBin, number[]>;
}

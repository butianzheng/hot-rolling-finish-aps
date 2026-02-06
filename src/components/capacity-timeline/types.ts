/**
 * CapacityTimeline 类型定义
 */

import { URGENCY_COLORS } from '../../theme';
import type { MaterialPoolMaterial } from '../material-pool/types';
import type { PlanItemStatusFilter } from '../../utils/planItemStatus';

export interface OpenScheduleCellOptions {
  statusFilter?: PlanItemStatusFilter;
}

export interface CapacityTimelineProps {
  data: import('../../types/capacity').CapacityTimelineData;
  height?: number;
  selectedMaterialIds?: string[]; // 选中的物料ID列表
  focusedMaterialId?: string | null; // 聚焦的物料ID
  materials?: MaterialPoolMaterial[]; // 所有物料数据（用于产能影响预测）
  // 联动：打开甘特图同日明细
  onOpenScheduleCell?: (
    machineCode: string,
    date: string,
    materialIds: string[],
    options?: OpenScheduleCellOptions
  ) => void;
}

export type RollStatus = 'critical' | 'warning' | 'healthy';

export interface SegmentWithWidth {
  urgencyLevel: string;
  tonnage: number;
  materialCount: number;
  widthPercent: number;
}

// 轧辊状态颜色映射
export const ROLL_STATUS_COLORS: Record<RollStatus, string> = {
  critical: '#ff4d4f',
  warning: '#faad14',
  healthy: '#52c41a',
};

// 图例项配置
export const URGENCY_LEGEND_ITEMS = [
  { key: 'L3_EMERGENCY', color: URGENCY_COLORS.L3_EMERGENCY, label: '三级 紧急' },
  { key: 'L2_HIGH', color: URGENCY_COLORS.L2_HIGH, label: '二级 高' },
  { key: 'L1_MEDIUM', color: URGENCY_COLORS.L1_MEDIUM, label: '一级 中' },
  { key: 'L0_NORMAL', color: URGENCY_COLORS.L0_NORMAL, label: '常规 正常' },
] as const;

export const LINE_LEGEND_ITEMS = [
  { color: '#1677ff', label: '目标产能', opacity: 1 },
  { color: '#ff4d4f', label: '限制产能', opacity: 1 },
  { color: '#faad14', label: '轧辊更换', opacity: 0.5 },
] as const;

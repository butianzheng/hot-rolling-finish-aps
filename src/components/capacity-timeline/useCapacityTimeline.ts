/**
 * CapacityTimeline 计算 Hook
 */

import { useMemo } from 'react';
import { ROLL_CHANGE_THRESHOLDS } from '../../theme';
import type { CapacityTimelineData } from '../../types/capacity';
import type { RollStatus, SegmentWithWidth } from './types';
import { ROLL_STATUS_COLORS } from './types';

export interface UseCapacityTimelineReturn {
  utilizationPercent: number;
  isOverLimit: boolean;
  rollStatus: RollStatus;
  rollStatusColor: string;
  segments: SegmentWithWidth[];
}

export function useCapacityTimeline(data: CapacityTimelineData): UseCapacityTimelineReturn {
  // 计算产能利用率
  const utilizationPercent = useMemo(() => {
    const target = Number(data.targetCapacity);
    const actual = Number(data.actualCapacity);
    if (!Number.isFinite(target) || target <= 0) return 0;
    if (!Number.isFinite(actual) || actual <= 0) return 0;
    return (actual / target) * 100;
  }, [data.actualCapacity, data.targetCapacity]);

  // 计算是否超限
  const isOverLimit =
    Number.isFinite(data.actualCapacity) &&
    Number.isFinite(data.limitCapacity) &&
    data.actualCapacity > data.limitCapacity;

  // 计算轧辊状态
  const rollStatus = useMemo<RollStatus>(() => {
    const progress = data.rollCampaignProgress;
    const threshold = data.rollChangeThreshold;
    if (progress >= threshold) return 'critical';
    if (progress >= ROLL_CHANGE_THRESHOLDS.WARNING) return 'warning';
    return 'healthy';
  }, [data.rollCampaignProgress, data.rollChangeThreshold]);

  const rollStatusColor = ROLL_STATUS_COLORS[rollStatus];

  // 计算每个分段的宽度百分比
  const segments = useMemo<SegmentWithWidth[]>(() => {
    const total = Number(data.actualCapacity);
    const safeTotal = Number.isFinite(total) && total > 0 ? total : 0;
    return data.segments.map((seg) => ({
      ...seg,
      widthPercent: safeTotal > 0 ? (seg.tonnage / safeTotal) * 100 : 0,
    }));
  }, [data.segments, data.actualCapacity]);

  return {
    utilizationPercent,
    isOverLimit,
    rollStatus,
    rollStatusColor,
    segments,
  };
}

export default useCapacityTimeline;

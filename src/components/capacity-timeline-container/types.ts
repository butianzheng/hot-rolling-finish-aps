/**
 * CapacityTimelineContainer 类型定义
 */

import type dayjs from 'dayjs';

export interface CapacityTimelineContainerProps {
  machineCode?: string | null;
}

export interface MachineOption {
  label: string;
  value: string;
}

export type DateRangeValue = [dayjs.Dayjs, dayjs.Dayjs];

export type UrgencyLevel = 'L0' | 'L1' | 'L2' | 'L3';

export interface UrgencyBucket {
  tonnage: number;
  count: number;
}

export type UrgencyBucketMap = Record<UrgencyLevel, UrgencyBucket>;

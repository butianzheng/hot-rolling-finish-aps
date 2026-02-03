/**
 * CapacityTimelineContainer 类型定义
 */

import type dayjs from 'dayjs';
import type { MaterialPoolMaterial } from '../material-pool/types';
import type { OpenScheduleCellOptions } from '../capacity-timeline/types';

// 导入通用类型（避免重复定义）
import type { UrgencyLevel } from '../../types/decision/d2-order-failure';

// 重新导出供其他组件使用
export type { UrgencyLevel };

export interface CapacityTimelineContainerProps {
  machineCode?: string | null;
  // 新增：外部日期范围
  dateRange?: [dayjs.Dayjs, dayjs.Dayjs];
  // 新增：受控联动回写（用于工作台多视图同步）
  onMachineCodeChange?: (machineCode: string | null) => void;
  onDateRangeChange?: (range: [dayjs.Dayjs, dayjs.Dayjs]) => void;
  onResetDateRange?: () => void;
  // 联动：点击产能卡片/条形图打开甘特同日明细
  onOpenScheduleCell?: (
    machineCode: string,
    date: string,
    materialIds: string[],
    options?: OpenScheduleCellOptions
  ) => void;
  // 新增：选中物料
  selectedMaterialIds?: string[];
  focusedMaterialId?: string | null;
  // 新增：所有物料数据（用于产能影响预测）
  materials?: MaterialPoolMaterial[];
}

export interface MachineOption {
  label: string;
  value: string;
}

export type DateRangeValue = [dayjs.Dayjs, dayjs.Dayjs];

export interface UrgencyBucket {
  tonnage: number;
  count: number;
}

export type UrgencyBucketMap = Record<UrgencyLevel, UrgencyBucket>;

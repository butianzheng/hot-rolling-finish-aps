/**
 * ScheduleCardView 类型定义和常量
 */

import type { Dayjs } from 'dayjs';
import type { PlanItemStatusFilter } from '../../utils/planItemStatus';

export interface PlanItemRow {
  material_id: string;
  machine_code: string;
  plan_date: string;
  seq_no: number;
  weight_t: number;
  urgent_level?: string;
  locked_in_plan?: boolean;
  force_release_in_plan?: boolean;
}

export interface ScheduleCardViewProps {
  machineCode?: string | null;
  urgentLevel?: string | null;
  // 受控日期范围（工作台多视图联动）
  dateRange?: [Dayjs, Dayjs];
  // 排程状态快速筛选（已排/冻结/强放/可调）
  statusFilter?: PlanItemStatusFilter;
  onStatusFilterChange?: (next: PlanItemStatusFilter) => void;
  refreshSignal?: number;
  selectedMaterialIds: string[];
  onSelectedMaterialIdsChange: (ids: string[]) => void;
  onInspectMaterialId?: (materialId: string) => void;
}

export const ROW_HEIGHT = 92;

// 树形分解图行高常量
export const MACHINE_HEADER_HEIGHT = 48;
export const DATE_ROW_HEIGHT = 52;

// 日期行状态统计
export interface DateStatusSummary {
  lockedCount: number;
  forceReleaseCount: number;
  adjustableCount: number;
}

// 树形分解图行类型（机组 → 日期条形图，无物料明细）
export type ScheduleTreeRow =
  | { type: 'machine'; machineCode: string; count: number; weightT: number; collapsed: boolean }
  | { type: 'date'; machineCode: string; date: string; count: number; weightT: number; status: DateStatusSummary };

/**
 * ScheduleGanttView 类型和常量定义
 */

// ==========================================
// 类型定义
// ==========================================

import type { Dayjs } from 'dayjs';
import type { PlanItemStatusFilter } from '../../utils/planItemStatus';

export type PlanItemRow = {
  material_id: string;
  machine_code: string;
  plan_date: string;
  seq_no: number;
  weight_t: number;
  urgent_level?: string;
  locked_in_plan?: boolean;
  force_release_in_plan?: boolean;
};

export interface ScheduleGanttViewProps {
  machineCode?: string | null;
  urgentLevel?: string | null;
  // 受控日期范围（用于工作台多视图联动）
  dateRange?: [Dayjs, Dayjs];
  // “重置范围”时回到该范围（通常是工作台的 AUTO 范围）
  suggestedDateRange?: [Dayjs, Dayjs];
  onDateRangeChange?: (range: [Dayjs, Dayjs]) => void;
  // 深链接/联动定位：滚动到日期列
  focusedDate?: string | null;
  // 深链接/联动定位：自动打开该单元格明细
  autoOpenCell?: { machine: string; date: string; nonce?: string | number; source?: string } | null;
  // 排程状态快速筛选（已排/冻结/强放/可调）
  statusFilter?: PlanItemStatusFilter;
  onStatusFilterChange?: (next: PlanItemStatusFilter) => void;
  // 甘特图内聚焦变更（点击日期/打开单元格明细时回调到工作台显示）
  onFocusChange?: (focus: { machine?: string; date: string; source?: string }) => void;
  // 外部受控聚焦（由工作台维护，甘特图负责展示）
  focus?: { machine?: string; date: string; source?: string } | null;
  onNavigateToMatrix?: (machine: string, date: string) => void;
  planItems?: unknown;
  loading?: boolean;
  error?: unknown;
  onRetry?: () => void;
  selectedMaterialIds: string[];
  onSelectedMaterialIdsChange: (ids: string[]) => void;
  onInspectMaterialId?: (materialId: string) => void;
  onRequestMoveToCell?: (machine: string, date: string) => void;
}

export type CapacityData = {
  target: number;
  limit: number;
  used: number;
};

export type CellDetail = {
  machine: string;
  date: string;
} | null;

// ==========================================
// 布局常量
// ==========================================

export const LEFT_COL_WIDTH = 168;
export const HEADER_HEIGHT = 44;
export const ROW_HEIGHT = 72;
export const COLUMN_WIDTH = 112;
export const CELL_PADDING_X = 6;
export const BAR_HEIGHT = 18;
export const BAR_GAP = 4;
export const MAX_ITEMS_PER_CELL = 2;
export const MAX_DAYS = 60;

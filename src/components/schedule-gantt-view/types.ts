/**
 * ScheduleGanttView 类型和常量定义
 */

// ==========================================
// 类型定义
// ==========================================

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

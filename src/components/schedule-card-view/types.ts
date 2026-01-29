/**
 * ScheduleCardView 类型定义和常量
 */

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
  refreshSignal?: number;
  selectedMaterialIds: string[];
  onSelectedMaterialIdsChange: (ids: string[]) => void;
  onInspectMaterialId?: (materialId: string) => void;
}

export const ROW_HEIGHT = 92;

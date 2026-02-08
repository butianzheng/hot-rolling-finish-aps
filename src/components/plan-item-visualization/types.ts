/**
 * 排产明细可视化 - 类型定义和常量
 */

import type { Dayjs } from 'dayjs';
import type { PlanItemStatusFilter } from '../../utils/planItemStatus';

// 排产明细类型
export interface PlanItem extends Record<string, unknown> {
  key: string;
  version_id: string;
  material_id: string;
  machine_code: string;
  plan_date: string;
  seq_no: number;
  weight_t: number;
  steel_grade?: string;
  contract_no?: string | null;
  due_date?: string | null;
  scheduled_date?: string | null;
  scheduled_machine_code?: string | null;
  width_mm?: number | null;
  thickness_mm?: number | null;
  urgent_level?: string;
  source_type: string;
  locked_in_plan: boolean;
  force_release_in_plan: boolean;
  sched_state?: string;
  assign_reason?: string;
}

// 统计信息类型
export interface Statistics {
  total_items: number;
  total_weight: number;
  by_machine: Record<string, number>;
  by_urgent_level: Record<string, number>;
  frozen_count: number;
}

// 组件 Props
export interface PlanItemVisualizationProps {
  onNavigateToPlan?: () => void;
  machineCode?: string | null;
  machineOptions?: string[];
  urgentLevel?: string | null;
  defaultDateRange?: [Dayjs, Dayjs] | null;
  statusFilter?: PlanItemStatusFilter;
  onStatusFilterChange?: (next: PlanItemStatusFilter) => void;
  focusRequest?: { machine?: string; date: string; nonce: string | number; searchText?: string } | null;
  selectedMaterialIds?: string[];
  onSelectedMaterialIdsChange?: (ids: string[]) => void;
}

// 紧急等级颜色映射
export const urgentLevelColors: Record<string, string> = {
  L3: 'red',
  L2: 'orange',
  L1: 'gold',
  L0: 'default',
};

// 来源类型标签
export const sourceTypeLabels: Record<string, { text: string; color: string }> = {
  CALC: { text: '计算', color: 'blue' },
  FROZEN: { text: '冻结', color: 'purple' },
  MANUAL: { text: '人工', color: 'green' },
};

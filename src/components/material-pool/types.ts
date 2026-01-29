/**
 * MaterialPool 类型定义
 */

import type { WorkbenchLockStatusFilter } from '../../stores/use-global-store';

export interface MaterialPoolMaterial {
  material_id: string;
  machine_code: string;
  weight_t: number;
  steel_mark: string;
  sched_state: string;
  urgent_level: string;
  lock_flag: boolean;
  manual_urgent_flag: boolean;
  is_frozen?: boolean;
  is_mature?: boolean;
  temp_issue?: boolean;
}

export interface MaterialPoolSelection {
  machineCode: string | null;
  schedState: string | null;
}

export interface MaterialPoolFilters {
  urgencyLevel: string | null;
  lockStatus: WorkbenchLockStatusFilter;
}

export interface MaterialPoolProps {
  materials: MaterialPoolMaterial[];
  loading?: boolean;
  error?: unknown;
  onRetry?: () => void;

  selection: MaterialPoolSelection;
  onSelectionChange: (next: MaterialPoolSelection) => void;

  filters?: MaterialPoolFilters;
  onFiltersChange?: (next: Partial<MaterialPoolFilters>) => void;

  selectedMaterialIds: string[];
  onSelectedMaterialIdsChange: (ids: string[]) => void;
  onInspectMaterial?: (material: MaterialPoolMaterial) => void;
}

export type UrgencyBucket = 'L3' | 'L2' | 'L1' | 'L0';

export const URGENCY_ORDER: UrgencyBucket[] = ['L3', 'L2', 'L1', 'L0'];

export const ROW_HEIGHT = 56;

export type PoolRow =
  | { type: 'header'; level: UrgencyBucket; count: number; weight: number; collapsed: boolean }
  | { type: 'material'; material: MaterialPoolMaterial };

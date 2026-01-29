/**
 * 共享类型定义文件
 * 用于版本对比相关的本地计算类型
 */

export interface Plan {
  plan_id: string;
  plan_name: string;
  created_by: string;
  created_at: string;
}

export interface Version {
  version_id: string;
  version_no: number;
  status: string;
  recalc_window_days: number;
  created_at: string;
  config_snapshot_json?: string | null;
}

export type LocalVersionDiffSummary = {
  totalChanges: number;
  addedCount: number;
  removedCount: number;
  modifiedCount: number;
  movedCount: number;
};

export type LocalCapacityDeltaRow = {
  machine_code: string;
  date: string;
  used_a: number;
  used_b: number;
  delta: number;
  target_a: number | null;
  limit_a: number | null;
  target_b: number | null;
  limit_b: number | null;
};

export const RETROSPECTIVE_NOTE_KEY_PREFIX = 'aps_retrospective_note';

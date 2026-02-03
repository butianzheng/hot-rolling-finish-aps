export type MoveSeqMode = 'APPEND' | 'START_SEQ';
export type MoveValidationMode = 'AUTO_FIX' | 'STRICT';
export type WorkbenchDateRangeMode = 'AUTO' | 'PINNED' | 'MANUAL';

export type MoveItemResultRow = {
  material_id: string;
  success: boolean;
  from_machine: string | null;
  from_date: string | null;
  to_machine: string;
  to_date: string;
  error: string | null;
  violation_type: string | null;
};

export type MoveImpactRow = {
  machine_code: string;
  date: string;
  before_t: number;
  delta_t: number;
  after_t: number;
  target_capacity_t: number | null;
  limit_capacity_t: number | null;
};

export type ConditionLockFilter = 'ALL' | 'LOCKED' | 'UNLOCKED';

export type ForceReleaseViolation = {
  material_id?: unknown;
  violation_type?: unknown;
  reason?: unknown;
};

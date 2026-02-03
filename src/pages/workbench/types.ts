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

export type SelectedPlanItemStats = {
  inPlan: number;
  frozenInPlan: number;
  outOfPlan: number;
};

export type MoveImpactPreview = {
  rows: MoveImpactRow[];
  overflowRows: MoveImpactRow[];
  loading: boolean;
};

export type MoveRecommendSummary = {
  machine: string;
  date: string;
  overLimitCount: number;
  unknownCount: number;
  totalOverT: number;
  maxUtilPct: number;
};

export type ConditionLockFilter = 'ALL' | 'LOCKED' | 'UNLOCKED';

export type ForceReleaseViolation = {
  material_id?: unknown;
  violation_type?: unknown;
  reason?: unknown;
};

export type MaterialOperationType = 'lock' | 'unlock' | 'urgent_on' | 'urgent_off';

// =============================
// Workbench UI: deep link context
// =============================

export type WorkbenchDeepLinkContext = {
  machine?: string;
  date?: string;
  urgency?: string;
  context?: string;
  focus?: string;
  openCell?: boolean;
};

// =============================
// Workbench UI: schedule focus
// =============================

export type WorkbenchScheduleFocus = {
  machine?: string;
  date: string;
  source?: string;
};

export type WorkbenchMatrixFocusRequest = {
  machine?: string;
  date: string;
  nonce: number;
};

export type WorkbenchGanttAutoOpenCell = {
  machine: string;
  date: string;
  nonce?: string | number;
  source?: string;
};

// =============================
// Workbench UI: path override
// =============================

export type WorkbenchPathOverrideContext = {
  machineCode: string | null;
  planDate: string | null;
};

export type WorkbenchPathOverrideSummaryRange = {
  from: string;
  to: string;
};

export type WorkbenchPathOverrideState = {
  context: WorkbenchPathOverrideContext;
  pendingCount: number;
  pendingIsFetching: boolean;
  pendingRefetch: () => void;
  summaryRange: WorkbenchPathOverrideSummaryRange;
  pendingTotalCount: number;
  summaryIsFetching: boolean;
  summaryRefetch: () => void;
  recalcAfterPathOverride: (baseDate: string) => Promise<void>;
};

// =============================
// Workbench UI: move modal
// =============================

export type MoveModalState = {
  open: boolean;
  targetMachine: string | null;
  targetDate: import('dayjs').Dayjs | null;
  seqMode: MoveSeqMode;
  startSeq: number;
  validationMode: MoveValidationMode;
  reason: string;
  submitting: boolean;
  recommendLoading: boolean;
  recommendSummary: MoveRecommendSummary | null;
  strategyLabel: string;
  selectedPlanItemStats: SelectedPlanItemStats;
  impactPreview: MoveImpactPreview | null;
};

export type MoveModalActions = {
  setOpen: import('react').Dispatch<import('react').SetStateAction<boolean>>;
  setTargetMachine: import('react').Dispatch<import('react').SetStateAction<string | null>>;
  setTargetDate: import('react').Dispatch<import('react').SetStateAction<import('dayjs').Dayjs | null>>;
  setSeqMode: import('react').Dispatch<import('react').SetStateAction<MoveSeqMode>>;
  setStartSeq: import('react').Dispatch<import('react').SetStateAction<number>>;
  setValidationMode: import('react').Dispatch<import('react').SetStateAction<MoveValidationMode>>;
  setReason: import('react').Dispatch<import('react').SetStateAction<string>>;
  recommendTarget: () => Promise<void>;
  openModal: () => void;
  openModalAt: (targetMachine: string, targetDate: string) => void;
  openModalWithRecommend: () => void;
  submit: () => Promise<void>;
};

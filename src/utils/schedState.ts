export type NormalizedSchedState =
  | 'PENDING_MATURE'
  | 'READY'
  | 'LOCKED'
  | 'FORCE_RELEASE'
  | 'BLOCKED'
  | 'SCHEDULED'
  | 'UNKNOWN';

// Normalize backend/legacy variants into a single canonical sched_state value.
// Backend canonical format: SCREAMING_SNAKE_CASE (e.g. READY / SCHEDULED).
export function normalizeSchedState(value: unknown): NormalizedSchedState {
  const raw = String(value ?? '').trim();
  if (!raw) return 'UNKNOWN';

  const upper = raw.toUpperCase();
  const compact = upper.replace(/_/g, '');

  switch (compact) {
    case 'PENDINGMATURE':
      return 'PENDING_MATURE';
    case 'READY':
      return 'READY';
    case 'LOCKED':
      return 'LOCKED';
    case 'FORCERELEASE':
      return 'FORCE_RELEASE';
    case 'BLOCKED':
      return 'BLOCKED';
    case 'SCHEDULED':
      return 'SCHEDULED';
    default:
      return 'UNKNOWN';
  }
}

export function isScheduled(value: unknown): boolean {
  return normalizeSchedState(value) === 'SCHEDULED';
}

export function getSchedStateLabel(value: unknown): string {
  switch (normalizeSchedState(value)) {
    case 'PENDING_MATURE':
      return '未成熟/冷料';
    case 'READY':
      return '待排/就绪';
    case 'LOCKED':
      return '已锁定';
    case 'FORCE_RELEASE':
      return '强制放行';
    case 'BLOCKED':
      return '阻断';
    case 'SCHEDULED':
      return '已排产';
    default:
      return '未知';
  }
}


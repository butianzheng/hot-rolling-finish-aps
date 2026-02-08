export const DEFAULT_LATEST_RUN_TTL_MS = 120_000;

export type LatestRunStatus = 'IDLE' | 'PENDING' | 'RUNNING' | 'DONE' | 'FAILED' | 'EXPIRED';

export interface LatestRunState {
  runId: string | null;
  status: LatestRunStatus;
  triggeredAt: number | null;
  updatedAt: number | null;
  expiresAt: number | null;
  versionId: string | null;
  planRev: number | null;
  error: string | null;
}

export interface BeginRunInput {
  runId: string;
  triggeredAt?: number;
  ttlMs?: number;
  versionId?: string | null;
}

export interface BeginRunResult {
  accepted: boolean;
  next: LatestRunState;
  reason?: 'OLDER_TRIGGER' | 'EXPIRED_PREVIOUS';
}

export function createInitialLatestRunState(): LatestRunState {
  return {
    runId: null,
    status: 'IDLE',
    triggeredAt: null,
    updatedAt: null,
    expiresAt: null,
    versionId: null,
    planRev: null,
    error: null,
  };
}

export function isLatestRunExpired(state: LatestRunState, now: number = Date.now()): boolean {
  if (!state.runId) return false;
  if (state.status === 'DONE' || state.status === 'FAILED' || state.status === 'EXPIRED' || state.status === 'IDLE') {
    return false;
  }
  return typeof state.expiresAt === 'number' && state.expiresAt > 0 && now >= state.expiresAt;
}

export function expireLatestRunState(state: LatestRunState, now: number = Date.now()): LatestRunState {
  if (!isLatestRunExpired(state, now)) return state;
  return {
    ...state,
    status: 'EXPIRED',
    updatedAt: now,
    error: state.error ?? 'RUN_TTL_EXPIRED',
  };
}

export function beginLatestRunState(prev: LatestRunState, input: BeginRunInput): BeginRunResult {
  const now = typeof input.triggeredAt === 'number' ? input.triggeredAt : Date.now();
  const ttlMs = Number.isFinite(Number(input.ttlMs)) && Number(input.ttlMs) > 0
    ? Number(input.ttlMs)
    : DEFAULT_LATEST_RUN_TTL_MS;

  const runId = String(input.runId || '').trim();
  if (!runId) {
    return { accepted: false, next: prev, reason: 'OLDER_TRIGGER' };
  }

  const expiredPrev = expireLatestRunState(prev, now);
  const previousDone = expiredPrev.status === 'DONE' || expiredPrev.status === 'FAILED' || expiredPrev.status === 'EXPIRED' || !expiredPrev.runId;

  const previousTriggeredAt = Number(expiredPrev.triggeredAt ?? 0);
  if (!previousDone && now <= previousTriggeredAt) {
    return {
      accepted: false,
      next: expiredPrev,
      reason: 'OLDER_TRIGGER',
    };
  }

  return {
    accepted: true,
    next: {
      runId,
      status: 'PENDING',
      triggeredAt: now,
      updatedAt: now,
      expiresAt: now + ttlMs,
      versionId: input.versionId ?? expiredPrev.versionId ?? null,
      planRev: expiredPrev.planRev,
      error: null,
    },
    reason: previousDone ? undefined : (expiredPrev.status === 'EXPIRED' ? 'EXPIRED_PREVIOUS' : undefined),
  };
}

export function markLatestRunRunningState(prev: LatestRunState, runId: string, now: number = Date.now()): LatestRunState {
  if (!prev.runId || prev.runId !== runId) return prev;
  if (prev.status === 'DONE' || prev.status === 'FAILED' || prev.status === 'EXPIRED') return prev;
  return {
    ...prev,
    status: 'RUNNING',
    updatedAt: now,
  };
}

export function markLatestRunDoneState(
  prev: LatestRunState,
  runId: string,
  payload?: { versionId?: string | null; planRev?: number | null },
  now: number = Date.now(),
): LatestRunState {
  if (!prev.runId || prev.runId !== runId) return prev;
  return {
    ...prev,
    status: 'DONE',
    updatedAt: now,
    error: null,
    versionId: payload?.versionId ?? prev.versionId,
    planRev: payload?.planRev ?? prev.planRev,
  };
}

export function markLatestRunFailedState(
  prev: LatestRunState,
  runId: string,
  error: string | null,
  now: number = Date.now(),
): LatestRunState {
  if (!prev.runId || prev.runId !== runId) return prev;
  return {
    ...prev,
    status: 'FAILED',
    updatedAt: now,
    error: error ?? 'UNKNOWN_ERROR',
  };
}

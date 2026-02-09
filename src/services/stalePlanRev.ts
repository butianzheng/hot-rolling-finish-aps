import { message } from 'antd';

export const DEFAULT_STALE_PLAN_REV_TOAST_COOLDOWN_MS = 4_000;

export interface StalePlanRevDetails {
  version_id?: string;
  expected_plan_rev?: number;
  actual_plan_rev?: number;
}

export interface StalePlanRevMeta {
  source?: 'query' | 'mutation' | 'ipc' | 'manual';
  command?: string;
}

export interface StalePlanRevRefreshContext {
  details: StalePlanRevDetails;
  meta?: StalePlanRevMeta;
}

type StaleLikeError = {
  code?: string;
  message?: string;
  details?: Record<string, unknown>;
};

const MIN_STALE_PLAN_REV_TOAST_COOLDOWN_MS = 1_000;
const MAX_STALE_PLAN_REV_TOAST_COOLDOWN_MS = 60_000;

let configuredToastCooldownMs = DEFAULT_STALE_PLAN_REV_TOAST_COOLDOWN_MS;
let lastToastAt = 0;
let inFlight: Promise<void> | null = null;
let refreshHandler: ((ctx: StalePlanRevRefreshContext) => Promise<void>) | null = null;

function sanitizeToastCooldownMs(value: unknown): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return DEFAULT_STALE_PLAN_REV_TOAST_COOLDOWN_MS;

  const rounded = Math.round(parsed);
  if (rounded < MIN_STALE_PLAN_REV_TOAST_COOLDOWN_MS) return MIN_STALE_PLAN_REV_TOAST_COOLDOWN_MS;
  if (rounded > MAX_STALE_PLAN_REV_TOAST_COOLDOWN_MS) return MAX_STALE_PLAN_REV_TOAST_COOLDOWN_MS;
  return rounded;
}

export function configureStalePlanRevToastCooldownMs(value: unknown): number {
  configuredToastCooldownMs = sanitizeToastCooldownMs(value);
  return configuredToastCooldownMs;
}

export function getStalePlanRevToastCooldownMs(): number {
  return configuredToastCooldownMs;
}

export function registerStalePlanRevRefreshHandler(
  handler: ((ctx: StalePlanRevRefreshContext) => Promise<void>) | null,
): void {
  refreshHandler = handler;
}

export function isStalePlanRevError(error: unknown): error is StaleLikeError {
  if (!error || typeof error !== 'object') return false;
  const code = String((error as Record<string, unknown>).code || '').trim().toUpperCase();
  return code === 'STALE_PLAN_REV';
}

export function readStalePlanRevDetails(error: unknown): StalePlanRevDetails {
  if (!error || typeof error !== 'object') return {};
  const details = (error as Record<string, unknown>).details;
  if (!details || typeof details !== 'object') return {};

  const raw = details as Record<string, unknown>;
  const expectedRaw = raw.expected_plan_rev;
  const actualRaw = raw.actual_plan_rev;

  const expected = Number(expectedRaw);
  const actual = Number(actualRaw);

  return {
    version_id: typeof raw.version_id === 'string' ? raw.version_id : undefined,
    expected_plan_rev: Number.isFinite(expected) ? expected : undefined,
    actual_plan_rev: Number.isFinite(actual) ? actual : undefined,
  };
}

export async function handleStalePlanRevError(error: unknown, meta?: StalePlanRevMeta): Promise<boolean> {
  if (!isStalePlanRevError(error)) return false;

  const now = Date.now();
  if (now - lastToastAt > configuredToastCooldownMs) {
    lastToastAt = now;
    message.warning({
      key: 'stale-plan-rev-warning',
      content: '检测到计划版本已更新，正在自动切换到最新计划…',
      duration: 2,
    });
  }

  if (!inFlight) {
    const details = readStalePlanRevDetails(error);
    inFlight = (async () => {
      try {
        if (refreshHandler) {
          await refreshHandler({ details, meta });
        }
      } finally {
        inFlight = null;
      }
    })();
  }

  await inFlight;
  return true;
}

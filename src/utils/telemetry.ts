import { invoke } from '@tauri-apps/api/tauri';
import { useGlobalStore } from '../stores/use-global-store';

type FrontendLogLevel = 'error' | 'warn' | 'info' | 'debug';

const MAX_EVENTS_PER_SESSION = 50;
const DEDUPE_WINDOW_MS = 30_000;
const MAX_TEXT_LEN = 16_000;

let sentCount = 0;
const lastSentAtByFingerprint = new Map<string, number>();

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && Boolean((window as any).__TAURI__);
}

function truncate(text: string, maxLen: number = MAX_TEXT_LEN): string {
  const s = String(text || '');
  if (s.length <= maxLen) return s;
  return `${s.slice(0, maxLen)}...<truncated ${s.length - maxLen} chars>`;
}

function safeJson(obj: any): any {
  try {
    return JSON.parse(JSON.stringify(obj));
  } catch {
    return String(obj);
  }
}

function normalizeUnknownError(err: any): { name?: string; message: string; stack?: string } {
  if (!err) return { message: 'Unknown error' };
  if (err instanceof Error) {
    return {
      name: err.name,
      message: err.message || String(err),
      stack: err.stack,
    };
  }
  if (typeof err === 'string') return { message: err };
  try {
    return { message: JSON.stringify(err) };
  } catch {
    return { message: String(err) };
  }
}

function makeFingerprint(level: string, message: string, extra?: string): string {
  const base = `${level}|${message}`;
  const suffix = extra ? `|${extra}` : '';
  return truncate(base + suffix, 512);
}

async function getLatestActiveVersionId(): Promise<string | null> {
  if (!isTauriRuntime()) return null;
  try {
    const raw = await invoke('get_latest_active_version_id', {});
    const parsed = typeof raw === 'string' ? JSON.parse(raw) : raw;
    if (typeof parsed === 'string' && parsed.trim()) return parsed.trim();
    return null;
  } catch {
    return null;
  }
}

async function reportToBackend(params: {
  version_id?: string | null;
  actor?: string | null;
  level: FrontendLogLevel | string;
  message: string;
  payload_json: any;
}): Promise<void> {
  if (!isTauriRuntime()) return;

  try {
    await invoke('report_frontend_event', {
      version_id: params.version_id ?? null,
      actor: params.actor ?? null,
      level: params.level,
      message: params.message,
      payload_json: params.payload_json,
    });
  } catch {
    // best-effort: ignore
  }
}

export async function reportFrontendEvent(
  level: FrontendLogLevel,
  message: string,
  payload: any = {},
): Promise<void> {
  if (sentCount >= MAX_EVENTS_PER_SESSION) return;

  const store = useGlobalStore.getState();
  const actor = String(store.currentUser || 'unknown');
  const versionIdFromStore = store.activeVersionId;

  const fingerprint = makeFingerprint(level, message);
  const now = Date.now();
  const last = lastSentAtByFingerprint.get(fingerprint) || 0;
  if (now - last < DEDUPE_WINDOW_MS) return;
  lastSentAtByFingerprint.set(fingerprint, now);
  sentCount += 1;

  const route =
    typeof window !== 'undefined'
      ? `${window.location.pathname}${window.location.search || ''}`
      : null;

  const version_id = versionIdFromStore || (await getLatestActiveVersionId());

  await reportToBackend({
    version_id,
    actor,
    level,
    message: truncate(message),
    payload_json: {
      route,
      url: typeof window !== 'undefined' ? window.location.href : null,
      user_agent: typeof navigator !== 'undefined' ? navigator.userAgent : null,
      env: (import.meta as any)?.env?.MODE || null,
      payload: safeJson(payload),
    },
  });
}

export async function reportFrontendError(
  error: unknown,
  context: Record<string, any> = {},
): Promise<void> {
  const normalized = normalizeUnknownError(error);

  const extra = truncate(normalized.stack || normalized.message, 256);
  const fingerprint = makeFingerprint('error', normalized.message, extra);
  const now = Date.now();
  const last = lastSentAtByFingerprint.get(fingerprint) || 0;
  if (now - last < DEDUPE_WINDOW_MS) return;
  lastSentAtByFingerprint.set(fingerprint, now);

  await reportFrontendEvent('error', normalized.message || 'Frontend error', {
    error: {
      name: normalized.name,
      message: truncate(normalized.message),
      stack: normalized.stack ? truncate(normalized.stack) : null,
    },
    context: safeJson(context),
  });
}


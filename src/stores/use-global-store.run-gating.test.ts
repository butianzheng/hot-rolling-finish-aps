import { describe, expect, it } from 'vitest';

function ensureMockLocalStorage() {
  const memory = new Map<string, string>();
  const mockStorage: Storage = {
    get length() {
      return memory.size;
    },
    clear() {
      memory.clear();
    },
    getItem(key: string) {
      return memory.has(key) ? memory.get(key)! : null;
    },
    key(index: number) {
      const keys = Array.from(memory.keys());
      return keys[index] ?? null;
    },
    removeItem(key: string) {
      memory.delete(key);
    },
    setItem(key: string, value: string) {
      memory.set(key, String(value));
    },
  };

  Object.defineProperty(globalThis, 'localStorage', {
    value: mockStorage,
    configurable: true,
  });

  if (typeof window !== 'undefined') {
    Object.defineProperty(window, 'localStorage', {
      value: mockStorage,
      configurable: true,
    });
  }
}

describe('global store run gating', () => {
  it('并发重算回包仅最新 run 可更新 activePlanRev', async () => {
    ensureMockLocalStorage();
    const { useGlobalStore } = await import('./use-global-store');

    useGlobalStore.getState().reset();

    const first = useGlobalStore.getState().beginLatestRun({
      runId: 'run-old',
      triggeredAt: 1_000,
      versionId: 'v1',
      ttlMs: 120_000,
    });
    expect(first.accepted).toBe(true);

    const second = useGlobalStore.getState().beginLatestRun({
      runId: 'run-new',
      triggeredAt: 2_000,
      versionId: 'v1',
      ttlMs: 120_000,
    });
    expect(second.accepted).toBe(true);

    // 旧回包先到：应被忽略
    useGlobalStore.getState().markLatestRunDone('run-old', {
      versionId: 'v1',
      planRev: 11,
    });

    expect(useGlobalStore.getState().latestRun.runId).toBe('run-new');
    expect(useGlobalStore.getState().activePlanRev).toBeNull();

    // 新回包到达：应生效
    useGlobalStore.getState().markLatestRunDone('run-new', {
      versionId: 'v1',
      planRev: 12,
    });

    expect(useGlobalStore.getState().latestRun.runId).toBe('run-new');
    expect(useGlobalStore.getState().latestRun.status).toBe('DONE');
    expect(useGlobalStore.getState().activePlanRev).toBe(12);
  });

  it('运行中 run 到达 TTL 后可被推进为 EXPIRED', async () => {
    ensureMockLocalStorage();
    const { useGlobalStore } = await import('./use-global-store');

    useGlobalStore.getState().reset();

    const begin = useGlobalStore.getState().beginLatestRun({
      runId: 'run-ttl',
      triggeredAt: 1_000,
      versionId: 'v1',
      ttlMs: 100,
    });
    expect(begin.accepted).toBe(true);

    useGlobalStore.getState().markLatestRunRunning('run-ttl');
    expect(useGlobalStore.getState().latestRun.status).toBe('RUNNING');

    useGlobalStore.getState().expireLatestRunIfNeeded(1_050);
    expect(useGlobalStore.getState().latestRun.status).toBe('RUNNING');

    useGlobalStore.getState().expireLatestRunIfNeeded(1_101);
    expect(useGlobalStore.getState().latestRun.status).toBe('EXPIRED');
    expect(useGlobalStore.getState().latestRun.error).toBe('RUN_TTL_EXPIRED');
  });
});

import { describe, expect, it } from 'vitest';
import {
  beginLatestRunState,
  configureLatestRunTtlMs,
  DEFAULT_LATEST_RUN_TTL_MS,
  createInitialLatestRunState,
  expireLatestRunState,
  getLatestRunTtlMs,
  isLatestRunExpired,
  markLatestRunDoneState,
} from './latestRun';

describe('latestRun 状态机', () => {
  it('较晚触发的 run 可覆盖较早 run', () => {
    const init = createInitialLatestRunState();
    const first = beginLatestRunState(init, {
      runId: 'run-1',
      triggeredAt: 1_000,
      ttlMs: 120_000,
    });
    expect(first.accepted).toBe(true);

    const second = beginLatestRunState(first.next, {
      runId: 'run-2',
      triggeredAt: 2_000,
      ttlMs: 120_000,
    });
    expect(second.accepted).toBe(true);
    expect(second.next.runId).toBe('run-2');
    expect(second.next.status).toBe('PENDING');
  });

  it('较早触发的 run 不能覆盖运行中 latest', () => {
    const init = beginLatestRunState(createInitialLatestRunState(), {
      runId: 'run-new',
      triggeredAt: 2_000,
      ttlMs: 120_000,
    }).next;

    const older = beginLatestRunState(init, {
      runId: 'run-old',
      triggeredAt: 1_500,
      ttlMs: 120_000,
    });

    expect(older.accepted).toBe(false);
    expect(older.reason).toBe('OLDER_TRIGGER');
    expect(older.next.runId).toBe('run-new');
  });

  it('TTL 超时后自动 EXPIRED 并允许新 run 覆盖', () => {
    const running = beginLatestRunState(createInitialLatestRunState(), {
      runId: 'run-ttl',
      triggeredAt: 1_000,
      ttlMs: 100,
    }).next;

    expect(isLatestRunExpired(running, 1_050)).toBe(false);
    expect(isLatestRunExpired(running, 1_101)).toBe(true);

    const expired = expireLatestRunState(running, 1_101);
    expect(expired.status).toBe('EXPIRED');

    const replace = beginLatestRunState(expired, {
      runId: 'run-next',
      triggeredAt: 1_102,
      ttlMs: 120_000,
    });

    expect(replace.accepted).toBe(true);
    expect(replace.next.runId).toBe('run-next');
  });

  it('DONE 状态允许直接被新 run 覆盖', () => {
    const pending = beginLatestRunState(createInitialLatestRunState(), {
      runId: 'run-done',
      triggeredAt: 5_000,
      ttlMs: 120_000,
    }).next;

    const done = markLatestRunDoneState(pending, 'run-done', { versionId: 'v2', planRev: 9 }, 5_100);
    expect(done.status).toBe('DONE');

    const next = beginLatestRunState(done, {
      runId: 'run-after-done',
      triggeredAt: 5_200,
      ttlMs: 120_000,
    });

    expect(next.accepted).toBe(true);
    expect(next.next.runId).toBe('run-after-done');
  });

  it('支持通过配置覆盖默认 TTL', () => {
    configureLatestRunTtlMs(30_000);
    expect(getLatestRunTtlMs()).toBe(30_000);

    const result = beginLatestRunState(createInitialLatestRunState(), {
      runId: 'run-config-ttl',
      triggeredAt: 1_000,
    });

    expect(result.next.expiresAt).toBe(31_000);

    configureLatestRunTtlMs(DEFAULT_LATEST_RUN_TTL_MS);
  });
});

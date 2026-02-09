import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { message } from 'antd';

import { QueryProvider, queryClient } from '../app/query-client';
import { useRecentDaysRisk } from './queries/use-decision-queries';
import { useStalePlanRevBootstrap } from './useStalePlanRevBootstrap';
import { useGlobalStore } from '../stores/use-global-store';
import * as tauriApi from '../api/tauri';
import {
  configureStalePlanRevToastCooldownMs,
  DEFAULT_STALE_PLAN_REV_TOAST_COOLDOWN_MS,
  handleStalePlanRevError,
  registerStalePlanRevRefreshHandler,
} from '../services/stalePlanRev';

vi.mock('../api/tauri', () => {
  class DecisionApiError extends Error {
    code: string;
    details?: Record<string, unknown>;

    constructor(code: string, message: string, details?: Record<string, unknown>) {
      super(message);
      this.name = 'DecisionApiError';
      this.code = code;
      this.details = details;
    }
  }

  class ValidationError extends Error {
    constructor(message: string) {
      super(message);
      this.name = 'ValidationError';
    }
  }

  return {
    DecisionApiError,
    ValidationError,
    getDecisionDaySummary: vi.fn(),
    getRiskSummaryForRecentDays: vi.fn(),
    getMachineBottleneckProfile: vi.fn(),
    getBottleneckForRecentDays: vi.fn(),
    listOrderFailureSet: vi.fn(),
    getAllFailedOrders: vi.fn(),
    getColdStockProfile: vi.fn(),
    getHighPressureColdStock: vi.fn(),
    getRollCampaignAlert: vi.fn(),
    getAllRollCampaignAlerts: vi.fn(),
    getCapacityOpportunity: vi.fn(),
    getCapacityOpportunityForRecentDays: vi.fn(),
    planApi: {
      getLatestActiveVersionId: vi.fn(),
      getVersionDetail: vi.fn(),
      listPlans: vi.fn(),
      listVersions: vi.fn(),
      createVersion: vi.fn(),
      deleteVersion: vi.fn(),
      deletePlan: vi.fn(),
      recalcFull: vi.fn(),
    },
  };
});

function DecisionProbe() {
  useStalePlanRevBootstrap();
  const query = useRecentDaysRisk('v1', 7, { retry: false, refetchOnWindowFocus: false });

  if (query.isLoading) return <div data-testid="status">loading</div>;
  if (query.error) return <div data-testid="status">error</div>;

  return (
    <div>
      <div data-testid="status">ok</div>
      <div data-testid="count">{query.data?.items?.length ?? 0}</div>
    </div>
  );
}

describe('Decision stale plan_rev 集成', () => {
  beforeEach(() => {
    queryClient.clear();
    vi.clearAllMocks();
    window.history.replaceState({}, '', '/');
    registerStalePlanRevRefreshHandler(null);
    configureStalePlanRevToastCooldownMs(DEFAULT_STALE_PLAN_REV_TOAST_COOLDOWN_MS);
    useGlobalStore.getState().reset();
    useGlobalStore.getState().setPlanContext({ versionId: 'v1', planRev: 1 });
  });

  it('命中 STALE_PLAN_REV 后自动刷新并恢复展示', async () => {
    vi.mocked(tauriApi.planApi.getLatestActiveVersionId).mockResolvedValue('v1');
    vi.mocked(tauriApi.planApi.getVersionDetail).mockResolvedValue({ revision: 2 } as any);

    const staleError = {
      code: 'STALE_PLAN_REV',
      message: '计划版本已过期',
      details: {
        version_id: 'v1',
        expected_plan_rev: 1,
        actual_plan_rev: 2,
      },
    };

    let callCount = 0;
    vi.mocked(tauriApi.getRiskSummaryForRecentDays).mockImplementation(async (_v, _days, expectedPlanRev) => {
      callCount += 1;

      if (callCount === 1) {
        expect(expectedPlanRev).toBe(1);
        throw staleError as any;
      }

      return {
        versionId: 'v1',
        asOf: '2026-02-08T12:00:00Z',
        totalCount: 1,
        items: [
          {
            planDate: '2026-02-09',
            riskScore: 88,
            riskLevel: 'HIGH',
            capacityUtilPct: 95,
            overloadWeightT: 11.2,
            urgentFailureCount: 3,
            topReasons: [],
            involvedMachines: ['H032'],
          },
        ],
      } as any;
    });

    render(
      <QueryProvider>
        <DecisionProbe />
      </QueryProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('status').textContent).toBe('ok');
      expect(screen.getByTestId('count').textContent).toBe('1');
    });

    expect(useGlobalStore.getState().activePlanRev).toBe(2);
    expect(vi.mocked(tauriApi.getRiskSummaryForRecentDays).mock.calls.length).toBeGreaterThanOrEqual(2);
    expect(
      vi.mocked(tauriApi.getRiskSummaryForRecentDays).mock.calls.some((call) => call[2] === 2),
    ).toBe(true);
  });

  it('deep link 场景命中 STALE_PLAN_REV 时提示已切换到最新计划', async () => {
    window.history.replaceState({}, '', '/workbench?machine=H032&date=2026-02-09&context=risk');

    const infoSpy = vi.spyOn(message, 'info').mockImplementation(() => (() => {}) as any);

    vi.mocked(tauriApi.planApi.getLatestActiveVersionId).mockResolvedValue('v1');
    vi.mocked(tauriApi.planApi.getVersionDetail).mockResolvedValue({ revision: 3 } as any);

    let callCount = 0;
    vi.mocked(tauriApi.getRiskSummaryForRecentDays).mockImplementation(async (_v, _days, expectedPlanRev) => {
      callCount += 1;
      if (callCount === 1) {
        expect(expectedPlanRev).toBe(1);
        throw {
          code: 'STALE_PLAN_REV',
          message: '计划版本已过期',
          details: {
            version_id: 'v1',
            expected_plan_rev: 1,
            actual_plan_rev: 3,
          },
        } as any;
      }

      return {
        versionId: 'v1',
        asOf: '2026-02-08T12:00:00Z',
        totalCount: 1,
        items: [
          {
            planDate: '2026-02-09',
            riskScore: 91,
            riskLevel: 'HIGH',
            capacityUtilPct: 96,
            overloadWeightT: 12.1,
            urgentFailureCount: 2,
            topReasons: [],
            involvedMachines: ['H032'],
          },
        ],
      } as any;
    });

    render(
      <QueryProvider>
        <DecisionProbe />
      </QueryProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('status').textContent).toBe('ok');
      expect(useGlobalStore.getState().activePlanRev).toBe(3);
    });

    await waitFor(() => {
      expect(infoSpy).toHaveBeenCalled();
    });

    expect(
      infoSpy.mock.calls.some((call) => {
        const payload = call[0] as { key?: string; content?: string };
        return payload?.key === 'stale-plan-rev-deeplink' && payload?.content?.includes('已切换到最新计划');
      }),
    ).toBe(true);
  });

  it('同一窗口内连续命中 STALE_PLAN_REV 仅弹一次 warning（防风暴）', async () => {
    const nowSpy = vi.spyOn(Date, 'now').mockReturnValue(9_999_999_999_999);
    const warningSpy = vi.spyOn(message, 'warning').mockImplementation(() => (() => {}) as any);

    let refreshCalls = 0;
    registerStalePlanRevRefreshHandler(async () => {
      refreshCalls += 1;
      await Promise.resolve();
    });

    const staleError = {
      code: 'STALE_PLAN_REV',
      message: '计划版本已过期',
      details: {
        version_id: 'v1',
        expected_plan_rev: 1,
        actual_plan_rev: 2,
      },
    };

    const results = await Promise.all([
      handleStalePlanRevError(staleError, { source: 'query' }),
      handleStalePlanRevError(staleError, { source: 'query' }),
      handleStalePlanRevError(staleError, { source: 'ipc', command: 'list_plan_items' }),
    ]);

    expect(results).toEqual([true, true, true]);
    expect(refreshCalls).toBe(1);
    expect(warningSpy).toHaveBeenCalledTimes(1);

    nowSpy.mockRestore();
  });

  it('超过 4s cooldown 后再次命中 STALE_PLAN_REV 会再次 warning', async () => {
    const base = 30_000_000_000_000;
    const times = [base, base + 1_000, base + 4_100];
    const nowSpy = vi.spyOn(Date, 'now').mockImplementation(() => times.shift() ?? base + 4_100);
    const warningSpy = vi.spyOn(message, 'warning').mockImplementation(() => (() => {}) as any);

    let refreshCalls = 0;
    registerStalePlanRevRefreshHandler(async () => {
      refreshCalls += 1;
      await Promise.resolve();
    });

    const staleError = {
      code: 'STALE_PLAN_REV',
      message: '计划版本已过期',
      details: {
        version_id: 'v1',
        expected_plan_rev: 1,
        actual_plan_rev: 2,
      },
    };

    await handleStalePlanRevError(staleError, { source: 'query' });
    await handleStalePlanRevError(staleError, { source: 'query' });
    await handleStalePlanRevError(staleError, { source: 'query' });

    expect(warningSpy).toHaveBeenCalledTimes(2);
    expect(refreshCalls).toBe(3);

    nowSpy.mockRestore();
  });

  it('配置冷却为 2s 时，2.1s 后再次命中应再次 warning', async () => {
    configureStalePlanRevToastCooldownMs(2_000);

    const base = 40_000_000_000_000;
    const times = [base, base + 1_000, base + 2_100];
    const nowSpy = vi.spyOn(Date, 'now').mockImplementation(() => times.shift() ?? base + 2_100);
    const warningSpy = vi.spyOn(message, 'warning').mockImplementation(() => (() => {}) as any);

    registerStalePlanRevRefreshHandler(async () => {
      await Promise.resolve();
    });

    const staleError = {
      code: 'STALE_PLAN_REV',
      message: '计划版本已过期',
      details: {
        version_id: 'v1',
        expected_plan_rev: 1,
        actual_plan_rev: 2,
      },
    };

    await handleStalePlanRevError(staleError, { source: 'query' });
    await handleStalePlanRevError(staleError, { source: 'query' });
    await handleStalePlanRevError(staleError, { source: 'query' });

    expect(warningSpy).toHaveBeenCalledTimes(2);

    nowSpy.mockRestore();
    configureStalePlanRevToastCooldownMs(DEFAULT_STALE_PLAN_REV_TOAST_COOLDOWN_MS);
  });
});

import { describe, expect, it } from 'vitest';

import { shouldBackfillPlanContextFromRunEvent } from './App';

describe('plan_updated run 事件兜底回填判断', () => {
  it('无 tracked run 且版本匹配时允许回填', () => {
    expect(
      shouldBackfillPlanContextFromRunEvent({
        hadTrackedRun: false,
        incomingVersionId: 'v1',
        incomingPlanRev: 12,
        currentVersionId: 'v1',
        currentPlanRev: 11,
      }),
    ).toBe(true);
  });

  it('存在 tracked run 时不允许兜底回填', () => {
    expect(
      shouldBackfillPlanContextFromRunEvent({
        hadTrackedRun: true,
        incomingVersionId: 'v1',
        incomingPlanRev: 12,
        currentVersionId: 'v1',
        currentPlanRev: 11,
      }),
    ).toBe(false);
  });

  it('版本不匹配时不允许回填', () => {
    expect(
      shouldBackfillPlanContextFromRunEvent({
        hadTrackedRun: false,
        incomingVersionId: 'v2',
        incomingPlanRev: 12,
        currentVersionId: 'v1',
        currentPlanRev: 11,
      }),
    ).toBe(false);
  });

  it('plan_rev 回退时不允许回填', () => {
    expect(
      shouldBackfillPlanContextFromRunEvent({
        hadTrackedRun: false,
        incomingVersionId: 'v1',
        incomingPlanRev: 10,
        currentVersionId: 'v1',
        currentPlanRev: 11,
      }),
    ).toBe(false);
  });
});

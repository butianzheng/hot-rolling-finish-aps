export type PlanItemStatusFilter = 'ALL' | 'LOCKED' | 'FORCE_RELEASE' | 'ADJUSTABLE';

export const PLAN_ITEM_STATUS_FILTER_META: Record<
  PlanItemStatusFilter,
  { label: string; color: string }
> = {
  ALL: { label: '已排', color: 'blue' },
  LOCKED: { label: '冻结', color: 'purple' },
  FORCE_RELEASE: { label: '强放', color: 'red' },
  ADJUSTABLE: { label: '可调', color: 'green' },
};

export type PlanItemStatusSummary = {
  totalCount: number;
  totalWeightT: number;
  lockedInPlanCount: number;
  lockedInPlanWeightT: number;
  forceReleaseCount: number;
  forceReleaseWeightT: number;
  adjustableCount: number;
  adjustableWeightT: number;
};

export function summarizePlanItemStatus(
  items: Array<{
    locked_in_plan?: boolean;
    force_release_in_plan?: boolean;
    weight_t?: number;
  }>
): PlanItemStatusSummary {
  const summary: PlanItemStatusSummary = {
    totalCount: 0,
    totalWeightT: 0,
    lockedInPlanCount: 0,
    lockedInPlanWeightT: 0,
    forceReleaseCount: 0,
    forceReleaseWeightT: 0,
    adjustableCount: 0,
    adjustableWeightT: 0,
  };

  items.forEach((it) => {
    const weight = Number(it.weight_t || 0);
    summary.totalCount += 1;
    summary.totalWeightT += weight;

    if (it.locked_in_plan) {
      summary.lockedInPlanCount += 1;
      summary.lockedInPlanWeightT += weight;
    } else {
      summary.adjustableCount += 1;
      summary.adjustableWeightT += weight;
    }

    if (it.force_release_in_plan) {
      summary.forceReleaseCount += 1;
      summary.forceReleaseWeightT += weight;
    }
  });

  return summary;
}

export function matchPlanItemStatusFilter(
  it: { locked_in_plan?: boolean; force_release_in_plan?: boolean },
  filter: PlanItemStatusFilter
): boolean {
  switch (filter) {
    case 'ALL':
      return true;
    case 'LOCKED':
      return !!it.locked_in_plan;
    case 'FORCE_RELEASE':
      return !!it.force_release_in_plan;
    case 'ADJUSTABLE':
      return !it.locked_in_plan;
    default:
      return true;
  }
}


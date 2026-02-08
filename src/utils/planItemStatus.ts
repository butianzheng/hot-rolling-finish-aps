export type PlanItemStatusFilter =
  | 'ALL'
  | 'LOCKED'
  | 'FORCE_RELEASE'
  | 'ADJUSTABLE'
  | 'READY'
  | 'PENDING_MATURE'
  | 'BLOCKED'
  | 'URGENT_L3'
  | 'URGENT_L2';

export const PLAN_ITEM_STATUS_FILTER_META: Record<
  PlanItemStatusFilter,
  { label: string; color: string }
> = {
  ALL: { label: '已排', color: 'blue' },
  LOCKED: { label: '冻结', color: 'purple' },
  FORCE_RELEASE: { label: '强放', color: 'red' },
  ADJUSTABLE: { label: '可调', color: 'green' },
  READY: { label: '就绪', color: 'cyan' },
  PENDING_MATURE: { label: '待成熟', color: 'default' },
  BLOCKED: { label: '阻断', color: 'volcano' },
  URGENT_L3: { label: 'L3紧急', color: 'magenta' },
  URGENT_L2: { label: 'L2紧急', color: 'orange' },
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

export function isPlanItemForceReleased(it: {
  force_release_in_plan?: boolean;
  sched_state?: string | null;
}): boolean {
  const schedState = String(it.sched_state || '').trim().toUpperCase();
  if (schedState) {
    return schedState === 'FORCE_RELEASE';
  }
  return !!it.force_release_in_plan;
}

export function summarizePlanItemStatus(
  items: Array<{
    locked_in_plan?: boolean;
    force_release_in_plan?: boolean;
    sched_state?: string | null;
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

    if (isPlanItemForceReleased(it)) {
      summary.forceReleaseCount += 1;
      summary.forceReleaseWeightT += weight;
    }
  });

  return summary;
}

export function matchPlanItemStatusFilter(
  it: {
    locked_in_plan?: boolean;
    force_release_in_plan?: boolean;
    sched_state?: string | null;
    urgent_level?: string | null;
  },
  filter: PlanItemStatusFilter
): boolean {
  const schedState = String(it.sched_state || '').trim().toUpperCase();
  const urgentLevel = String(it.urgent_level || '').trim().toUpperCase();

  switch (filter) {
    case 'ALL':
      return true;
    case 'LOCKED':
      return !!it.locked_in_plan;
    case 'FORCE_RELEASE':
      return isPlanItemForceReleased(it);
    case 'ADJUSTABLE':
      return !it.locked_in_plan;
    case 'READY':
      return schedState === 'READY' || schedState === 'SCHEDULED';
    case 'PENDING_MATURE':
      return schedState === 'PENDING_MATURE';
    case 'BLOCKED':
      return schedState === 'BLOCKED';
    case 'URGENT_L3':
      return urgentLevel === 'L3';
    case 'URGENT_L2':
      return urgentLevel === 'L2';
    default:
      return true;
  }
}

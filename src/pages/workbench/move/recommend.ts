import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';

import { capacityApi } from '../../../api/tauri';
import type { MoveRecommendSummary, MoveValidationMode } from '../types';
import type { IpcPlanItem } from './planItems';
import { makeMachineDateKey } from './key';

export type IpcCapacityPool = Awaited<ReturnType<typeof capacityApi.getCapacityPools>>[number];

type MoveRecommendMovable = {
  material_id: string;
  from_machine: string;
  from_date: string;
  weight_t: number;
};

type CapacityLimit = { target: number | null; limit: number | null };

type CandidateScore = {
  date: string;
  overLimitCount: number;
  unknownCount: number;
  totalOverT: number;
  maxUtilPct: number;
  distance: number;
};

export function buildTonnageMap(planItems: IpcPlanItem[]): Map<string, number> {
  const tonnageMap = new Map<string, number>();
  (planItems ?? []).forEach((it) => {
    const machine = String(it.machine_code ?? '').trim();
    const date = String(it.plan_date ?? '').trim();
    if (!machine || !date) return;
    const weight = Number(it.weight_t ?? 0);
    if (!Number.isFinite(weight) || weight <= 0) return;
    const key = makeMachineDateKey(machine, date);
    tonnageMap.set(key, (tonnageMap.get(key) ?? 0) + weight);
  });
  return tonnageMap;
}

export function pickMovableItems(params: {
  selectedMaterialIds: string[];
  byId: Map<string, IpcPlanItem>;
  moveValidationMode: MoveValidationMode;
}): MoveRecommendMovable[] {
  const { selectedMaterialIds, byId, moveValidationMode } = params;
  return (selectedMaterialIds ?? [])
    .map((id) => byId.get(id))
    .filter((it): it is IpcPlanItem => Boolean(it))
    .filter((it) => !(moveValidationMode === 'AUTO_FIX' && it.locked_in_plan === true))
    .map((it) => ({
      material_id: String(it.material_id ?? '').trim(),
      from_machine: String(it.machine_code ?? '').trim(),
      from_date: String(it.plan_date ?? '').trim(),
      weight_t: Number(it.weight_t ?? 0),
    }))
    .filter(
      (it) =>
        it.material_id && it.from_machine && it.from_date && Number.isFinite(it.weight_t) && it.weight_t > 0
    );
}

export function buildMoveDeltaBase(movable: MoveRecommendMovable[]): {
  totalWeight: number;
  deltaBase: Map<string, number>;
} {
  const totalWeight = (movable ?? []).reduce((sum, it) => sum + it.weight_t, 0);
  const deltaBase = new Map<string, number>();
  (movable ?? []).forEach((it) => {
    const fromKey = makeMachineDateKey(it.from_machine, it.from_date);
    deltaBase.set(fromKey, (deltaBase.get(fromKey) ?? 0) - it.weight_t);
  });
  return { totalWeight, deltaBase };
}

export function buildCandidateDates(params: {
  focus: Dayjs;
  rangeStart: Dayjs;
  rangeEnd: Dayjs;
  radius?: number;
}): string[] {
  const { focus, rangeStart, rangeEnd, radius = 15 } = params;
  const spanDays = rangeEnd.diff(rangeStart, 'day');
  const candidates: string[] = [];

  if (spanDays <= radius * 2) {
    for (let i = 0; i <= spanDays; i += 1) {
      candidates.push(rangeStart.add(i, 'day').format('YYYY-MM-DD'));
    }
    return candidates;
  }

  for (let offset = -radius; offset <= radius; offset += 1) {
    const d = focus.add(offset, 'day');
    if (d.isBefore(rangeStart) || d.isAfter(rangeEnd)) continue;
    candidates.push(d.format('YYYY-MM-DD'));
  }
  return candidates;
}

export function computeRecommendQueryRange(params: {
  targetMachine: string;
  movable: MoveRecommendMovable[];
  candidates: string[];
}): { affectedMachines: string[]; dateFrom: string; dateTo: string } {
  const { targetMachine, movable, candidates } = params;

  const affectedMachines = Array.from(new Set<string>([targetMachine, ...(movable ?? []).map((it) => it.from_machine)])).sort();

  const originDates = (movable ?? []).map((it) => it.from_date).filter(Boolean).sort();
  const candidateDates = [...(candidates ?? [])].sort();
  const dateFrom = [originDates[0], candidateDates[0]].filter(Boolean).sort()[0] || candidateDates[0];
  const dateTo =
    [originDates[originDates.length - 1], candidateDates[candidateDates.length - 1]]
      .filter(Boolean)
      .sort()
      .slice(-1)[0] || candidateDates[candidateDates.length - 1];

  return { affectedMachines, dateFrom, dateTo };
}

export function buildCapacityPoolMap(pools: IpcCapacityPool[]): Map<string, CapacityLimit> {
  const poolMap = new Map<string, CapacityLimit>();
  (pools ?? []).forEach((p: IpcCapacityPool) => {
    const machine = String(p.machine_code ?? '').trim();
    const date = String(p.plan_date ?? '').trim();
    if (!machine || !date) return;
    const target = Number(p.target_capacity_t ?? 0);
    const limit = Number(p.limit_capacity_t ?? 0);
    poolMap.set(makeMachineDateKey(machine, date), {
      target: Number.isFinite(target) && target > 0 ? target : null,
      limit: Number.isFinite(limit) && limit > 0 ? limit : null,
    });
  });
  return poolMap;
}

export function scoreCandidateDates(params: {
  candidates: string[];
  deltaBase: Map<string, number>;
  totalWeight: number;
  targetMachine: string;
  tonnageMap: Map<string, number>;
  poolMap: Map<string, CapacityLimit>;
  focus: Dayjs;
}): CandidateScore[] {
  const { candidates, deltaBase, totalWeight, targetMachine, tonnageMap, poolMap, focus } = params;

  const scored = (candidates ?? [])
    .map((date) => {
      const deltaMap = new Map<string, number>(deltaBase);
      const toKey = makeMachineDateKey(targetMachine, date);
      deltaMap.set(toKey, (deltaMap.get(toKey) ?? 0) + totalWeight);

      const keys = Array.from(deltaMap.entries()).filter(([, d]) => Number.isFinite(d) && Math.abs(d) > 1e-9);
      if (keys.length === 0) return null;

      let overLimitCount = 0;
      let unknownCount = 0;
      let totalOverT = 0;
      let maxUtilPct = 0;

      keys.forEach(([key, delta]) => {
        const before = tonnageMap.get(key) ?? 0;
        const after = before + delta;
        const cap = poolMap.get(key);
        const limit = cap?.limit ?? cap?.target ?? null;
        if (limit == null || limit <= 0) {
          unknownCount += 1;
          return;
        }
        const pct = (after / limit) * 100;
        if (pct > maxUtilPct) maxUtilPct = pct;
        if (after > limit + 1e-9) {
          overLimitCount += 1;
          totalOverT += after - limit;
        }
      });

      const distance = Math.abs(dayjs(date).diff(focus, 'day'));
      return { date, overLimitCount, unknownCount, totalOverT, maxUtilPct, distance };
    })
    .filter(Boolean) as CandidateScore[];

  return scored;
}

export function pickBestCandidate(scored: CandidateScore[], strategy: string): CandidateScore | null {
  const list = [...(scored ?? [])];
  if (list.length === 0) return null;

  list.sort((a, b) => {
    if (a.overLimitCount !== b.overLimitCount) return a.overLimitCount - b.overLimitCount;
    if (a.unknownCount !== b.unknownCount) return a.unknownCount - b.unknownCount;

    if (strategy === 'capacity_first') {
      if (a.maxUtilPct !== b.maxUtilPct) return a.maxUtilPct - b.maxUtilPct;
      if (a.totalOverT !== b.totalOverT) return a.totalOverT - b.totalOverT;
      if (a.distance !== b.distance) return a.distance - b.distance;
    } else {
      if (a.totalOverT !== b.totalOverT) return a.totalOverT - b.totalOverT;
      if (a.distance !== b.distance) return a.distance - b.distance;
      if (a.maxUtilPct !== b.maxUtilPct) return a.maxUtilPct - b.maxUtilPct;
    }

    if (strategy === 'urgent_first') return a.date.localeCompare(b.date);
    if (strategy === 'cold_stock_first') return b.date.localeCompare(a.date);
    return a.date.localeCompare(b.date);
  });

  return list[0] || null;
}

export function buildRecommendSummary(targetMachine: string, best: CandidateScore): MoveRecommendSummary {
  return {
    machine: targetMachine,
    date: best.date,
    overLimitCount: best.overLimitCount,
    unknownCount: best.unknownCount,
    totalOverT: best.totalOverT,
    maxUtilPct: best.maxUtilPct,
  };
}

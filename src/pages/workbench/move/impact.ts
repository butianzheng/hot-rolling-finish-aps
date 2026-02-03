import type { Dayjs } from 'dayjs';

import { formatDate } from '../../../utils/formatters';
import type { MoveImpactRow, MoveValidationMode } from '../types';
import { buildPlanItemByIdMap, type IpcPlanItem } from './planItems';
import { buildMoveDeltaBase, buildTonnageMap, pickMovableItems } from './recommend';
import { makeMachineDateKey, splitMachineDateKey } from './key';

export type MoveImpactBase = {
  targetDate: string;
  affectedMachines: string[];
  dateFrom: string;
  dateTo: string;
  rows: MoveImpactRow[];
};

export function computeMoveImpactBase(params: {
  moveModalOpen: boolean;
  moveTargetMachine: string | null;
  moveTargetDate: Dayjs | null;
  moveValidationMode: MoveValidationMode;
  planItems: IpcPlanItem[];
  selectedMaterialIds: string[];
}): MoveImpactBase | null {
  const { moveModalOpen, moveTargetMachine, moveTargetDate, moveValidationMode, planItems, selectedMaterialIds } = params;

  if (!moveModalOpen) return null;
  const targetMachine = String(moveTargetMachine || '').trim();
  if (!targetMachine) return null;
  if (!moveTargetDate || !moveTargetDate.isValid()) return null;

  const targetDate = formatDate(moveTargetDate);
  const raw = planItems ?? [];

  const tonnageMap = buildTonnageMap(raw);
  const byId = buildPlanItemByIdMap(raw);
  const movable = pickMovableItems({ selectedMaterialIds, byId, moveValidationMode });
  const { totalWeight, deltaBase } = buildMoveDeltaBase(movable);

  const deltaMap = new Map<string, number>(deltaBase);
  const toKey = makeMachineDateKey(targetMachine, targetDate);
  deltaMap.set(toKey, (deltaMap.get(toKey) ?? 0) + totalWeight);

  const affectedKeys = Array.from(deltaMap.entries())
    .filter(([, delta]) => Number.isFinite(delta) && Math.abs(delta) > 1e-9)
    .map(([key]) => key);

  if (affectedKeys.length === 0) {
    return {
      targetDate,
      affectedMachines: [targetMachine],
      dateFrom: targetDate,
      dateTo: targetDate,
      rows: [],
    };
  }

  const dates = affectedKeys.map((k) => splitMachineDateKey(k).date).filter(Boolean).sort();
  const dateFrom = dates[0] || targetDate;
  const dateTo = dates[dates.length - 1] || targetDate;
  const affectedMachines = Array.from(
    new Set(affectedKeys.map((k) => splitMachineDateKey(k).machine).filter(Boolean))
  ).sort();

  const rows: MoveImpactRow[] = affectedKeys
    .map((key) => {
      const { machine, date } = splitMachineDateKey(key);
      const before = tonnageMap.get(key) ?? 0;
      const delta = deltaMap.get(key) ?? 0;
      const after = before + delta;
      return {
        machine_code: machine,
        date,
        before_t: before,
        delta_t: delta,
        after_t: after,
        target_capacity_t: null,
        limit_capacity_t: null,
      };
    })
    .sort((a, b) => (a.date === b.date ? a.machine_code.localeCompare(b.machine_code) : a.date.localeCompare(b.date)));

  return { targetDate, affectedMachines, dateFrom, dateTo, rows };
}

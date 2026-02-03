import { useMemo } from 'react';
import dayjs from 'dayjs';
import { useQuery } from '@tanstack/react-query';

import { capacityApi } from '../../../api/tauri';
import { formatDate } from '../../../utils/formatters';
import type { MoveImpactPreview, MoveImpactRow } from '../types';
import { buildPlanItemByIdMap, type IpcPlanItem } from '../move/planItems';
import { buildCapacityPoolMap, buildTonnageMap } from '../move/recommend';

type IpcCapacityPool = Awaited<ReturnType<typeof capacityApi.getCapacityPools>>[number];

type MoveImpactBase = {
  targetDate: string;
  affectedMachines: string[];
  dateFrom: string;
  dateTo: string;
  rows: MoveImpactRow[];
};

export function useWorkbenchMoveImpactPreview(params: {
  activeVersionId: string | null;
  moveModalOpen: boolean;
  moveTargetMachine: string | null;
  moveTargetDate: dayjs.Dayjs | null;
  planItems: IpcPlanItem[];
  selectedMaterialIds: string[];
}): MoveImpactPreview | null {
  const { activeVersionId, moveModalOpen, moveTargetMachine, moveTargetDate, planItems, selectedMaterialIds } = params;

  const moveImpactBase = useMemo<MoveImpactBase | null>(() => {
    if (!moveModalOpen) return null;
    const targetMachine = String(moveTargetMachine || '').trim();
    if (!targetMachine) return null;
    if (!moveTargetDate || !moveTargetDate.isValid()) return null;

    const targetDate = formatDate(moveTargetDate);
    const raw = planItems ?? [];

    const tonnageMap = buildTonnageMap(raw);
    const byId = buildPlanItemByIdMap(raw);

    const deltaMap = new Map<string, number>();
    selectedMaterialIds.forEach((id) => {
      const it = byId.get(id);
      if (!it) return;
      const fromMachine = String(it.machine_code ?? '').trim();
      const fromDate = String(it.plan_date ?? '').trim();
      if (!fromMachine || !fromDate) return;
      const weight = Number(it.weight_t ?? 0);
      if (!Number.isFinite(weight) || weight <= 0) return;

      const fromKey = `${fromMachine}__${fromDate}`;
      const toKey = `${targetMachine}__${targetDate}`;
      if (fromKey === toKey) return;
      deltaMap.set(fromKey, (deltaMap.get(fromKey) ?? 0) - weight);
      deltaMap.set(toKey, (deltaMap.get(toKey) ?? 0) + weight);
    });

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

    const dates = affectedKeys.map((k) => k.split('__')[1]).filter(Boolean).sort();
    const dateFrom = dates[0] || targetDate;
    const dateTo = dates[dates.length - 1] || targetDate;
    const affectedMachines = Array.from(new Set(affectedKeys.map((k) => k.split('__')[0]).filter(Boolean))).sort();

    const rows: MoveImpactRow[] = affectedKeys
      .map((key) => {
        const [machine, date] = key.split('__');
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
  }, [moveModalOpen, moveTargetMachine, moveTargetDate, planItems, selectedMaterialIds]);

  const moveImpactCapacityQuery = useQuery({
    queryKey: [
      'moveImpactCapacity',
      activeVersionId,
      moveImpactBase?.affectedMachines.join(',') || '',
      moveImpactBase?.dateFrom || '',
      moveImpactBase?.dateTo || '',
    ],
    enabled:
      !!activeVersionId &&
      !!moveModalOpen &&
      !!moveImpactBase &&
      moveImpactBase.affectedMachines.length > 0 &&
      !!moveImpactBase.dateFrom &&
      !!moveImpactBase.dateTo,
    queryFn: async () => {
      if (!activeVersionId || !moveImpactBase) return [];
      return capacityApi.getCapacityPools(
        moveImpactBase.affectedMachines,
        moveImpactBase.dateFrom,
        moveImpactBase.dateTo,
        activeVersionId
      );
    },
    staleTime: 30 * 1000,
  });

  return useMemo<MoveImpactPreview | null>(() => {
    if (!moveImpactBase) return null;
    const pools: IpcCapacityPool[] = moveImpactCapacityQuery.data ?? [];
    const poolMap = buildCapacityPoolMap(pools);

    const rows = moveImpactBase.rows.map((r) => {
      const cap = poolMap.get(`${r.machine_code}__${r.date}`);
      return {
        ...r,
        target_capacity_t: cap?.target ?? null,
        limit_capacity_t: cap?.limit ?? cap?.target ?? null,
      };
    });

    const overflowRows = rows.filter((r) => {
      if (r.limit_capacity_t == null) return false;
      return r.after_t > r.limit_capacity_t + 1e-9;
    });

    return { rows, overflowRows, loading: moveImpactCapacityQuery.isLoading };
  }, [moveImpactBase, moveImpactCapacityQuery.data, moveImpactCapacityQuery.isLoading]);
}

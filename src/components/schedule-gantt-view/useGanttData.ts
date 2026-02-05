/**
 * 甘特图数据处理 Hook
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Dayjs } from 'dayjs';
import dayjs from 'dayjs';
import { message } from 'antd';
import type { PlanItemRow, CellDetail } from './types';
import { MAX_DAYS } from './types';
import { normalizeDateKey, computeSuggestedRange } from './utils';
import type { PlanItemStatusFilter, PlanItemStatusSummary } from '../../utils/planItemStatus';
import { matchPlanItemStatusFilter } from '../../utils/planItemStatus';

interface UseGanttDataOptions {
  machineCode?: string | null;
  urgentLevel?: string | null;
  statusFilter?: PlanItemStatusFilter;
  planItems?: unknown;
  // 受控日期范围（工作台联动）；提供时，Hook 不再自行维护范围
  externalRange?: [Dayjs, Dayjs] | null;
  onExternalRangeChange?: (range: [Dayjs, Dayjs]) => void;
  // 受控模式下“重置范围”回到该值（通常由工作台提供）
  externalSuggestedRange?: [Dayjs, Dayjs];
  selectedMaterialIds: string[];
  onSelectedMaterialIdsChange: (ids: string[]) => void;
}

export function useGanttData({
  machineCode,
  urgentLevel,
  statusFilter = 'ALL',
  planItems,
  externalRange,
  onExternalRangeChange,
  externalSuggestedRange,
  selectedMaterialIds,
  onSelectedMaterialIdsChange,
}: UseGanttDataOptions) {
  // 标准化数据
  const normalized = useMemo<PlanItemRow[]>(() => {
    const raw = Array.isArray(planItems) ? planItems : [];
    return raw.map((it: unknown) => {
      const r = (it && typeof it === 'object' ? it : {}) as Record<string, unknown>;
      const widthMm = r.width_mm == null ? null : Number(r.width_mm);
      const thicknessMm = r.thickness_mm == null ? null : Number(r.thickness_mm);
      return {
        material_id: String(r.material_id ?? ''),
        machine_code: String(r.machine_code ?? ''),
        plan_date: String(r.plan_date ?? ''),
        seq_no: Number(r.seq_no ?? 0),
        weight_t: Number(r.weight_t ?? 0),
        width_mm: Number.isFinite(widthMm) ? widthMm : null,
        thickness_mm: Number.isFinite(thicknessMm) ? thicknessMm : null,
        urgent_level: r.urgent_level ? String(r.urgent_level) : undefined,
        locked_in_plan: !!r.locked_in_plan,
        force_release_in_plan: !!r.force_release_in_plan,
      };
    });
  }, [planItems]);

  // 可用日期键
  const availableDateKeys = useMemo(() => {
    const set = new Set<string>();
    normalized.forEach((it) => {
      const machine = String(it.machine_code || '').trim();
      if (!machine) return;
      if (machineCode && machineCode !== 'all' && machine !== machineCode) return;
      if (urgentLevel && urgentLevel !== 'all') {
        const want = String(urgentLevel).toUpperCase();
        if (String(it.urgent_level || 'L0').toUpperCase() !== want) return;
      }
      if (statusFilter && statusFilter !== 'ALL') {
        if (!matchPlanItemStatusFilter(it, statusFilter)) return;
      }
      const key = normalizeDateKey(it.plan_date);
      if (!key) return;
      set.add(key);
    });
    return Array.from(set);
  }, [machineCode, normalized, statusFilter, urgentLevel]);

  // 建议范围
  const suggestedRange = useMemo(() => computeSuggestedRange(availableDateKeys), [availableDateKeys]);
  const didUserAdjustRangeRef = useRef(false);
  const lastMachineRef = useRef<string | null | undefined>(undefined);

  // 状态
  const [internalRange, setInternalRange] = useState<[Dayjs, Dayjs]>(() => suggestedRange);
  const [cellDetail, setCellDetail] = useState<CellDetail>(null);

  // 机组变化时重置范围
  useEffect(() => {
    if (externalRange) return;
    const machineKey = machineCode ?? null;
    if (lastMachineRef.current !== machineKey) {
      lastMachineRef.current = machineKey;
      didUserAdjustRangeRef.current = false;
    }
    if (!didUserAdjustRangeRef.current) {
      setInternalRange(suggestedRange);
    }
  }, [externalRange, machineCode, suggestedRange]);

  const range = externalRange || internalRange;

  // 日期键数组
  const dateKeys = useMemo(() => {
    const [start, end] = range;
    const startDay = start.startOf('day');
    const endDay = end.startOf('day');
    if (!startDay.isValid() || !endDay.isValid()) return [];
    const days = endDay.diff(startDay, 'day') + 1;
    const limited = Math.max(0, Math.min(days, MAX_DAYS));
    return Array.from({ length: limited }, (_, idx) => startDay.add(idx, 'day').format('YYYY-MM-DD'));
  }, [range]);

  // 日期索引映射
  const dateIndexByKey = useMemo(() => {
    const map = new Map<string, number>();
    dateKeys.forEach((k, idx) => map.set(k, idx));
    return map;
  }, [dateKeys]);

  // 今天
  const todayKey = useMemo(() => dayjs().format('YYYY-MM-DD'), []);
  const todayIndex = dateIndexByKey.get(todayKey) ?? -1;

  // 机组和数据
  const { machines, itemsByMachineDate, filteredCount, filteredTotalWeight, statusSummary } = useMemo(() => {
    const byMachine = new Map<string, Map<string, PlanItemRow[]>>();
    const machineSet = new Set<string>();
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
    if (dateKeys.length === 0) {
      return {
        machines: [] as string[],
        itemsByMachineDate: byMachine,
        filteredCount: 0,
        filteredTotalWeight: 0,
        statusSummary: summary,
      };
    }

    if (machineCode && machineCode !== 'all') {
      machineSet.add(machineCode);
    }

    const startKey = dateKeys[0];
    const endKey = dateKeys[dateKeys.length - 1];
    let count = 0;
    let totalWeight = 0;

    normalized.forEach((it) => {
      const machine = String(it.machine_code || '').trim();
      if (!machine) return;
      if (machineCode && machineCode !== 'all' && machine !== machineCode) return;
      if (urgentLevel && urgentLevel !== 'all') {
        const want = String(urgentLevel).toUpperCase();
        if (String(it.urgent_level || 'L0').toUpperCase() !== want) return;
      }
      const dateKey = normalizeDateKey(it.plan_date);
      if (!dateKey) return;
      if (dateKey < startKey || dateKey > endKey) return;

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

      if (statusFilter && statusFilter !== 'ALL') {
        if (!matchPlanItemStatusFilter(it, statusFilter)) return;
      }

      machineSet.add(machine);
      let byDate = byMachine.get(machine);
      if (!byDate) {
        byDate = new Map();
        byMachine.set(machine, byDate);
      }
      const list = byDate.get(dateKey);
      if (list) list.push(it);
      else byDate.set(dateKey, [it]);
      count += 1;
      totalWeight += weight;
    });

    byMachine.forEach((byDate) => {
      byDate.forEach((list) => {
        list.sort((a, b) => a.seq_no - b.seq_no);
      });
    });

    return {
      machines: Array.from(machineSet).sort(),
      itemsByMachineDate: byMachine,
      filteredCount: count,
      filteredTotalWeight: totalWeight,
      statusSummary: summary,
    };
  }, [dateKeys, machineCode, normalized, statusFilter, urgentLevel]);

  // 选中集合
  const selectedSet = useMemo(() => new Set(selectedMaterialIds), [selectedMaterialIds]);

  // 切换选择
  const toggleSelection = useCallback(
    (materialId: string, checked: boolean) => {
      const next = new Set(selectedSet);
      if (checked) next.add(materialId);
      else next.delete(materialId);
      onSelectedMaterialIdsChange(Array.from(next));
    },
    [onSelectedMaterialIdsChange, selectedSet]
  );

  // 范围变更
  const onRangeChange = useCallback(
    (values: null | [Dayjs | null, Dayjs | null]) => {
      if (!values || !values[0] || !values[1]) {
        if (externalRange) {
          onExternalRangeChange?.(externalSuggestedRange || suggestedRange);
          return;
        }
        didUserAdjustRangeRef.current = false;
        setInternalRange(suggestedRange);
        return;
      }
      let start = values[0].startOf('day');
      let end = values[1].startOf('day');
      if (end.isBefore(start)) {
        const tmp = start;
        start = end;
        end = tmp;
      }
      const days = end.diff(start, 'day') + 1;
      if (days > MAX_DAYS) {
        message.warning(`时间跨度过大，已限制为${MAX_DAYS}天`);
        end = start.add(MAX_DAYS - 1, 'day');
      }
      if (externalRange) {
        onExternalRangeChange?.([start, end]);
        return;
      }
      didUserAdjustRangeRef.current = true;
      setInternalRange([start, end]);
    },
    [externalRange, externalSuggestedRange, onExternalRangeChange, suggestedRange]
  );

  // 重置范围
  const resetRange = useCallback(() => {
    if (externalRange) {
      onExternalRangeChange?.(externalSuggestedRange || suggestedRange);
      return;
    }
    didUserAdjustRangeRef.current = false;
    setInternalRange(suggestedRange);
  }, [externalRange, externalSuggestedRange, onExternalRangeChange, suggestedRange]);

  return {
    normalized,
    dateKeys,
    dateIndexByKey,
    todayKey,
    todayIndex,
    machines,
    itemsByMachineDate,
    filteredCount,
    filteredTotalWeight,
    statusSummary,
    selectedSet,
    toggleSelection,
    range,
    onRangeChange,
    resetRange,
    cellDetail,
    setCellDetail,
  };
}

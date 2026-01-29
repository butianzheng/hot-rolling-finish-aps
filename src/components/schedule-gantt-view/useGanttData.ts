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

interface UseGanttDataOptions {
  machineCode?: string | null;
  urgentLevel?: string | null;
  planItems?: unknown;
  selectedMaterialIds: string[];
  onSelectedMaterialIdsChange: (ids: string[]) => void;
}

export function useGanttData({
  machineCode,
  urgentLevel,
  planItems,
  selectedMaterialIds,
  onSelectedMaterialIdsChange,
}: UseGanttDataOptions) {
  // 标准化数据
  const normalized = useMemo<PlanItemRow[]>(() => {
    const raw = Array.isArray(planItems) ? planItems : [];
    return raw.map((it: any) => ({
      material_id: String(it?.material_id ?? ''),
      machine_code: String(it?.machine_code ?? ''),
      plan_date: String(it?.plan_date ?? ''),
      seq_no: Number(it?.seq_no ?? 0),
      weight_t: Number(it?.weight_t ?? 0),
      urgent_level: it?.urgent_level ? String(it.urgent_level) : undefined,
      locked_in_plan: !!it?.locked_in_plan,
      force_release_in_plan: !!it?.force_release_in_plan,
    }));
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
      const key = normalizeDateKey(it.plan_date);
      if (!key) return;
      set.add(key);
    });
    return Array.from(set);
  }, [machineCode, normalized, urgentLevel]);

  // 建议范围
  const suggestedRange = useMemo(() => computeSuggestedRange(availableDateKeys), [availableDateKeys]);
  const didUserAdjustRangeRef = useRef(false);
  const lastMachineRef = useRef<string | null | undefined>(undefined);

  // 状态
  const [range, setRange] = useState<[Dayjs, Dayjs]>(() => suggestedRange);
  const [cellDetail, setCellDetail] = useState<CellDetail>(null);

  // 机组变化时重置范围
  useEffect(() => {
    const machineKey = machineCode ?? null;
    if (lastMachineRef.current !== machineKey) {
      lastMachineRef.current = machineKey;
      didUserAdjustRangeRef.current = false;
    }
    if (!didUserAdjustRangeRef.current) {
      setRange(suggestedRange);
    }
  }, [machineCode, suggestedRange]);

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
  const { machines, itemsByMachineDate, filteredCount, filteredTotalWeight } = useMemo(() => {
    const byMachine = new Map<string, Map<string, PlanItemRow[]>>();
    const machineSet = new Set<string>();
    if (dateKeys.length === 0) {
      return { machines: [] as string[], itemsByMachineDate: byMachine, filteredCount: 0, filteredTotalWeight: 0 };
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
      totalWeight += Number(it.weight_t || 0);
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
    };
  }, [dateKeys, machineCode, normalized, urgentLevel]);

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
        didUserAdjustRangeRef.current = false;
        setRange(suggestedRange);
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
      didUserAdjustRangeRef.current = true;
      setRange([start, end]);
    },
    [suggestedRange]
  );

  // 重置范围
  const resetRange = useCallback(() => {
    didUserAdjustRangeRef.current = false;
    setRange(suggestedRange);
  }, [suggestedRange]);

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
    selectedSet,
    toggleSelection,
    range,
    onRangeChange,
    resetRange,
    cellDetail,
    setCellDetail,
  };
}

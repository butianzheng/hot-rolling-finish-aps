/**
 * 树形分解图数据构建 Hook
 *
 * 将扁平的 PlanItemRow[] 转换为 机组→日期（含状态占比） 的层级结构
 */

import { useState, useMemo, useCallback } from 'react';
import type { PlanItemRow, ScheduleTreeRow, DateStatusSummary } from './types';

export interface UseScheduleTreeResult {
  rows: ScheduleTreeRow[];
  toggleMachine: (machineCode: string) => void;
  expandAll: () => void;
  collapseAll: () => void;
}

function computeDateStatus(items: PlanItemRow[]): DateStatusSummary {
  let lockedCount = 0;
  let forceReleaseCount = 0;
  let adjustableCount = 0;
  for (const it of items) {
    if (it.locked_in_plan) lockedCount++;
    else if (it.force_release_in_plan) forceReleaseCount++;
    else adjustableCount++;
  }
  return { lockedCount, forceReleaseCount, adjustableCount };
}

export function useScheduleTree(items: PlanItemRow[]): UseScheduleTreeResult {
  // 机组折叠状态：true = 折叠
  const [collapsedMachines, setCollapsedMachines] = useState<Record<string, boolean>>({});

  const toggleMachine = useCallback((machineCode: string) => {
    setCollapsedMachines((prev) => ({
      ...prev,
      [machineCode]: !prev[machineCode],
    }));
  }, []);

  const expandAll = useCallback(() => {
    setCollapsedMachines({});
  }, []);

  const collapseAll = useCallback(() => {
    const machines: Record<string, boolean> = {};
    items.forEach((it) => {
      if (it.machine_code) machines[it.machine_code] = true;
    });
    setCollapsedMachines(machines);
  }, [items]);

  const rows = useMemo<ScheduleTreeRow[]>(() => {
    // 1. 按机组分组
    const byMachine = new Map<string, PlanItemRow[]>();
    items.forEach((it) => {
      const machine = it.machine_code || '';
      if (!byMachine.has(machine)) {
        byMachine.set(machine, []);
      }
      byMachine.get(machine)!.push(it);
    });

    // 2. 机组排序
    const machinesSorted = Array.from(byMachine.keys()).sort();

    const result: ScheduleTreeRow[] = [];

    for (const machineCode of machinesSorted) {
      const machineItems = byMachine.get(machineCode)!;
      const machineWeight = machineItems.reduce((s, it) => s + (it.weight_t || 0), 0);
      const machineCollapsed = !!collapsedMachines[machineCode];

      result.push({
        type: 'machine',
        machineCode,
        count: machineItems.length,
        weightT: machineWeight,
        collapsed: machineCollapsed,
      });

      if (machineCollapsed) continue;

      // 3. 按日期分组
      const byDate = new Map<string, PlanItemRow[]>();
      machineItems.forEach((it) => {
        const date = it.plan_date || '';
        if (!byDate.has(date)) {
          byDate.set(date, []);
        }
        byDate.get(date)!.push(it);
      });

      // 4. 日期排序，每个日期一行（含状态占比）
      const datesSorted = Array.from(byDate.keys()).sort();

      for (const date of datesSorted) {
        const dateItems = byDate.get(date)!;
        const dateWeight = dateItems.reduce((s, it) => s + (it.weight_t || 0), 0);

        result.push({
          type: 'date',
          machineCode,
          date,
          count: dateItems.length,
          weightT: dateWeight,
          status: computeDateStatus(dateItems),
        });
      }
    }

    return result;
  }, [items, collapsedMachines]);

  return {
    rows,
    toggleMachine,
    expandAll,
    collapseAll,
  };
}

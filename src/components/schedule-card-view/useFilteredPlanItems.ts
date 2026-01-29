import { useMemo } from 'react';
import type { PlanItemRow } from './types';

export const useFilteredPlanItems = (
  items: PlanItemRow[],
  machineCode?: string | null,
  urgentLevel?: string | null
): PlanItemRow[] => {
  return useMemo(() => {
    let list = items;
    if (machineCode && machineCode !== 'all') {
      list = list.filter((it) => it.machine_code === machineCode);
    }
    if (urgentLevel && urgentLevel !== 'all') {
      const want = String(urgentLevel).toUpperCase();
      list = list.filter((it) => String(it.urgent_level || 'L0').toUpperCase() === want);
    }
    return [...list].sort((a, b) => {
      if (a.plan_date !== b.plan_date) return a.plan_date.localeCompare(b.plan_date);
      if (a.machine_code !== b.machine_code) return a.machine_code.localeCompare(b.machine_code);
      return a.seq_no - b.seq_no;
    });
  }, [items, machineCode, urgentLevel]);
};

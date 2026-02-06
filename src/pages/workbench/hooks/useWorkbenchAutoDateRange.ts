import { useCallback, useEffect, useMemo } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import dayjs from 'dayjs';
import { useQuery } from '@tanstack/react-query';
import { planApi } from '../../../api/tauri';
import { formatDate } from '../../../utils/formatters';
import type { WorkbenchDateRangeMode } from '../types';
import { workbenchQueryKeys } from '../queryKeys';

export function useWorkbenchAutoDateRange(params: {
  activeVersionId: string | null;
  machineCode: string | null;
  dateRangeMode: WorkbenchDateRangeMode;
  setDateRangeMode: Dispatch<SetStateAction<WorkbenchDateRangeMode>>;
  setWorkbenchDateRange: Dispatch<SetStateAction<[dayjs.Dayjs, dayjs.Dayjs]>>;
}): {
  autoDateRange: [dayjs.Dayjs, dayjs.Dayjs];
  applyWorkbenchDateRange: (next: [dayjs.Dayjs, dayjs.Dayjs]) => void;
  resetWorkbenchDateRangeToAuto: () => void;
} {
  const { activeVersionId, machineCode, dateRangeMode, setDateRangeMode, setWorkbenchDateRange } = params;

  const normalizedMachineCode =
    machineCode && machineCode !== 'all' ? String(machineCode).trim() : undefined;

  const boundsQuery = useQuery({
    queryKey: workbenchQueryKeys.planItems.dateBounds(activeVersionId, normalizedMachineCode ?? null),
    enabled: !!activeVersionId,
    queryFn: async () => {
      if (!activeVersionId) return null;
      return planApi.getPlanItemDateBounds(activeVersionId, normalizedMachineCode);
    },
    staleTime: 30 * 1000,
  });

  const autoDateRange = useMemo<[dayjs.Dayjs, dayjs.Dayjs]>(() => {
    const fallback: [dayjs.Dayjs, dayjs.Dayjs] = [dayjs().subtract(3, 'day'), dayjs().add(10, 'day')];

    const minStr = boundsQuery.data?.min_plan_date || null;
    const maxStr = boundsQuery.data?.max_plan_date || null;
    if (!minStr || !maxStr) return fallback;

    const min = dayjs(minStr);
    const max = dayjs(maxStr);
    if (!min.isValid() || !max.isValid()) return fallback;

    const minDate = min.subtract(1, 'day'); // 前面留 1 天余量
    const maxDate = max.add(3, 'day'); // 后面留 3 天余量
    return [minDate, maxDate];
  }, [boundsQuery.data?.max_plan_date, boundsQuery.data?.min_plan_date]);

  useEffect(() => {
    if (dateRangeMode !== 'AUTO') return;
    setWorkbenchDateRange(autoDateRange);
  }, [autoDateRange, dateRangeMode, setWorkbenchDateRange]);

  const applyWorkbenchDateRange = useCallback((next: [dayjs.Dayjs, dayjs.Dayjs]) => {
    if (!next || !next[0] || !next[1]) return;
    let start = next[0].startOf('day');
    let end = next[1].startOf('day');
    if (end.isBefore(start)) {
      const tmp = start;
      start = end;
      end = tmp;
    }
    setWorkbenchDateRange([start, end]);

    // C3修复：如果当前处于PINNED模式，用户手动调整日期应该切换到MANUAL模式，
    // 而不是AUTO模式，以保持用户的意图
    if (dateRangeMode === 'PINNED') {
      setDateRangeMode('MANUAL');
      return;
    }

    const isAuto =
      formatDate(start) === formatDate(autoDateRange[0]) &&
      formatDate(end) === formatDate(autoDateRange[1]);
    setDateRangeMode(isAuto ? 'AUTO' : 'MANUAL');
  }, [autoDateRange, dateRangeMode, setDateRangeMode, setWorkbenchDateRange]);

  const resetWorkbenchDateRangeToAuto = useCallback(() => {
    setDateRangeMode('AUTO');
    setWorkbenchDateRange(autoDateRange);
  }, [autoDateRange, setDateRangeMode, setWorkbenchDateRange]);

  return {
    autoDateRange,
    applyWorkbenchDateRange,
    resetWorkbenchDateRangeToAuto,
  };
}

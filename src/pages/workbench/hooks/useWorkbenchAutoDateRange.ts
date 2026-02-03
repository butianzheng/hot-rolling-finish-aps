import { useCallback, useEffect, useMemo } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import dayjs from 'dayjs';
import { formatDate } from '../../../utils/formatters';
import type { WorkbenchDateRangeMode } from '../types';

type PlanItemLike = {
  machine_code: string;
  plan_date: string;
};

export function useWorkbenchAutoDateRange(params: {
  planItems: PlanItemLike[];
  machineCode: string | null;
  dateRangeMode: WorkbenchDateRangeMode;
  setDateRangeMode: Dispatch<SetStateAction<WorkbenchDateRangeMode>>;
  setWorkbenchDateRange: Dispatch<SetStateAction<[dayjs.Dayjs, dayjs.Dayjs]>>;
}): {
  autoDateRange: [dayjs.Dayjs, dayjs.Dayjs];
  applyWorkbenchDateRange: (next: [dayjs.Dayjs, dayjs.Dayjs]) => void;
  resetWorkbenchDateRangeToAuto: () => void;
} {
  const { planItems, machineCode, dateRangeMode, setDateRangeMode, setWorkbenchDateRange } = params;

  const autoDateRange = useMemo<[dayjs.Dayjs, dayjs.Dayjs]>(() => {
    const filteredItems = (planItems || []).filter(
      (item) => !machineCode || machineCode === 'all' || item.machine_code === machineCode
    );

    if (filteredItems.length === 0) {
      // 默认日期范围：今天前 3 天到后 10 天
      return [dayjs().subtract(3, 'day'), dayjs().add(10, 'day')];
    }

    // 提取所有排程日期
    const dates = filteredItems
      .map((item) => dayjs(item.plan_date))
      .filter((d) => d.isValid());

    if (dates.length === 0) {
      return [dayjs().subtract(3, 'day'), dayjs().add(10, 'day')];
    }

    // 找到最早和最晚的日期
    const sortedDates = dates.sort((a, b) => a.valueOf() - b.valueOf());
    const minDate = sortedDates[0].subtract(1, 'day'); // 前面留 1 天余量
    const maxDate = sortedDates[sortedDates.length - 1].add(3, 'day'); // 后面留 3 天余量

    return [minDate, maxDate];
  }, [machineCode, planItems]);

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
    const isAuto =
      formatDate(start) === formatDate(autoDateRange[0]) &&
      formatDate(end) === formatDate(autoDateRange[1]);
    setDateRangeMode(isAuto ? 'AUTO' : 'MANUAL');
  }, [autoDateRange, setDateRangeMode, setWorkbenchDateRange]);

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

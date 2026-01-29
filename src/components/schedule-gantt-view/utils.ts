/**
 * ScheduleGanttView 工具函数
 */

import type { Dayjs } from 'dayjs';
import dayjs from 'dayjs';
import { URGENCY_COLORS } from '../../theme/tokens';

/**
 * 标准化日期键
 */
export function normalizeDateKey(value: string): string {
  const trimmed = String(value || '').trim();
  if (!trimmed) return '';
  if (/^\d{4}-\d{2}-\d{2}/.test(trimmed)) return trimmed.slice(0, 10);
  return trimmed;
}

/**
 * 紧急级别转颜色
 */
export function urgencyToColor(level: string | undefined): { background: string; text: string } {
  const urgent = String(level || 'L0');
  if (urgent === 'L3') return { background: URGENCY_COLORS.L3_EMERGENCY, text: '#fff' };
  if (urgent === 'L2') return { background: URGENCY_COLORS.L2_HIGH, text: 'rgba(0, 0, 0, 0.85)' };
  if (urgent === 'L1') return { background: URGENCY_COLORS.L1_MEDIUM, text: '#fff' };
  return { background: URGENCY_COLORS.L0_NORMAL, text: '#fff' };
}

/**
 * 计算建议的日期范围
 */
export function computeSuggestedRange(dateKeys: string[]): [Dayjs, Dayjs] {
  const today = dayjs().startOf('day');
  if (dateKeys.length === 0) {
    return [today, today.add(13, 'day')];
  }
  const sorted = [...dateKeys].sort();
  const min = dayjs(sorted[0]).startOf('day');
  const max = dayjs(sorted[sorted.length - 1]).startOf('day');
  if (!min.isValid() || !max.isValid()) {
    return [today, today.add(13, 'day')];
  }
  const totalDays = max.diff(min, 'day') + 1;
  if (totalDays <= 14) {
    return [min, max];
  }

  let start = today.subtract(3, 'day');
  let end = start.add(13, 'day');
  if (end.isBefore(min)) {
    start = min;
    end = min.add(13, 'day');
  }
  if (start.isAfter(max)) {
    end = max;
    start = max.subtract(13, 'day');
  }
  if (start.isBefore(min)) start = min;
  if (end.isAfter(max)) end = max;
  if (end.isBefore(start)) {
    start = min;
    const candidate = min.add(13, 'day');
    end = candidate.isAfter(max) ? max : candidate;
  }
  return [start, end];
}

/**
 * 获取紧急级别对应的Tag颜色
 */
export function getUrgentTagColor(urgent: string): string {
  if (urgent === 'L3') return 'red';
  if (urgent === 'L2') return 'orange';
  if (urgent === 'L1') return 'blue';
  return 'default';
}

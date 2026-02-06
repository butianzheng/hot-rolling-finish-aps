// ==========================================
// 统一数据格式化工具
// ==========================================
// 职责: 提供统一的数据格式化函数
// ==========================================

import dayjs, { Dayjs } from 'dayjs';

const numberFormatterCache = new Map<string, Intl.NumberFormat>();

function getNumberFormatter(decimals: number, useGrouping: boolean): Intl.NumberFormat {
  const safeDecimals = Number.isFinite(decimals) ? Math.max(0, Math.floor(decimals)) : 0;
  const cacheKey = `${safeDecimals}-${useGrouping ? 'group' : 'plain'}`;
  const cached = numberFormatterCache.get(cacheKey);
  if (cached) {
    return cached;
  }

  const formatter = new Intl.NumberFormat('zh-CN', {
    minimumFractionDigits: safeDecimals,
    maximumFractionDigits: safeDecimals,
    useGrouping,
  });
  numberFormatterCache.set(cacheKey, formatter);
  return formatter;
}

// ==========================================
// 日期格式化
// ==========================================

/**
 * 格式化日期为 YYYY-MM-DD
 */
export const formatDate = (date: string | Dayjs | Date): string => {
  return dayjs(date).format('YYYY-MM-DD');
};

/**
 * 格式化日期时间为 YYYY-MM-DD HH:mm:ss
 */
export const formatDateTime = (date: string | Dayjs | Date): string => {
  return dayjs(date).format('YYYY-MM-DD HH:mm:ss');
};

// ==========================================
// 数字格式化
// ==========================================

/**
 * 格式化数字为指定小数位
 */
export const formatNumber = (
  value: number | null | undefined,
  decimals: number = 1,
  options?: { useGrouping?: boolean }
): string => {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return '-';
  }
  const useGrouping = options?.useGrouping ?? true;
  return getNumberFormatter(decimals, useGrouping).format(value);
};

/**
 * 格式化吨位（四舍五入保留3位小数）
 */
export const formatWeight = (value: number | null | undefined): string => {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return '-';
  }
  return `${formatNumber(value, 3)}吨`;
};

/**
 * 格式化百分比（保留1位小数）
 */
export const formatPercent = (value: number | null | undefined): string => {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return '-';
  }
  return `${formatNumber(value, 1)}%`;
};

/**
 * 格式化产能（四舍五入保留3位小数）
 */
export const formatCapacity = (value: number | null | undefined): string => {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return '-';
  }
  return formatNumber(value, 3);
};

// ==========================================
// 统一数据格式化工具
// ==========================================
// 职责: 提供统一的数据格式化函数
// ==========================================

import dayjs, { Dayjs } from 'dayjs';

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
export const formatNumber = (value: number | null | undefined, decimals: number = 1): string => {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return '-';
  }
  return value.toFixed(decimals);
};

/**
 * 格式化吨位（保留3位小数）
 */
export const formatWeight = (value: number | null | undefined): string => {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return '-';
  }
  return `${value.toFixed(3)}t`;
};

/**
 * 格式化百分比（保留1位小数）
 */
export const formatPercent = (value: number | null | undefined): string => {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return '-';
  }
  return `${value.toFixed(1)}%`;
};

/**
 * 格式化产能（保留1位小数）
 */
export const formatCapacity = (value: number | null | undefined): string => {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return '-';
  }
  return value.toFixed(1);
};

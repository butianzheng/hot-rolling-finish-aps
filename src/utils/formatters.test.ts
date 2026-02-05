import { describe, it, expect } from 'vitest';
import {
  formatDate,
  formatDateTime,
  formatNumber,
  formatWeight,
  formatPercent,
  formatCapacity,
} from './formatters';
import dayjs from 'dayjs';

describe('formatters', () => {
  describe('formatDate', () => {
    it('应该格式化日期字符串为 YYYY-MM-DD', () => {
      expect(formatDate('2026-02-04')).toBe('2026-02-04');
      expect(formatDate('2026-02-04 15:30:00')).toBe('2026-02-04');
    });

    it('应该格式化 Date 对象为 YYYY-MM-DD', () => {
      const date = new Date('2026-02-04T15:30:00');
      expect(formatDate(date)).toBe('2026-02-04');
    });

    it('应该格式化 Dayjs 对象为 YYYY-MM-DD', () => {
      const date = dayjs('2026-02-04 15:30:00');
      expect(formatDate(date)).toBe('2026-02-04');
    });

    it('应该处理不同的日期格式', () => {
      expect(formatDate('2026/02/04')).toBe('2026-02-04');
      expect(formatDate('20260204')).toBe('2026-02-04');
    });
  });

  describe('formatDateTime', () => {
    it('应该格式化日期时间为 YYYY-MM-DD HH:mm:ss', () => {
      expect(formatDateTime('2026-02-04 15:30:45')).toBe('2026-02-04 15:30:45');
    });

    it('应该格式化 Date 对象为 YYYY-MM-DD HH:mm:ss', () => {
      const date = new Date('2026-02-04T15:30:45');
      const result = formatDateTime(date);
      expect(result).toMatch(/2026-02-04 \d{2}:\d{2}:\d{2}/);
    });

    it('应该格式化 Dayjs 对象为 YYYY-MM-DD HH:mm:ss', () => {
      const date = dayjs('2026-02-04 15:30:45');
      expect(formatDateTime(date)).toBe('2026-02-04 15:30:45');
    });

    it('应该包含时分秒', () => {
      const result = formatDateTime('2026-02-04 08:05:03');
      expect(result).toBe('2026-02-04 08:05:03');
    });
  });

  describe('formatNumber', () => {
    it('应该格式化数字为指定小数位（默认1位）', () => {
      expect(formatNumber(123.456)).toBe('123.5');
      expect(formatNumber(123.456, 2)).toBe('123.46');
      expect(formatNumber(123.456, 0)).toBe('123');
    });

    it('应该处理整数', () => {
      expect(formatNumber(123)).toBe('123.0');
      expect(formatNumber(123, 2)).toBe('123.00');
    });

    it('应该处理 null 返回 -', () => {
      expect(formatNumber(null)).toBe('-');
    });

    it('应该处理 undefined 返回 -', () => {
      expect(formatNumber(undefined)).toBe('-');
    });

    it('应该处理 NaN 返回 -', () => {
      expect(formatNumber(NaN)).toBe('-');
    });

    it('应该处理 Infinity 返回 -', () => {
      expect(formatNumber(Infinity)).toBe('-');
      expect(formatNumber(-Infinity)).toBe('-');
    });

    it('应该处理负数', () => {
      expect(formatNumber(-123.456)).toBe('-123.5');
      expect(formatNumber(-123.456, 2)).toBe('-123.46');
    });

    it('应该处理零', () => {
      expect(formatNumber(0)).toBe('0.0');
      expect(formatNumber(0, 2)).toBe('0.00');
    });
  });

  describe('formatWeight', () => {
    it('应该格式化吨位（保留2位小数）', () => {
      expect(formatWeight(123.456789)).toBe('123.46t');
      expect(formatWeight(100.123456)).toBe('100.12t');
    });

    it('应该处理整数吨位', () => {
      expect(formatWeight(100)).toBe('100.00t');
    });

    it('应该处理小数吨位', () => {
      expect(formatWeight(0.123)).toBe('0.12t');
      expect(formatWeight(0.001)).toBe('0.00t');
    });

    it('应该处理 null 返回 -', () => {
      expect(formatWeight(null)).toBe('-');
    });

    it('应该处理 undefined 返回 -', () => {
      expect(formatWeight(undefined)).toBe('-');
    });

    it('应该处理 NaN 返回 -', () => {
      expect(formatWeight(NaN)).toBe('-');
    });

    it('应该处理 Infinity 返回 -', () => {
      expect(formatWeight(Infinity)).toBe('-');
      expect(formatWeight(-Infinity)).toBe('-');
    });

    it('应该处理负数吨位', () => {
      expect(formatWeight(-50.123)).toBe('-50.12t');
    });

    it('应该处理零吨位', () => {
      expect(formatWeight(0)).toBe('0.00t');
    });
  });

  describe('formatPercent', () => {
    it('应该格式化百分比（保留1位小数）', () => {
      expect(formatPercent(85.678)).toBe('85.7%');
      expect(formatPercent(100.12)).toBe('100.1%');
    });

    it('应该处理整数百分比', () => {
      expect(formatPercent(50)).toBe('50.0%');
      expect(formatPercent(100)).toBe('100.0%');
    });

    it('应该处理小数百分比', () => {
      expect(formatPercent(0.5)).toBe('0.5%');
      expect(formatPercent(99.99)).toBe('100.0%');
    });

    it('应该处理 null 返回 -', () => {
      expect(formatPercent(null)).toBe('-');
    });

    it('应该处理 undefined 返回 -', () => {
      expect(formatPercent(undefined)).toBe('-');
    });

    it('应该处理 NaN 返回 -', () => {
      expect(formatPercent(NaN)).toBe('-');
    });

    it('应该处理 Infinity 返回 -', () => {
      expect(formatPercent(Infinity)).toBe('-');
      expect(formatPercent(-Infinity)).toBe('-');
    });

    it('应该处理零百分比', () => {
      expect(formatPercent(0)).toBe('0.0%');
    });

    it('应该处理超过 100% 的情况', () => {
      expect(formatPercent(150.5)).toBe('150.5%');
    });
  });

  describe('formatCapacity', () => {
    it('应该格式化产能（保留2位小数）', () => {
      expect(formatCapacity(1234.567)).toBe('1234.57');
      expect(formatCapacity(100.12)).toBe('100.12');
    });

    it('应该处理整数产能', () => {
      expect(formatCapacity(1000)).toBe('1000.00');
    });

    it('应该处理小数产能', () => {
      expect(formatCapacity(0.5)).toBe('0.50');
      expect(formatCapacity(99.99)).toBe('99.99');
    });

    it('应该处理 null 返回 -', () => {
      expect(formatCapacity(null)).toBe('-');
    });

    it('应该处理 undefined 返回 -', () => {
      expect(formatCapacity(undefined)).toBe('-');
    });

    it('应该处理 NaN 返回 -', () => {
      expect(formatCapacity(NaN)).toBe('-');
    });

    it('应该处理 Infinity 返回 -', () => {
      expect(formatCapacity(Infinity)).toBe('-');
      expect(formatCapacity(-Infinity)).toBe('-');
    });

    it('应该处理零产能', () => {
      expect(formatCapacity(0)).toBe('0.00');
    });

    it('应该处理负数产能', () => {
      expect(formatCapacity(-50.5)).toBe('-50.50');
    });
  });
});

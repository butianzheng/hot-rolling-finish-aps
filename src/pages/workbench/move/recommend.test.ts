/**
 * workbench/move/recommend.ts 单元测试
 */

import dayjs from 'dayjs';
import { describe, it, expect } from 'vitest';
import { buildCandidateDates, pickBestCandidate } from './recommend';

describe('workbench/move/recommend', () => {
  describe('buildCandidateDates', () => {
    it('范围较小时返回完整范围（含首尾）', () => {
      const focus = dayjs('2026-02-01');
      const rangeStart = dayjs('2026-02-01');
      const rangeEnd = dayjs('2026-02-05');
      const dates = buildCandidateDates({ focus, rangeStart, rangeEnd });
      expect(dates).toEqual(['2026-02-01', '2026-02-02', '2026-02-03', '2026-02-04', '2026-02-05']);
    });

    it('范围较大时返回围绕 focus 的窗口，并裁剪到范围内', () => {
      const focus = dayjs('2026-01-10');
      const rangeStart = dayjs('2026-01-01');
      const rangeEnd = dayjs('2026-02-20');
      const dates = buildCandidateDates({ focus, rangeStart, rangeEnd, radius: 15 });
      expect(dates[0]).toBe('2026-01-01'); // 被裁剪到 rangeStart
      expect(dates[dates.length - 1]).toBe('2026-01-25');
      expect(dates).toHaveLength(25);
    });
  });

  describe('pickBestCandidate', () => {
    it('空列表返回 null', () => {
      expect(pickBestCandidate([], 'balanced')).toBeNull();
    });

    it('capacity_first 优先选择更低 maxUtilPct', () => {
      const scored = [
        { date: '2026-02-01', overLimitCount: 0, unknownCount: 0, totalOverT: 0, maxUtilPct: 60, distance: 1 },
        { date: '2026-02-02', overLimitCount: 0, unknownCount: 0, totalOverT: 99, maxUtilPct: 40, distance: 2 },
      ];
      expect(pickBestCandidate(scored as any, 'capacity_first')?.date).toBe('2026-02-02');
    });

    it('balanced 优先选择更低 totalOverT', () => {
      const scored = [
        { date: '2026-02-01', overLimitCount: 0, unknownCount: 0, totalOverT: 5, maxUtilPct: 10, distance: 0 },
        { date: '2026-02-02', overLimitCount: 0, unknownCount: 0, totalOverT: 0, maxUtilPct: 90, distance: 10 },
      ];
      expect(pickBestCandidate(scored as any, 'balanced')?.date).toBe('2026-02-02');
    });

    it('urgent_first / cold_stock_first 在完全平局时按日期早/晚偏好', () => {
      const tie = [
        { date: '2026-02-01', overLimitCount: 0, unknownCount: 0, totalOverT: 0, maxUtilPct: 50, distance: 0 },
        { date: '2026-02-10', overLimitCount: 0, unknownCount: 0, totalOverT: 0, maxUtilPct: 50, distance: 0 },
      ];
      expect(pickBestCandidate(tie as any, 'urgent_first')?.date).toBe('2026-02-01');
      expect(pickBestCandidate(tie as any, 'cold_stock_first')?.date).toBe('2026-02-10');
    });
  });
});


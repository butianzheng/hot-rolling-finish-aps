/**
 * workbench/move/recommend.ts 单元测试
 */

import dayjs from 'dayjs';
import { describe, it, expect } from 'vitest';
import {
  buildCandidateDates,
  buildCapacityPoolMap,
  buildTonnageMap,
  pickBestCandidate,
  pickMovableItems,
  scoreCandidateDates,
} from './recommend';
import { makeMachineDateKey } from './key';

describe('workbench/move/recommend', () => {
  describe('buildTonnageMap', () => {
    it('按 machine+date 聚合吨位，并过滤非法数据', () => {
      const tonnageMap = buildTonnageMap([
        { machine_code: 'M1', plan_date: '2026-02-01', weight_t: 10 },
        { machine_code: ' M1 ', plan_date: '2026-02-01', weight_t: 5 },
        { machine_code: 'M1', plan_date: '', weight_t: 100 },
        { machine_code: '', plan_date: '2026-02-01', weight_t: 100 },
        { machine_code: 'M1', plan_date: '2026-02-01', weight_t: 0 },
        { machine_code: 'M1', plan_date: '2026-02-01', weight_t: -2 },
        { machine_code: 'M1', plan_date: '2026-02-01', weight_t: Number.NaN },
      ] as any);

      expect(tonnageMap.get(makeMachineDateKey('M1', '2026-02-01'))).toBe(15);
      expect(tonnageMap.size).toBe(1);
    });
  });

  describe('buildCapacityPoolMap', () => {
    it('target/limit 非正数或非有限数时置为 null，并保留 limit 优先级', () => {
      const poolMap = buildCapacityPoolMap([
        { machine_code: 'M1', plan_date: '2026-02-01', target_capacity_t: 100, limit_capacity_t: 80 },
        { machine_code: 'M1', plan_date: '2026-02-02', target_capacity_t: 0, limit_capacity_t: 0 },
        { machine_code: '', plan_date: '2026-02-03', target_capacity_t: 100, limit_capacity_t: 100 },
      ] as any);

      expect(poolMap.get(makeMachineDateKey('M1', '2026-02-01'))).toEqual({ target: 100, limit: 80 });
      expect(poolMap.get(makeMachineDateKey('M1', '2026-02-02'))).toEqual({ target: null, limit: null });
      expect(poolMap.has(makeMachineDateKey('', '2026-02-03'))).toBe(false);
    });
  });

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

  describe('pickMovableItems', () => {
    it('AUTO_FIX 会跳过 locked_in_plan 的条目；STRICT 不跳过', () => {
      const locked = {
        material_id: 'mat1',
        machine_code: 'M1',
        plan_date: '2026-02-01',
        weight_t: 10,
        locked_in_plan: true,
      };
      const unlocked = {
        material_id: 'mat2',
        machine_code: 'M1',
        plan_date: '2026-02-01',
        weight_t: 10,
        locked_in_plan: false,
      };
      const byId = new Map([
        ['mat1', locked as any],
        ['mat2', unlocked as any],
      ]);

      const autoFix = pickMovableItems({ selectedMaterialIds: ['mat1', 'mat2'], byId, moveValidationMode: 'AUTO_FIX' });
      expect(autoFix.map((it) => it.material_id)).toEqual(['mat2']);

      const strict = pickMovableItems({ selectedMaterialIds: ['mat1', 'mat2'], byId, moveValidationMode: 'STRICT' });
      expect(strict.map((it) => it.material_id).sort()).toEqual(['mat1', 'mat2']);
    });
  });

  describe('scoreCandidateDates', () => {
    it('正确计算 overLimit/unknown/maxUtil/totalOver，并优先使用 limit', () => {
      const focus = dayjs('2026-02-01');
      const candidates = ['2026-02-02'];

      const deltaBase = new Map<string, number>([[makeMachineDateKey('M2', '2026-02-01'), -10]]);
      const totalWeight = 10;

      const tonnageMap = new Map<string, number>([
        [makeMachineDateKey('M1', '2026-02-02'), 75],
        [makeMachineDateKey('M2', '2026-02-01'), 50],
      ]);

      const poolMap = new Map<string, { target: number | null; limit: number | null }>([
        [makeMachineDateKey('M1', '2026-02-02'), { target: 100, limit: 80 }],
        [makeMachineDateKey('M2', '2026-02-01'), { target: 60, limit: null }],
      ]);

      const scored = scoreCandidateDates({
        candidates,
        deltaBase,
        totalWeight,
        targetMachine: 'M1',
        tonnageMap,
        poolMap,
        focus,
      });

      expect(scored).toHaveLength(1);
      expect(scored[0]).toMatchObject({
        date: '2026-02-02',
        overLimitCount: 1,
        unknownCount: 0,
        totalOverT: 5,
        maxUtilPct: 106.25,
        distance: 1,
      });
    });

    it('产能未知时计入 unknownCount（仅统计 delta 非 0 的 key）', () => {
      const focus = dayjs('2026-02-01');
      const candidates = ['2026-02-02'];

      const deltaBase = new Map<string, number>([[makeMachineDateKey('M2', '2026-02-01'), -10]]);
      const totalWeight = 10;

      const tonnageMap = new Map<string, number>([
        [makeMachineDateKey('M1', '2026-02-02'), 10],
        [makeMachineDateKey('M2', '2026-02-01'), 50],
      ]);

      const poolMap = new Map<string, { target: number | null; limit: number | null }>([
        [makeMachineDateKey('M2', '2026-02-01'), { target: 80, limit: null }],
      ]);

      const scored = scoreCandidateDates({
        candidates,
        deltaBase,
        totalWeight,
        targetMachine: 'M1',
        tonnageMap,
        poolMap,
        focus,
      });

      expect(scored).toHaveLength(1);
      expect(scored[0]).toMatchObject({
        date: '2026-02-02',
        overLimitCount: 0,
        unknownCount: 1,
        totalOverT: 0,
        maxUtilPct: 50,
        distance: 1,
      });
    });

    it('totalWeight=0 且无 delta 时返回空列表', () => {
      const scored = scoreCandidateDates({
        candidates: ['2026-02-01'],
        deltaBase: new Map(),
        totalWeight: 0,
        targetMachine: 'M1',
        tonnageMap: new Map(),
        poolMap: new Map(),
        focus: dayjs('2026-02-01'),
      });
      expect(scored).toEqual([]);
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

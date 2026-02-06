/**
 * comparison/utils.ts 单元测试
 */

import { describe, it, expect } from 'vitest';
import type { PlanItemSnapshot } from '../../types/comparison';
import {
  normalizeDateOnly,
  extractVersionNameCn,
  formatVersionLabel,
  formatVersionLabelWithCode,
  normalizePlanItem,
  computeVersionDiffs,
  computeCapacityMap,
  computeDailyTotals,
  makeRetrospectiveKey,
} from './utils';
import type { Version } from './types';

describe('comparison/utils', () => {
  describe('normalizeDateOnly', () => {
    it('应该提取 YYYY-MM-DD 部分', () => {
      const result = normalizeDateOnly('2026-01-30 14:30:00');
      expect(result).toBe('2026-01-30');
    });

    it('应该处理只有日期的输入', () => {
      const result = normalizeDateOnly('2026-01-30');
      expect(result).toBe('2026-01-30');
    });

    it('空输入应返回空字符串', () => {
      expect(normalizeDateOnly('')).toBe('');
      expect(normalizeDateOnly(null as any)).toBe('');
      expect(normalizeDateOnly(undefined as any)).toBe('');
    });

    it('应该处理各种日期格式', () => {
      expect(normalizeDateOnly('2026-01-30')).toBe('2026-01-30');
      expect(normalizeDateOnly('2026-1-30')).toBe('2026-1-30'); // 不标准格式
      expect(normalizeDateOnly('2026-01-30T10:00:00')).toBe('2026-01-30');
    });
  });

  describe('extractVersionNameCn', () => {
    it('应该从 JSON 中提取中文名称', () => {
      const version: Version = {
        version_id: 'v1',
        version_no: 1,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: JSON.stringify({
          __meta_version_name_cn: '生产版本 v1',
        }),
      };
      const result = extractVersionNameCn(version);
      expect(result).toBe('生产版本 v1');
    });

    it('JSON 中无中文名称时返回 null', () => {
      const version: Version = {
        version_id: 'v1',
        version_no: 1,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: JSON.stringify({}),
      };
      expect(extractVersionNameCn(version)).toBeNull();
    });

    it('无 config_snapshot_json 时返回 null', () => {
      const version: Version = {
        version_id: 'v1',
        version_no: 1,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: null,
      };
      expect(extractVersionNameCn(version)).toBeNull();
    });

    it('应该忽略空白中文名称', () => {
      const version: Version = {
        version_id: 'v1',
        version_no: 1,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: JSON.stringify({
          __meta_version_name_cn: '   ', // 空白
        }),
      };
      expect(extractVersionNameCn(version)).toBeNull();
    });

    it('JSON 解析错误时应返回 null', () => {
      const version: Version = {
        version_id: 'v1',
        version_no: 1,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: 'invalid json {',
      };
      expect(extractVersionNameCn(version)).toBeNull();
    });
  });

  describe('formatVersionLabel', () => {
    it('有中文名称时优先返回中文名称', () => {
      const version: Version = {
        version_id: 'v123',
        version_no: 1,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: JSON.stringify({
          __meta_version_name_cn: '生产版本 v1',
        }),
      };
      expect(formatVersionLabel(version)).toBe('生产版本 v1');
    });

    it('无中文名称时返回版本号', () => {
      const version: Version = {
        version_id: 'v123',
        version_no: 2,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: null,
      };
      expect(formatVersionLabel(version)).toBe('V2');
    });

    it('版本号无效时返回版本ID', () => {
      const version: Version = {
        version_id: 'v_special_123',
        version_no: 0,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: null,
      };
      expect(formatVersionLabel(version)).toBe('v_special_123');
    });
  });

  describe('normalizePlanItem', () => {
    it('应该规范化计划项数据', () => {
      const raw = {
        material_id: 'M001',
        machine_code: 'M1',
        plan_date: '2026-01-30 10:00:00',
        seq_no: 1,
        weight_t: 5.5,
        urgent_level: 'L2',
      };
      const result = normalizePlanItem(raw);
      expect(result).not.toBeNull();
      expect(result?.material_id).toBe('M001');
      expect(result?.plan_date).toBe('2026-01-30');
      expect(result?.weight_t).toBe(5.5);
      expect(result?.urgent_level).toBe('L2');
    });

    it('material_id 缺失时返回 null', () => {
      const result = normalizePlanItem({ plan_date: '2026-01-30' });
      expect(result).toBeNull();
    });

    it('应该处理可选字段', () => {
      const raw = {
        material_id: 'M001',
        machine_code: 'M1',
        plan_date: '2026-01-30',
        seq_no: 1,
        // weight_t 缺失
      };
      const result = normalizePlanItem(raw);
      expect(result?.weight_t).toBeUndefined();
    });

    it('应该处理布尔字段', () => {
      const raw = {
        material_id: 'M001',
        machine_code: 'M1',
        plan_date: '2026-01-30',
        seq_no: 1,
        locked_in_plan: true,
        force_release_in_plan: false,
      };
      const result = normalizePlanItem(raw);
      expect(result?.locked_in_plan).toBe(true);
      expect(result?.force_release_in_plan).toBe(false);
    });
  });

  describe('computeVersionDiffs', () => {
    it('应该检测新增项目 (ADDED)', () => {
      const itemsA: PlanItemSnapshot[] = [];
      const itemsB: PlanItemSnapshot[] = [
        {
          material_id: 'M1',
          machine_code: 'MA',
          plan_date: '2026-01-30',
          seq_no: 1,
        },
      ];
      const result = computeVersionDiffs(itemsA, itemsB);
      expect(result.diffs).toHaveLength(1);
      expect(result.diffs[0].changeType).toBe('ADDED');
      expect(result.summary.addedCount).toBe(1);
    });

    it('应该检测删除项目 (REMOVED)', () => {
      const itemsA: PlanItemSnapshot[] = [
        {
          material_id: 'M1',
          machine_code: 'MA',
          plan_date: '2026-01-30',
          seq_no: 1,
        },
      ];
      const itemsB: PlanItemSnapshot[] = [];
      const result = computeVersionDiffs(itemsA, itemsB);
      expect(result.diffs).toHaveLength(1);
      expect(result.diffs[0].changeType).toBe('REMOVED');
      expect(result.summary.removedCount).toBe(1);
    });

    it('应该检测移动项目 (MOVED)', () => {
      const itemsA: PlanItemSnapshot[] = [
        {
          material_id: 'M1',
          machine_code: 'MA',
          plan_date: '2026-01-30',
          seq_no: 1,
        },
      ];
      const itemsB: PlanItemSnapshot[] = [
        {
          material_id: 'M1',
          machine_code: 'MB',
          plan_date: '2026-01-31',
          seq_no: 2,
        },
      ];
      const result = computeVersionDiffs(itemsA, itemsB);
      expect(result.diffs).toHaveLength(1);
      expect(result.diffs[0].changeType).toBe('MOVED');
      expect(result.summary.movedCount).toBe(1);
    });

    it('应该检测修改项目 (MODIFIED)', () => {
      const itemsA: PlanItemSnapshot[] = [
        {
          material_id: 'M1',
          machine_code: 'MA',
          plan_date: '2026-01-30',
          seq_no: 1,
          weight_t: 10,
        },
      ];
      const itemsB: PlanItemSnapshot[] = [
        {
          material_id: 'M1',
          machine_code: 'MA',
          plan_date: '2026-01-30',
          seq_no: 1,
          weight_t: 15, // 重量改变
        },
      ];
      const result = computeVersionDiffs(itemsA, itemsB);
      expect(result.diffs).toHaveLength(1);
      expect(result.diffs[0].changeType).toBe('MODIFIED');
      expect(result.summary.modifiedCount).toBe(1);
    });

    it('应该正确计算汇总统计', () => {
      const itemsA: PlanItemSnapshot[] = [
        { material_id: 'M1', machine_code: 'MA', plan_date: '2026-01-30', seq_no: 1 }, // REMOVED
        { material_id: 'M2', machine_code: 'MA', plan_date: '2026-01-30', seq_no: 2 }, // MOVED
      ];
      const itemsB: PlanItemSnapshot[] = [
        { material_id: 'M2', machine_code: 'MB', plan_date: '2026-01-31', seq_no: 2 }, // MOVED
        { material_id: 'M3', machine_code: 'MA', plan_date: '2026-01-30', seq_no: 3 }, // ADDED
      ];
      const result = computeVersionDiffs(itemsA, itemsB);
      expect(result.summary.totalChanges).toBe(3);
      expect(result.summary.removedCount).toBe(1);
      expect(result.summary.movedCount).toBe(1);
      expect(result.summary.addedCount).toBe(1);
    });
  });

  describe('computeCapacityMap', () => {
    it('应该按机组+日期聚合重量', () => {
      const items: PlanItemSnapshot[] = [
        { material_id: 'M1', machine_code: 'M1', plan_date: '2026-01-30', seq_no: 1, weight_t: 10 },
        { material_id: 'M2', machine_code: 'M1', plan_date: '2026-01-30', seq_no: 2, weight_t: 15 },
        { material_id: 'M3', machine_code: 'M2', plan_date: '2026-01-30', seq_no: 1, weight_t: 20 },
      ];
      const map = computeCapacityMap(items);
      expect(map.get('M1__2026-01-30')).toBe(25);
      expect(map.get('M2__2026-01-30')).toBe(20);
    });

    it('应该处理无效的机组编码', () => {
      const items: PlanItemSnapshot[] = [
        { material_id: 'M1', machine_code: '', plan_date: '2026-01-30', seq_no: 1, weight_t: 10 },
        { material_id: 'M2', machine_code: 'M1', plan_date: '2026-01-30', seq_no: 2, weight_t: 15 },
      ];
      const map = computeCapacityMap(items);
      expect(map.get('M1__2026-01-30')).toBe(15);
      expect(map.size).toBe(1); // 只有有效的条目
    });

    it('空列表应返回空 map', () => {
      const map = computeCapacityMap([]);
      expect(map.size).toBe(0);
    });
  });

  describe('computeDailyTotals', () => {
    it('应该按日期聚合总产量', () => {
      const items: PlanItemSnapshot[] = [
        { material_id: 'M1', machine_code: 'M1', plan_date: '2026-01-30', seq_no: 1, weight_t: 10 },
        { material_id: 'M2', machine_code: 'M2', plan_date: '2026-01-30', seq_no: 1, weight_t: 20 },
        { material_id: 'M3', machine_code: 'M1', plan_date: '2026-01-31', seq_no: 1, weight_t: 15 },
      ];
      const map = computeDailyTotals(items);
      expect(map.get('2026-01-30')).toBe(30);
      expect(map.get('2026-01-31')).toBe(15);
    });

    it('应该忽略无效的日期', () => {
      const items: PlanItemSnapshot[] = [
        { material_id: 'M1', machine_code: 'M1', plan_date: '', seq_no: 1, weight_t: 10 },
        { material_id: 'M2', machine_code: 'M2', plan_date: '2026-01-30', seq_no: 1, weight_t: 20 },
      ];
      const map = computeDailyTotals(items);
      expect(map.get('2026-01-30')).toBe(20);
      expect(map.size).toBe(1);
    });

    it('应该处理缺失的权重', () => {
      const items: PlanItemSnapshot[] = [
        { material_id: 'M1', machine_code: 'M1', plan_date: '2026-01-30', seq_no: 1 }, // 无 weight_t
        { material_id: 'M2', machine_code: 'M2', plan_date: '2026-01-30', seq_no: 1, weight_t: 20 },
      ];
      const map = computeDailyTotals(items);
      expect(map.get('2026-01-30')).toBe(20); // 只计算有效的
    });

    it('空列表应返回空 map', () => {
      const map = computeDailyTotals([]);
      expect(map.size).toBe(0);
    });
  });

  describe('formatVersionLabelWithCode', () => {
    it('有中文名称和版本号时应返回完整格式', () => {
      const version: Version = {
        version_id: 'v123',
        version_no: 10,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: JSON.stringify({
          __meta_version_name_cn: '重排产优化',
        }),
      };
      const result = formatVersionLabelWithCode(version);
      expect(result).toBe('重排产优化 (V10)');
    });

    it('只有中文名称时应返回中文名称', () => {
      const version = {
        version_id: 'v123',
        version_no: null,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: JSON.stringify({
          __meta_version_name_cn: '测试方案A',
        }),
      } as unknown as Version;
      const result = formatVersionLabelWithCode(version);
      expect(result).toBe('测试方案A');
    });

    it('只有版本号时应返回版本号', () => {
      const version: Version = {
        version_id: 'v123',
        version_no: 12,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: null,
      };
      const result = formatVersionLabelWithCode(version);
      expect(result).toBe('V12');
    });

    it('版本号为 0 或负数时应忽略版本号', () => {
      const version: Version = {
        version_id: 'v123',
        version_no: 0,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: null,
      };
      // 应该降级到 UUID 前8位
      expect(formatVersionLabelWithCode(version).length).toBeGreaterThan(0);
    });

    it('UUID 格式的 ID 应返回前 8 位', () => {
      const version = {
        version_id: '31c46b4d-1234-5678-9abc-def012345678',
        version_no: null,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: null,
      } as unknown as Version;
      const result = formatVersionLabelWithCode(version);
      expect(result).toBe('31c46b4d');
    });

    it('非 UUID 格式的 ID 应返回完整 ID', () => {
      const version = {
        version_id: 'test_version_custom_name',
        version_no: null,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: null,
      } as unknown as Version;
      const result = formatVersionLabelWithCode(version);
      expect(result).toBe('test_version_custom_name');
    });

    it('应该忽略无效的版本号', () => {
      const version: Version = {
        version_id: 'test-id',
        version_no: NaN as any,
        status: 'ACTIVE',
        recalc_window_days: 30,
        created_at: '2026-01-30',
        config_snapshot_json: null,
      };
      // NaN 不是有限数，应该降级显示 ID
      expect(formatVersionLabelWithCode(version)).toBe('test-id');
    });
  });

  describe('makeRetrospectiveKey', () => {
    it('应该生成正确的存储键', () => {
      const key = makeRetrospectiveKey('v1', 'v2');
      expect(key).toContain('v1');
      expect(key).toContain('v2');
      expect(key).toContain('aps_retrospective_note');
    });

    it('应该保证键的顺序一致性', () => {
      const key1 = makeRetrospectiveKey('v1', 'v2');
      const key2 = makeRetrospectiveKey('v2', 'v1');
      // 无论输入顺序如何，生成的键应该相同
      expect(key1).toBe(key2);
    });

    it('应该处理空值输入', () => {
      const key = makeRetrospectiveKey('', '');
      expect(key).toContain('aps_retrospective_note');
      // 空字符串应该被排序和拼接
      expect(key).toBe('aps_retrospective_note____');
    });

    it('应该处理 null/undefined 输入', () => {
      const key1 = makeRetrospectiveKey(null as any, 'v1');
      const key2 = makeRetrospectiveKey('v1', undefined as any);
      // null/undefined 应该被转换为空字符串
      expect(key1).toContain('v1');
      expect(key2).toContain('v1');
    });
  });
});

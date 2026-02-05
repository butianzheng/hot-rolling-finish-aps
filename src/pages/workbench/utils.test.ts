import { describe, it, expect } from 'vitest';
import { extractForceReleaseViolations, getStrategyLabel } from './utils';

describe('workbench/utils 工具函数', () => {
  describe('extractForceReleaseViolations', () => {
    it('应该提取有效的强制放行违规', () => {
      const details = {
        violations: [
          {
            type: 'MATURITY_CONSTRAINT',
            message: '材料未适温',
            severity: 'warning',
          },
          {
            type: 'CAPACITY_FIRST',
            message: '产能超限',
            severity: 'error',
          },
        ],
      };

      const result = extractForceReleaseViolations(details);
      expect(result).toHaveLength(2);
      expect(result[0]).toHaveProperty('type', 'MATURITY_CONSTRAINT');
      expect(result[1]).toHaveProperty('type', 'CAPACITY_FIRST');
    });

    it('应该过滤 null 违规', () => {
      const details = {
        violations: [
          {
            type: 'MATURITY_CONSTRAINT',
            message: '材料未适温',
            severity: 'warning',
          },
          null,
          {
            type: 'CAPACITY_FIRST',
            message: '产能超限',
            severity: 'error',
          },
        ],
      };

      const result = extractForceReleaseViolations(details);
      expect(result).toHaveLength(2);
      expect(result.some((v) => v === null)).toBe(false);
    });

    it('应该处理空 details', () => {
      expect(extractForceReleaseViolations(null)).toEqual([]);
      expect(extractForceReleaseViolations(undefined)).toEqual([]);
      expect(extractForceReleaseViolations({})).toEqual([]);
    });

    it('应该处理非对象 details', () => {
      expect(extractForceReleaseViolations('invalid')).toEqual([]);
      expect(extractForceReleaseViolations(123)).toEqual([]);
      expect(extractForceReleaseViolations(true)).toEqual([]);
    });

    it('应该处理非数组 violations', () => {
      const details = {
        violations: 'not an array',
      };
      expect(extractForceReleaseViolations(details)).toEqual([]);
    });

    it('应该处理空 violations 数组', () => {
      const details = {
        violations: [],
      };
      expect(extractForceReleaseViolations(details)).toEqual([]);
    });

    it('应该处理 violations 缺失', () => {
      const details = {
        other: 'data',
      };
      expect(extractForceReleaseViolations(details)).toEqual([]);
    });

    it('应该保留违规对象的所有字段', () => {
      const details = {
        violations: [
          {
            type: 'MATURITY_CONSTRAINT',
            message: '材料未适温',
            severity: 'warning',
            details: '距离适温还需 3 天',
            affectedEntities: ['MAT001'],
          },
        ],
      };

      const result = extractForceReleaseViolations(details);
      expect(result[0]).toHaveProperty('details', '距离适温还需 3 天');
      expect(result[0]).toHaveProperty('affectedEntities');
      expect((result[0] as any).affectedEntities).toEqual(['MAT001']);
    });
  });

  describe('getStrategyLabel', () => {
    it('应该返回紧急优先标签', () => {
      expect(getStrategyLabel('urgent_first')).toBe('紧急优先');
    });

    it('应该返回产能优先标签', () => {
      expect(getStrategyLabel('capacity_first')).toBe('产能优先');
    });

    it('应该返回冷坯消化标签', () => {
      expect(getStrategyLabel('cold_stock_first')).toBe('冷坯消化');
    });

    it('应该返回手动调整标签', () => {
      expect(getStrategyLabel('manual')).toBe('手动调整');
    });

    it('应该返回均衡方案标签（默认）', () => {
      expect(getStrategyLabel('balanced')).toBe('均衡方案');
    });

    it('应该处理 null 并返回默认标签', () => {
      expect(getStrategyLabel(null)).toBe('均衡方案');
    });

    it('应该处理 undefined 并返回默认标签', () => {
      expect(getStrategyLabel(undefined)).toBe('均衡方案');
    });

    it('应该处理空字符串并返回默认标签', () => {
      expect(getStrategyLabel('')).toBe('均衡方案');
    });

    it('应该处理未知策略并返回默认标签', () => {
      expect(getStrategyLabel('unknown_strategy')).toBe('均衡方案');
    });

    it('应该处理所有标准策略', () => {
      const strategies = [
        { key: 'urgent_first', label: '紧急优先' },
        { key: 'capacity_first', label: '产能优先' },
        { key: 'cold_stock_first', label: '冷坯消化' },
        { key: 'manual', label: '手动调整' },
        { key: 'balanced', label: '均衡方案' },
      ];

      strategies.forEach(({ key, label }) => {
        expect(getStrategyLabel(key)).toBe(label);
      });
    });
  });
});

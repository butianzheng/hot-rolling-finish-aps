import { describe, expect, it } from 'vitest';
import {
  normalizeStrategyKey,
  getStrategyLabelByKey,
  BUILTIN_STRATEGY_OPTIONS,
} from './strategy';

describe('strategy 工具函数', () => {
  describe('normalizeStrategyKey', () => {
    it('应该处理空值并返回默认策略', () => {
      expect(normalizeStrategyKey(null)).toBe('balanced');
      expect(normalizeStrategyKey(undefined)).toBe('balanced');
      expect(normalizeStrategyKey('')).toBe('balanced');
      expect(normalizeStrategyKey('   ')).toBe('balanced');
    });

    it('应该返回内置策略键', () => {
      expect(normalizeStrategyKey('balanced')).toBe('balanced');
      expect(normalizeStrategyKey('urgent_first')).toBe('urgent_first');
      expect(normalizeStrategyKey('capacity_first')).toBe('capacity_first');
      expect(normalizeStrategyKey('cold_stock_first')).toBe('cold_stock_first');
      expect(normalizeStrategyKey('manual')).toBe('manual');
    });

    it('应该处理自定义策略键', () => {
      expect(normalizeStrategyKey('custom:my-strategy')).toBe('custom:my-strategy');
      expect(normalizeStrategyKey('custom:123')).toBe('custom:123');
    });

    it('应该处理空的自定义策略 ID', () => {
      expect(normalizeStrategyKey('custom:')).toBe('balanced');
      expect(normalizeStrategyKey('custom:   ')).toBe('balanced');
    });

    it('应该修剪空白字符', () => {
      expect(normalizeStrategyKey('  balanced  ')).toBe('balanced');
      expect(normalizeStrategyKey('  custom:my-strategy  ')).toBe('custom:my-strategy');
    });
  });

  describe('getStrategyLabelByKey', () => {
    it('应该返回内置策略的标签', () => {
      expect(getStrategyLabelByKey('balanced')).toBe('均衡方案');
      expect(getStrategyLabelByKey('urgent_first')).toBe('紧急优先');
      expect(getStrategyLabelByKey('capacity_first')).toBe('产能优先');
      expect(getStrategyLabelByKey('cold_stock_first')).toBe('冷坯消化');
      expect(getStrategyLabelByKey('manual')).toBe('手动调整');
    });

    it('应该返回自定义策略的标签', () => {
      expect(getStrategyLabelByKey('custom:my-strategy')).toBe('自定义策略（my-strategy）');
      expect(getStrategyLabelByKey('custom:123')).toBe('自定义策略（123）');
    });

    it('应该处理空的自定义策略 ID', () => {
      // normalizeStrategyKey 会将空的自定义策略 ID 转换为 'balanced'
      // 所以 getStrategyLabelByKey 也会返回 '均衡方案'
      expect(getStrategyLabelByKey('custom:')).toBe('均衡方案');
      expect(getStrategyLabelByKey('custom:   ')).toBe('均衡方案');
    });

    it('应该处理空值并返回默认标签', () => {
      expect(getStrategyLabelByKey(null)).toBe('均衡方案');
      expect(getStrategyLabelByKey(undefined)).toBe('均衡方案');
      expect(getStrategyLabelByKey('')).toBe('均衡方案');
    });

    it('应该处理未知策略键', () => {
      expect(getStrategyLabelByKey('unknown')).toBe('均衡方案');
    });
  });

  describe('BUILTIN_STRATEGY_OPTIONS', () => {
    it('应该包含所有内置策略��项', () => {
      expect(BUILTIN_STRATEGY_OPTIONS).toHaveLength(5);

      const values = BUILTIN_STRATEGY_OPTIONS.map(opt => opt.value);
      expect(values).toContain('balanced');
      expect(values).toContain('urgent_first');
      expect(values).toContain('capacity_first');
      expect(values).toContain('cold_stock_first');
      expect(values).toContain('manual');
    });

    it('应该包含正确的标签', () => {
      const option = BUILTIN_STRATEGY_OPTIONS.find(opt => opt.value === 'balanced');
      expect(option?.label).toBe('均衡方案');
    });
  });
});

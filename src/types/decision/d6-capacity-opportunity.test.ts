import { describe, expect, it } from 'vitest';
import {
  isUnderutilized,
  isHighPriorityOpportunity,
  getUtilizationColor,
  OPPORTUNITY_TYPE_COLORS,
  OPPORTUNITY_TYPE_LABELS,
} from './d6-capacity-opportunity';

describe('d6-capacity-opportunity 工具函数', () => {
  describe('isUnderutilized', () => {
    it('应该识别未充分利用的容量', () => {
      expect(isUnderutilized(0)).toBe(true);
      expect(isUnderutilized(50)).toBe(true);
      expect(isUnderutilized(69)).toBe(true);
    });

    it('应该识别充分利用的容量', () => {
      expect(isUnderutilized(70)).toBe(false);
      expect(isUnderutilized(80)).toBe(false);
      expect(isUnderutilized(100)).toBe(false);
    });

    it('应该处理边界值', () => {
      expect(isUnderutilized(69.9)).toBe(true);
      expect(isUnderutilized(70.0)).toBe(false);
      expect(isUnderutilized(70.1)).toBe(false);
    });
  });

  describe('isHighPriorityOpportunity', () => {
    it('应该识别高优先级机会', () => {
      expect(isHighPriorityOpportunity(70)).toBe(true);
      expect(isHighPriorityOpportunity(80)).toBe(true);
      expect(isHighPriorityOpportunity(100)).toBe(true);
    });

    it('应该识别低优先级机会', () => {
      expect(isHighPriorityOpportunity(0)).toBe(false);
      expect(isHighPriorityOpportunity(50)).toBe(false);
      expect(isHighPriorityOpportunity(69)).toBe(false);
    });

    it('应该处理边界值', () => {
      expect(isHighPriorityOpportunity(69.9)).toBe(false);
      expect(isHighPriorityOpportunity(70.0)).toBe(true);
      expect(isHighPriorityOpportunity(70.1)).toBe(true);
    });
  });

  describe('getUtilizationColor', () => {
    it('应该返回绿色表示容量充裕', () => {
      expect(getUtilizationColor(0)).toBe('#52c41a');
      expect(getUtilizationColor(30)).toBe('#52c41a');
      expect(getUtilizationColor(59)).toBe('#52c41a');
    });

    it('应该返回蓝色表示正常', () => {
      expect(getUtilizationColor(60)).toBe('#1677ff');
      expect(getUtilizationColor(70)).toBe('#1677ff');
      expect(getUtilizationColor(79)).toBe('#1677ff');
    });

    it('应该返回橙色表示接近满载', () => {
      expect(getUtilizationColor(80)).toBe('#faad14');
      expect(getUtilizationColor(90)).toBe('#faad14');
      expect(getUtilizationColor(99)).toBe('#faad14');
    });

    it('应该返回红色表示超载', () => {
      expect(getUtilizationColor(100)).toBe('#ff4d4f');
      expect(getUtilizationColor(110)).toBe('#ff4d4f');
      expect(getUtilizationColor(150)).toBe('#ff4d4f');
    });

    it('应该处理边界值', () => {
      expect(getUtilizationColor(59.9)).toBe('#52c41a');
      expect(getUtilizationColor(60.0)).toBe('#1677ff');
      expect(getUtilizationColor(79.9)).toBe('#1677ff');
      expect(getUtilizationColor(80.0)).toBe('#faad14');
      expect(getUtilizationColor(99.9)).toBe('#faad14');
      expect(getUtilizationColor(100.0)).toBe('#ff4d4f');
    });
  });

  describe('常量定义', () => {
    it('OPPORTUNITY_TYPE_COLORS 应该包含所有机会类型', () => {
      expect(OPPORTUNITY_TYPE_COLORS.UNDERUTILIZED).toBeDefined();
      expect(OPPORTUNITY_TYPE_COLORS.MOVABLE_LOAD).toBeDefined();
      expect(OPPORTUNITY_TYPE_COLORS.STRUCTURE_FIX).toBeDefined();
      expect(OPPORTUNITY_TYPE_COLORS.URGENT_INSERTION).toBeDefined();
      expect(OPPORTUNITY_TYPE_COLORS.LOAD_BALANCE).toBeDefined();
    });

    it('OPPORTUNITY_TYPE_LABELS 应该包含所有机会类型', () => {
      expect(OPPORTUNITY_TYPE_LABELS.UNDERUTILIZED).toBe('未充分利用');
      expect(OPPORTUNITY_TYPE_LABELS.MOVABLE_LOAD).toBe('可移动负载');
      expect(OPPORTUNITY_TYPE_LABELS.STRUCTURE_FIX).toBe('结构优化');
      expect(OPPORTUNITY_TYPE_LABELS.URGENT_INSERTION).toBe('紧急插入');
      expect(OPPORTUNITY_TYPE_LABELS.LOAD_BALANCE).toBe('负载均衡');
    });
  });
});

import { describe, expect, it } from 'vitest';
import {
  parseAlertLevel,
  hasAlert,
  isSevereAlert,
  getAlertLevelColor,
  getAlertLevelLabel,
  calculateUtilization,
  ROLL_STATUS_COLORS,
  ROLL_STATUS_LABELS,
} from './d5-roll-campaign';

describe('d5-roll-campaign 工具函数', () => {
  describe('parseAlertLevel', () => {
    it('应该解析 D5 领域新口径', () => {
      expect(parseAlertLevel('EMERGENCY')).toBe('HARD_STOP');
      expect(parseAlertLevel('CRITICAL')).toBe('WARNING');
      expect(parseAlertLevel('WARNING')).toBe('SUGGEST');
      expect(parseAlertLevel('NONE')).toBe('NORMAL');
    });

    it('应该兼容旧口径/前端自定义状态', () => {
      expect(parseAlertLevel('HARD_STOP')).toBe('HARD_STOP');
      expect(parseAlertLevel('SUGGEST')).toBe('SUGGEST');
      expect(parseAlertLevel('NORMAL')).toBe('NORMAL');
    });

    it('应该兼容风险等级类枚举', () => {
      expect(parseAlertLevel('HIGH')).toBe('WARNING');
      expect(parseAlertLevel('MEDIUM')).toBe('SUGGEST');
    });

    it('应该处理小写输入', () => {
      expect(parseAlertLevel('emergency')).toBe('HARD_STOP');
      expect(parseAlertLevel('critical')).toBe('WARNING');
      expect(parseAlertLevel('warning')).toBe('SUGGEST');
    });

    it('应该处理未知状态', () => {
      expect(parseAlertLevel('UNKNOWN')).toBe('NORMAL');
      expect(parseAlertLevel('')).toBe('NORMAL');
      expect(parseAlertLevel('LOW')).toBe('NORMAL');
    });

    it('应该处理 null 和 undefined', () => {
      expect(parseAlertLevel(null as any)).toBe('NORMAL');
      expect(parseAlertLevel(undefined as any)).toBe('NORMAL');
    });
  });

  describe('hasAlert', () => {
    it('应该识别有警报的状态', () => {
      expect(hasAlert('SUGGEST')).toBe(true);
      expect(hasAlert('WARNING')).toBe(true);
      expect(hasAlert('HARD_STOP')).toBe(true);
    });

    it('应该识别无警报的状态', () => {
      expect(hasAlert('NORMAL')).toBe(false);
    });
  });

  describe('isSevereAlert', () => {
    it('应该识别严重警报', () => {
      expect(isSevereAlert('WARNING')).toBe(true);
      expect(isSevereAlert('HARD_STOP')).toBe(true);
    });

    it('应该识别非严重警报', () => {
      expect(isSevereAlert('NORMAL')).toBe(false);
      expect(isSevereAlert('SUGGEST')).toBe(false);
    });
  });

  describe('getAlertLevelColor', () => {
    it('应该返回正确的颜色', () => {
      expect(getAlertLevelColor('EMERGENCY')).toBe(ROLL_STATUS_COLORS.HARD_STOP);
      expect(getAlertLevelColor('CRITICAL')).toBe(ROLL_STATUS_COLORS.WARNING);
      expect(getAlertLevelColor('WARNING')).toBe(ROLL_STATUS_COLORS.SUGGEST);
      expect(getAlertLevelColor('NONE')).toBe(ROLL_STATUS_COLORS.NORMAL);
    });

    it('应该处理未知状态', () => {
      expect(getAlertLevelColor('UNKNOWN')).toBe(ROLL_STATUS_COLORS.NORMAL);
    });
  });

  describe('getAlertLevelLabel', () => {
    it('应该返回正确的标签', () => {
      expect(getAlertLevelLabel('EMERGENCY')).toBe(ROLL_STATUS_LABELS.HARD_STOP);
      expect(getAlertLevelLabel('CRITICAL')).toBe(ROLL_STATUS_LABELS.WARNING);
      expect(getAlertLevelLabel('WARNING')).toBe(ROLL_STATUS_LABELS.SUGGEST);
      expect(getAlertLevelLabel('NONE')).toBe(ROLL_STATUS_LABELS.NORMAL);
    });

    it('应该处理未知状态', () => {
      expect(getAlertLevelLabel('UNKNOWN')).toBe(ROLL_STATUS_LABELS.NORMAL);
    });
  });

  describe('calculateUtilization', () => {
    it('应该正确计算利用率', () => {
      expect(calculateUtilization(50, 100)).toBe(50);
      expect(calculateUtilization(75, 100)).toBe(75);
      expect(calculateUtilization(100, 100)).toBe(100);
    });

    it('应该处理超过软限制的情况', () => {
      expect(calculateUtilization(150, 100)).toBe(150);
    });

    it('应该处理零软限制', () => {
      expect(calculateUtilization(50, 0)).toBe(0);
    });

    it('应该处理负数软限制', () => {
      expect(calculateUtilization(50, -100)).toBe(0);
    });

    it('应该四舍五入到整数', () => {
      expect(calculateUtilization(33, 100)).toBe(33);
      expect(calculateUtilization(66, 100)).toBe(66);
      expect(calculateUtilization(67, 100)).toBe(67);
    });

    it('应该处理零当前吨位', () => {
      expect(calculateUtilization(0, 100)).toBe(0);
    });
  });

  describe('常量定义', () => {
    it('ROLL_STATUS_COLORS 应该包含所有状态', () => {
      expect(ROLL_STATUS_COLORS.NORMAL).toBeDefined();
      expect(ROLL_STATUS_COLORS.SUGGEST).toBeDefined();
      expect(ROLL_STATUS_COLORS.WARNING).toBeDefined();
      expect(ROLL_STATUS_COLORS.HARD_STOP).toBeDefined();
    });

    it('ROLL_STATUS_LABELS 应该包含所有状态', () => {
      expect(ROLL_STATUS_LABELS.NORMAL).toBe('正常');
      expect(ROLL_STATUS_LABELS.SUGGEST).toBe('建议换辊');
      expect(ROLL_STATUS_LABELS.WARNING).toBe('警告');
      expect(ROLL_STATUS_LABELS.HARD_STOP).toBe('硬停止');
    });
  });
});

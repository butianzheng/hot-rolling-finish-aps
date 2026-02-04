import { describe, it, expect } from 'vitest';
import {
  normalizeSchedState,
  isScheduled,
  getSchedStateLabel,
  type NormalizedSchedState,
} from './schedState';

describe('schedState 工具函数', () => {
  describe('normalizeSchedState', () => {
    it('应该处理空值和 undefined', () => {
      expect(normalizeSchedState(null)).toBe('UNKNOWN');
      expect(normalizeSchedState(undefined)).toBe('UNKNOWN');
      expect(normalizeSchedState('')).toBe('UNKNOWN');
      expect(normalizeSchedState('   ')).toBe('UNKNOWN');
    });

    it('应该规范化 PENDING_MATURE 状态', () => {
      expect(normalizeSchedState('PENDING_MATURE')).toBe('PENDING_MATURE');
      expect(normalizeSchedState('pending_mature')).toBe('PENDING_MATURE');
      expect(normalizeSchedState('PendingMature')).toBe('PENDING_MATURE');
      expect(normalizeSchedState('PENDINGMATURE')).toBe('PENDING_MATURE');
    });

    it('应该规范化 READY 状态', () => {
      expect(normalizeSchedState('READY')).toBe('READY');
      expect(normalizeSchedState('ready')).toBe('READY');
      expect(normalizeSchedState('Ready')).toBe('READY');
    });

    it('应该规范化 LOCKED 状态', () => {
      expect(normalizeSchedState('LOCKED')).toBe('LOCKED');
      expect(normalizeSchedState('locked')).toBe('LOCKED');
      expect(normalizeSchedState('Locked')).toBe('LOCKED');
    });

    it('应该规范化 FORCE_RELEASE 状态', () => {
      expect(normalizeSchedState('FORCE_RELEASE')).toBe('FORCE_RELEASE');
      expect(normalizeSchedState('force_release')).toBe('FORCE_RELEASE');
      expect(normalizeSchedState('ForceRelease')).toBe('FORCE_RELEASE');
      expect(normalizeSchedState('FORCERELEASE')).toBe('FORCE_RELEASE');
    });

    it('应该规范化 BLOCKED 状态', () => {
      expect(normalizeSchedState('BLOCKED')).toBe('BLOCKED');
      expect(normalizeSchedState('blocked')).toBe('BLOCKED');
      expect(normalizeSchedState('Blocked')).toBe('BLOCKED');
    });

    it('应该规范化 SCHEDULED 状态', () => {
      expect(normalizeSchedState('SCHEDULED')).toBe('SCHEDULED');
      expect(normalizeSchedState('scheduled')).toBe('SCHEDULED');
      expect(normalizeSchedState('Scheduled')).toBe('SCHEDULED');
    });

    it('应该处理未知状态', () => {
      expect(normalizeSchedState('INVALID_STATE')).toBe('UNKNOWN');
      expect(normalizeSchedState('random')).toBe('UNKNOWN');
      expect(normalizeSchedState(123)).toBe('UNKNOWN');
      expect(normalizeSchedState({})).toBe('UNKNOWN');
    });

    it('应该处理带下划线和不带下划线的混合格式', () => {
      expect(normalizeSchedState('PENDING_MATURE')).toBe('PENDING_MATURE');
      expect(normalizeSchedState('pending_mature')).toBe('PENDING_MATURE');
      expect(normalizeSchedState('FORCE_RELEASE')).toBe('FORCE_RELEASE');
      expect(normalizeSchedState('force_release')).toBe('FORCE_RELEASE');
    });

    it('应该去除前后空格', () => {
      expect(normalizeSchedState('  READY  ')).toBe('READY');
      expect(normalizeSchedState(' SCHEDULED ')).toBe('SCHEDULED');
      expect(normalizeSchedState('\tLOCKED\n')).toBe('LOCKED');
    });
  });

  describe('isScheduled', () => {
    it('应该判断 SCHEDULED 状态为 true', () => {
      expect(isScheduled('SCHEDULED')).toBe(true);
      expect(isScheduled('scheduled')).toBe(true);
      expect(isScheduled('Scheduled')).toBe(true);
    });

    it('应该判断非 SCHEDULED 状态为 false', () => {
      expect(isScheduled('READY')).toBe(false);
      expect(isScheduled('LOCKED')).toBe(false);
      expect(isScheduled('PENDING_MATURE')).toBe(false);
      expect(isScheduled('FORCE_RELEASE')).toBe(false);
      expect(isScheduled('BLOCKED')).toBe(false);
      expect(isScheduled('UNKNOWN')).toBe(false);
      expect(isScheduled(null)).toBe(false);
      expect(isScheduled(undefined)).toBe(false);
      expect(isScheduled('')).toBe(false);
    });
  });

  describe('getSchedStateLabel', () => {
    it('应该返回 PENDING_MATURE 的中文标签', () => {
      expect(getSchedStateLabel('PENDING_MATURE')).toBe('未成熟/冷料');
      expect(getSchedStateLabel('pending_mature')).toBe('未成熟/冷料');
      expect(getSchedStateLabel('PendingMature')).toBe('未成熟/冷料');
    });

    it('应该返回 READY 的中文标签', () => {
      expect(getSchedStateLabel('READY')).toBe('待排/就绪');
      expect(getSchedStateLabel('ready')).toBe('待排/就绪');
      expect(getSchedStateLabel('Ready')).toBe('待排/就绪');
    });

    it('应该返回 LOCKED 的中文标签', () => {
      expect(getSchedStateLabel('LOCKED')).toBe('已锁定');
      expect(getSchedStateLabel('locked')).toBe('已锁定');
      expect(getSchedStateLabel('Locked')).toBe('已锁定');
    });

    it('应该返回 FORCE_RELEASE 的中文标签', () => {
      expect(getSchedStateLabel('FORCE_RELEASE')).toBe('强制放行');
      expect(getSchedStateLabel('force_release')).toBe('强制放行');
      expect(getSchedStateLabel('ForceRelease')).toBe('强制放行');
    });

    it('应该返回 BLOCKED 的中文标签', () => {
      expect(getSchedStateLabel('BLOCKED')).toBe('阻断');
      expect(getSchedStateLabel('blocked')).toBe('阻断');
      expect(getSchedStateLabel('Blocked')).toBe('阻断');
    });

    it('应该返回 SCHEDULED 的中文标签', () => {
      expect(getSchedStateLabel('SCHEDULED')).toBe('已排产');
      expect(getSchedStateLabel('scheduled')).toBe('已排产');
      expect(getSchedStateLabel('Scheduled')).toBe('已排产');
    });

    it('应该返回未知状态的中文标签', () => {
      expect(getSchedStateLabel('INVALID')).toBe('未知');
      expect(getSchedStateLabel(null)).toBe('未知');
      expect(getSchedStateLabel(undefined)).toBe('未知');
      expect(getSchedStateLabel('')).toBe('未知');
      expect(getSchedStateLabel('random')).toBe('未知');
    });

    it('应该处理所有状态的边界情况', () => {
      expect(getSchedStateLabel('  READY  ')).toBe('待排/就绪');
      expect(getSchedStateLabel('\tSCHEDULED\n')).toBe('已排产');
      expect(getSchedStateLabel(123)).toBe('未知');
      expect(getSchedStateLabel({})).toBe('未知');
    });
  });

  describe('类型推断', () => {
    it('normalizeSchedState 应该返回 NormalizedSchedState 类型', () => {
      const result: NormalizedSchedState = normalizeSchedState('READY');
      expect(result).toBe('READY');
    });

    it('isScheduled 应该返回 boolean 类型', () => {
      const result: boolean = isScheduled('SCHEDULED');
      expect(result).toBe(true);
    });

    it('getSchedStateLabel 应该返回 string 类型', () => {
      const result: string = getSchedStateLabel('READY');
      expect(result).toBe('待排/就绪');
    });
  });
});

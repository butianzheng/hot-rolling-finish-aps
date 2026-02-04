import { describe, it, expect } from 'vitest';
import {
  MACHINE_DATE_KEY_SEP,
  makeMachineDateKey,
  splitMachineDateKey,
} from './key';

describe('workbench/move/key 工具函数', () => {
  describe('makeMachineDateKey', () => {
    it('应该正确生成机器日期键', () => {
      const key = makeMachineDateKey('M001', '2026-01-30');
      expect(key).toBe('M001__2026-01-30');
    });

    it('应该去除前后空格', () => {
      const key = makeMachineDateKey('  M001  ', '  2026-01-30  ');
      expect(key).toBe('M001__2026-01-30');
    });

    it('应该处理 null/undefined 机器名', () => {
      const key1 = makeMachineDateKey(null as any, '2026-01-30');
      expect(key1).toBe('__2026-01-30');

      const key2 = makeMachineDateKey(undefined as any, '2026-01-30');
      expect(key2).toBe('__2026-01-30');
    });

    it('应该处理 null/undefined 日期', () => {
      const key1 = makeMachineDateKey('M001', null as any);
      expect(key1).toBe('M001__');

      const key2 = makeMachineDateKey('M001', undefined as any);
      expect(key2).toBe('M001__');
    });

    it('应该使用正确的分隔符', () => {
      const key = makeMachineDateKey('M001', '2026-01-30');
      expect(key).toContain(MACHINE_DATE_KEY_SEP);
      expect(key).toContain('__');
    });
  });

  describe('splitMachineDateKey', () => {
    it('应该正确拆分机器日期键', () => {
      const result = splitMachineDateKey('M001__2026-01-30');
      expect(result).toEqual({
        machine: 'M001',
        date: '2026-01-30',
      });
    });

    it('应该处理没有分隔符的键', () => {
      const result = splitMachineDateKey('M001');
      expect(result).toEqual({
        machine: 'M001',
        date: '',
      });
    });

    it('应该处理空键', () => {
      const result = splitMachineDateKey('');
      expect(result).toEqual({
        machine: '',
        date: '',
      });
    });

    it('应该处理 null/undefined 键', () => {
      const result1 = splitMachineDateKey(null as any);
      expect(result1).toEqual({
        machine: '',
        date: '',
      });

      const result2 = splitMachineDateKey(undefined as any);
      expect(result2).toEqual({
        machine: '',
        date: '',
      });
    });

    it('应该处理多个分隔符的情况', () => {
      const result = splitMachineDateKey('M001__2026-01-30__extra');
      expect(result).toEqual({
        machine: 'M001',
        date: '2026-01-30__extra',
      });
    });

    it('应该处理空机器名', () => {
      const result = splitMachineDateKey('__2026-01-30');
      expect(result).toEqual({
        machine: '',
        date: '2026-01-30',
      });
    });

    it('应该处理空日期', () => {
      const result = splitMachineDateKey('M001__');
      expect(result).toEqual({
        machine: 'M001',
        date: '',
      });
    });
  });

  describe('roundtrip 往返测试', () => {
    it('make 和 split 应该互为逆操作', () => {
      const machine = 'M001';
      const date = '2026-01-30';
      const key = makeMachineDateKey(machine, date);
      const result = splitMachineDateKey(key);
      expect(result.machine).toBe(machine);
      expect(result.date).toBe(date);
    });

    it('应该处理特殊字符', () => {
      const machine = 'M-001';
      const date = '2026-01-30';
      const key = makeMachineDateKey(machine, date);
      const result = splitMachineDateKey(key);
      expect(result.machine).toBe(machine);
      expect(result.date).toBe(date);
    });
  });
});

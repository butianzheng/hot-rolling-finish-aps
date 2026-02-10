import { describe, expect, it } from 'vitest';
import { getErrorMessage, isError, normalizeError } from './errorUtils';

describe('errorUtils', () => {
  describe('getErrorMessage', () => {
    it('应该从 Error 实例中提取消息', () => {
      const error = new Error('测试错误');
      expect(getErrorMessage(error)).toBe('测试错误');
    });

    it('应该处理字符串类型的错误', () => {
      expect(getErrorMessage('字符串错误')).toBe('字符串错误');
    });

    it('应该从包含 message 属性的对象中提取消息', () => {
      const error = { message: '对象错误' };
      expect(getErrorMessage(error)).toBe('对象错误');
    });

    it('应该将其他类型转换为字符串', () => {
      expect(getErrorMessage(123)).toBe('123');
      expect(getErrorMessage(null)).toBe('null');
      expect(getErrorMessage(undefined)).toBe('undefined');
    });

    it('应该处理包含非字符串 message 的对象', () => {
      const error = { message: 456 };
      expect(getErrorMessage(error)).toBe('456');
    });
  });

  describe('isError', () => {
    it('应该识别 Error 实例', () => {
      expect(isError(new Error('测试'))).toBe(true);
      expect(isError(new TypeError('类型错误'))).toBe(true);
      expect(isError(new RangeError('范围错误'))).toBe(true);
    });

    it('应该拒绝非 Error 类型', () => {
      expect(isError('字符串')).toBe(false);
      expect(isError(123)).toBe(false);
      expect(isError(null)).toBe(false);
      expect(isError(undefined)).toBe(false);
      expect(isError({ message: '对象' })).toBe(false);
    });
  });

  describe('normalizeError', () => {
    it('应该规范化 Error 实例', () => {
      const error = new Error('测试错误');
      const normalized = normalizeError(error);

      expect(normalized.name).toBe('Error');
      expect(normalized.message).toBe('测试错误');
      expect(normalized.stack).toBeDefined();
    });

    it('应该规范化自定义 Error 类型', () => {
      const error = new TypeError('类型错误');
      const normalized = normalizeError(error);

      expect(normalized.name).toBe('TypeError');
      expect(normalized.message).toBe('类型错误');
      expect(normalized.stack).toBeDefined();
    });

    it('应该规范化字符串错误', () => {
      const normalized = normalizeError('字符串错误');

      expect(normalized.name).toBe('UnknownError');
      expect(normalized.message).toBe('字符串错误');
      expect(normalized.stack).toBeUndefined();
    });

    it('应该规范化对象错误', () => {
      const normalized = normalizeError({ message: '对象错误' });

      expect(normalized.name).toBe('UnknownError');
      expect(normalized.message).toBe('对象错误');
      expect(normalized.stack).toBeUndefined();
    });

    it('应该规范化其他类型的错误', () => {
      const normalized = normalizeError(123);

      expect(normalized.name).toBe('UnknownError');
      expect(normalized.message).toBe('123');
      expect(normalized.stack).toBeUndefined();
    });

    it('应该规范化 null 和 undefined', () => {
      const normalizedNull = normalizeError(null);
      expect(normalizedNull.name).toBe('UnknownError');
      expect(normalizedNull.message).toBe('null');

      const normalizedUndefined = normalizeError(undefined);
      expect(normalizedUndefined.name).toBe('UnknownError');
      expect(normalizedUndefined.message).toBe('undefined');
    });
  });
});

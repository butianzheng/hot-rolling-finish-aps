import { z } from 'zod';

// H14修复：使用严格的正则表达式验证日期格式，防止无效日期字符串通过验证
export const DateString = z.string().regex(/^\d{4}-\d{2}-\d{2}$/, {
  message: 'DateString must be in YYYY-MM-DD format',
});

// H14扩展：同样增强DateTimeString的验证（ISO 8601格式）
export const DateTimeString = z.string().regex(
  /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?$/,
  {
    message: 'DateTimeString must be in ISO 8601 format (YYYY-MM-DDTHH:mm:ss)',
  }
);


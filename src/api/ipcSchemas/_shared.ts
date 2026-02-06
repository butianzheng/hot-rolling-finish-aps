import { z } from 'zod';

// H14修复：使用严格的正则表达式验证日期格式，防止无效日期字符串通过验证
export const DateString = z.string().regex(/^\d{4}-\d{2}-\d{2}$/, {
  message: 'DateString must be in YYYY-MM-DD format',
});
export const DateTimeString = z.string().min(1);


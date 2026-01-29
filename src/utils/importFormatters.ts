/**
 * 格式化工具函数
 * 用于材料导入相关的数据格式化和标签转换
 */

/**
 * 格式化毫秒时间为易读的字符串
 * @param ms 毫秒数
 * @returns 格式化后的字符串（如 "500ms" 或 "2.50s"）
 */
export function formatMs(ms?: number): string {
  if (typeof ms !== 'number' || Number.isNaN(ms)) return '-';
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

/**
 * 将冲突类型代码转换为中文标签
 * @param t 冲突类型代码
 * @returns 中文标签
 */
export function conflictTypeLabel(t: string): string {
  const map: Record<string, string> = {
    PrimaryKeyMissing: '主键缺失',
    PrimaryKeyDuplicate: '主键重复',
    ForeignKeyViolation: '外键/引用错误',
    DataTypeError: '数据类型错误',
  };
  return map[t] || t || '-';
}

/**
 * 错误处理工具函数
 * 用于安全地从 unknown 类型的错误中提取错误信息
 */

/**
 * 从 unknown 类型的错误中提取错误消息
 * @param error - 捕获的错误（类型为 unknown）
 * @returns 错误消息字符串
 */
export function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === 'string') {
    return error;
  }
  if (error && typeof error === 'object' && 'message' in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

/**
 * 判断错误是否为 Error 实例
 * @param error - 捕获的错误
 * @returns 是否为 Error 实例
 */
export function isError(error: unknown): error is Error {
  return error instanceof Error;
}

/**
 * 从 unknown 类型的错误中提取完整的错误信息对象
 * @param error - 捕获的错误
 * @returns 包含 name、message、stack 的对象
 */
export function normalizeError(error: unknown): {
  name: string;
  message: string;
  stack?: string;
} {
  if (error instanceof Error) {
    return {
      name: error.name,
      message: error.message,
      stack: error.stack,
    };
  }
  return {
    name: 'UnknownError',
    message: getErrorMessage(error),
  };
}

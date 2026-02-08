import { invoke } from '@tauri-apps/api/tauri';
import { Modal } from 'antd';
import { reportFrontendEvent } from '../utils/telemetry';
import { handleStalePlanRevError } from '../services/stalePlanRev';

export interface IpcError {
  code: string;
  message: string;
  /** 错误详情（JSON 结构，需运行时检查） */
  details?: Record<string, unknown>;
}

// H12修复：定义统一的超时时间常量和分级策略
export const IPC_TIMEOUT = {
  /** 快速操作（查询、状态检查） - 5秒 */
  FAST: 5_000,
  /** 标准操作（列表加载、单条更新） - 30秒 */
  NORMAL: 30_000,
  /** 慢操作（批量更新、导入） - 60秒 */
  SLOW: 60_000,
  /** 较长操作（大批量数据处理） - 2分钟 */
  LONG: 120_000,
  /** 超长操作（重算、大批量导入） - 5分钟 */
  VERY_SLOW: 300_000,
} as const;

export interface IpcOptions {
  retry?: number;
  timeout?: number;
  showError?: boolean;
  // Optional runtime validation for IPC responses (e.g. Zod schema).
  // If provided and validation throws, the error will go through the same
  // retry/telemetry/modal flow as normal IPC errors.
  validate?: (value: unknown) => unknown;
}

// M7修复：定义import.meta类型接口，替换any提升类型安全
interface ImportMetaEnv {
  DEV?: boolean;
  TAURI_DEBUG?: string;
  [key: string]: unknown;
}

interface ImportMeta {
  env?: ImportMetaEnv;
}

const DEBUG_IPC = Boolean(
  // Vite dev server
  (import.meta as ImportMeta)?.env?.DEV ||
    // Optional: allow enabling IPC logs in Tauri builds via envPrefix ['TAURI_']
    String((import.meta as ImportMeta)?.env?.TAURI_DEBUG || '').toLowerCase() === 'true'
);

export class IpcClient {
  // H11说明：参数传递不一致的根本原因在于后端Tauri命令签名不统一
  // - 部分命令接收对象参数（如 {machine_code: string}）
  // - 部分命令期望JSON字符串参数（如 {machine_codes: string}，其中string是JSON）
  // 当前解决方案：在各API调用层使用JSON.stringify（见src/api/tauri/）
  // 理想方案：统一后端命令签名为对象参数，前端移除所有JSON.stringify
  // TODO: 后端API重构完成后，在IpcClient层实现自动参数规范化

  static async call<T>(
    command: string,
    params: unknown = {},
    options: IpcOptions = {}
  ): Promise<T> {
    // H12修复：使用统一的超时时间常量（默认30秒）
    const { retry = 0, timeout = IPC_TIMEOUT.NORMAL, showError = true } = options;

    if (DEBUG_IPC) {
      // 调试日志：打印 IPC 调用信息（生产环境默认关闭，避免噪声/泄露敏感信息）
      console.log(`[IPC Debug] ===== START =====`);
      console.log(`[IPC Debug] Command: ${command}`);
      console.log(`[IPC Debug] Params JSON:`, JSON.stringify(params));
      if (typeof params === 'object' && params !== null && !Array.isArray(params)) {
        console.log(`[IPC Debug] Params keys:`, Object.keys(params));
      }
      console.log(`[IPC Debug] ===== END =====`);
    }

    let lastError: IpcError | null = null;
    for (let i = 0; i <= retry; i++) {
      try {
        // 确保 params 是 object 类型（Tauri 要求）
        const invokeParams = (typeof params === 'object' && params !== null)
          ? params as Record<string, unknown>
          : {};

        const timeoutError: IpcError = {
          code: 'Timeout',
          message: 'Timeout',
          details: {
            command,
            timeout_ms: timeout,
          },
        };

        const result = await Promise.race([
          invoke(command, invokeParams),
          new Promise((_, reject) =>
            setTimeout(() => reject(timeoutError), timeout)
          )
        ]);
        if (DEBUG_IPC) console.log(`[IPC Debug] Success! Result:`, result);

        // 后端历史实现返回 JSON 字符串；同时兼容直接返回对象的情况。
        const parsed = typeof result === 'string' ? JSON.parse(result) : result;
        const validated = typeof options.validate === 'function' ? options.validate(parsed) : parsed;
        return validated as T;
      } catch (error: unknown) {
        if (DEBUG_IPC) {
          console.error(`[IPC Debug] Error caught:`, error);
          console.error(`[IPC Debug] Error type:`, typeof error);
        }
        lastError = this.parseError(error);
        if (i < retry && this.isRetryable(lastError)) {
          await this.delay(1000 * (i + 1));
          continue;
        }
        break;
      }
    }

    const staleHandled = lastError
      ? await handleStalePlanRevError(lastError, { source: 'ipc', command })
      : false;

    if (lastError && showError && !staleHandled) {
      // best-effort: 将"会弹窗的错误"同步写入后端 action_log，便于线下排查
      const paramsKeys = (typeof params === 'object' && params !== null && !Array.isArray(params))
        ? Object.keys(params)
        : [];
      void reportFrontendEvent('error', `IPC 调用失败: ${command}`, {
        command,
        params_keys: paramsKeys,
        error: lastError,
      });
    }

    if (showError && lastError && !staleHandled) {
      this.showError(lastError);
    }
    throw lastError;
  }

  private static parseError(error: unknown): IpcError {
    if (typeof error === 'string') {
      try {
        return JSON.parse(error);
      } catch {
        return { code: 'Unknown', message: error };
      }
    }
    if (error instanceof Error) {
      return {
        code: 'Unknown',
        message: error.message,
      };
    }
    if (typeof error === 'object' && error !== null) {
      const obj = error as Record<string, unknown>;
      return {
        code: typeof obj.code === 'string' ? obj.code : 'Unknown',
        message: typeof obj.message === 'string' ? obj.message : String(error),
        details: typeof obj.details === 'object' && obj.details !== null
          ? obj.details as Record<string, unknown>
          : undefined,
      };
    }
    return { code: 'Unknown', message: String(error) };
  }

  private static isRetryable(error: IpcError): boolean {
    return ['Timeout', 'NetworkError'].includes(error.code);
  }

  private static showError(error: IpcError) {
    const errorText = JSON.stringify(error, null, 2);
    Modal.error({
      title: `错误: ${error.code}`,
      content: (
        <div>
          <p>{error.message}</p>
          {error.details && (
            <pre style={{ maxHeight: 200, overflow: 'auto', fontSize: 12 }}>
              {JSON.stringify(error.details, null, 2)}
            </pre>
          )}
        </div>
      ),
      okText: '复制错误信息',
      onOk: () => {
        navigator.clipboard.writeText(errorText);
      }
    });
  }

  private static delay(ms: number) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

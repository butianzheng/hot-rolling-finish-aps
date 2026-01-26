import { invoke } from '@tauri-apps/api/tauri';
import { Modal } from 'antd';

export interface IpcError {
  code: string;
  message: string;
  details?: any;
}

export interface IpcOptions {
  retry?: number;
  timeout?: number;
  showError?: boolean;
}

export class IpcClient {
  static async call<T>(
    command: string,
    params: any = {},
    options: IpcOptions = {}
  ): Promise<T> {
    const { retry = 0, timeout = 30000, showError = true } = options;

    // 调试日志：打印 IPC 调用信息
    console.log(`[IPC Debug] ===== START =====`);
    console.log(`[IPC Debug] Command: ${command}`);
    console.log(`[IPC Debug] Params JSON:`, JSON.stringify(params));
    console.log(`[IPC Debug] Params keys:`, Object.keys(params));
    console.log(`[IPC Debug] ===== END =====`);

    let lastError: IpcError | null = null;
    for (let i = 0; i <= retry; i++) {
      try {
        const result = await Promise.race([
          invoke(command, params),
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error('Timeout')), timeout)
          )
        ]);
        console.log(`[IPC Debug] Success! Result:`, result);
        return JSON.parse(result as string);
      } catch (error: any) {
        console.error(`[IPC Debug] Error caught:`, error);
        console.error(`[IPC Debug] Error type:`, typeof error);
        lastError = this.parseError(error);
        if (i < retry && this.isRetryable(lastError)) {
          await this.delay(1000 * (i + 1));
          continue;
        }
        break;
      }
    }

    if (showError && lastError) {
      this.showError(lastError);
    }
    throw lastError;
  }

  private static parseError(error: any): IpcError {
    if (typeof error === 'string') {
      try {
        return JSON.parse(error);
      } catch {
        return { code: 'Unknown', message: error };
      }
    }
    return {
      code: error.code || 'Unknown',
      message: error.message || String(error),
      details: error.details
    };
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

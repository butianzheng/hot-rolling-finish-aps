import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { reportFrontendEvent, reportFrontendError } from './telemetry';

// Mock IpcClient
vi.mock('../api/ipcClient', () => ({
  IpcClient: {
    call: vi.fn().mockResolvedValue(null),
  },
}));

// Mock useGlobalStore
vi.mock('../stores/use-global-store', () => ({
  useGlobalStore: {
    getState: vi.fn(() => ({
      currentUser: 'test_user',
      activeVersionId: 'test_version',
    })),
  },
}));

describe('telemetry', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Mock Tauri runtime
    (global as any).window = {
      __TAURI__: true,
      location: {
        pathname: '/test',
        search: '?param=value',
        href: 'http://localhost/test?param=value',
      },
    };
    (global as any).navigator = {
      userAgent: 'Test User Agent',
    };
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('reportFrontendEvent', () => {
    it('应该报告前端事件', async () => {
      const { IpcClient } = await import('../api/ipcClient');

      await reportFrontendEvent('info', '测试消息', { key: 'value' });

      expect(IpcClient.call).toHaveBeenCalledWith(
        'report_frontend_event',
        expect.objectContaining({
          level: 'info',
          message: '测试消息',
          version_id: 'test_version',
          actor: 'test_user',
          payload_json: expect.objectContaining({
            route: '/test?param=value',
            url: 'http://localhost/test?param=value',
            user_agent: 'Test User Agent',
          }),
        }),
        { showError: false }
      );
    });

    it('应该处理不同的日志级别', async () => {
      const { IpcClient } = await import('../api/ipcClient');

      await reportFrontendEvent('error', '错误消息');
      expect(IpcClient.call).toHaveBeenCalledWith(
        'report_frontend_event',
        expect.objectContaining({ level: 'error' }),
        { showError: false }
      );

      await reportFrontendEvent('warn', '警告消息');
      expect(IpcClient.call).toHaveBeenCalledWith(
        'report_frontend_event',
        expect.objectContaining({ level: 'warn' }),
        { showError: false }
      );

      await reportFrontendEvent('debug', '调试消息');
      expect(IpcClient.call).toHaveBeenCalledWith(
        'report_frontend_event',
        expect.objectContaining({ level: 'debug' }),
        { showError: false }
      );
    });

    it('应该在非 Tauri 环境中静默失败', async () => {
      delete (global as any).window.__TAURI__;

      await reportFrontendEvent('info', '测试消息');

      const { IpcClient } = await import('../api/ipcClient');
      expect(IpcClient.call).not.toHaveBeenCalled();
    });

    it('应该截断过长的消息', async () => {
      const { IpcClient } = await import('../api/ipcClient');
      const longMessage = 'a'.repeat(20000);

      await reportFrontendEvent('info', longMessage);

      const callArgs = (IpcClient.call as any).mock.calls[0][1];
      expect(callArgs.message.length).toBeLessThan(20000);
      expect(callArgs.message).toContain('...<truncated');
    });
  });

  describe('reportFrontendError', () => {
    it('应该报告 Error 实例', async () => {
      const { IpcClient } = await import('../api/ipcClient');
      const error = new Error('测试错误');

      await reportFrontendError(error, { context: 'test' });

      expect(IpcClient.call).toHaveBeenCalled();
      const callArgs = (IpcClient.call as any).mock.calls[0];
      expect(callArgs[0]).toBe('report_frontend_event');
      expect(callArgs[1].level).toBe('error');
      expect(callArgs[1].message).toBe('测试错误');
      expect(callArgs[1].payload_json.payload.error.name).toBe('Error');
      expect(callArgs[1].payload_json.payload.error.message).toBe('测试错误');
      expect(callArgs[1].payload_json.payload.error.stack).toBeDefined();
      expect(callArgs[1].payload_json.payload.context).toEqual({ context: 'test' });
      expect(callArgs[2]).toEqual({ showError: false });
    });

    it('应该报告字符串错误', async () => {
      const { IpcClient } = await import('../api/ipcClient');

      await reportFrontendError('字符串错误');

      expect(IpcClient.call).toHaveBeenCalledWith(
        'report_frontend_event',
        expect.objectContaining({
          level: 'error',
          message: '字符串错误',
        }),
        { showError: false }
      );
    });

    it('应该报告未知类型的错误', async () => {
      const { IpcClient } = await import('../api/ipcClient');

      await reportFrontendError({ custom: 'error' });

      expect(IpcClient.call).toHaveBeenCalledWith(
        'report_frontend_event',
        expect.objectContaining({
          level: 'error',
        }),
        { showError: false }
      );
    });

    it('应该处理 null 错误', async () => {
      const { IpcClient } = await import('../api/ipcClient');

      await reportFrontendError(null, { type: 'null_test' });

      expect(IpcClient.call).toHaveBeenCalled();
      const callArgs = (IpcClient.call as any).mock.calls[0];
      expect(callArgs[1].message).toBe('Unknown error');
    });

    it('应该在非 Tauri 环境中静默失败', async () => {
      delete (global as any).window.__TAURI__;

      await reportFrontendError(new Error('测试'));

      const { IpcClient } = await import('../api/ipcClient');
      expect(IpcClient.call).not.toHaveBeenCalled();
    });
  });
});

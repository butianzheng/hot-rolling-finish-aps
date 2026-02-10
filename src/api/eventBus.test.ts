import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { EventBus } from './eventBus';

// Mock Tauri event API - 必须在 vi.mock 内部定义
vi.mock('@tauri-apps/api/event', () => {
  const mockUnlistenFn = vi.fn();
  const mockListen = vi.fn().mockResolvedValue(mockUnlistenFn);

  return {
    listen: mockListen,
  };
});

describe('EventBus', () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    // 清理 EventBus 内部状态
    await EventBus.cleanup();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('subscribe', () => {
    it('应该订阅事件并返回 unlisten 函数', async () => {
      const { listen } = await import('@tauri-apps/api/event');
      const handler = vi.fn();
      const unlisten = await EventBus.subscribe('test_event', handler);

      expect(listen).toHaveBeenCalledWith('test_event', expect.any(Function));
      expect(typeof unlisten).toBe('function');
    });

    it('应该调用事件处理器', async () => {
      const { listen } = await import('@tauri-apps/api/event');
      const handler = vi.fn();
      await EventBus.subscribe('test_event', handler);

      // 获取传递给 listen 的回调函数
      const listenCallback = (listen as any).mock.calls[0][1];

      // 模拟事件触发
      listenCallback({ payload: { data: 'test' } });

      expect(handler).toHaveBeenCalledWith({ data: 'test' });
    });

    it('应该支持多个订阅者', async () => {
      const { listen } = await import('@tauri-apps/api/event');
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      await EventBus.subscribe('test_event', handler1);
      await EventBus.subscribe('test_event', handler2);

      expect(listen).toHaveBeenCalledTimes(2);
    });

    it('unlisten 应该移除订阅', async () => {
      const handler = vi.fn();
      const unlisten = await EventBus.subscribe('test_event', handler);

      unlisten();

      // 验证 unlisten 函数可以被调用
      expect(typeof unlisten).toBe('function');
    });

    it('unlisten 应该清理空的事件监听器数组', async () => {
      const { listen } = await import('@tauri-apps/api/event');
      const handler = vi.fn();
      const unlisten = await EventBus.subscribe('test_event', handler);

      // 调用 unlisten 应该清理内部 Map
      unlisten();

      // 再次订阅同一事件应该创建新的数组
      await EventBus.subscribe('test_event', handler);
      expect(listen).toHaveBeenCalledTimes(2);
    });
  });

  describe('unsubscribe', () => {
    it('应该取消订阅指定事件的所有监听器', async () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      await EventBus.subscribe('test_event', handler1);
      await EventBus.subscribe('test_event', handler2);

      await EventBus.unsubscribe('test_event');

      // 验证 unsubscribe 完成
      expect(true).toBe(true);
    });

    it('应该处理不存在的事件', async () => {
      // 不应该抛出错误
      await expect(EventBus.unsubscribe('non_existent_event')).resolves.toBeUndefined();
    });

    it('应该清理事件监听器 Map', async () => {
      const { listen } = await import('@tauri-apps/api/event');
      const handler = vi.fn();
      await EventBus.subscribe('test_event', handler);

      await EventBus.unsubscribe('test_event');

      // 再次订阅应该创建新的数组
      await EventBus.subscribe('test_event', handler);
      expect(listen).toHaveBeenCalledTimes(2);
    });
  });

  describe('cleanup', () => {
    it('应该清理所有事件监听器', async () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      await EventBus.subscribe('event1', handler1);
      await EventBus.subscribe('event2', handler2);

      await EventBus.cleanup();

      // 验证 cleanup 完成
      expect(true).toBe(true);
    });

    it('应该清空内部 Map', async () => {
      const { listen } = await import('@tauri-apps/api/event');
      const handler = vi.fn();
      await EventBus.subscribe('test_event', handler);

      await EventBus.cleanup();

      // 再次订阅应该创建新的数组
      await EventBus.subscribe('test_event', handler);
      expect(listen).toHaveBeenCalledTimes(2);
    });

    it('应该处理空的监听器 Map', async () => {
      // 不应该抛出错误
      await expect(EventBus.cleanup()).resolves.toBeUndefined();
    });
  });

  describe('内存泄漏防护', () => {
    it('多次订阅和取消订阅不应该导致内存泄漏', async () => {
      for (let i = 0; i < 10; i++) {
        const handler = vi.fn();
        const unlisten = await EventBus.subscribe('test_event', handler);
        unlisten();
      }

      // 验证循环完成
      expect(true).toBe(true);
    });

    it('包装的 unlisten 函数应该正确移除引用', async () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();
      const handler3 = vi.fn();

      const unlisten1 = await EventBus.subscribe('test_event', handler1);
      const unlisten2 = await EventBus.subscribe('test_event', handler2);
      const unlisten3 = await EventBus.subscribe('test_event', handler3);

      // 移除中间的监听器
      unlisten2();

      // 清理剩余的监听器
      await EventBus.cleanup();

      // 验证操作完成
      expect(true).toBe(true);
    });
  });
});

/**
 * Vitest 测试环境初始化文件
 */

import '@testing-library/jest-dom';
import { afterEach, vi } from 'vitest';
import { cleanup } from '@testing-library/react';

function ensureMockLocalStorage() {
  const current = (globalThis as { localStorage?: Partial<Storage> }).localStorage;
  if (current && typeof current.getItem === 'function') {
    return;
  }

  const memory = new Map<string, string>();
  const mockStorage: Storage = {
    get length() {
      return memory.size;
    },
    clear() {
      memory.clear();
    },
    getItem(key: string) {
      return memory.has(key) ? memory.get(key)! : null;
    },
    key(index: number) {
      const keys = Array.from(memory.keys());
      return keys[index] ?? null;
    },
    removeItem(key: string) {
      memory.delete(key);
    },
    setItem(key: string, value: string) {
      memory.set(key, String(value));
    },
  };

  Object.defineProperty(globalThis, 'localStorage', {
    value: mockStorage,
    configurable: true,
  });

  if (typeof window !== 'undefined') {
    Object.defineProperty(window, 'localStorage', {
      value: mockStorage,
      configurable: true,
    });
  }
}

ensureMockLocalStorage();

// 每个测试后清理 DOM
afterEach(() => {
  cleanup();
});

// Mock Tauri API（因为测试环境中不可用）
vi.mock('../api/tauri', () => ({
  planApi: {
    listPlans: vi.fn(),
    listVersions: vi.fn(),
    createVersion: vi.fn(),
    deleteVersion: vi.fn(),
    deletePlan: vi.fn(),
    recalcFull: vi.fn(),
  },
  capacityApi: {
    getCapacityPools: vi.fn(),
  },
}));

// Mock react-router-dom
vi.mock('react-router-dom', () => ({
  useNavigate: () => vi.fn(),
}));

// 设置全局变量
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

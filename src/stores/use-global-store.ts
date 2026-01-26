// ==========================================
// Zustand 全局状态管理
// ==========================================
// 替代原有的 React Context 方案
// 提供更好的性能、中间件支持和 DevTools 集成
// ==========================================

import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';

// ==========================================
// 全局状态接口
// ==========================================

export interface GlobalState {
  // 当前激活的排产版本ID
  activeVersionId: string | null;

  // 任务状态标志
  isRecalculating: boolean;
  isImporting: boolean;

  // 用户信息
  currentUser: string;

  // 管理员覆盖模式
  adminOverrideMode: boolean;
}

// ==========================================
// 全局状态 Actions
// ==========================================

export interface GlobalActions {
  // 设置激活版本
  setActiveVersion: (versionId: string | null) => void;

  // 设置重算状态
  setRecalculating: (flag: boolean) => void;

  // 设置导入状态
  setImporting: (flag: boolean) => void;

  // 设置当前用户
  setCurrentUser: (user: string) => void;

  // 设置管理员覆盖模式
  setAdminOverrideMode: (flag: boolean) => void;

  // 重置所有状态
  reset: () => void;
}

// ==========================================
// 初始状态
// ==========================================

const LEGACY_CURRENT_USER_KEY = 'aps_current_user';

const initialState: GlobalState = {
  activeVersionId: null,
  isRecalculating: false,
  isImporting: false,
  // 兼容旧的 React Context 持久化 key，避免升级后用户选择被“重置”
  currentUser:
    typeof window !== 'undefined'
      ? localStorage.getItem(LEGACY_CURRENT_USER_KEY) || 'admin'
      : 'admin',
  adminOverrideMode: false,
};

// ==========================================
// Zustand Store
// ==========================================

export const useGlobalStore = create<GlobalState & GlobalActions>()(
  // immer 中间件 - 允许直接修改 state（自动转为不可变更新）
  immer(
    // persist 中间件 - 持久化存储
    persist(
      (set) => ({
        // 初始状态
        ...initialState,

        // Actions
        setActiveVersion: (versionId) =>
          set((state) => {
            state.activeVersionId = versionId;
          }),

        setRecalculating: (flag) =>
          set((state) => {
            state.isRecalculating = flag;
          }),

        setImporting: (flag) =>
          set((state) => {
            state.isImporting = flag;
          }),

        setCurrentUser: (user) =>
          set((state) => {
            state.currentUser = user;
          }),

        setAdminOverrideMode: (flag) =>
          set((state) => {
            state.adminOverrideMode = flag;
          }),

        reset: () => set(initialState),
      }),
      {
        name: 'aps-global-state', // localStorage key
        storage: createJSONStorage(() => localStorage),
        // 只持久化部分字段（临时状态不持久化）
        partialize: (state) => ({
          currentUser: state.currentUser,
          adminOverrideMode: state.adminOverrideMode,
        }),
      }
    )
  )
);

// ==========================================
// Selector Hooks（性能优化 - 只订阅需要的状态）
// ==========================================

// 获取当前用户
export const useCurrentUser = () => useGlobalStore((state) => state.currentUser);

// 获取激活版本ID
export const useActiveVersionId = () => useGlobalStore((state) => state.activeVersionId);

// 获取重算状态
export const useIsRecalculating = () => useGlobalStore((state) => state.isRecalculating);

// 获取导入状态
export const useIsImporting = () => useGlobalStore((state) => state.isImporting);

// 获取管理员覆盖模式
export const useAdminOverrideMode = () => useGlobalStore((state) => state.adminOverrideMode);

// 获取所有 Actions（不会触发重新渲染）
export const useGlobalActions = () =>
  useGlobalStore((state) => ({
    setActiveVersion: state.setActiveVersion,
    setRecalculating: state.setRecalculating,
    setImporting: state.setImporting,
    setCurrentUser: state.setCurrentUser,
    setAdminOverrideMode: state.setAdminOverrideMode,
    reset: state.reset,
  }));

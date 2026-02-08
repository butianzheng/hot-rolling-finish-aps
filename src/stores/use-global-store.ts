// ==========================================
// Zustand 全局状态管理
// ==========================================
// 替代原有的 React Context 方案
// 提供更好的性能、中间件支持和 DevTools 集成
// ==========================================

import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';
import type { UserPreferences } from '../types/preferences';
import {
  beginLatestRunState,
  createInitialLatestRunState,
  expireLatestRunState,
  markLatestRunDoneState,
  markLatestRunFailedState,
  markLatestRunRunningState,
  type LatestRunState,
} from './latestRun';

// ==========================================
// 全局状态接口
// ==========================================

export type VersionComparisonMode = 'DRAFT_COMPARISON' | 'HISTORICAL';
export type WorkbenchViewMode = 'MATRIX' | 'GANTT' | 'CARD';
export type WorkbenchLockStatusFilter = 'ALL' | 'LOCKED' | 'UNLOCKED';

export interface WorkbenchFilters {
  machineCode: string | null;
  urgencyLevel: string | null;
  lockStatus: WorkbenchLockStatusFilter;
}

export interface GlobalState {
  // 当前激活的排产版本ID
  activeVersionId: string | null;
  // 当前激活版本的 plan_rev（plan_version.revision）
  activePlanRev: number | null;

  // 重算 latest run（前端状态治理，不持久化）
  latestRun: LatestRunState;

  // 任务状态标志
  isRecalculating: boolean;
  isImporting: boolean;

  // 用户信息
  currentUser: string;

  // 管理员覆盖模式
  adminOverrideMode: boolean;

  // 版本对比
  versionComparisonMode: VersionComparisonMode | null;
  selectedVersionA: string | null;
  selectedVersionB: string | null;

  // 工作台
  workbenchViewMode: WorkbenchViewMode;
  workbenchFilters: WorkbenchFilters;

  // 用户偏好
  userPreferences: UserPreferences;
}

// ==========================================
// 全局状态 Actions
// ==========================================

export interface GlobalActions {
  // 设置激活版本
  setActiveVersion: (versionId: string | null) => void;

  // 设置当前计划上下文
  setPlanContext: (context: { versionId: string | null; planRev: number | null }) => void;

  // latest run 状态机
  beginLatestRun: (input: { runId: string; triggeredAt?: number; ttlMs?: number; versionId?: string | null }) => {
    accepted: boolean;
    reason?: 'OLDER_TRIGGER' | 'EXPIRED_PREVIOUS';
  };
  markLatestRunRunning: (runId: string) => void;
  markLatestRunDone: (runId: string, payload?: { versionId?: string | null; planRev?: number | null }) => void;
  markLatestRunFailed: (runId: string, error?: string | null) => void;
  expireLatestRunIfNeeded: (now?: number) => void;

  // 设置重算状态
  setRecalculating: (flag: boolean) => void;

  // 设置导入状态
  setImporting: (flag: boolean) => void;

  // 设置当前用户
  setCurrentUser: (user: string) => void;

  // 设置管理员覆盖模式
  setAdminOverrideMode: (flag: boolean) => void;

  // 设置版本对比模式
  setVersionComparisonMode: (mode: VersionComparisonMode | null) => void;

  // 设置对比版本
  setSelectedVersionA: (versionId: string | null) => void;
  setSelectedVersionB: (versionId: string | null) => void;

  // 工作台视图/筛选
  setWorkbenchViewMode: (mode: WorkbenchViewMode) => void;
  setWorkbenchFilters: (filters: Partial<WorkbenchFilters>) => void;

  // 用户偏好
  updateUserPreferences: (updates: Partial<UserPreferences>) => void;

  // 重置所有状态
  reset: () => void;
}

// ==========================================
// 初始状态
// ==========================================

const LEGACY_CURRENT_USER_KEY = 'aps_current_user';

const initialState: GlobalState = {
  activeVersionId: null,
  activePlanRev: null,
  latestRun: createInitialLatestRunState(),
  isRecalculating: false,
  isImporting: false,
  // 兼容旧的 React Context 持久化 key，避免升级后用户选择被“重置”
  currentUser:
    typeof window !== 'undefined'
      ? localStorage.getItem(LEGACY_CURRENT_USER_KEY) || 'admin'
      : 'admin',
  adminOverrideMode: false,

  versionComparisonMode: null,
  selectedVersionA: null,
  selectedVersionB: null,

  workbenchViewMode: 'MATRIX',
  workbenchFilters: {
    machineCode: null,
    urgencyLevel: null,
    lockStatus: 'ALL',
  },

  userPreferences: {
    defaultTheme:
      typeof window !== 'undefined'
        ? ((localStorage.getItem('theme') as UserPreferences['defaultTheme'] | null) || 'dark')
        : 'dark',
    autoRefreshInterval: 30_000,
    sidebarCollapsed: false,
    defaultStrategy: 'balanced',
  },
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
            const nextVersionId = String(versionId || '').trim() || null;
            const changed = state.activeVersionId !== nextVersionId;
            state.activeVersionId = nextVersionId;
            if (changed) {
              state.activePlanRev = null;
            }
          }),

        setPlanContext: ({ versionId, planRev }) =>
          set((state) => {
            state.activeVersionId = String(versionId || '').trim() || null;
            state.activePlanRev = typeof planRev === 'number' ? planRev : null;
          }),

        beginLatestRun: (input) => {
          let accepted = false;
          let reason: 'OLDER_TRIGGER' | 'EXPIRED_PREVIOUS' | undefined;

          set((state) => {
            const result = beginLatestRunState(state.latestRun, input);
            accepted = result.accepted;
            reason = result.reason;
            state.latestRun = result.next;
          });

          return { accepted, reason };
        },

        markLatestRunRunning: (runId) =>
          set((state) => {
            state.latestRun = markLatestRunRunningState(state.latestRun, runId);
          }),

        markLatestRunDone: (runId, payload) =>
          set((state) => {
            state.latestRun = markLatestRunDoneState(state.latestRun, runId, payload);
            if (state.latestRun.runId === runId && state.latestRun.status === 'DONE') {
              if (payload?.versionId) {
                state.activeVersionId = payload.versionId;
              }
              if (typeof payload?.planRev === 'number') {
                state.activePlanRev = payload.planRev;
              }
            }
          }),

        markLatestRunFailed: (runId, error) =>
          set((state) => {
            state.latestRun = markLatestRunFailedState(state.latestRun, runId, error ?? null);
          }),

        expireLatestRunIfNeeded: (now) =>
          set((state) => {
            state.latestRun = expireLatestRunState(state.latestRun, now);
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

        setVersionComparisonMode: (mode) =>
          set((state) => {
            state.versionComparisonMode = mode;
          }),

        setSelectedVersionA: (versionId) =>
          set((state) => {
            state.selectedVersionA = versionId;
          }),

        setSelectedVersionB: (versionId) =>
          set((state) => {
            state.selectedVersionB = versionId;
          }),

        setWorkbenchViewMode: (mode) =>
          set((state) => {
            state.workbenchViewMode = mode;
          }),

        setWorkbenchFilters: (filters) =>
          set((state) => {
            state.workbenchFilters = { ...state.workbenchFilters, ...filters };
          }),

        updateUserPreferences: (updates) =>
          set((state) => {
            state.userPreferences = { ...state.userPreferences, ...updates };
          }),

        reset: () => set(initialState),
      }),
      {
        name: 'aps-global-state', // localStorage key
        storage: createJSONStorage(() => localStorage),
        // H10修复：持久化版本对比状态，避免刷新后丢失对比选择
        partialize: (state) => ({
          activeVersionId: state.activeVersionId,
          currentUser: state.currentUser,
          adminOverrideMode: state.adminOverrideMode,
          versionComparisonMode: state.versionComparisonMode,
          selectedVersionA: state.selectedVersionA,
          selectedVersionB: state.selectedVersionB,
          workbenchViewMode: state.workbenchViewMode,
          workbenchFilters: state.workbenchFilters,
          userPreferences: state.userPreferences,
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

// 获取激活版本 plan_rev
export const useActivePlanRev = () => useGlobalStore((state) => state.activePlanRev);

// 获取 latest run
export const useLatestRun = () => useGlobalStore((state) => state.latestRun);

// 获取重算状态
export const useIsRecalculating = () => useGlobalStore((state) => state.isRecalculating);

// 获取导入状态
export const useIsImporting = () => useGlobalStore((state) => state.isImporting);

// 获取管理员覆盖模式
export const useAdminOverrideMode = () => useGlobalStore((state) => state.adminOverrideMode);

// 获取用户偏好
export const useUserPreferences = () => useGlobalStore((state) => state.userPreferences);

// 获取所有 Actions（不会触发重新渲染）
export const useGlobalActions = () =>
  useGlobalStore((state) => ({
    setActiveVersion: state.setActiveVersion,
    setPlanContext: state.setPlanContext,
    beginLatestRun: state.beginLatestRun,
    markLatestRunRunning: state.markLatestRunRunning,
    markLatestRunDone: state.markLatestRunDone,
    markLatestRunFailed: state.markLatestRunFailed,
    expireLatestRunIfNeeded: state.expireLatestRunIfNeeded,
    setRecalculating: state.setRecalculating,
    setImporting: state.setImporting,
    setCurrentUser: state.setCurrentUser,
    setAdminOverrideMode: state.setAdminOverrideMode,
    setVersionComparisonMode: state.setVersionComparisonMode,
    setSelectedVersionA: state.setSelectedVersionA,
    setSelectedVersionB: state.setSelectedVersionB,
    setWorkbenchViewMode: state.setWorkbenchViewMode,
    setWorkbenchFilters: state.setWorkbenchFilters,
    updateUserPreferences: state.updateUserPreferences,
    reset: state.reset,
  }));

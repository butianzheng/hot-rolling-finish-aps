// ==========================================
// Zustand 排产方案状态管理
// ==========================================
// 管理排产方案相关的状态和操作
// ==========================================

import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import type { StrategyDraft } from '../types/comparison';

// ==========================================
// 类型定义（与后端保持一致）
// ==========================================

export interface Plan {
  plan_id: string;
  plan_name: string;
  created_at: string;
  created_by: string;
  active_version_id?: string | null;
}

export interface PlanVersion {
  version_id: string;
  plan_id: string;
  version_seq: number;
  window_days: number;
  frozen_from_date?: string | null;
  note?: string | null;
  created_at: string;
  created_by: string;
  status: 'DRAFT' | 'ACTIVE' | 'ARCHIVED';
}

// ==========================================
// 排产状态接口
// ==========================================

export interface PlanState {
  // 排产方案列表
  plans: Plan[];

  // 当前选中的方案ID
  selectedPlanId: string | null;

  // 当前选中方案的版本列表
  versions: PlanVersion[];

  // 当前选中的版本ID
  selectedVersionId: string | null;

  // 加载状态
  isLoadingPlans: boolean;
  isLoadingVersions: boolean;

  // 策略草案（Phase 3：多策略对比）
  draftVersions: StrategyDraft[];
}

// ==========================================
// 排产状态 Actions
// ==========================================

export interface PlanActions {
  // 设置排产方案列表
  setPlans: (plans: Plan[]) => void;

  // 设置选中的方案ID
  setSelectedPlanId: (planId: string | null) => void;

  // 设置版本列表
  setVersions: (versions: PlanVersion[]) => void;

  // 设置选中的版本ID
  setSelectedVersionId: (versionId: string | null) => void;

  // 设置加载状态
  setLoadingPlans: (loading: boolean) => void;
  setLoadingVersions: (loading: boolean) => void;

  // 添加新方案
  addPlan: (plan: Plan) => void;

  // 添加新版本
  addVersion: (version: PlanVersion) => void;

  // 更新方案
  updatePlan: (planId: string, updates: Partial<Plan>) => void;

  // 更新版本
  updateVersion: (versionId: string, updates: Partial<PlanVersion>) => void;

  // 激活版本
  activateVersion: (versionId: string) => void;

  // 重置状态
  reset: () => void;

  // 策略草案（Phase 3：多策略对比）
  setDraftVersions: (drafts: StrategyDraft[]) => void;
  clearDraftVersions: () => void;

  // 预留：多策略草案 API（后端实现后接入）
  createDraftVersion: (sourceVersionId: string, note?: string) => Promise<string>;
  publishDraft: (draftId: string) => Promise<string>;
}

// ==========================================
// 初始状态
// ==========================================

const initialState: PlanState = {
  plans: [],
  selectedPlanId: null,
  versions: [],
  selectedVersionId: null,
  isLoadingPlans: false,
  isLoadingVersions: false,
  draftVersions: [],
};

// ==========================================
// Zustand Store
// ==========================================

export const usePlanStore = create<PlanState & PlanActions>()(
  immer((set) => ({
    // 初始状态
    ...initialState,

    // Actions
    setPlans: (plans) =>
      set((state) => {
        state.plans = plans;
      }),

    setSelectedPlanId: (planId) =>
      set((state) => {
        const prev = state.selectedPlanId;
        state.selectedPlanId = planId;
        // 切换方案时清空版本列表
        if (planId !== prev) {
          state.versions = [];
          state.selectedVersionId = null;
        }
      }),

    setVersions: (versions) =>
      set((state) => {
        state.versions = versions;
      }),

    setSelectedVersionId: (versionId) =>
      set((state) => {
        state.selectedVersionId = versionId;
      }),

    setLoadingPlans: (loading) =>
      set((state) => {
        state.isLoadingPlans = loading;
      }),

    setLoadingVersions: (loading) =>
      set((state) => {
        state.isLoadingVersions = loading;
      }),

    addPlan: (plan) =>
      set((state) => {
        state.plans.push(plan);
      }),

    addVersion: (version) =>
      set((state) => {
        state.versions.push(version);
      }),

    updatePlan: (planId, updates) =>
      set((state) => {
        const planIndex = state.plans.findIndex((p) => p.plan_id === planId);
        if (planIndex !== -1) {
          state.plans[planIndex] = { ...state.plans[planIndex], ...updates };
        }
      }),

    updateVersion: (versionId, updates) =>
      set((state) => {
        const versionIndex = state.versions.findIndex((v) => v.version_id === versionId);
        if (versionIndex !== -1) {
          state.versions[versionIndex] = { ...state.versions[versionIndex], ...updates };
        }
      }),

    // C9修复：此方法已废弃并移除，违反工业规范
    // 正确做法：
    //   1. 调用后端API: await planApi.activateVersion(versionId, operator);
    //   2. 重新加载版本列表或刷新缓存
    // 直接修改UI层的version.status字段会导致前后端数据不一致
    activateVersion: (_versionId) => {
      throw new Error(
        '[DEPRECATED] usePlanStore.activateVersion() 已废弃。\n' +
          '原因：直接修改UI状态违反工业规范，可能导致数据不一致。\n' +
          '正确做法：\n' +
          '  1. 调用后端API: await planApi.activateVersion(versionId, operator);\n' +
          '  2. 调用 useGlobalStore.setActiveVersion(versionId); 更新全局激活版本ID\n' +
          '  3. 重新加载版本列表: await planActions.loadVersions(planId);'
      );
    },

    reset: () => set(initialState),

    setDraftVersions: (drafts) =>
      set((state) => {
        state.draftVersions = drafts;
      }),

    clearDraftVersions: () =>
      set((state) => {
        state.draftVersions = [];
      }),

    createDraftVersion: async () => {
      throw new Error('createDraftVersion is not implemented yet');
    },

    publishDraft: async () => {
      throw new Error('publishDraft is not implemented yet');
    },
  }))
);

// ==========================================
// Selector Hooks（性能优化）
// ==========================================

// 获取排产方案列表
export const usePlans = () => usePlanStore((state) => state.plans);

// 获取当前选中的方案ID
export const useSelectedPlanId = () => usePlanStore((state) => state.selectedPlanId);

// 获取版本列表
export const useVersions = () => usePlanStore((state) => state.versions);

// 获取当前选中的版本ID
export const useSelectedVersionId = () => usePlanStore((state) => state.selectedVersionId);

// 获取加载状态
export const useIsLoadingPlans = () => usePlanStore((state) => state.isLoadingPlans);
export const useIsLoadingVersions = () => usePlanStore((state) => state.isLoadingVersions);

// 获取当前选中的方案
export const useSelectedPlan = () =>
  usePlanStore((state) => state.plans.find((p) => p.plan_id === state.selectedPlanId) || null);

// 获取当前选中的版本
export const useSelectedVersion = () =>
  usePlanStore((state) =>
    state.versions.find((v) => v.version_id === state.selectedVersionId) || null
  );

// 获取激活的版本
export const useActiveVersion = () =>
  usePlanStore((state) => state.versions.find((v) => v.status === 'ACTIVE') || null);

// 获取所有 Actions (C9修复：移除activateVersion导出)
export const usePlanActions = () =>
  usePlanStore((state) => ({
    setPlans: state.setPlans,
    setSelectedPlanId: state.setSelectedPlanId,
    setVersions: state.setVersions,
    setSelectedVersionId: state.setSelectedVersionId,
    setLoadingPlans: state.setLoadingPlans,
    setLoadingVersions: state.setLoadingVersions,
    addPlan: state.addPlan,
    addVersion: state.addVersion,
    updatePlan: state.updatePlan,
    updateVersion: state.updateVersion,
    // activateVersion 已废弃并移除，请使用 planApi.activateVersion()
    reset: state.reset,
    setDraftVersions: state.setDraftVersions,
    clearDraftVersions: state.clearDraftVersions,
    createDraftVersion: state.createDraftVersion,
    publishDraft: state.publishDraft,
  }));

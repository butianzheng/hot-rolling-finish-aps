// ==========================================
// Zustand 排产方案状态管理
// ==========================================
// 管理排产方案相关的状态和操作
// ==========================================

import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';

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
        state.selectedPlanId = planId;
        // 切换方案时清空版本列表
        if (planId !== state.selectedPlanId) {
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

    activateVersion: (versionId) =>
      set((state) => {
        // 将所有版本设为非激活
        state.versions.forEach((v) => {
          v.status = v.version_id === versionId ? 'ACTIVE' : 'ARCHIVED';
        });
        // 更新当前选中版本
        state.selectedVersionId = versionId;
      }),

    reset: () => set(initialState),
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

// 获取所有 Actions
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
    activateVersion: state.activateVersion,
    reset: state.reset,
  }));

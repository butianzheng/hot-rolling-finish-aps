/**
 * Workbench 弹窗状态管理 Hook
 *
 * 目标：聚合 PlanningWorkbench 中分散的弹窗状态，减少 prop drilling
 *
 * 原来的实现：
 * - const [rhythmModalOpen, setRhythmModalOpen] = useState(false);
 * - const [pathOverrideModalOpen, setPathOverrideModalOpen] = useState(false);
 * - const [pathOverrideCenterOpen, setPathOverrideCenterOpen] = useState(false);
 * - const [conditionalSelectOpen, setConditionalSelectOpen] = useState(false);
 *
 * 重构后：
 * - const { modals, openModal, closeModal } = useWorkbenchModalState();
 * - openModal('rhythm'), closeModal('pathOverrideConfirm'), ...
 *
 * 注意：moveModalOpen 和 inspectorOpen 已通过各自的 hook 返回，不在此聚合。
 */

import { useCallback, useState } from 'react';

export type WorkbenchModalKey =
  | 'rhythm'
  | 'pathOverrideConfirm'
  | 'pathOverrideCenter'
  | 'conditionalSelect';

export type WorkbenchModalState = Record<WorkbenchModalKey, boolean>;

export function useWorkbenchModalState() {
  const [modals, setModals] = useState<WorkbenchModalState>({
    rhythm: false,
    pathOverrideConfirm: false,
    pathOverrideCenter: false,
    conditionalSelect: false,
  });

  const openModal = useCallback((key: WorkbenchModalKey) => {
    setModals(prev => ({ ...prev, [key]: true }));
  }, []);

  const closeModal = useCallback((key: WorkbenchModalKey) => {
    setModals(prev => ({ ...prev, [key]: false }));
  }, []);

  const toggleModal = useCallback((key: WorkbenchModalKey) => {
    setModals(prev => ({ ...prev, [key]: !prev[key] }));
  }, []);

  /**
   * 创建向后兼容的 setter（模拟 useState 的 setXxxOpen）
   *
   * 使用示例：
   *   setRhythmModalOpen={createSetter('rhythm')}
   */
  const createSetter = useCallback((key: WorkbenchModalKey) => {
    return (valueOrUpdater: boolean | ((prev: boolean) => boolean)) => {
      setModals(prev => {
        const currentValue = prev[key];
        const nextValue = typeof valueOrUpdater === 'function'
          ? valueOrUpdater(currentValue)
          : valueOrUpdater;
        return { ...prev, [key]: nextValue };
      });
    };
  }, []);

  return {
    modals,
    openModal,
    closeModal,
    toggleModal,
    setModals,
    createSetter,
  };
}

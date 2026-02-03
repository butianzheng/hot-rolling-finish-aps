import { useCallback, useMemo, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import dayjs from 'dayjs';
import { message } from 'antd';
import { planApi } from '../../../api/tauri';
import { DEFAULT_MOVE_REASON } from '../constants';
import { getStrategyLabel } from '../utils';
import { useWorkbenchMoveImpactPreview } from './useWorkbenchMoveImpactPreview';
import { useWorkbenchMoveRecommend } from './useWorkbenchMoveRecommend';
import { useWorkbenchMoveSubmit } from './useWorkbenchMoveSubmit';
import type {
  MoveModalActions,
  MoveModalState,
  MoveSeqMode,
  MoveValidationMode,
  SelectedPlanItemStats,
} from '../types';

type IpcPlanItem = Awaited<ReturnType<typeof planApi.listPlanItems>>[number];

// Re-export types for convenience
export type { MoveModalState, MoveModalActions } from '../types';

/**
 * Workbench 移位弹窗状态管理（M1-3 瘦身后）
 *
 * 返回值从 30+ 字段精简到 5 字段：
 * - moveModalState: 聚合状态对象
 * - moveModalActions: 聚合操作对象
 * - openMoveModal / openMoveModalAt / openMoveModalWithRecommend: 外部调用方法
 */
export function useWorkbenchMoveModal(params: {
  activeVersionId: string | null;
  operator: string | null;
  deepLinkDate: string | null;
  poolMachineCode: string | null;
  machineOptions: string[];
  defaultStrategy: string | null | undefined;
  workbenchDateRange: [dayjs.Dayjs, dayjs.Dayjs];
  planItems: IpcPlanItem[];
  selectedMaterialIds: string[];
  setSelectedMaterialIds: Dispatch<SetStateAction<string[]>>;
}): {
  moveModalState: MoveModalState;
  moveModalActions: MoveModalActions;
  openMoveModal: () => void;
  openMoveModalAt: (targetMachine: string, targetDate: string) => void;
  openMoveModalWithRecommend: () => void;
} {
  const {
    activeVersionId,
    operator,
    deepLinkDate,
    poolMachineCode,
    machineOptions,
    defaultStrategy,
    workbenchDateRange,
    planItems,
    selectedMaterialIds,
    setSelectedMaterialIds,
  } = params;

  const [moveModalOpen, setMoveModalOpen] = useState(false);
  const [moveTargetMachine, setMoveTargetMachine] = useState<string | null>(null);
  const [moveTargetDate, setMoveTargetDate] = useState<dayjs.Dayjs | null>(dayjs());
  const [moveSeqMode, setMoveSeqMode] = useState<MoveSeqMode>('APPEND');
  const [moveStartSeq, setMoveStartSeq] = useState<number>(1);
  const [moveValidationMode, setMoveValidationMode] = useState<MoveValidationMode>('AUTO_FIX');
  const [moveReason, setMoveReason] = useState<string>('');

  const strategyLabel = useMemo(() => getStrategyLabel(defaultStrategy), [defaultStrategy]);

  const planItemById = useMemo(() => {
    const map = new Map<string, IpcPlanItem>();
    const raw = planItems ?? [];
    raw.forEach((it) => {
      const id = String(it.material_id ?? '').trim();
      if (id) map.set(id, it);
    });
    return map;
  }, [planItems]);

  const selectedPlanItemStats = useMemo<SelectedPlanItemStats>(() => {
    let inPlan = 0;
    let frozenInPlan = 0;
    selectedMaterialIds.forEach((id) => {
      const it = planItemById.get(id);
      if (!it) return;
      inPlan += 1;
      if (it?.locked_in_plan === true) frozenInPlan += 1;
    });
    return { inPlan, frozenInPlan, outOfPlan: selectedMaterialIds.length - inPlan };
  }, [planItemById, selectedMaterialIds]);

  const moveImpactPreview = useWorkbenchMoveImpactPreview({
    activeVersionId,
    moveModalOpen,
    moveTargetMachine,
    moveTargetDate,
    moveValidationMode,
    planItems,
    selectedMaterialIds,
  });

  const {
    moveRecommendLoading,
    moveRecommendSummary,
    clearMoveRecommendSummary,
    scheduleAutoRecommendOnOpen,
    recommendMoveTarget,
  } = useWorkbenchMoveRecommend({
    activeVersionId,
    moveModalOpen,
    moveTargetMachine,
    moveTargetDate,
    setMoveTargetDate,
    moveValidationMode,
    planItems,
    selectedMaterialIds,
    defaultStrategy,
    strategyLabel,
    workbenchDateRange,
  });

  const { moveSubmitting, submitMove } = useWorkbenchMoveSubmit({
    activeVersionId,
    operator,
    moveTargetMachine,
    moveTargetDate,
    moveReason,
    moveSeqMode,
    moveStartSeq,
    moveValidationMode,
    planItems,
    selectedMaterialIds,
    setMoveModalOpen,
    setMoveReason,
    setSelectedMaterialIds,
  });

  /**
   * 重置弹窗状态并打开（内部辅助函数）
   */
  const resetAndOpenModal = useCallback(
    (machine: string | null, date: dayjs.Dayjs | null) => {
      setMoveTargetMachine(machine);
      setMoveTargetDate(date?.isValid() ? date : dayjs());
      setMoveSeqMode('APPEND');
      setMoveStartSeq(1);
      setMoveValidationMode('AUTO_FIX');
      setMoveReason(DEFAULT_MOVE_REASON);
      clearMoveRecommendSummary();
      setMoveModalOpen(true);
    },
    [clearMoveRecommendSummary]
  );

  const openMoveModal = useCallback(() => {
    if (selectedMaterialIds.length === 0) {
      message.warning('请先选择物料');
      return;
    }
    const fallbackMachine = poolMachineCode || machineOptions[0] || null;
    const focusDate = deepLinkDate ? dayjs(deepLinkDate) : dayjs();
    resetAndOpenModal(fallbackMachine, focusDate);
  }, [deepLinkDate, machineOptions, poolMachineCode, resetAndOpenModal, selectedMaterialIds.length]);

  const openMoveModalAt = useCallback(
    (targetMachine: string, targetDate: string) => {
      if (selectedMaterialIds.length === 0) {
        message.warning('请先选择物料');
        return;
      }
      const machine = String(targetMachine || '').trim() || poolMachineCode || machineOptions[0] || null;
      const date = dayjs(targetDate);
      resetAndOpenModal(machine, date);
    },
    [machineOptions, poolMachineCode, resetAndOpenModal, selectedMaterialIds.length]
  );

  const openMoveModalWithRecommend = useCallback(() => {
    if (selectedMaterialIds.length === 0) {
      message.warning('请先选择物料');
      return;
    }
    openMoveModal();
    scheduleAutoRecommendOnOpen();
  }, [openMoveModal, scheduleAutoRecommendOnOpen, selectedMaterialIds.length]);

  // 聚合的状态对象（新增，推荐使用）
  const moveModalState = useMemo(
    () => ({
      open: moveModalOpen,
      targetMachine: moveTargetMachine,
      targetDate: moveTargetDate,
      seqMode: moveSeqMode,
      startSeq: moveStartSeq,
      validationMode: moveValidationMode,
      reason: moveReason,
      submitting: moveSubmitting,
      recommendLoading: moveRecommendLoading,
      recommendSummary: moveRecommendSummary,
      strategyLabel,
      selectedPlanItemStats,
      impactPreview: moveImpactPreview,
    }),
    [
      moveModalOpen,
      moveTargetMachine,
      moveTargetDate,
      moveSeqMode,
      moveStartSeq,
      moveValidationMode,
      moveReason,
      moveSubmitting,
      moveRecommendLoading,
      moveRecommendSummary,
      strategyLabel,
      selectedPlanItemStats,
      moveImpactPreview,
    ]
  );

  // 聚合的操作对象（新增，推荐使用）
  const moveModalActions = useMemo(
    () => ({
      setOpen: setMoveModalOpen,
      setTargetMachine: setMoveTargetMachine,
      setTargetDate: setMoveTargetDate,
      setSeqMode: setMoveSeqMode,
      setStartSeq: setMoveStartSeq,
      setValidationMode: setMoveValidationMode,
      setReason: setMoveReason,
      recommendTarget: recommendMoveTarget,
      openModal: openMoveModal,
      openModalAt: openMoveModalAt,
      openModalWithRecommend: openMoveModalWithRecommend,
      submit: submitMove,
    }),
    [
      setMoveModalOpen,
      setMoveTargetMachine,
      setMoveTargetDate,
      setMoveSeqMode,
      setMoveStartSeq,
      setMoveValidationMode,
      setMoveReason,
      recommendMoveTarget,
      openMoveModal,
      openMoveModalAt,
      openMoveModalWithRecommend,
      submitMove,
    ]
  );

  return {
    moveModalState,
    moveModalActions,
    openMoveModal,
    openMoveModalAt,
    openMoveModalWithRecommend,
  };
}

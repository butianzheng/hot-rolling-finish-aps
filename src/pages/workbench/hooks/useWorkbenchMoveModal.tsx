import { useCallback, useMemo, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import dayjs from 'dayjs';
import { message } from 'antd';
import { planApi } from '../../../api/tauri';
import { DEFAULT_MOVE_REASON } from '../constants';
import { useWorkbenchMoveImpactPreview } from './useWorkbenchMoveImpactPreview';
import { useWorkbenchMoveRecommend } from './useWorkbenchMoveRecommend';
import { useWorkbenchMoveSubmit } from './useWorkbenchMoveSubmit';
import type {
  MoveImpactPreview,
  MoveRecommendSummary,
  MoveSeqMode,
  MoveValidationMode,
  SelectedPlanItemStats,
} from '../types';

type IpcPlanItem = Awaited<ReturnType<typeof planApi.listPlanItems>>[number];

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
  moveModalOpen: boolean;
  setMoveModalOpen: Dispatch<SetStateAction<boolean>>;
  moveTargetMachine: string | null;
  setMoveTargetMachine: Dispatch<SetStateAction<string | null>>;
  moveTargetDate: dayjs.Dayjs | null;
  setMoveTargetDate: Dispatch<SetStateAction<dayjs.Dayjs | null>>;
  moveSeqMode: MoveSeqMode;
  setMoveSeqMode: Dispatch<SetStateAction<MoveSeqMode>>;
  moveStartSeq: number;
  setMoveStartSeq: Dispatch<SetStateAction<number>>;
  moveValidationMode: MoveValidationMode;
  setMoveValidationMode: Dispatch<SetStateAction<MoveValidationMode>>;
  moveSubmitting: boolean;
  moveReason: string;
  setMoveReason: Dispatch<SetStateAction<string>>;
  moveRecommendLoading: boolean;
  moveRecommendSummary: MoveRecommendSummary | null;
  strategyLabel: string;
  selectedPlanItemStats: SelectedPlanItemStats;
  moveImpactPreview: MoveImpactPreview | null;
  recommendMoveTarget: () => Promise<void>;
  openMoveModal: () => void;
  openMoveModalAt: (targetMachine: string, targetDate: string) => void;
  openMoveModalWithRecommend: () => void;
  submitMove: () => Promise<void>;
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

  const strategyLabel = useMemo(() => {
    const v = String(defaultStrategy || 'balanced');
    if (v === 'urgent_first') return '紧急优先';
    if (v === 'capacity_first') return '产能优先';
    if (v === 'cold_stock_first') return '冷坯消化';
    if (v === 'manual') return '手动调整';
    return '均衡方案';
  }, [defaultStrategy]);

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

  const openMoveModal = useCallback(() => {
    if (selectedMaterialIds.length === 0) {
      message.warning('请先选择物料');
      return;
    }

    const fallbackMachine = poolMachineCode || machineOptions[0] || null;
    const focusDate = deepLinkDate ? dayjs(deepLinkDate) : dayjs();
    setMoveTargetMachine(fallbackMachine);
    setMoveTargetDate(focusDate.isValid() ? focusDate : dayjs());
    setMoveSeqMode('APPEND');
    setMoveStartSeq(1);
    setMoveValidationMode('AUTO_FIX');
    setMoveReason(DEFAULT_MOVE_REASON);
    clearMoveRecommendSummary();
    setMoveModalOpen(true);
  }, [clearMoveRecommendSummary, deepLinkDate, machineOptions, poolMachineCode, selectedMaterialIds.length]);

  const openMoveModalAt = useCallback(
    (targetMachine: string, targetDate: string) => {
      if (selectedMaterialIds.length === 0) {
        message.warning('请先选择物料');
        return;
      }

      const machine = String(targetMachine || '').trim() || poolMachineCode || machineOptions[0] || null;
      const date = dayjs(targetDate);

      setMoveTargetMachine(machine);
      setMoveTargetDate(date.isValid() ? date : dayjs());
      setMoveSeqMode('APPEND');
      setMoveStartSeq(1);
      setMoveValidationMode('AUTO_FIX');
      setMoveReason(DEFAULT_MOVE_REASON);
      clearMoveRecommendSummary();
      setMoveModalOpen(true);
    },
    [clearMoveRecommendSummary, machineOptions, poolMachineCode, selectedMaterialIds.length]
  );

  const openMoveModalWithRecommend = useCallback(() => {
    if (selectedMaterialIds.length === 0) {
      message.warning('请先选择物料');
      return;
    }
    openMoveModal();
    scheduleAutoRecommendOnOpen();
  }, [openMoveModal, scheduleAutoRecommendOnOpen, selectedMaterialIds.length]);

  return {
    moveModalOpen,
    setMoveModalOpen,
    moveTargetMachine,
    setMoveTargetMachine,
    moveTargetDate,
    setMoveTargetDate,
    moveSeqMode,
    setMoveSeqMode,
    moveStartSeq,
    setMoveStartSeq,
    moveValidationMode,
    setMoveValidationMode,
    moveSubmitting,
    moveReason,
    setMoveReason,
    moveRecommendLoading,
    moveRecommendSummary,
    strategyLabel,
    selectedPlanItemStats,
    moveImpactPreview,
    recommendMoveTarget,
    openMoveModal,
    openMoveModalAt,
    openMoveModalWithRecommend,
    submitMove,
  };
}

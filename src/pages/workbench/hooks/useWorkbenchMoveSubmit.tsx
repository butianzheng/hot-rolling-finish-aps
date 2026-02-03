import { useCallback, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import type { Dayjs } from 'dayjs';
import { message } from 'antd';

import { planApi } from '../../../api/tauri';
import { formatDate } from '../../../utils/formatters';
import { getErrorMessage } from '../../../utils/errorUtils';
import type { MoveSeqMode, MoveValidationMode } from '../types';
import {
  buildPlanItemByIdMap,
  loadPlanItemsIfEmpty,
  splitSelectedMaterialIds,
  sortMaterialIdsByPlan,
  type IpcPlanItem,
} from '../move/planItems';
import { buildMoveRequests, computeMoveStartSeq, validateMoveSubmitParams } from '../move/submit';
import { showMoveSubmitResult } from '../move/showMoveSubmitResult';
import { useWorkbenchRefreshActions } from './useWorkbenchRefreshActions';

/**
 * Workbench 移位提交逻辑
 *
 * 使用统一的刷新协调器
 */
export function useWorkbenchMoveSubmit(params: {
  activeVersionId: string | null;
  operator: string | null;
  moveTargetMachine: string | null;
  moveTargetDate: Dayjs | null;
  moveReason: string;
  moveSeqMode: MoveSeqMode;
  moveStartSeq: number;
  moveValidationMode: MoveValidationMode;
  planItems: IpcPlanItem[];
  selectedMaterialIds: string[];
  setMoveModalOpen: Dispatch<SetStateAction<boolean>>;
  setMoveReason: Dispatch<SetStateAction<string>>;
  setSelectedMaterialIds: Dispatch<SetStateAction<string[]>>;
}): {
  moveSubmitting: boolean;
  submitMove: () => Promise<void>;
} {
  const {
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
  } = params;

  const [moveSubmitting, setMoveSubmitting] = useState(false);
  const { refreshAll } = useWorkbenchRefreshActions();

  const submitMove = useCallback(async () => {
    const invalid = validateMoveSubmitParams({
      activeVersionId,
      moveTargetMachine,
      moveTargetDateValid: Boolean(moveTargetDate && moveTargetDate.isValid()),
      moveReason,
    });
    if (invalid) {
      message.warning(invalid);
      return;
    }

    if (!activeVersionId) return;
    if (!moveTargetMachine) return;
    if (!moveTargetDate || !moveTargetDate.isValid()) return;

    setMoveSubmitting(true);
    try {
      const targetMachine = moveTargetMachine;
      const targetDate = formatDate(moveTargetDate);
      const reason = moveReason.trim();

      const planItemsRaw = await loadPlanItemsIfEmpty(activeVersionId, planItems ?? []);
      const byId = buildPlanItemByIdMap(planItemsRaw);
      const { eligible, missing } = splitSelectedMaterialIds(selectedMaterialIds, byId);

      if (eligible.length === 0) {
        message.error('所选物料不在当前版本排程中，无法移动');
        return;
      }

      const ordered = sortMaterialIdsByPlan(eligible, byId);
      const startSeq = computeMoveStartSeq({
        moveSeqMode,
        moveStartSeq,
        planItems: planItemsRaw,
        moveTargetMachine: targetMachine,
        targetDate,
      });
      const moves = buildMoveRequests({ orderedMaterialIds: ordered, targetMachine, targetDate, startSeq });

      const actualOperator = operator || 'admin';
      const res = await planApi.moveItems(activeVersionId, moves, moveValidationMode, actualOperator, reason);

      setMoveModalOpen(false);
      setMoveReason('');
      setSelectedMaterialIds([]);

      // 使用统一刷新协调器
      await refreshAll();

      showMoveSubmitResult(res, missing);
    } catch (e: unknown) {
      console.error('[Workbench] moveItems failed:', e);
      message.error(getErrorMessage(e) || '移动失败');
    } finally {
      setMoveSubmitting(false);
    }
  }, [
    activeVersionId,
    moveReason,
    moveSeqMode,
    moveStartSeq,
    moveTargetDate,
    moveTargetMachine,
    moveValidationMode,
    operator,
    planItems,
    selectedMaterialIds,
    setMoveModalOpen,
    setMoveReason,
    setSelectedMaterialIds,
    refreshAll,
  ]);

  return { moveSubmitting, submitMove };
}

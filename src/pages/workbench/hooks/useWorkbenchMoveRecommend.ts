import { useCallback, useEffect, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import { message } from 'antd';

import { capacityApi } from '../../../api/tauri';
import { getErrorMessage } from '../../../utils/errorUtils';
import type { MoveRecommendSummary, MoveValidationMode } from '../types';
import { buildPlanItemByIdMap, loadPlanItemsIfEmpty, type IpcPlanItem } from '../move/planItems';
import {
  buildCapacityPoolMap,
  buildCandidateDates,
  buildMoveDeltaBase,
  buildRecommendSummary,
  buildTonnageMap,
  computeRecommendQueryRange,
  pickBestCandidate,
  pickMovableItems,
  scoreCandidateDates,
} from '../move/recommend';

export function useWorkbenchMoveRecommend(params: {
  activeVersionId: string | null;
  moveModalOpen: boolean;
  moveTargetMachine: string | null;
  moveTargetDate: Dayjs | null;
  setMoveTargetDate: Dispatch<SetStateAction<Dayjs | null>>;
  moveValidationMode: MoveValidationMode;
  planItems: IpcPlanItem[];
  selectedMaterialIds: string[];
  defaultStrategy: string | null | undefined;
  strategyLabel: string;
  workbenchDateRange: [Dayjs, Dayjs];
}): {
  moveRecommendLoading: boolean;
  moveRecommendSummary: MoveRecommendSummary | null;
  clearMoveRecommendSummary: () => void;
  scheduleAutoRecommendOnOpen: () => void;
  recommendMoveTarget: () => Promise<void>;
} {
  const {
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
  } = params;

  const [moveRecommendLoading, setMoveRecommendLoading] = useState(false);
  const [moveRecommendSummary, setMoveRecommendSummary] = useState<MoveRecommendSummary | null>(null);
  const [autoRecommendOnOpen, setAutoRecommendOnOpen] = useState(false);

  const clearMoveRecommendSummary = useCallback(() => setMoveRecommendSummary(null), []);
  const scheduleAutoRecommendOnOpen = useCallback(() => setAutoRecommendOnOpen(true), []);

  const recommendMoveTarget = useCallback(async () => {
    if (!activeVersionId) {
      message.warning('请先激活一个版本');
      return;
    }
    const targetMachine = String(moveTargetMachine || '').trim();
    if (!targetMachine) {
      message.warning('请先选择目标机组');
      return;
    }

    // 仅基于“可移动”的已选物料做推荐（AUTO_FIX 模式下，冻结项会被跳过）
    const planItemsRaw = await loadPlanItemsIfEmpty(activeVersionId, planItems ?? []);
    const byId = buildPlanItemByIdMap(planItemsRaw);
    const tonnageMap = buildTonnageMap(planItemsRaw);
    const movable = pickMovableItems({ selectedMaterialIds, byId, moveValidationMode });

    if (movable.length === 0) {
      message.warning('所选物料在当前版本中不可移动（可能均为冻结或不在排程）');
      return;
    }

    const focus = moveTargetDate && moveTargetDate.isValid() ? moveTargetDate.startOf('day') : dayjs().startOf('day');
    const rangeStart = workbenchDateRange[0].startOf('day');
    const rangeEnd = workbenchDateRange[1].startOf('day');
    const candidates = buildCandidateDates({ focus, rangeStart, rangeEnd });
    const { totalWeight, deltaBase } = buildMoveDeltaBase(movable);

    if (candidates.length === 0) {
      message.warning('当前日期范围过窄，无法推荐（可先调整范围）');
      return;
    }

    const { affectedMachines, dateFrom, dateTo } = computeRecommendQueryRange({ targetMachine, movable, candidates });

    setMoveRecommendLoading(true);
    try {
      const pools = await capacityApi.getCapacityPools(affectedMachines, dateFrom, dateTo, activeVersionId);
      const poolMap = buildCapacityPoolMap(pools);
      const scored = scoreCandidateDates({
        candidates,
        deltaBase,
        totalWeight,
        targetMachine,
        tonnageMap,
        poolMap,
        focus,
      });
      if (scored.length === 0) {
        message.warning('暂无可推荐的位置（可能全为无变化/未知容量）');
        return;
      }

      const strategy = String(defaultStrategy || 'balanced');
      const best = pickBestCandidate(scored, strategy);
      if (!best) {
        message.warning('暂无可推荐的位置（可能全为无变化/未知容量）');
        return;
      }

      setMoveTargetDate(dayjs(best.date));
      setMoveRecommendSummary(buildRecommendSummary(targetMachine, best));

      message.success(`推荐位置：${targetMachine} / ${best.date}（策略：${strategyLabel}）`);
    } catch (error: unknown) {
      console.error('推荐位置失败:', error);
      message.error(`推荐位置失败: ${getErrorMessage(error)}`);
    } finally {
      setMoveRecommendLoading(false);
    }
  }, [
    activeVersionId,
    defaultStrategy,
    moveTargetDate,
    moveTargetMachine,
    moveValidationMode,
    planItems,
    selectedMaterialIds,
    setMoveTargetDate,
    strategyLabel,
    workbenchDateRange,
  ]);

  useEffect(() => {
    if (!moveModalOpen) return;
    if (!autoRecommendOnOpen) return;
    setAutoRecommendOnOpen(false);
    recommendMoveTarget();
  }, [autoRecommendOnOpen, moveModalOpen, recommendMoveTarget]);

  return {
    moveRecommendLoading,
    moveRecommendSummary,
    clearMoveRecommendSummary,
    scheduleAutoRecommendOnOpen,
    recommendMoveTarget,
  };
}

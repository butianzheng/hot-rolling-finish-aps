import { useCallback, useEffect, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import { message } from 'antd';

import { capacityApi, planApi } from '../../../api/tauri';
import { getErrorMessage } from '../../../utils/errorUtils';
import type { MoveRecommendSummary, MoveValidationMode } from '../types';

type IpcPlanItem = Awaited<ReturnType<typeof planApi.listPlanItems>>[number];
type IpcCapacityPool = Awaited<ReturnType<typeof capacityApi.getCapacityPools>>[number];

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
    let planItemsRaw: IpcPlanItem[] = planItems ?? [];
    if (planItemsRaw.length === 0) {
      const fetched = await planApi.listPlanItems(activeVersionId);
      planItemsRaw = fetched;
    }

    const byId = new Map<string, IpcPlanItem>();
    const tonnageMap = new Map<string, number>();
    planItemsRaw.forEach((it) => {
      const id = String(it.material_id ?? '').trim();
      if (id) byId.set(id, it);
      const machine = String(it.machine_code ?? '').trim();
      const date = String(it.plan_date ?? '').trim();
      if (!machine || !date) return;
      const weight = Number(it.weight_t ?? 0);
      if (!Number.isFinite(weight) || weight <= 0) return;
      const key = `${machine}__${date}`;
      tonnageMap.set(key, (tonnageMap.get(key) ?? 0) + weight);
    });

    const movable = selectedMaterialIds
      .map((id) => byId.get(id))
      .filter((it): it is IpcPlanItem => Boolean(it))
      .filter((it) => !(moveValidationMode === 'AUTO_FIX' && it.locked_in_plan === true))
      .map((it) => ({
        material_id: String(it.material_id ?? '').trim(),
        from_machine: String(it.machine_code ?? '').trim(),
        from_date: String(it.plan_date ?? '').trim(),
        weight_t: Number(it.weight_t ?? 0),
      }))
      .filter((it) => it.material_id && it.from_machine && it.from_date && Number.isFinite(it.weight_t) && it.weight_t > 0);

    if (movable.length === 0) {
      message.warning('所选物料在当前版本中不可移动（可能均为冻结或不在排程）');
      return;
    }

    const totalWeight = movable.reduce((sum, it) => sum + it.weight_t, 0);
    const deltaBase = new Map<string, number>();
    movable.forEach((it) => {
      const fromKey = `${it.from_machine}__${it.from_date}`;
      deltaBase.set(fromKey, (deltaBase.get(fromKey) ?? 0) - it.weight_t);
    });

    const focus = moveTargetDate && moveTargetDate.isValid() ? moveTargetDate.startOf('day') : dayjs().startOf('day');
    const rangeStart = workbenchDateRange[0].startOf('day');
    const rangeEnd = workbenchDateRange[1].startOf('day');
    const spanDays = rangeEnd.diff(rangeStart, 'day');
    const candidates: string[] = [];

    // 默认最多评估 31 天（围绕焦点日期）
    const radius = 15;
    if (spanDays <= radius * 2) {
      for (let i = 0; i <= spanDays; i += 1) {
        candidates.push(rangeStart.add(i, 'day').format('YYYY-MM-DD'));
      }
    } else {
      for (let offset = -radius; offset <= radius; offset += 1) {
        const d = focus.add(offset, 'day');
        if (d.isBefore(rangeStart) || d.isAfter(rangeEnd)) continue;
        candidates.push(d.format('YYYY-MM-DD'));
      }
    }

    if (candidates.length === 0) {
      message.warning('当前日期范围过窄，无法推荐（可先调整范围）');
      return;
    }

    const affectedMachines = Array.from(new Set<string>([targetMachine, ...movable.map((it) => it.from_machine)])).sort();

    const originDates = movable.map((it) => it.from_date).filter(Boolean).sort();
    const candidateDates = [...candidates].sort();
    const dateFrom = [originDates[0], candidateDates[0]].filter(Boolean).sort()[0] || candidateDates[0];
    const dateTo =
      [originDates[originDates.length - 1], candidateDates[candidateDates.length - 1]]
        .filter(Boolean)
        .sort()
        .slice(-1)[0] || candidateDates[candidateDates.length - 1];

    setMoveRecommendLoading(true);
    try {
      const pools = await capacityApi.getCapacityPools(affectedMachines, dateFrom, dateTo, activeVersionId);
      const poolMap = new Map<string, { target: number | null; limit: number | null }>();
      pools.forEach((p: IpcCapacityPool) => {
        const machine = String(p.machine_code ?? '').trim();
        const date = String(p.plan_date ?? '').trim();
        if (!machine || !date) return;
        const target = Number(p.target_capacity_t ?? 0);
        const limit = Number(p.limit_capacity_t ?? 0);
        poolMap.set(`${machine}__${date}`, {
          target: Number.isFinite(target) && target > 0 ? target : null,
          limit: Number.isFinite(limit) && limit > 0 ? limit : null,
        });
      });

      const scored = candidates
        .map((date) => {
          const deltaMap = new Map<string, number>(deltaBase);
          const toKey = `${targetMachine}__${date}`;
          deltaMap.set(toKey, (deltaMap.get(toKey) ?? 0) + totalWeight);

          // 过滤掉无变化的 key
          const keys = Array.from(deltaMap.entries()).filter(([, d]) => Number.isFinite(d) && Math.abs(d) > 1e-9);
          if (keys.length === 0) return null;

          let overLimitCount = 0;
          let unknownCount = 0;
          let totalOverT = 0;
          let maxUtilPct = 0;

          keys.forEach(([key, delta]) => {
            const before = tonnageMap.get(key) ?? 0;
            const after = before + delta;
            const cap = poolMap.get(key);
            const limit = cap?.limit ?? cap?.target ?? null;
            if (limit == null || limit <= 0) {
              unknownCount += 1;
              return;
            }
            const pct = (after / limit) * 100;
            if (pct > maxUtilPct) maxUtilPct = pct;
            if (after > limit + 1e-9) {
              overLimitCount += 1;
              totalOverT += after - limit;
            }
          });

          const distance = Math.abs(dayjs(date).diff(focus, 'day'));
          return { date, overLimitCount, unknownCount, totalOverT, maxUtilPct, distance };
        })
        .filter(Boolean) as Array<{
        date: string;
        overLimitCount: number;
        unknownCount: number;
        totalOverT: number;
        maxUtilPct: number;
        distance: number;
      }>;

      if (scored.length === 0) {
        message.warning('暂无可推荐的位置（可能全为无变化/未知容量）');
        return;
      }

      const strategy = String(defaultStrategy || 'balanced');
      scored.sort((a, b) => {
        if (a.overLimitCount !== b.overLimitCount) return a.overLimitCount - b.overLimitCount;
        if (a.unknownCount !== b.unknownCount) return a.unknownCount - b.unknownCount;

        // 策略差异（尽量贴合“当前方案策略”的偏好）
        if (strategy === 'capacity_first') {
          if (a.maxUtilPct !== b.maxUtilPct) return a.maxUtilPct - b.maxUtilPct;
          if (a.totalOverT !== b.totalOverT) return a.totalOverT - b.totalOverT;
          if (a.distance !== b.distance) return a.distance - b.distance;
        } else {
          if (a.totalOverT !== b.totalOverT) return a.totalOverT - b.totalOverT;
          if (a.distance !== b.distance) return a.distance - b.distance;
          if (a.maxUtilPct !== b.maxUtilPct) return a.maxUtilPct - b.maxUtilPct;
        }

        if (strategy === 'urgent_first') return a.date.localeCompare(b.date); // 越早越好
        if (strategy === 'cold_stock_first') return b.date.localeCompare(a.date); // 越晚越好
        return a.date.localeCompare(b.date);
      });

      const best = scored[0];
      setMoveTargetDate(dayjs(best.date));
      setMoveRecommendSummary({
        machine: targetMachine,
        date: best.date,
        overLimitCount: best.overLimitCount,
        unknownCount: best.unknownCount,
        totalOverT: best.totalOverT,
        maxUtilPct: best.maxUtilPct,
      });

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


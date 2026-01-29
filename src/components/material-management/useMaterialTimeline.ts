/**
 * 产能时间线 Hook
 * 管理时间线相关的状态和数据加载
 */

import { useCallback, useEffect, useState } from 'react';
import dayjs, { type Dayjs } from 'dayjs';
import { capacityApi, materialApi, planApi } from '../../api/tauri';
import { useActiveVersionId, useGlobalStore } from '../../stores/use-global-store';
import { formatDate } from '../../utils/formatters';
import type { CapacityTimelineData } from '../../types/capacity';

export interface UseMaterialTimelineReturn {
  machineOptions: Array<{ label: string; value: string }>;
  timelineMachine: string | undefined;
  timelineDate: Dayjs;
  timelineData: CapacityTimelineData | null;
  timelineLoading: boolean;
  timelineError: string | null;
  activeVersionId: string | null;
  setTimelineMachine: (machine: string | undefined) => void;
  setTimelineDate: (date: Dayjs) => void;
  loadMachineOptions: () => Promise<Array<{ label: string; value: string }>>;
  loadTimeline: (opts?: { machineCode?: string; date?: Dayjs }) => Promise<void>;
}

export function useMaterialTimeline(): UseMaterialTimelineReturn {
  const activeVersionId = useActiveVersionId();
  const preferredMachineCode = useGlobalStore((state) => state.workbenchFilters.machineCode);

  const [machineOptions, setMachineOptions] = useState<Array<{ label: string; value: string }>>([]);
  const [timelineMachine, setTimelineMachine] = useState<string | undefined>(undefined);
  const [timelineDate, setTimelineDate] = useState(() => dayjs());
  const [timelineData, setTimelineData] = useState<CapacityTimelineData | null>(null);
  const [timelineLoading, setTimelineLoading] = useState(false);
  const [timelineError, setTimelineError] = useState<string | null>(null);

  const loadMachineOptions = useCallback(async () => {
    const result = await materialApi.listMaterials({ limit: 1000, offset: 0 });
    const codes = new Set<string>();
    (Array.isArray(result) ? result : []).forEach((m: any) => {
      const code = String(m?.machine_code ?? '').trim();
      if (code) codes.add(code);
    });
    const options = Array.from(codes)
      .sort()
      .map((code) => ({ label: code, value: code }));
    setMachineOptions(options);
    return options;
  }, []);

  const loadTimeline = useCallback(
    async (opts?: { machineCode?: string; date?: Dayjs }) => {
      const machineCode = opts?.machineCode ?? timelineMachine;
      const date = opts?.date ?? timelineDate;

      if (!machineCode) return;
      if (!activeVersionId) {
        setTimelineData(null);
        setTimelineError(null);
        return;
      }

      const dateStr = formatDate(date);
      setTimelineLoading(true);
      setTimelineError(null);
      try {
        const [capacityPools, itemsByDate] = await Promise.all([
          capacityApi.getCapacityPools([machineCode], dateStr, dateStr, activeVersionId),
          planApi.listItemsByDate(activeVersionId, dateStr),
        ]);

        const pools = Array.isArray(capacityPools) ? capacityPools : [];
        const pool = pools.find(
          (p: any) => String(p?.machine_code ?? '') === machineCode && String(p?.plan_date ?? '') === dateStr
        );

        const planItems = (Array.isArray(itemsByDate) ? itemsByDate : []).filter(
          (it: any) => String(it?.machine_code ?? '') === machineCode
        );

        const buckets: Record<'L0' | 'L1' | 'L2' | 'L3', { tonnage: number; count: number }> = {
          L0: { tonnage: 0, count: 0 },
          L1: { tonnage: 0, count: 0 },
          L2: { tonnage: 0, count: 0 },
          L3: { tonnage: 0, count: 0 },
        };

        planItems.forEach((it: any) => {
          const raw = String(it?.urgent_level ?? 'L0').toUpperCase();
          const level = (['L0', 'L1', 'L2', 'L3'].includes(raw) ? raw : 'L0') as 'L0' | 'L1' | 'L2' | 'L3';
          const weight = Number(it?.weight_t ?? 0);
          if (!Number.isFinite(weight) || weight <= 0) return;
          buckets[level].tonnage += weight;
          buckets[level].count += 1;
        });

        const segmentTotal = (Object.keys(buckets) as Array<keyof typeof buckets>).reduce(
          (sum, k) => sum + buckets[k].tonnage,
          0
        );

        const poolUsed = Number(pool?.used_capacity_t ?? 0);
        const actualCapacity =
          Number.isFinite(segmentTotal) && segmentTotal > 0
            ? segmentTotal
            : Number.isFinite(poolUsed) && poolUsed > 0
              ? poolUsed
              : 0;

        if (segmentTotal <= 0 && actualCapacity > 0) {
          buckets.L0.tonnage = actualCapacity;
        }

        const target = Number(pool?.target_capacity_t ?? 0);
        const limit = Number(pool?.limit_capacity_t ?? 0);

        const targetCapacity = Number.isFinite(target) && target > 0 ? target : Math.max(actualCapacity, 1);
        const limitCapacity = Number.isFinite(limit) && limit > 0 ? limit : targetCapacity;

        const accumulated = Number(pool?.accumulated_tonnage_t ?? 0);

        setTimelineData({
          date: dateStr,
          machineCode,
          targetCapacity,
          limitCapacity,
          actualCapacity,
          segments: (['L3', 'L2', 'L1', 'L0'] as const).map((level) => ({
            urgencyLevel: level,
            tonnage: buckets[level].tonnage,
            materialCount: buckets[level].count,
          })),
          rollCampaignProgress: Number.isFinite(accumulated) ? accumulated : 0,
          rollChangeThreshold: 2500,
        });
      } catch (error: any) {
        setTimelineError(error?.message || String(error) || '加载失败');
        setTimelineData(null);
      } finally {
        setTimelineLoading(false);
      }
    },
    [activeVersionId, timelineDate, timelineMachine]
  );

  // 预加载机组列表
  useEffect(() => {
    loadMachineOptions()
      .then((options) => {
        setTimelineMachine((prev) => {
          if (prev) return prev;
          const preferred =
            preferredMachineCode && options.some((o) => o.value === preferredMachineCode)
              ? preferredMachineCode
              : undefined;
          return preferred || options[0]?.value;
        });
      })
      .catch((e) => {
        console.error('加载机组列表失败:', e);
      });
  }, [loadMachineOptions, preferredMachineCode]);

  // 激活版本 / 选择变化时刷新时间线
  useEffect(() => {
    if (!activeVersionId || !timelineMachine) return;
    loadTimeline();
  }, [activeVersionId, timelineMachine, timelineDate, loadTimeline]);

  return {
    machineOptions,
    timelineMachine,
    timelineDate,
    timelineData,
    timelineLoading,
    timelineError,
    activeVersionId,
    setTimelineMachine,
    setTimelineDate,
    loadMachineOptions,
    loadTimeline,
  };
}

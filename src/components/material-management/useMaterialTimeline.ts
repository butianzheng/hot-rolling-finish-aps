/**
 * 产能时间线 Hook
 * 管理时间线相关的状态和数据加载
 */

import { useCallback, useEffect, useState } from 'react';
import dayjs, { type Dayjs } from 'dayjs';
import { capacityApi, materialApi, planApi } from '../../api/tauri';
import { configApi } from '../../api/tauri/configApi';
import { machineConfigApi } from '../../api/tauri/machineConfigApi';
import { useActiveVersionId, useGlobalStore } from '../../stores/use-global-store';
import { formatDate } from '../../utils/formatters';
import { getErrorMessage } from '../../utils/errorUtils';
import type { CapacityTimelineData } from '../../types/capacity';

type IpcMaterialWithState = Awaited<ReturnType<typeof materialApi.listMaterials>>[number];
type IpcCapacityPool = Awaited<ReturnType<typeof capacityApi.getCapacityPools>>[number];
type IpcPlanItem = Awaited<ReturnType<typeof planApi.listItemsByDate>>[number];
type UrgencyLevel = 'L0' | 'L1' | 'L2' | 'L3';

export interface UseMaterialTimelineReturn {
  machineOptions: Array<{ label: string; value: string }>;
  timelineMachine: string;
  timelineDate: Dayjs;
  timelineData: CapacityTimelineData[];
  timelineLoading: boolean;
  timelineError: string | null;
  activeVersionId: string | null;
  setTimelineMachine: (machine: string) => void;
  setTimelineDate: (date: Dayjs) => void;
  loadMachineOptions: () => Promise<Array<{ label: string; value: string }>>;
  loadTimeline: (opts?: { machineCode?: string; date?: Dayjs }) => Promise<void>;
}

const ALL_MACHINE_VALUE = 'all';

function normalizeMachineCode(value: unknown): string {
  return String(value ?? '').trim();
}

function parseMachineCodesCsv(raw: string | null | undefined): string[] {
  return String(raw ?? '')
    .split(',')
    .map((x) => normalizeMachineCode(x))
    .filter((x) => !!x && x.toUpperCase() !== 'UNKNOWN');
}

function createEmptyBuckets(): Record<UrgencyLevel, { tonnage: number; count: number }> {
  return {
    L0: { tonnage: 0, count: 0 },
    L1: { tonnage: 0, count: 0 },
    L2: { tonnage: 0, count: 0 },
    L3: { tonnage: 0, count: 0 },
  };
}

export function useMaterialTimeline(): UseMaterialTimelineReturn {
  const activeVersionId = useActiveVersionId();
  const preferredMachineCode = useGlobalStore((state) => state.workbenchFilters.machineCode);

  const [machineOptions, setMachineOptions] = useState<Array<{ label: string; value: string }>>([]);
  const [timelineMachine, setTimelineMachine] = useState<string>(ALL_MACHINE_VALUE);
  const [timelineDate, setTimelineDate] = useState(() => dayjs());
  const [timelineData, setTimelineData] = useState<CapacityTimelineData[]>([]);
  const [timelineLoading, setTimelineLoading] = useState(false);
  const [timelineError, setTimelineError] = useState<string | null>(null);

  const loadMachineOptions = useCallback(async () => {
    const codes = new Set<string>();

    try {
      const summary = await materialApi.getMaterialPoolSummary();
      const machines = Array.isArray(summary?.machines) ? summary.machines : [];
      machines.forEach((m) => {
        const code = normalizeMachineCode((m as any)?.machine_code);
        if (code && code.toUpperCase() !== 'UNKNOWN') codes.add(code);
      });
    } catch (e) {
      console.warn('加载物料池机组汇总失败，继续使用其他来源:', e);
    }

    try {
      const cfg = await configApi.getConfig('global', 'standard_finishing_machines');
      parseMachineCodesCsv(cfg?.value).forEach((code) => codes.add(code));
    } catch (e) {
      console.warn('读取 standard_finishing_machines 失败，继续使用其他来源:', e);
    }

    if (activeVersionId) {
      try {
        const configs = await machineConfigApi.getMachineCapacityConfigs(activeVersionId);
        (Array.isArray(configs) ? configs : []).forEach((row) => {
          const code = normalizeMachineCode((row as any)?.machine_code);
          if (code) codes.add(code);
        });
      } catch (e) {
        console.warn('加载机组产能配置失败，继续使用其他来源:', e);
      }
    }

    if (codes.size === 0) {
      const fallback = await materialApi.listMaterials({ limit: 1000, offset: 0 });
      (fallback as IpcMaterialWithState[]).forEach((m) => {
        const code = normalizeMachineCode(m.machine_code);
        if (code && code.toUpperCase() !== 'UNKNOWN') codes.add(code);
      });
    }

    const options = Array.from(codes)
      .sort()
      .map((code) => ({ label: code, value: code }));
    setMachineOptions(options);
    return options;
  }, [activeVersionId]);

  const loadTimeline = useCallback(
    async (opts?: { machineCode?: string; date?: Dayjs }) => {
      const machineCode = normalizeMachineCode(opts?.machineCode ?? timelineMachine) || ALL_MACHINE_VALUE;
      const date = opts?.date ?? timelineDate;

      if (!activeVersionId) {
        setTimelineData([]);
        setTimelineError(null);
        return;
      }

      const dateStr = formatDate(date);
      const machineCodes =
        machineCode === ALL_MACHINE_VALUE
          ? machineOptions.map((o) => normalizeMachineCode(o.value)).filter((x) => !!x)
          : [machineCode];

      if (machineCodes.length === 0) {
        setTimelineData([]);
        setTimelineError(null);
        return;
      }

      setTimelineLoading(true);
      setTimelineError(null);
      try {
        const [capacityPools, itemsByDate] = await Promise.all([
          capacityApi.getCapacityPools(machineCodes, dateStr, dateStr, activeVersionId),
          planApi.listItemsByDate(activeVersionId, dateStr),
        ]);

        const pools: IpcCapacityPool[] = Array.isArray(capacityPools) ? capacityPools : [];
        const items: IpcPlanItem[] = Array.isArray(itemsByDate) ? itemsByDate : [];
        const poolByMachine = new Map<string, IpcCapacityPool>();
        const bucketMap = new Map<string, Record<UrgencyLevel, { tonnage: number; count: number }>>();
        const materialIdsMap = new Map<string, string[]>();
        const statusSummaryMap = new Map<
          string,
          {
            totalCount: number;
            totalWeightT: number;
            lockedInPlanCount: number;
            lockedInPlanWeightT: number;
            forceReleaseCount: number;
            forceReleaseWeightT: number;
          }
        >();

        const ensureKey = (key: string) => {
          if (!bucketMap.has(key)) bucketMap.set(key, createEmptyBuckets());
          if (!materialIdsMap.has(key)) materialIdsMap.set(key, []);
          if (!statusSummaryMap.has(key)) {
            statusSummaryMap.set(key, {
              totalCount: 0,
              totalWeightT: 0,
              lockedInPlanCount: 0,
              lockedInPlanWeightT: 0,
              forceReleaseCount: 0,
              forceReleaseWeightT: 0,
            });
          }
        };

        pools.forEach((p) => {
          const code = normalizeMachineCode(p.machine_code);
          const planDate = String((p as any)?.plan_date ?? '').trim();
          if (!code || !machineCodes.includes(code) || planDate !== dateStr) return;
          poolByMachine.set(code, p);
          ensureKey(`${code}__${planDate}`);
        });

        items.forEach((it) => {
          const code = normalizeMachineCode(it.machine_code);
          const planDate = String((it as any)?.plan_date ?? '').trim();
          if (!code || !machineCodes.includes(code) || planDate !== dateStr) return;
          const weight = Number(it.weight_t ?? 0);
          if (!Number.isFinite(weight) || weight <= 0) return;
          const raw = String(it.urgent_level ?? 'L0').toUpperCase();
          const level = (['L0', 'L1', 'L2', 'L3'].includes(raw) ? raw : 'L0') as UrgencyLevel;

          const key = `${code}__${planDate}`;
          ensureKey(key);

          const bucket = bucketMap.get(key)!;
          bucket[level].tonnage += weight;
          bucket[level].count += 1;

          const materialId = String((it as any)?.material_id ?? '').trim();
          if (materialId) materialIdsMap.get(key)!.push(materialId);

          const summary = statusSummaryMap.get(key)!;
          summary.totalCount += 1;
          summary.totalWeightT += weight;
          if ((it as any)?.locked_in_plan === true) {
            summary.lockedInPlanCount += 1;
            summary.lockedInPlanWeightT += weight;
          }
          if ((it as any)?.force_release_in_plan === true) {
            summary.forceReleaseCount += 1;
            summary.forceReleaseWeightT += weight;
          }
        });

        const nextRows: CapacityTimelineData[] = machineCodes
          .map((code) => {
            const key = `${code}__${dateStr}`;
            const pool = poolByMachine.get(code);
            const bucket = bucketMap.get(key) ?? createEmptyBuckets();
            const materialIds = Array.from(new Set(materialIdsMap.get(key) ?? []));
            const summary = statusSummaryMap.get(key) ?? {
              totalCount: 0,
              totalWeightT: 0,
              lockedInPlanCount: 0,
              lockedInPlanWeightT: 0,
              forceReleaseCount: 0,
              forceReleaseWeightT: 0,
            };

            const segmentTotal = (['L0', 'L1', 'L2', 'L3'] as const).reduce(
              (sum, level) => sum + bucket[level].tonnage,
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
              bucket.L0.tonnage = actualCapacity;
            }

            const target = Number(pool?.target_capacity_t ?? 0);
            const limit = Number(pool?.limit_capacity_t ?? 0);
            const targetCapacity =
              Number.isFinite(target) && target > 0 ? target : Math.max(actualCapacity, 1);
            const limitCapacity = Number.isFinite(limit) && limit > 0 ? limit : targetCapacity;
            const accumulated = Number(pool?.accumulated_tonnage_t ?? 0);

            return {
              date: dateStr,
              machineCode: code,
              targetCapacity,
              limitCapacity,
              actualCapacity,
              segments: (['L3', 'L2', 'L1', 'L0'] as const).map((level) => ({
                urgencyLevel: level,
                tonnage: bucket[level].tonnage,
                materialCount: bucket[level].count,
              })),
              rollCampaignProgress: Number.isFinite(accumulated) ? accumulated : 0,
              rollChangeThreshold: 2500,
              materialIds,
              statusSummary: summary,
            };
          })
          .sort((a, b) => a.machineCode.localeCompare(b.machineCode));

        setTimelineData(nextRows);
      } catch (error: unknown) {
        setTimelineError(getErrorMessage(error) || '加载失败');
        setTimelineData([]);
      } finally {
        setTimelineLoading(false);
      }
    },
    [activeVersionId, machineOptions, timelineDate, timelineMachine]
  );

  // 预加载机组列表
  useEffect(() => {
    loadMachineOptions()
      .then((options) => {
        setTimelineMachine((prev) => {
          if (prev && (prev === ALL_MACHINE_VALUE || options.some((o) => o.value === prev))) return prev;
          const preferred =
            preferredMachineCode && options.some((o) => o.value === preferredMachineCode)
              ? preferredMachineCode
              : undefined;
          return preferred || ALL_MACHINE_VALUE;
        });
      })
      .catch((e: unknown) => {
        console.error('加载机组列表失败:', e);
      });
  }, [loadMachineOptions, preferredMachineCode]);

  // 激活版本 / 选择变化时刷新时间线
  useEffect(() => {
    if (!activeVersionId) return;
    loadTimeline();
  }, [activeVersionId, loadTimeline, timelineDate, timelineMachine]);

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

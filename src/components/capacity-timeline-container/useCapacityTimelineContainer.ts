/**
 * CapacityTimelineContainer 状态管理 Hook
 */

import { useCallback, useEffect, useState } from 'react';
import { message } from 'antd';
import dayjs from 'dayjs';
import { capacityApi, materialApi, planApi } from '../../api/tauri';
import { useActiveVersionId } from '../../stores/use-global-store';
import { formatDate } from '../../utils/formatters';
import type { CapacityTimelineData } from '../../types/capacity';
import type { DateRangeValue, MachineOption, UrgencyBucketMap, UrgencyLevel } from './types';

export interface UseCapacityTimelineContainerReturn {
  // 数据
  timelineData: CapacityTimelineData[];
  machineOptions: MachineOption[];
  activeVersionId: string | null;

  // 状态
  loading: boolean;
  dateRange: DateRangeValue;
  selectedMachine: string;

  // 操作
  setDateRange: (range: DateRangeValue) => void;
  setSelectedMachine: (machine: string) => void;
  loadTimelineData: () => Promise<void>;
}

export function useCapacityTimelineContainer(
  machineCode?: string | null
): UseCapacityTimelineContainerReturn {
  const [timelineData, setTimelineData] = useState<CapacityTimelineData[]>([]);
  const [loading, setLoading] = useState(false);
  const [machineOptions, setMachineOptions] = useState<MachineOption[]>([]);
  const [dateRange, setDateRange] = useState<DateRangeValue>([
    dayjs(),
    dayjs().add(7, 'day'),
  ]);
  const [selectedMachine, setSelectedMachine] = useState<string>('all');
  const activeVersionId = useActiveVersionId();

  useEffect(() => {
    if (!machineCode) return;
    setSelectedMachine(machineCode);
  }, [machineCode]);

  // 预加载机组选项
  const loadMachineOptions = async () => {
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
  };

  // 数据加载
  const loadTimelineData = useCallback(async () => {
    if (!activeVersionId) {
      setTimelineData([]);
      return;
    }

    setLoading(true);
    try {
      const [start, end] = dateRange;
      const dateFrom = formatDate(start);
      const dateTo = formatDate(end);

      const machineCodes =
        selectedMachine === 'all'
          ? machineOptions.map((o) => o.value)
          : selectedMachine
          ? [selectedMachine]
          : [];

      if (machineCodes.length === 0) {
        setTimelineData([]);
        return;
      }

      const [capacityPools, planItems] = await Promise.all([
        capacityApi.getCapacityPools(machineCodes, dateFrom, dateTo, activeVersionId),
        planApi.listPlanItems(activeVersionId, {
          plan_date_from: dateFrom,
          plan_date_to: dateTo,
          machine_code: selectedMachine && selectedMachine !== 'all' ? selectedMachine : undefined,
        }),
      ]);

      const pools = Array.isArray(capacityPools) ? capacityPools : [];
      const items = Array.isArray(planItems) ? planItems : [];

      // (machine_code, plan_date) -> urgency buckets
      const bucketMap = new Map<string, UrgencyBucketMap>();
      // (machine_code, plan_date) -> material IDs
      const materialIdsMap = new Map<string, string[]>();

      const inRange = (d: string) => {
        const day = dayjs(d);
        return day.isValid() && (day.isSame(start, 'day') || day.isSame(end, 'day') || (day.isAfter(start, 'day') && day.isBefore(end, 'day')));
      };

      items.forEach((it: any) => {
        const machine = String(it?.machine_code ?? '').trim();
        const planDate = String(it?.plan_date ?? '').trim();
        if (!machine || !planDate) return;
        if (!machineCodes.includes(machine)) return;
        if (!inRange(planDate)) return;

        const raw = String(it?.urgent_level ?? 'L0').toUpperCase();
        const level = (['L0', 'L1', 'L2', 'L3'].includes(raw) ? raw : 'L0') as UrgencyLevel;
        const weight = Number(it?.weight_t ?? 0);
        if (!Number.isFinite(weight) || weight <= 0) return;

        const key = `${machine}__${planDate}`;
        if (!bucketMap.has(key)) {
          bucketMap.set(key, {
            L0: { tonnage: 0, count: 0 },
            L1: { tonnage: 0, count: 0 },
            L2: { tonnage: 0, count: 0 },
            L3: { tonnage: 0, count: 0 },
          });
          materialIdsMap.set(key, []);
        }

        const bucket = bucketMap.get(key)!;
        bucket[level].tonnage += weight;
        bucket[level].count += 1;

        // 收集物料ID
        const materialId = String(it?.material_id ?? '').trim();
        if (materialId) {
          materialIdsMap.get(key)!.push(materialId);
        }
      });

      const normalized = pools
        .filter((p: any) => {
          const machine = String(p?.machine_code ?? '').trim();
          const planDate = String(p?.plan_date ?? '').trim();
          return machine && planDate && machineCodes.includes(machine) && inRange(planDate);
        })
        .map((p: any) => {
          const machineCodeVal = String(p?.machine_code ?? '').trim();
          const date = String(p?.plan_date ?? '').trim();
          const key = `${machineCodeVal}__${date}`;
          const bucket: UrgencyBucketMap =
            bucketMap.get(key) || {
              L0: { tonnage: 0, count: 0 },
              L1: { tonnage: 0, count: 0 },
              L2: { tonnage: 0, count: 0 },
              L3: { tonnage: 0, count: 0 },
            };

          const segmentTotal = (['L0', 'L1', 'L2', 'L3'] as const).reduce(
            (sum, k) => sum + bucket[k].tonnage,
            0
          );

          const poolUsed = Number(p?.used_capacity_t ?? 0);
          const actualCapacity =
            Number.isFinite(segmentTotal) && segmentTotal > 0
              ? segmentTotal
              : Number.isFinite(poolUsed) && poolUsed > 0
              ? poolUsed
              : 0;

          const target = Number(p?.target_capacity_t ?? 0);
          const limit = Number(p?.limit_capacity_t ?? 0);
          const targetCapacity =
            Number.isFinite(target) && target > 0 ? target : Math.max(actualCapacity, 1);
          const limitCapacity =
            Number.isFinite(limit) && limit > 0 ? limit : targetCapacity;
          const accumulated = Number(p?.accumulated_tonnage_t ?? 0);

          return {
            date,
            machineCode: machineCodeVal,
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
            materialIds: materialIdsMap.get(key) || [],
          } satisfies CapacityTimelineData;
        })
        .sort((a, b) => {
          if (a.date === b.date) return a.machineCode.localeCompare(b.machineCode);
          return a.date.localeCompare(b.date);
        });

      setTimelineData(normalized);
    } catch (error: any) {
      message.error(`加载失败: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  }, [activeVersionId, dateRange, machineOptions, selectedMachine]);

  useEffect(() => {
    loadMachineOptions().catch((e) => console.error('加载机组选项失败:', e));
  }, []);

  useEffect(() => {
    loadTimelineData();
  }, [loadTimelineData]);

  return {
    timelineData,
    machineOptions,
    activeVersionId,
    loading,
    dateRange,
    selectedMachine,
    setDateRange,
    setSelectedMachine,
    loadTimelineData,
  };
}

export default useCapacityTimelineContainer;

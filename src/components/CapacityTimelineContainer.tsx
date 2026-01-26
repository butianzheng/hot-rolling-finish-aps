// ==========================================
// 产能时间线容器组件
// ==========================================
// 显示多天的产能时间线
// ==========================================

import React, { useCallback, useEffect, useState } from 'react';
import { Space, Button, DatePicker, Select, message, Empty, Alert, Spin } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import { CapacityTimeline } from './CapacityTimeline';
import type { CapacityTimelineData } from '../types/capacity';
import dayjs from 'dayjs';
import { capacityApi, materialApi, planApi } from '../api/tauri';
import { useActiveVersionId } from '../stores/use-global-store';
import { formatDate } from '../utils/formatters';

const { RangePicker } = DatePicker;

export const CapacityTimelineContainer: React.FC = () => {
  const [timelineData, setTimelineData] = useState<CapacityTimelineData[]>([]);
  const [loading, setLoading] = useState(false);
  const [machineOptions, setMachineOptions] = useState<Array<{ label: string; value: string }>>(
    []
  );
  const [dateRange, setDateRange] = useState<[dayjs.Dayjs, dayjs.Dayjs]>([
    dayjs(),
    dayjs().add(7, 'day'),
  ]);
  const [selectedMachine, setSelectedMachine] = useState<string>('all');
  const activeVersionId = useActiveVersionId();

  // 预加载机组选项（从材料列表中聚合，避免额外的 machine_master API）
  const loadMachineOptions = async () => {
    const result = await materialApi.listMaterials({ limit: 0, offset: 0 });
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

  // 数据加载（产能池 + 排产明细 -> 多天时间线）
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
        planApi.listPlanItems(activeVersionId),
      ]);

      const pools = Array.isArray(capacityPools) ? capacityPools : [];
      const items = Array.isArray(planItems) ? planItems : [];

      // (machine_code, plan_date) -> urgency buckets
      const bucketMap = new Map<
        string,
        Record<'L0' | 'L1' | 'L2' | 'L3', { tonnage: number; count: number }>
      >();

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
        const level = (['L0', 'L1', 'L2', 'L3'].includes(raw) ? raw : 'L0') as
          | 'L0'
          | 'L1'
          | 'L2'
          | 'L3';
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
        }

        const bucket = bucketMap.get(key)!;
        bucket[level].tonnage += weight;
        bucket[level].count += 1;
      });

      const normalized = pools
        .filter((p: any) => {
          const machine = String(p?.machine_code ?? '').trim();
          const planDate = String(p?.plan_date ?? '').trim();
          return machine && planDate && machineCodes.includes(machine) && inRange(planDate);
        })
        .map((p: any) => {
          const machineCode = String(p?.machine_code ?? '').trim();
          const date = String(p?.plan_date ?? '').trim();
          const key = `${machineCode}__${date}`;
          const bucket =
            bucketMap.get(key) ||
            ({
              L0: { tonnage: 0, count: 0 },
              L1: { tonnage: 0, count: 0 },
              L2: { tonnage: 0, count: 0 },
              L3: { tonnage: 0, count: 0 },
            } as const);

          const segmentTotal = (Object.keys(bucket) as Array<keyof typeof bucket>).reduce(
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
            machineCode,
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

  return (
    <div>
      {/* 工具栏 */}
      <Space style={{ marginBottom: 16 }} size={16}>
        <RangePicker
          value={dateRange}
          onChange={(dates) => dates && setDateRange(dates as [dayjs.Dayjs, dayjs.Dayjs])}
          format="YYYY-MM-DD"
        />
        <Select
          style={{ width: 150 }}
          value={selectedMachine}
          onChange={setSelectedMachine}
          options={[
            { label: '全部机组', value: 'all' },
            ...machineOptions,
          ]}
        />
        <Button icon={<ReloadOutlined />} onClick={loadTimelineData} loading={loading}>
          刷新
        </Button>
      </Space>

      {/* 时间线列表 */}
      {!activeVersionId ? (
        <Alert
          message="请先激活排产版本"
          description="产能时间线依赖排产版本数据，激活版本后可查看多天产能分布。"
          type="warning"
          showIcon
        />
      ) : null}

      {timelineData.length === 0 ? (
        <Empty description="暂无数据" />
      ) : (
        <Spin spinning={loading}>
          <Space direction="vertical" style={{ width: '100%' }} size={0}>
            {timelineData.map((data, index) => (
              <CapacityTimeline key={`${data.machineCode}__${data.date}__${index}`} data={data} />
            ))}
          </Space>
        </Spin>
      )}
    </div>
  );
};

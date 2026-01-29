/**
 * 产能池管理状态管理 Hook
 */

import { useCallback, useEffect, useMemo, useState } from 'react';
import { message } from 'antd';
import dayjs, { Dayjs } from 'dayjs';
import { capacityApi, materialApi } from '../../api/tauri';
import { useActiveVersionId, useCurrentUser, useGlobalStore } from '../../stores/use-global-store';
import { formatDate } from '../../utils/formatters';
import type { CapacityPool, TotalStats } from './types';

export interface UseCapacityPoolManagementReturn {
  // 加载状态
  loading: boolean;
  loadError: string | null;

  // 数据
  capacityPools: CapacityPool[];
  machineOptions: string[];
  totalStats: TotalStats;

  // 筛选状态
  selectedMachines: string[];
  setSelectedMachines: (machines: string[]) => void;
  dateRange: [Dayjs, Dayjs];
  setDateRange: (range: [Dayjs, Dayjs]) => void;

  // 选中状态
  selectedRowKeys: React.Key[];
  setSelectedRowKeys: (keys: React.Key[]) => void;
  selectedPools: CapacityPool[];
  setSelectedPools: (pools: CapacityPool[]) => void;

  // 编辑模态框
  editModalVisible: boolean;
  editingPool: CapacityPool | null;
  targetCapacity: number;
  setTargetCapacity: (value: number) => void;
  limitCapacity: number;
  setLimitCapacity: (value: number) => void;
  updateReason: string;
  setUpdateReason: (reason: string) => void;
  handleEdit: (record: CapacityPool) => void;
  handleUpdate: () => Promise<void>;
  closeEditModal: () => void;

  // 批量编辑模态框
  batchModalVisible: boolean;
  batchTargetCapacity: number | null;
  setBatchTargetCapacity: (value: number | null) => void;
  batchLimitCapacity: number | null;
  setBatchLimitCapacity: (value: number | null) => void;
  batchReason: string;
  setBatchReason: (reason: string) => void;
  openBatchModal: () => void;
  closeBatchModal: () => void;
  handleBatchUpdate: () => Promise<void>;

  // 操作
  loadCapacityPools: () => Promise<void>;

  // 版本
  activeVersionId: string | null;
}

export function useCapacityPoolManagement(): UseCapacityPoolManagementReturn {
  const activeVersionId = useActiveVersionId();
  const currentUser = useCurrentUser();
  const preferredMachineCode = useGlobalStore((state) => state.workbenchFilters.machineCode);

  // 加载状态
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  // 数据
  const [capacityPools, setCapacityPools] = useState<CapacityPool[]>([]);
  const [machineOptions, setMachineOptions] = useState<string[]>([]);

  // 筛选状态
  const [selectedMachines, setSelectedMachines] = useState<string[]>([]);
  const [dateRange, setDateRange] = useState<[Dayjs, Dayjs]>([
    dayjs(),
    dayjs().add(7, 'day'),
  ]);

  // 选中状态
  const [selectedRowKeys, setSelectedRowKeys] = useState<React.Key[]>([]);
  const [selectedPools, setSelectedPools] = useState<CapacityPool[]>([]);

  // 编辑模态框
  const [editModalVisible, setEditModalVisible] = useState(false);
  const [editingPool, setEditingPool] = useState<CapacityPool | null>(null);
  const [targetCapacity, setTargetCapacity] = useState(0);
  const [limitCapacity, setLimitCapacity] = useState(0);
  const [updateReason, setUpdateReason] = useState('');

  // 批量编辑模态框
  const [batchModalVisible, setBatchModalVisible] = useState(false);
  const [batchTargetCapacity, setBatchTargetCapacity] = useState<number | null>(null);
  const [batchLimitCapacity, setBatchLimitCapacity] = useState<number | null>(null);
  const [batchReason, setBatchReason] = useState('');

  // 统计计算
  const totalStats = useMemo<TotalStats>(() => {
    return capacityPools.reduce(
      (acc, pool) => ({
        totalTarget: acc.totalTarget + pool.target_capacity_t,
        totalUsed: acc.totalUsed + pool.used_capacity_t,
        totalAvailable: acc.totalAvailable + pool.available_capacity_t,
      }),
      { totalTarget: 0, totalUsed: 0, totalAvailable: 0 }
    );
  }, [capacityPools]);

  // 加载机组选项
  const loadMachineOptions = useCallback(async () => {
    try {
      const result = await materialApi.listMaterials({ limit: 1000, offset: 0 });
      const codes = new Set<string>();
      (Array.isArray(result) ? result : []).forEach((m: any) => {
        const code = String(m?.machine_code ?? '').trim();
        if (code) codes.add(code);
      });
      const list = Array.from(codes).sort();
      setMachineOptions(list);
      setSelectedMachines((prev) => {
        if (prev.length > 0) return prev;
        if (preferredMachineCode && list.includes(preferredMachineCode)) return [preferredMachineCode];
        return list;
      });
    } catch (e) {
      console.error('加载机组选项失败:', e);
      message.error('加载机组选项失败');
    }
  }, [preferredMachineCode]);

  // 加载产能池数据
  const loadCapacityPools = useCallback(async () => {
    if (!dateRange) {
      message.warning('请选择日期范围');
      return;
    }
    if (selectedMachines.length === 0) {
      message.warning('请选择机组');
      return;
    }

    setLoading(true);
    setLoadError(null);
    try {
      const result = await capacityApi.getCapacityPools(
        selectedMachines,
        formatDate(dateRange[0]),
        formatDate(dateRange[1]),
        activeVersionId || undefined
      );

      const normalized: CapacityPool[] = (Array.isArray(result) ? result : []).map((row: any) => {
        const target = Number(row?.target_capacity_t ?? 0);
        const limit = Number(row?.limit_capacity_t ?? 0);
        const used = Number(row?.used_capacity_t ?? 0);
        const available = Math.max(limit - used, 0);

        return {
          machine_code: String(row?.machine_code ?? ''),
          plan_date: String(row?.plan_date ?? ''),
          target_capacity_t: Number.isFinite(target) ? target : 0,
          limit_capacity_t: Number.isFinite(limit) ? limit : 0,
          used_capacity_t: Number.isFinite(used) ? used : 0,
          available_capacity_t: available,
        };
      });

      setCapacityPools(normalized);
      setSelectedRowKeys([]);
      setSelectedPools([]);
      message.success(`成功加载 ${normalized.length} 条产能池数据`);
    } catch (error: any) {
      console.error('加载产能池失败:', error);
      const msg = String(error?.message || error || '加载失败');
      setLoadError(msg);
      message.error(`加载失败: ${msg}`);
    } finally {
      setLoading(false);
    }
  }, [dateRange, selectedMachines, activeVersionId]);

  // 编辑操作
  const handleEdit = useCallback((record: CapacityPool) => {
    setEditingPool(record);
    setTargetCapacity(record.target_capacity_t);
    setLimitCapacity(record.limit_capacity_t);
    setUpdateReason('');
    setEditModalVisible(true);
  }, []);

  const closeEditModal = useCallback(() => {
    setEditModalVisible(false);
  }, []);

  const handleUpdate = useCallback(async () => {
    if (!editingPool) return;
    if (!updateReason.trim()) {
      message.warning('请输入调整原因');
      return;
    }

    setLoading(true);
    try {
      const operator = currentUser || 'system';
      await capacityApi.updateCapacityPool(
        editingPool.machine_code,
        editingPool.plan_date,
        targetCapacity,
        limitCapacity,
        updateReason,
        operator,
        activeVersionId || undefined
      );
      message.success('产能池更新成功');
      setEditModalVisible(false);
      await loadCapacityPools();
    } catch (error: any) {
      console.error('更新产能池失败:', error);
    } finally {
      setLoading(false);
    }
  }, [editingPool, updateReason, targetCapacity, limitCapacity, currentUser, activeVersionId, loadCapacityPools]);

  // 批量编辑操作
  const openBatchModal = useCallback(() => {
    setBatchTargetCapacity(null);
    setBatchLimitCapacity(null);
    setBatchReason('');
    setBatchModalVisible(true);
  }, []);

  const closeBatchModal = useCallback(() => {
    setBatchModalVisible(false);
  }, []);

  const handleBatchUpdate = useCallback(async () => {
    if (selectedPools.length === 0) {
      message.warning('请先选择需要批量调整的记录');
      return;
    }
    if (!batchReason.trim()) {
      message.warning('请输入调整原因');
      return;
    }
    if (batchTargetCapacity === null && batchLimitCapacity === null) {
      message.warning('请至少填写一个需要批量调整的字段（目标/极限）');
      return;
    }

    const updates = selectedPools.map((p) => {
      const target = batchTargetCapacity === null ? p.target_capacity_t : batchTargetCapacity;
      const limit = batchLimitCapacity === null ? p.limit_capacity_t : batchLimitCapacity;
      return {
        machine_code: p.machine_code,
        plan_date: p.plan_date,
        target_capacity_t: target,
        limit_capacity_t: limit,
      };
    });

    const invalid = updates.find((u) => u.target_capacity_t < 0 || u.limit_capacity_t < 0 || u.limit_capacity_t < u.target_capacity_t);
    if (invalid) {
      message.error(`参数不合法：${invalid.machine_code} ${invalid.plan_date}（极限不能小于目标，且不能为负）`);
      return;
    }

    setLoading(true);
    try {
      const operator = currentUser || 'system';
      const resp = await capacityApi.batchUpdateCapacityPools(
        updates,
        batchReason,
        operator,
        activeVersionId || undefined
      );

      const updated = Number(resp?.updated ?? 0);
      const skipped = Number(resp?.skipped ?? 0);
      message.success(`${resp?.message || '批量更新完成'}：更新 ${updated} 条，跳过 ${skipped} 条`);

      if (resp?.refresh) {
        const refresh = resp.refresh as any;
        const text = String(refresh?.message || '').trim();
        if (text) {
          if (refresh?.success) message.info(text);
          else message.warning(text);
        }
      }

      setBatchModalVisible(false);
      setBatchTargetCapacity(null);
      setBatchLimitCapacity(null);
      setBatchReason('');
      setSelectedRowKeys([]);
      setSelectedPools([]);
      await loadCapacityPools();
    } catch (error: any) {
      console.error('批量更新产能池失败:', error);
      message.error(error?.message || '批量更新失败');
    } finally {
      setLoading(false);
    }
  }, [selectedPools, batchReason, batchTargetCapacity, batchLimitCapacity, currentUser, activeVersionId, loadCapacityPools]);

  // 初始化加载
  useEffect(() => {
    loadMachineOptions().catch(() => void 0);
  }, [activeVersionId, loadMachineOptions]);

  useEffect(() => {
    if (!activeVersionId) return;
    if (selectedMachines.length === 0) return;
    loadCapacityPools().catch(() => void 0);
  }, [activeVersionId, selectedMachines, loadCapacityPools]);

  return {
    loading,
    loadError,
    capacityPools,
    machineOptions,
    totalStats,
    selectedMachines,
    setSelectedMachines,
    dateRange,
    setDateRange,
    selectedRowKeys,
    setSelectedRowKeys,
    selectedPools,
    setSelectedPools,
    editModalVisible,
    editingPool,
    targetCapacity,
    setTargetCapacity,
    limitCapacity,
    setLimitCapacity,
    updateReason,
    setUpdateReason,
    handleEdit,
    handleUpdate,
    closeEditModal,
    batchModalVisible,
    batchTargetCapacity,
    setBatchTargetCapacity,
    batchLimitCapacity,
    setBatchLimitCapacity,
    batchReason,
    setBatchReason,
    openBatchModal,
    closeBatchModal,
    handleBatchUpdate,
    loadCapacityPools,
    activeVersionId,
  };
}

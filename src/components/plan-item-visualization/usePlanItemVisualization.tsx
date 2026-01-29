/**
 * 排产明细可视化状态管理 Hook
 */

import { useCallback, useEffect, useMemo, useState } from 'react';
import { message, Modal } from 'antd';
import dayjs, { Dayjs } from 'dayjs';
import { DragEndEvent, PointerSensor, useSensor, useSensors } from '@dnd-kit/core';
import { arrayMove } from '@dnd-kit/sortable';
import { planApi, materialApi } from '../../api/tauri';
import { useEvent } from '../../api/eventBus';
import { useActiveVersionId, useCurrentUser } from '../../stores/use-global-store';
import { formatDate } from '../../utils/formatters';
import type { PlanItem, Statistics, PlanItemVisualizationProps } from './types';

export interface UsePlanItemVisualizationReturn {
  // 加载状态
  loading: boolean;

  // 数据
  planItems: PlanItem[];
  filteredItems: PlanItem[];
  statistics: Statistics | null;

  // 筛选状态
  selectedMachine: string;
  setSelectedMachine: (value: string) => void;
  selectedUrgentLevel: string;
  setSelectedUrgentLevel: (value: string) => void;
  selectedDate: Dayjs | null;
  setSelectedDate: (date: Dayjs | null) => void;
  dateRange: [Dayjs, Dayjs] | null;
  setDateRange: (range: [Dayjs, Dayjs] | null) => void;
  searchText: string;
  setSearchText: (text: string) => void;
  machineOptions: string[];

  // 选中状态
  selectedMaterialIds: string[];
  setSelectedMaterialIds: (ids: string[]) => void;

  // 详情模态框
  selectedItem: PlanItem | null;
  showDetailModal: boolean;
  handleViewDetail: (item: PlanItem) => void;
  closeDetailModal: () => void;

  // 强制放行模态框
  forceReleaseModalVisible: boolean;
  forceReleaseReason: string;
  setForceReleaseReason: (reason: string) => void;
  forceReleaseMode: 'AutoFix' | 'Strict';
  setForceReleaseMode: (mode: 'AutoFix' | 'Strict') => void;
  openForceReleaseModal: () => void;
  closeForceReleaseModal: () => void;
  handleBatchForceRelease: () => Promise<void>;

  // 拖拽
  sensors: ReturnType<typeof useSensors>;
  handleDragEnd: (event: DragEndEvent) => Promise<void>;
  setFilteredItems: (items: PlanItem[]) => void;

  // 操作
  loadPlanItems: (versionId?: string, date?: string) => Promise<void>;
  clearFilters: () => void;

  // 版本
  activeVersionId: string | null;
}

export function usePlanItemVisualization(
  props: PlanItemVisualizationProps
): UsePlanItemVisualizationReturn {
  const {
    machineCode,
    urgentLevel,
    refreshSignal,
    selectedMaterialIds: controlledSelectedMaterialIds,
    onSelectedMaterialIdsChange,
  } = props;

  const activeVersionId = useActiveVersionId();
  const currentUser = useCurrentUser();

  // 加载状态
  const [loading, setLoading] = useState(false);

  // 数据
  const [planItems, setPlanItems] = useState<PlanItem[]>([]);
  const [filteredItems, setFilteredItems] = useState<PlanItem[]>([]);
  const [statistics, setStatistics] = useState<Statistics | null>(null);

  // 筛选状态
  const [selectedMachine, setSelectedMachine] = useState<string>('all');
  const [selectedUrgentLevel, setSelectedUrgentLevel] = useState<string>('all');
  const [selectedDate, setSelectedDate] = useState<Dayjs | null>(null);
  const [dateRange, setDateRange] = useState<[Dayjs, Dayjs] | null>(null);
  const [searchText, setSearchText] = useState('');

  // 选中状态
  const [internalSelectedMaterialIds, setInternalSelectedMaterialIds] = useState<string[]>([]);
  const selectedMaterialIds = controlledSelectedMaterialIds ?? internalSelectedMaterialIds;
  const setSelectedMaterialIds = useCallback(
    (ids: string[]) => {
      if (onSelectedMaterialIdsChange) {
        onSelectedMaterialIdsChange(ids);
        return;
      }
      setInternalSelectedMaterialIds(ids);
    },
    [onSelectedMaterialIdsChange]
  );

  // 详情模态框
  const [selectedItem, setSelectedItem] = useState<PlanItem | null>(null);
  const [showDetailModal, setShowDetailModal] = useState(false);

  // 强制放行模态框
  const [forceReleaseModalVisible, setForceReleaseModalVisible] = useState(false);
  const [forceReleaseReason, setForceReleaseReason] = useState('');
  const [forceReleaseMode, setForceReleaseMode] = useState<'AutoFix' | 'Strict'>('AutoFix');

  // 拖拽传感器
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 1 },
    })
  );

  // 计算机组选项
  const machineOptions = useMemo(() => {
    const codes = new Set<string>();
    planItems.forEach((it) => {
      const code = String(it.machine_code ?? '').trim();
      if (code) codes.add(code);
    });
    return Array.from(codes).sort();
  }, [planItems]);

  // 计算统计信息
  const calculateStatistics = useCallback((items: PlanItem[]) => {
    const stats: Statistics = {
      total_items: items.length,
      total_weight: items.reduce((sum, item) => sum + item.weight_t, 0),
      by_machine: {},
      by_urgent_level: {},
      frozen_count: items.filter((item) => item.locked_in_plan).length,
    };

    items.forEach((item) => {
      stats.by_machine[item.machine_code] = (stats.by_machine[item.machine_code] || 0) + 1;
      if (item.urgent_level) {
        stats.by_urgent_level[item.urgent_level] =
          (stats.by_urgent_level[item.urgent_level] || 0) + 1;
      }
    });

    setStatistics(stats);
  }, []);

  // 加载排产明细
  const loadPlanItems = useCallback(
    async (versionId?: string, date?: string) => {
      if (!versionId && !activeVersionId) {
        message.warning('请先激活一个版本');
        return;
      }

      const targetVersionId = versionId || activeVersionId;
      setLoading(true);
      try {
        let result;
        if (date) {
          result = await planApi.listItemsByDate(targetVersionId!, date);
        } else {
          result = await planApi.listPlanItems(targetVersionId!);
        }

        const items = (Array.isArray(result) ? result : []).map((item: any) => ({
          key: String(item.material_id ?? ''),
          ...item,
        }));

        setPlanItems(items);
        setFilteredItems(items);
        calculateStatistics(items);
        message.success(`成功加载 ${items.length} 条排产明细`);
      } catch (error: any) {
        console.error('加载排产明细失败:', error);
      } finally {
        setLoading(false);
      }
    },
    [activeVersionId, calculateStatistics]
  );

  // 筛选数据
  const filterData = useCallback(() => {
    let filtered = [...planItems];

    if (selectedMachine !== 'all') {
      filtered = filtered.filter((item) => item.machine_code === selectedMachine);
    }

    if (selectedDate) {
      const dateStr = formatDate(selectedDate);
      filtered = filtered.filter((item) => item.plan_date === dateStr);
    }

    if (dateRange) {
      const [start, end] = dateRange;
      filtered = filtered.filter((item) => {
        const itemDate = dayjs(item.plan_date);
        return itemDate.isAfter(start.subtract(1, 'day')) && itemDate.isBefore(end.add(1, 'day'));
      });
    }

    if (searchText) {
      const searchLower = searchText.toLowerCase();
      filtered = filtered.filter(
        (item) =>
          item.material_id.toLowerCase().includes(searchLower) ||
          item.steel_grade?.toLowerCase().includes(searchLower)
      );
    }

    if (selectedUrgentLevel !== 'all') {
      filtered = filtered.filter((item) => item.urgent_level === selectedUrgentLevel);
    }

    setFilteredItems(filtered);
    calculateStatistics(filtered);
  }, [planItems, selectedMachine, selectedDate, dateRange, searchText, selectedUrgentLevel, calculateStatistics]);

  // 查看详情
  const handleViewDetail = useCallback((item: PlanItem) => {
    setSelectedItem(item);
    setShowDetailModal(true);
  }, []);

  const closeDetailModal = useCallback(() => {
    setShowDetailModal(false);
  }, []);

  // 强制放行模态框
  const openForceReleaseModal = useCallback(() => {
    setForceReleaseModalVisible(true);
  }, []);

  const closeForceReleaseModal = useCallback(() => {
    setForceReleaseModalVisible(false);
    setForceReleaseReason('');
    setForceReleaseMode('AutoFix');
  }, []);

  // 批量强制放行
  const handleBatchForceRelease = useCallback(async () => {
    if (!forceReleaseReason.trim()) {
      message.warning('请输入强制放行原因');
      return;
    }

    setLoading(true);
    try {
      const res: any = await materialApi.batchForceRelease(
        selectedMaterialIds,
        currentUser,
        forceReleaseReason,
        forceReleaseMode
      );
      message.success(String(res?.message || `成功强制放行 ${selectedMaterialIds.length} 个材料`));

      const violations = Array.isArray(res?.details?.violations) ? res.details.violations : [];
      if (violations.length > 0) {
        Modal.info({
          title: '强制放行警告（未适温材料）',
          width: 720,
          content: (
            <div style={{ maxHeight: 420, overflow: 'auto' }}>
              <pre style={{ fontSize: 12, whiteSpace: 'pre-wrap' }}>
                {JSON.stringify(violations, null, 2)}
              </pre>
            </div>
          ),
        });
      }
      closeForceReleaseModal();
      setSelectedMaterialIds([]);
      if (activeVersionId) {
        await loadPlanItems(activeVersionId);
      }
    } catch (error: any) {
      console.error('强制放行失败:', error);
    } finally {
      setLoading(false);
    }
  }, [
    forceReleaseReason,
    forceReleaseMode,
    selectedMaterialIds,
    currentUser,
    activeVersionId,
    loadPlanItems,
    closeForceReleaseModal,
    setSelectedMaterialIds,
  ]);

  // 拖拽结束处理
  const handleDragEnd = useCallback(
    async (event: DragEndEvent) => {
      const { active, over } = event;

      if (!over || active.id === over.id) return;

      const oldIndex = filteredItems.findIndex((item) => item.key === active.id);
      const newIndex = filteredItems.findIndex((item) => item.key === over.id);

      if (oldIndex === -1 || newIndex === -1) return;

      const draggedItem = filteredItems[oldIndex];
      const targetItem = filteredItems[newIndex];

      if (draggedItem.locked_in_plan) {
        message.warning('冻结材料不可调整顺序');
        return;
      }

      if (
        draggedItem.plan_date !== targetItem.plan_date ||
        draggedItem.machine_code !== targetItem.machine_code
      ) {
        message.warning('只能在同一日期和机组内调整顺序');
        return;
      }

      // 乐观更新UI
      const newItems = arrayMove(filteredItems, oldIndex, newIndex);
      setFilteredItems(newItems);

      try {
        await planApi.moveItems(
          activeVersionId!,
          [
            {
              material_id: draggedItem.material_id,
              to_date: targetItem.plan_date,
              to_seq: targetItem.seq_no,
              to_machine: targetItem.machine_code,
            },
          ],
          'AUTO_FIX',
          currentUser || 'admin',
          '拖拽调整顺序'
        );
        message.success('顺序调整成功');
        if (activeVersionId) {
          await loadPlanItems(activeVersionId);
        }
      } catch (error: any) {
        console.error('调整顺序失败:', error);
        setFilteredItems(filteredItems); // 恢复原顺序
      }
    },
    [filteredItems, activeVersionId, currentUser, loadPlanItems]
  );

  // 清除筛选
  const clearFilters = useCallback(() => {
    setSelectedMachine('all');
    setSelectedUrgentLevel('all');
    setSelectedDate(null);
    setDateRange(null);
    setSearchText('');
  }, []);

  // 订阅 plan_updated 事件
  useEvent('plan_updated', () => {
    if (activeVersionId) {
      loadPlanItems(activeVersionId);
    }
  });

  // 外部筛选控制
  useEffect(() => {
    setSelectedMachine(machineCode ?? 'all');
  }, [machineCode]);

  useEffect(() => {
    setSelectedUrgentLevel(urgentLevel ?? 'all');
  }, [urgentLevel]);

  // 初始加载
  useEffect(() => {
    if (activeVersionId) {
      loadPlanItems(activeVersionId);
    }
  }, [activeVersionId, refreshSignal, loadPlanItems]);

  // 筛选条件变化时重新筛选
  useEffect(() => {
    filterData();
  }, [selectedMachine, selectedDate, dateRange, searchText, selectedUrgentLevel, filterData]);

  return {
    loading,
    planItems,
    filteredItems,
    statistics,
    selectedMachine,
    setSelectedMachine,
    selectedUrgentLevel,
    setSelectedUrgentLevel,
    selectedDate,
    setSelectedDate,
    dateRange,
    setDateRange,
    searchText,
    setSearchText,
    machineOptions,
    selectedMaterialIds,
    setSelectedMaterialIds,
    selectedItem,
    showDetailModal,
    handleViewDetail,
    closeDetailModal,
    forceReleaseModalVisible,
    forceReleaseReason,
    setForceReleaseReason,
    forceReleaseMode,
    setForceReleaseMode,
    openForceReleaseModal,
    closeForceReleaseModal,
    handleBatchForceRelease,
    sensors,
    handleDragEnd,
    setFilteredItems,
    loadPlanItems,
    clearFilters,
    activeVersionId,
  };
}

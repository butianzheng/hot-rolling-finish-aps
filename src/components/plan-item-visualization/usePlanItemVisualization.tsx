/**
 * 排产明细可视化状态管理 Hook
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { message, Modal } from 'antd';
import dayjs, { Dayjs } from 'dayjs';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { DragEndEvent, PointerSensor, useSensor, useSensors } from '@dnd-kit/core';
import { arrayMove } from '@dnd-kit/sortable';
import { planApi, materialApi } from '../../api/tauri';
import { useEvent } from '../../api/eventBus';
import { useActiveVersionId, useCurrentUser } from '../../stores/use-global-store';
import { workbenchQueryKeys } from '../../pages/workbench/queryKeys';
import { formatDate } from '../../utils/formatters';
import type { PlanItemStatusSummary } from '../../utils/planItemStatus';
import { matchPlanItemStatusFilter, summarizePlanItemStatus } from '../../utils/planItemStatus';
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
  handleBatchClearForceRelease: () => Promise<void>;

  // 拖拽
  sensors: ReturnType<typeof useSensors>;
  handleDragEnd: (event: DragEndEvent) => Promise<void>;
  setFilteredItems: (items: PlanItem[]) => void;

  // 操作
  loadPlanItems: (versionId?: string, date?: string) => Promise<void>;
  clearFilters: () => void;

  // 版本
  activeVersionId: string | null;

  // 状态快速筛选统计（不受状态筛选影响）
  statusSummary: PlanItemStatusSummary;
}

export function usePlanItemVisualization(
  props: PlanItemVisualizationProps
): UsePlanItemVisualizationReturn {
  const {
    machineCode,
    machineOptions: externalMachineOptions,
    urgentLevel,
    defaultDateRange,
    statusFilter = 'ALL',
    focusRequest,
    selectedMaterialIds: controlledSelectedMaterialIds,
    onSelectedMaterialIdsChange,
  } = props;

  const activeVersionId = useActiveVersionId();
  const currentUser = useCurrentUser();
  const queryClient = useQueryClient();

  // 筛选状态
  const [selectedMachine, setSelectedMachine] = useState<string>(() => machineCode ?? 'all');
  const [selectedUrgentLevel, setSelectedUrgentLevel] = useState<string>(() => urgentLevel ?? 'all');
  const [selectedDate, setSelectedDate] = useState<Dayjs | null>(null);
  const [dateRange, setDateRange] = useState<[Dayjs, Dayjs] | null>(() => defaultDateRange ?? null);
  const [searchText, setSearchText] = useState('');

  // 跟随外部默认范围（Workbench 联动），除非用户已手动调整
  const lastDefaultRangeKeyRef = useRef<string | null>(null);
  useEffect(() => {
    if (!defaultDateRange) return;
    const nextKey = `${formatDate(defaultDateRange[0])}_${formatDate(defaultDateRange[1])}`;
    const prevKey = lastDefaultRangeKeyRef.current;
    lastDefaultRangeKeyRef.current = nextKey;

    const currentKey = dateRange ? `${formatDate(dateRange[0])}_${formatDate(dateRange[1])}` : null;
    const userHasCustomRange = !!currentKey && !!prevKey && currentKey !== prevKey;
    if (!selectedDate && (!dateRange || !userHasCustomRange)) {
      setDateRange(defaultDateRange);
    }
  }, [defaultDateRange, dateRange, selectedDate]);

  const queryPlanDateFrom = useMemo(() => {
    if (selectedDate) return formatDate(selectedDate);
    if (dateRange?.[0]) return formatDate(dateRange[0]);
    if (defaultDateRange?.[0]) return formatDate(defaultDateRange[0]);
    return formatDate(dayjs().subtract(3, 'day'));
  }, [defaultDateRange, dateRange, selectedDate]);

  const queryPlanDateTo = useMemo(() => {
    if (selectedDate) return formatDate(selectedDate);
    if (dateRange?.[1]) return formatDate(dateRange[1]);
    if (defaultDateRange?.[1]) return formatDate(defaultDateRange[1]);
    return formatDate(dayjs().add(10, 'day'));
  }, [defaultDateRange, dateRange, selectedDate]);

  // 使用 React Query 获取排产数据（按范围/分页）
  const planItemsQuery = useQuery({
    queryKey: workbenchQueryKeys.planItems.list({
      version_id: activeVersionId,
      machine_code: selectedMachine !== 'all' ? selectedMachine : undefined,
      plan_date_from: queryPlanDateFrom,
      plan_date_to: queryPlanDateTo,
    }),
    enabled: !!activeVersionId,
    queryFn: async ({ signal }) => {
      if (!activeVersionId) return [];

      const pageSize = 5000;
      const maxItems = 200_000;
      let offset = 0;
      const all: any[] = [];

      while (true) {
        if (signal?.aborted) {
          throw new DOMException('Aborted', 'AbortError');
        }

        const page = await planApi.listPlanItems(activeVersionId, {
          machine_code: selectedMachine !== 'all' ? selectedMachine : undefined,
          plan_date_from: queryPlanDateFrom,
          plan_date_to: queryPlanDateTo,
          limit: pageSize,
          offset,
        });

        all.push(...page);
        if (page.length < pageSize) break;
        offset += pageSize;
        if (offset >= maxItems) break;
      }

      return all.map((item: any) => ({
        key: String(item.material_id ?? ''),
        ...item,
        // 优先使用 material_state 快照判定强放，确保取消强放后无需重算即可即时反映
        force_release_in_plan:
          String(item?.sched_state || '')
            .trim()
            .toUpperCase() === 'FORCE_RELEASE'
            ? true
            : !!item?.force_release_in_plan,
      }));
    },
    staleTime: 30 * 1000,
  });

  const planItems = useMemo(() => planItemsQuery.data || [], [planItemsQuery.data]);

  // 数据
  const [filteredItems, setFilteredItems] = useState<PlanItem[]>([]);
  const [statistics, setStatistics] = useState<Statistics | null>(null);
  const [statusSummary, setStatusSummary] = useState<PlanItemStatusSummary>(() =>
    summarizePlanItemStatus([])
  );

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
  const pendingLocateMaterialRef = useRef<string | null>(null);

  // 详情模态框
  const [selectedItem, setSelectedItem] = useState<PlanItem | null>(null);
  const [showDetailModal, setShowDetailModal] = useState(false);

  // 强制放行模态框
  const [forceReleaseModalVisible, setForceReleaseModalVisible] = useState(false);
  const [forceReleaseReason, setForceReleaseReason] = useState('');
  const [forceReleaseMode, setForceReleaseMode] = useState<'AutoFix' | 'Strict'>('AutoFix');
  const [operationLoading, setOperationLoading] = useState(false);

  // 合并 loading 状态（数据加载 + 操作中）
  const loading = planItemsQuery.isLoading || operationLoading;

  // 拖拽传感器
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 1 },
    })
  );

  // 计算机组选项
  const machineOptions = useMemo(() => {
    const external = (externalMachineOptions ?? [])
      .map((s) => String(s).trim())
      .filter(Boolean);
    if (external.length > 0) {
      return Array.from(new Set(external)).sort();
    }

    const codes = new Set<string>();
    planItems.forEach((it) => {
      const code = String(it.machine_code ?? '').trim();
      if (code) codes.add(code);
    });
    return Array.from(codes).sort();
  }, [externalMachineOptions, planItems]);

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

  // 加载排产明细（兼容性包装，实际使用 React Query refetch）
  const loadPlanItems = useCallback(
    async (_versionId?: string, _date?: string) => {
      if (!_versionId && !activeVersionId) {
        message.warning('请先激活一个版本');
        return;
      }
      // 忽略参数（原实现支持 date 参数但实际未使用）
      await planItemsQuery.refetch();
    },
    [activeVersionId, planItemsQuery]
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
      filtered = filtered.filter((item) => {
        const materialId = String(item.material_id || '').toLowerCase();
        const steelGrade = String(item.steel_grade || '').toLowerCase();
        const contractNo = String(item.contract_no || '').toLowerCase();
        return (
          materialId.includes(searchLower) ||
          steelGrade.includes(searchLower) ||
          contractNo.includes(searchLower)
        );
      });
    }

    if (selectedUrgentLevel !== 'all') {
      filtered = filtered.filter((item) => item.urgent_level === selectedUrgentLevel);
    }

    // 状态统计：基于状态筛选前的数据，确保快筛标签数量稳定可见
    setStatusSummary(summarizePlanItemStatus(filtered));

    // 状态筛选：由工作台统一控制（已排/冻结/强放/可调）
    if (statusFilter && statusFilter !== 'ALL') {
      filtered = filtered.filter((item) => matchPlanItemStatusFilter(item, statusFilter));
    }

    setFilteredItems(filtered);
    calculateStatistics(filtered);
  }, [
    planItems,
    selectedMachine,
    selectedDate,
    dateRange,
    searchText,
    selectedUrgentLevel,
    statusFilter,
    calculateStatistics,
  ]);

  const applyMaterialLocation = useCallback(
    (item: PlanItem, materialId: string) => {
      const machine = String(item.machine_code || '').trim();
      const planDate = String(item.plan_date || '').trim();
      const d = dayjs(planDate);

      setSelectedUrgentLevel('all');
      setSearchText('');
      if (machine) setSelectedMachine(machine);
      if (d.isValid()) {
        setSelectedDate(d);
        setDateRange(null);
      }
      setSelectedMaterialIds([materialId]);
      message.success(`已定位材料 ${materialId}：${machine || '-'} / ${planDate || '-'}`);
    },
    [setSelectedMaterialIds]
  );

  const findPlanItemByMaterialId = useCallback(
    (materialId: string) => {
      const target = String(materialId || '').trim().toLowerCase();
      if (!target) return null;
      return (
        planItems.find((it) => String(it.material_id || '').trim().toLowerCase() === target) ?? null
      );
    },
    [planItems]
  );

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

    setOperationLoading(true);
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
      setOperationLoading(false);
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


  const handleBatchClearForceRelease = useCallback(async () => {
    if (selectedMaterialIds.length === 0) {
      message.warning('请先选择材料');
      return;
    }

    setOperationLoading(true);
    try {
      const res: any = await materialApi.batchClearForceRelease(
        selectedMaterialIds,
        currentUser || 'system',
        '工作台批量取消强放'
      );
      message.success(String(res?.message || `成功取消强放 ${selectedMaterialIds.length} 个材料`));
      setSelectedMaterialIds([]);
      if (activeVersionId) {
        await loadPlanItems(activeVersionId);
      }
    } catch (error: any) {
      console.error('取消强放失败:', error);
      message.error(String(error?.message || '取消强放失败'));
    } finally {
      setOperationLoading(false);
    }
  }, [selectedMaterialIds, currentUser, activeVersionId, loadPlanItems, setSelectedMaterialIds]);

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
    setDateRange(defaultDateRange ?? null);
    setSearchText('');
  }, [defaultDateRange]);

  // 订阅 plan_updated 事件（使用 React Query invalidate）
  useEvent('plan_updated', () => {
    if (activeVersionId) {
      queryClient.invalidateQueries({
        queryKey: workbenchQueryKeys.planItems.all,
      });
    }
  });

  // 外部筛选控制
  useEffect(() => {
    setSelectedMachine(machineCode ?? 'all');
  }, [machineCode]);

  useEffect(() => {
    setSelectedUrgentLevel(urgentLevel ?? 'all');
  }, [urgentLevel]);

  const resolveMaterialFocus = useCallback(
    async (materialIdRaw: string) => {
      const materialId = String(materialIdRaw || '').trim();
      if (!materialId) return;

      const matched = findPlanItemByMaterialId(materialId);
      if (matched) {
        pendingLocateMaterialRef.current = null;
        applyMaterialLocation(matched, materialId);
        return;
      }

      try {
        const detail: any = await materialApi.getMaterialDetail(materialId);
        const state = detail?.state ?? null;
        const master = detail?.master ?? null;

        const scheduledMachine = String(
          state?.scheduled_machine_code || master?.current_machine_code || ''
        ).trim();
        const scheduledDate = String(state?.scheduled_date || '').trim();
        const d = dayjs(scheduledDate);

        setSelectedUrgentLevel('all');
        setSearchText('');
        if (scheduledMachine) setSelectedMachine(scheduledMachine);
        if (d.isValid()) {
          setSelectedDate(d);
          setDateRange(null);
          pendingLocateMaterialRef.current = materialId;
          message.info(`正在定位材料 ${materialId} 的排程单元`);
          return;
        }

        pendingLocateMaterialRef.current = null;
        setSelectedMaterialIds([]);
        message.warning(`材料 ${materialId} 未排产，已定位到物料池`);
      } catch (error) {
        pendingLocateMaterialRef.current = null;
        console.error('材料定位失败:', error);
        message.warning(`材料 ${materialId} 定位失败，已定位到物料池`);
      }
    },
    [applyMaterialLocation, findPlanItemByMaterialId, setSelectedMaterialIds]
  );

  useEffect(() => {
    const pendingMaterialId = pendingLocateMaterialRef.current;
    if (!pendingMaterialId) return;
    if (planItemsQuery.isFetching) return;

    const matched = findPlanItemByMaterialId(pendingMaterialId);
    if (matched) {
      pendingLocateMaterialRef.current = null;
      applyMaterialLocation(matched, pendingMaterialId);
      return;
    }

    pendingLocateMaterialRef.current = null;
    setSelectedMaterialIds([]);
    message.warning(`材料 ${pendingMaterialId} 未排产，已定位到物料池`);
  }, [applyMaterialLocation, findPlanItemByMaterialId, planItemsQuery.isFetching, setSelectedMaterialIds]);

  // 外部聚焦（从风险/问题卡片跳转到矩阵时）
  useEffect(() => {
    if (!focusRequest) return;
    const mode = String(focusRequest.mode || '').trim();
    const materialId = String(focusRequest.materialId || '').trim();
    const contractNo = String(focusRequest.contractNo || '').trim();
    const machine = String(focusRequest.machine || '').trim();
    const date = String(focusRequest.date || '').trim();
    const nextSearchText = String(focusRequest.searchText || contractNo || '').trim();

    if (materialId && mode !== 'SEARCH') {
      void resolveMaterialFocus(materialId);
      return;
    }

    if (machine) setSelectedMachine(machine);
    if (date) {
      const d = dayjs(date);
      if (d.isValid()) {
        setSelectedDate(d);
        setDateRange(null);
      }
    }

    if (nextSearchText) {
      setSearchText(nextSearchText);
      setSelectedUrgentLevel('all');
      if (materialId) {
        setSelectedMaterialIds([materialId]);
      } else {
        setSelectedMaterialIds([]);
      }
      if (!machine) setSelectedMachine('all');
      if (!date) {
        setSelectedDate(null);
        setDateRange(defaultDateRange ?? null);
      }
    }
  }, [defaultDateRange, focusRequest?.nonce, resolveMaterialFocus]);

  // 筛选条件变化时重新筛选
  useEffect(() => {
    filterData();
  }, [selectedMachine, selectedDate, dateRange, searchText, selectedUrgentLevel, statusFilter, filterData]);

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
    handleBatchClearForceRelease,
    sensors,
    handleDragEnd,
    setFilteredItems,
    loadPlanItems,
    clearFilters,
    activeVersionId,
    statusSummary,
  };
}

import { useEffect, useMemo, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import dayjs from 'dayjs';
import { message } from 'antd';
import type { MaterialPoolSelection } from '../../../components/workbench/MaterialPool';
import type { WorkbenchFilters, WorkbenchViewMode } from '../../../stores/use-global-store';
import type { WorkbenchDateRangeMode, WorkbenchDeepLinkContext } from '../types';

export type { WorkbenchDeepLinkContext } from '../types';

function getDeepLinkContextLabel(context: string): string {
  if (context === 'risk') return '风险日';
  if (context === 'bottleneck') return '瓶颈点';
  if (context === 'capacityOpportunity') return '容量优化机会';
  if (context === 'orders') return '材料失败';
  if (context === 'coldStock') return '冷坨高压力';
  if (context === 'roll') return '换辊警报';
  return '';
}

function normalizeDeepLinkFocus(focus: string | null): string | undefined {
  const raw = String(focus || '').trim();
  if (!raw) return undefined;
  if (raw === 'gantt') return 'calendar';
  return raw;
}

export function useWorkbenchDeepLink(params: {
  searchParams: URLSearchParams;
  globalMachineCode: WorkbenchFilters['machineCode'];
  setPoolSelection: Dispatch<SetStateAction<MaterialPoolSelection>>;
  setWorkbenchFilters: (filters: Partial<WorkbenchFilters>) => void;
  setWorkbenchViewMode: (mode: WorkbenchViewMode) => void;
  setDateRangeMode: Dispatch<SetStateAction<WorkbenchDateRangeMode>>;
  setWorkbenchDateRange: Dispatch<SetStateAction<[dayjs.Dayjs, dayjs.Dayjs]>>;
  setInspectorOpen: Dispatch<SetStateAction<boolean>>;
  setInspectedMaterialId: Dispatch<SetStateAction<string | null>>;
}): {
  deepLinkContext: WorkbenchDeepLinkContext | null;
  deepLinkContextLabel: string;
} {
  const {
    searchParams,
    globalMachineCode,
    setPoolSelection,
    setWorkbenchFilters,
    setWorkbenchViewMode,
    setDateRangeMode,
    setWorkbenchDateRange,
    setInspectorOpen,
    setInspectedMaterialId,
  } = params;

  // 与全局筛选同步：允许其他页面（如风险下钻）回填机组筛选
  useEffect(() => {
    const nextMachine = globalMachineCode ?? null;
    setPoolSelection((prev) => {
      if (prev.machineCode === nextMachine) return prev;
      return { ...prev, machineCode: nextMachine, schedState: null };
    });
  }, [globalMachineCode, setPoolSelection]);

  // 深链接：material_id 进入工作台
  // - matrix/决策上下文：统一走矩阵定位，不自动开详情，不写左侧搜索词
  // - 其他上下文：保持原行为（可自动开详情）
  useEffect(() => {
    const materialId = searchParams.get('material_id');
    const id = String(materialId || '').trim();
    if (!id) return;
    const context = String(searchParams.get('context') || '').trim();
    const focus = normalizeDeepLinkFocus(searchParams.get('focus'));
    const matrixLocateMode = focus === 'matrix' || context === 'orders' || context === 'risk' || context === 'bottleneck' || context === 'capacityOpportunity' || context === 'coldStock' || context === 'roll';
    const skipAutoInspect =
      matrixLocateMode;
    if (skipAutoInspect) {
      setInspectedMaterialId(null);
      setInspectorOpen(false);
    } else {
      setInspectedMaterialId(id);
      setInspectorOpen(true);
    }
    setWorkbenchViewMode('MATRIX');
    setPoolSelection((prev) => {
      const next = {
        ...prev,
        machineCode: null,
        // matrix 定位时，左侧不再按单材料过滤，避免“只剩一条”的错觉
        searchText: matrixLocateMode ? '' : id,
        schedState: null,
      };
      if (
        prev.machineCode === next.machineCode &&
        prev.searchText === next.searchText &&
        prev.schedState === next.schedState
      ) {
        return prev;
      }
      return next;
    });
    // 存在明确目标材料时，避免被机组/紧急度/锁定筛选误伤导致“看不见”。
    setWorkbenchFilters({ machineCode: null, urgencyLevel: null, lockStatus: 'ALL' });
  }, [
    searchParams,
    setInspectedMaterialId,
    setInspectorOpen,
    setPoolSelection,
    setWorkbenchFilters,
    setWorkbenchViewMode,
  ]);

  // 深链接：contract_no 回退定位（无法直接定位 material_id 时）
  // 统一走矩阵定位，不再改左侧搜索词（右侧矩阵由 focusRequest 驱动合同搜索）。
  useEffect(() => {
    const materialId = String(searchParams.get('material_id') || '').trim();
    if (materialId) return; // material_id 优先

    const contractNo = String(searchParams.get('contract_no') || '').trim();
    if (!contractNo) return;
    setWorkbenchViewMode('MATRIX');
    setPoolSelection((prev) => {
      const next = {
        ...prev,
        machineCode: null,
        searchText: '',
        schedState: null,
      };
      if (
        prev.machineCode === next.machineCode &&
        prev.searchText === next.searchText &&
        prev.schedState === next.schedState
      ) {
        return prev;
      }
      return next;
    });
    setWorkbenchFilters({ machineCode: null, urgencyLevel: null, lockStatus: 'ALL' });
  }, [searchParams, setPoolSelection, setWorkbenchFilters, setWorkbenchViewMode]);

  const [deepLinkContext, setDeepLinkContext] = useState<WorkbenchDeepLinkContext | null>(null);

  // 深链接：从风险概览跳转时，处理上下文参数（第三阶段）
  useEffect(() => {
    const machine = searchParams.get('machine');
    const date = searchParams.get('date');
    const urgency = searchParams.get('urgency');
    const context = searchParams.get('context');
    const focus = normalizeDeepLinkFocus(searchParams.get('focus'));
    const openCell = searchParams.get('openCell');
    const materialId = String(searchParams.get('material_id') || '').trim();
    const contractNo = String(searchParams.get('contract_no') || '').trim();
    const hasDirectTarget = !!materialId || !!contractNo;

    // 如果有深链接参数，保存到状态并应用
    if (machine || date || urgency || context || focus || openCell) {
      const openCellFlag = openCell === '1' || openCell === 'true';
      setDeepLinkContext({
        machine: machine || undefined,
        date: date || undefined,
        urgency: urgency || undefined,
        context: context || undefined,
        focus,
        openCell: openCellFlag,
        materialId: materialId || undefined,
        contractNo: contractNo || undefined,
      });

      // 应用机组筛选
      if (machine && !hasDirectTarget) {
        setPoolSelection((prev) => {
          if (prev.machineCode === machine) return prev;
          return { ...prev, machineCode: machine, schedState: null };
        });
        setWorkbenchFilters({ machineCode: machine });
      }

      // 应用紧急度筛选（扩展功能）
      if (urgency && !hasDirectTarget) {
        setWorkbenchFilters({ urgencyLevel: urgency });
      }

      // 深链接日期：默认聚焦前后各 3 天，并锁定范围，避免被自动范围覆盖
      if (date && !hasDirectTarget) {
        const focusDate = dayjs(date);
        if (focusDate.isValid()) {
          setWorkbenchDateRange([focusDate.subtract(3, 'day'), focusDate.add(3, 'day')]);
          setDateRangeMode('PINNED');
        }
      }

      // 深链接指定排程日历定位（风险日/瓶颈点等）
      // 兼容历史链接 focus=gantt
      if (focus === 'matrix') {
        setWorkbenchViewMode('MATRIX');
      } else if (focus === 'calendar' || openCellFlag) {
        setWorkbenchViewMode('CALENDAR');
      }

      // 显示来源提示
      const contextLabel = getDeepLinkContextLabel(String(context || '').trim());
      if (contextLabel) {
        const filterHints = [];
        if (machine && !hasDirectTarget) filterHints.push(`机组: ${machine}`);
        if (urgency && !hasDirectTarget) filterHints.push(`紧急度: ${urgency}`);
        if (date && !hasDirectTarget) filterHints.push(`日期: ${date}`);
        if (hasDirectTarget && materialId) filterHints.push(`材料: ${materialId}`);
        if (hasDirectTarget && !materialId && contractNo) filterHints.push(`合同: ${contractNo}`);

        const filterInfo = filterHints.length > 0 ? `（${filterHints.join('、')}）` : '';
        message.info(`已从「${contextLabel}」跳转，自动应用相关筛选条件${filterInfo}`);
      }
    }
  }, [
    searchParams,
    setDateRangeMode,
    setPoolSelection,
    setWorkbenchDateRange,
    setWorkbenchFilters,
    setWorkbenchViewMode,
  ]);

  const deepLinkContextLabel = useMemo(() => {
    const ctx = String(deepLinkContext?.context || '').trim();
    return getDeepLinkContextLabel(ctx);
  }, [deepLinkContext?.context]);

  return { deepLinkContext, deepLinkContextLabel };
}

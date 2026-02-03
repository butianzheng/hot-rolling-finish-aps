import { useEffect, useMemo, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import dayjs from 'dayjs';
import { message } from 'antd';
import type { MaterialPoolSelection } from '../../../components/workbench/MaterialPool';
import type { WorkbenchFilters, WorkbenchViewMode } from '../../../stores/use-global-store';
import type { WorkbenchDateRangeMode } from '../types';

export type WorkbenchDeepLinkContext = {
  machine?: string;
  date?: string;
  urgency?: string;
  context?: string;
  focus?: string;
  openCell?: boolean;
};

function getDeepLinkContextLabel(context: string): string {
  if (context === 'risk') return '风险日';
  if (context === 'bottleneck') return '瓶颈点';
  if (context === 'capacityOpportunity') return '容量优化机会';
  if (context === 'orders') return '订单失败';
  if (context === 'coldStock') return '冷坨高压力';
  if (context === 'roll') return '换辊警报';
  return '';
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
      return { machineCode: nextMachine, schedState: null };
    });
  }, [globalMachineCode, setPoolSelection]);

  // 深链接：从“策略对比/变更明细”等页面跳转到工作台时，可携带 material_id 自动打开详情侧栏
  useEffect(() => {
    const materialId = searchParams.get('material_id');
    const id = String(materialId || '').trim();
    if (!id) return;
    setInspectedMaterialId(id);
    setInspectorOpen(true);
  }, [searchParams, setInspectedMaterialId, setInspectorOpen]);

  const [deepLinkContext, setDeepLinkContext] = useState<WorkbenchDeepLinkContext | null>(null);

  // 深链接：从风险概览跳转时，处理上下文参数（第三阶段）
  useEffect(() => {
    const machine = searchParams.get('machine');
    const date = searchParams.get('date');
    const urgency = searchParams.get('urgency');
    const context = searchParams.get('context');
    const focus = searchParams.get('focus');
    const openCell = searchParams.get('openCell');

    // 如果有深链接参数，保存到状态并应用
    if (machine || date || urgency || context || focus || openCell) {
      const openCellFlag = openCell === '1' || openCell === 'true';
      setDeepLinkContext({
        machine: machine || undefined,
        date: date || undefined,
        urgency: urgency || undefined,
        context: context || undefined,
        focus: focus || undefined,
        openCell: openCellFlag,
      });

      // 应用机组筛选
      if (machine) {
        setPoolSelection((prev) => {
          if (prev.machineCode === machine) return prev;
          return { machineCode: machine, schedState: null };
        });
        setWorkbenchFilters({ machineCode: machine });
      }

      // 应用紧急度筛选（扩展功能）
      if (urgency) {
        setWorkbenchFilters({ urgencyLevel: urgency });
      }

      // 深链接日期：默认聚焦前后各 3 天，并锁定范围，避免被自动范围覆盖
      if (date) {
        const focusDate = dayjs(date);
        if (focusDate.isValid()) {
          setWorkbenchDateRange([focusDate.subtract(3, 'day'), focusDate.add(3, 'day')]);
          setDateRangeMode('PINNED');
        }
      }

      // 深链接指定甘特图定位（风险日/瓶颈点等）
      if (focus === 'gantt' || openCellFlag) {
        setWorkbenchViewMode('GANTT');
      }

      // 显示来源提示
      const contextLabel = getDeepLinkContextLabel(String(context || '').trim());
      if (contextLabel) {
        const filterHints = [];
        if (machine) filterHints.push(`机组: ${machine}`);
        if (urgency) filterHints.push(`紧急度: ${urgency}`);
        if (date) filterHints.push(`日期: ${date}`);

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


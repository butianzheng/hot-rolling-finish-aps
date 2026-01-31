/**
 * 计划工作台 - 统一联动状态管理
 *
 * 功能：
 * - 集中管理物料池、产能概览、排程视图的联动状态
 * - 提供统一的 API 用于组件间通信
 * - 支持撤销/重做操作
 * - 提供调试信息输出
 *
 * 依赖：
 * - React Query for 数据缓存和同步
 * - Zustand for 全局状态（跨页面）
 * - Immer for 不可变更新（支持撤销/重做）
 */

import { useCallback, useEffect, useRef, useState, useMemo } from 'react';
import dayjs, { Dayjs } from 'dayjs';
import { useGlobalStore } from '@/store/global';
import { message } from 'antd';

// ===== 类型定义 =====

export interface WorkbenchSyncState {
  // 基础选择
  machineCode: string | null;           // 当前选中机组（或 'all'）
  selectedMaterialIds: string[];        // 当前选中的物料 ID 列表

  // 日期范围
  dateRange: [Dayjs, Dayjs];           // 当前显示的日期范围 [start, end]
  autoDateRange: boolean;               // 是否自动调整日期范围

  // 聚焦控制
  focusedMaterialId: string | null;    // 当前聚焦的物料 ID（用于视图滚动聚焦）
  focusedMachineCode: string | null;   // 当前聚焦的机组

  // 视图状态
  materialPoolExpanded: boolean;        // 物料池是否展开（移动设备）
  capacityViewMode: 'timeline' | 'histogram'; // 产能概览视图模式

  // 操作历史
  historyStack: WorkbenchSyncState[];   // 历史栈（用于撤销）
  futureStack: WorkbenchSyncState[];    // 未来栈（用于重做）

  // 调试模式
  debugMode: boolean;
}

export interface WorkbenchSyncAPI {
  // 机组选择
  selectMachine: (machineCode: string | null) => void;

  // 物料选择
  selectMaterial: (materialId: string, multiSelect?: boolean) => void;
  selectMaterials: (materialIds: string[], replace?: boolean) => void;
  clearSelection: () => void;
  toggleMaterialSelection: (materialId: string) => void;

  // 日期范围
  setDateRange: (range: [Dayjs, Dayjs]) => void;
  resetDateRangeToAuto: () => void;

  // 视图聚焦
  focusMaterial: (materialId: string, machineCode?: string) => Promise<void>;
  focusMachine: (machineCode: string) => void;
  clearFocus: () => void;

  // 视图切换
  toggleMaterialPoolExpand: () => void;
  setCapacityViewMode: (mode: 'timeline' | 'histogram') => void;

  // 历史操作
  undo: () => void;
  redo: () => void;
  canUndo: () => boolean;
  canRedo: () => boolean;

  // 调试
  toggleDebugMode: () => void;
  getDebugInfo: () => object;
  logStateChange: (action: string, fromState: WorkbenchSyncState, toState: WorkbenchSyncState) => void;
}

// ===== 初始状态 =====

const initialState: WorkbenchSyncState = {
  machineCode: null,
  selectedMaterialIds: [],
  dateRange: [dayjs().subtract(3, 'day'), dayjs().add(10, 'day')],
  autoDateRange: true,
  focusedMaterialId: null,
  focusedMachineCode: null,
  materialPoolExpanded: true,
  capacityViewMode: 'timeline',
  historyStack: [],
  futureStack: [],
  debugMode: process.env.NODE_ENV === 'development',
};

// ===== Hook 实现 =====

export function useWorkbenchSync(): [WorkbenchSyncState, WorkbenchSyncAPI] {
  // 状态
  const [state, setState] = useState<WorkbenchSyncState>(initialState);
  const stateRef = useRef<WorkbenchSyncState>(state);

  // 全局存储（用于跨页面同步）
  const { workbenchFilters, setWorkbenchFilters } = useGlobalStore();

  // 同步全局状态变化到本地状态
  useEffect(() => {
    if (workbenchFilters.machineCode !== state.machineCode) {
      setState(prev => ({
        ...prev,
        machineCode: workbenchFilters.machineCode,
        selectedMaterialIds: [], // 机组变化时清空选择
      }));
    }
  }, [workbenchFilters.machineCode]);

  // 保持 ref 与 state 同步
  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  // ===== 核心状态更新函数 =====

  const updateState = useCallback(
    (updater: (prev: WorkbenchSyncState) => WorkbenchSyncState, action?: string) => {
      setState(prev => {
        const next = updater(prev);

        // 调试日志
        if (state.debugMode && action) {
          console.log(`[WorkbenchSync] ${action}`, {
            from: prev,
            to: next,
            timestamp: new Date().toISOString(),
          });
        }

        return next;
      });
    },
    [state.debugMode]
  );

  const pushHistory = useCallback((currentState: WorkbenchSyncState) => {
    setState(prev => ({
      ...prev,
      historyStack: [...prev.historyStack, prev],
      futureStack: [], // 清空重做栈
    }));
  }, []);

  // ===== API 实现 =====

  const api: WorkbenchSyncAPI = {
    selectMachine: useCallback((machineCode: string | null) => {
      pushHistory(state);

      setWorkbenchFilters({ machineCode });

      updateState(
        prev => ({
          ...prev,
          machineCode,
          selectedMaterialIds: [],
          focusedMachineCode: machineCode,
        }),
        `selectMachine(${machineCode})`
      );
    }, [state, pushHistory, updateState, setWorkbenchFilters]),

    selectMaterial: useCallback(
      (materialId: string, multiSelect: boolean = false) => {
        pushHistory(state);

        updateState(
          prev => ({
            ...prev,
            selectedMaterialIds: multiSelect
              ? [...prev.selectedMaterialIds, materialId]
              : [materialId],
            focusedMaterialId: materialId,
          }),
          `selectMaterial(${materialId}, multiSelect=${multiSelect})`
        );
      },
      [state, pushHistory, updateState]
    ),

    selectMaterials: useCallback(
      (materialIds: string[], replace: boolean = true) => {
        pushHistory(state);

        updateState(
          prev => ({
            ...prev,
            selectedMaterialIds: replace
              ? materialIds
              : [...new Set([...prev.selectedMaterialIds, ...materialIds])],
          }),
          `selectMaterials([...${materialIds.length}], replace=${replace})`
        );
      },
      [state, pushHistory, updateState]
    ),

    clearSelection: useCallback(() => {
      pushHistory(state);

      updateState(
        prev => ({
          ...prev,
          selectedMaterialIds: [],
          focusedMaterialId: null,
        }),
        'clearSelection()'
      );
    }, [state, pushHistory, updateState]),

    toggleMaterialSelection: useCallback((materialId: string) => {
      pushHistory(state);

      updateState(
        prev => ({
          ...prev,
          selectedMaterialIds: prev.selectedMaterialIds.includes(materialId)
            ? prev.selectedMaterialIds.filter(id => id !== materialId)
            : [...prev.selectedMaterialIds, materialId],
        }),
        `toggleMaterialSelection(${materialId})`
      );
    }, [state, pushHistory, updateState]),

    setDateRange: useCallback((range: [Dayjs, Dayjs]) => {
      pushHistory(state);

      updateState(
        prev => ({
          ...prev,
          dateRange: range,
          autoDateRange: false, // 手动设置日期范围时关闭自动调整
        }),
        `setDateRange([${range[0].format('YYYY-MM-DD')}, ${range[1].format('YYYY-MM-DD')}])`
      );
    }, [state, pushHistory, updateState]),

    resetDateRangeToAuto: useCallback(() => {
      pushHistory(state);

      updateState(
        prev => ({
          ...prev,
          autoDateRange: true,
        }),
        'resetDateRangeToAuto()'
      );
    }, [state, pushHistory, updateState]),

    focusMaterial: useCallback(
      async (materialId: string, machineCode?: string) => {
        pushHistory(state);

        updateState(
          prev => ({
            ...prev,
            focusedMaterialId: materialId,
            ...(machineCode && { focusedMachineCode: machineCode }),
          }),
          `focusMaterial(${materialId}, machine=${machineCode})`
        );

        // 触发滚动到视图事件（由父组件处理）
        const event = new CustomEvent('workbench-focus-material', {
          detail: { materialId, machineCode },
        });
        window.dispatchEvent(event);

        // 给组件一些时间来完成滚动
        await new Promise(resolve => setTimeout(resolve, 300));
      },
      [state, pushHistory, updateState]
    ),

    focusMachine: useCallback((machineCode: string) => {
      pushHistory(state);

      updateState(
        prev => ({
          ...prev,
          focusedMachineCode: machineCode,
        }),
        `focusMachine(${machineCode})`
      );
    }, [state, pushHistory, updateState]),

    clearFocus: useCallback(() => {
      pushHistory(state);

      updateState(
        prev => ({
          ...prev,
          focusedMaterialId: null,
          focusedMachineCode: null,
        }),
        'clearFocus()'
      );
    }, [state, pushHistory, updateState]),

    toggleMaterialPoolExpand: useCallback(() => {
      updateState(
        prev => ({
          ...prev,
          materialPoolExpanded: !prev.materialPoolExpanded,
        }),
        `toggleMaterialPoolExpand(${!state.materialPoolExpanded})`
      );
    }, [state, updateState]),

    setCapacityViewMode: useCallback((mode: 'timeline' | 'histogram') => {
      updateState(
        prev => ({
          ...prev,
          capacityViewMode: mode,
        }),
        `setCapacityViewMode(${mode})`
      );
    }, [updateState]),

    undo: useCallback(() => {
      setState(prev => {
        if (prev.historyStack.length === 0) {
          if (prev.debugMode) console.warn('[WorkbenchSync] 无法撤销：历史栈为空');
          return prev;
        }

        const [popped, ...remaining] = prev.historyStack.reverse();
        return {
          ...popped,
          futureStack: [...prev.futureStack, prev],
          historyStack: remaining.reverse(),
          debugMode: prev.debugMode,
        };
      });
    }, []),

    redo: useCallback(() => {
      setState(prev => {
        if (prev.futureStack.length === 0) {
          if (prev.debugMode) console.warn('[WorkbenchSync] 无法重做：未来栈为空');
          return prev;
        }

        const [popped, ...remaining] = prev.futureStack.reverse();
        return {
          ...popped,
          historyStack: [...prev.historyStack, prev],
          futureStack: remaining.reverse(),
          debugMode: prev.debugMode,
        };
      });
    }, []),

    canUndo: useCallback(() => state.historyStack.length > 0, [state.historyStack.length]),

    canRedo: useCallback(() => state.futureStack.length > 0, [state.futureStack.length]),

    toggleDebugMode: useCallback(() => {
      setState(prev => ({
        ...prev,
        debugMode: !prev.debugMode,
      }));
    }, []),

    getDebugInfo: useCallback(() => {
      return {
        state,
        timestamp: new Date().toISOString(),
        historyDepth: state.historyStack.length,
        futureDepth: state.futureStack.length,
      };
    }, [state]),

    logStateChange: useCallback((action: string, fromState: WorkbenchSyncState, toState: WorkbenchSyncState) => {
      if (state.debugMode) {
        console.group(`[WorkbenchSync] ${action}`);
        console.log('Previous State:', fromState);
        console.log('Current State:', toState);
        console.log('Changes:', {
          machineCode: fromState.machineCode !== toState.machineCode ? `${fromState.machineCode} → ${toState.machineCode}` : '-',
          selectedMaterials: fromState.selectedMaterialIds.length !== toState.selectedMaterialIds.length ? `${fromState.selectedMaterialIds.length} → ${toState.selectedMaterialIds.length}` : '-',
          dateRange: `${fromState.dateRange[0].format('YYYY-MM-DD')} ~ ${fromState.dateRange[1].format('YYYY-MM-DD')}`,
          focusedMaterial: fromState.focusedMaterialId !== toState.focusedMaterialId ? `${fromState.focusedMaterialId} → ${toState.focusedMaterialId}` : '-',
        });
        console.groupEnd();
      }
    }, [state.debugMode]),
  };

  return [state, api];
}

// ===== 辅助 Hook：监听聚焦事件 =====

export function useWorkbenchFocusListener(
  onFocusMaterial?: (materialId: string, machineCode?: string) => void
) {
  useEffect(() => {
    const handler = (event: Event) => {
      const customEvent = event as CustomEvent;
      const { materialId, machineCode } = customEvent.detail;
      onFocusMaterial?.(materialId, machineCode);
    };

    window.addEventListener('workbench-focus-material', handler);
    return () => window.removeEventListener('workbench-focus-material', handler);
  }, [onFocusMaterial]);
}

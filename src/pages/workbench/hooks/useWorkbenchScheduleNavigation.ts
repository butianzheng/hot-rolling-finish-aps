import { useCallback, useMemo, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import dayjs from 'dayjs';

import type { WorkbenchViewMode } from '../../../stores/use-global-store';
import { formatDate } from '../../../utils/formatters';
import type {
  WorkbenchDeepLinkContext,
  WorkbenchGanttAutoOpenCell,
  WorkbenchMatrixFocusRequest,
  WorkbenchScheduleFocus,
} from '../types';

export type { WorkbenchMatrixFocusRequest, WorkbenchScheduleFocus } from '../types';

export function useWorkbenchScheduleNavigation(params: {
  deepLinkContext: WorkbenchDeepLinkContext | null;
  poolMachineCode: string | null;
  setWorkbenchViewMode: (mode: WorkbenchViewMode) => void;
}): {
  scheduleFocus: WorkbenchScheduleFocus | null;
  setScheduleFocus: Dispatch<SetStateAction<WorkbenchScheduleFocus | null>>;
  matrixFocusRequest: WorkbenchMatrixFocusRequest | null;
  focusedDate: string | null;
  autoOpenCell: WorkbenchGanttAutoOpenCell | null;
  openGanttCellDetail: (machine: string, date: string, source: string) => void;
  navigateToMatrix: (machine: string, date: string) => void;
} {
  const { deepLinkContext, poolMachineCode, setWorkbenchViewMode } = params;

  const [scheduleFocus, setScheduleFocus] = useState<WorkbenchScheduleFocus | null>(null);
  const [matrixFocusRequest, setMatrixFocusRequest] = useState<WorkbenchMatrixFocusRequest | null>(null);

  const focusedDate = useMemo(() => {
    const d = String(deepLinkContext?.date || '').trim();
    return d || null;
  }, [deepLinkContext?.date]);

  const deepLinkAutoOpenCell = useMemo<WorkbenchGanttAutoOpenCell | null>(() => {
    if (!deepLinkContext?.openCell) return null;
    const machine = String(deepLinkContext.machine || poolMachineCode || '').trim();
    const date = String(deepLinkContext.date || '').trim();
    if (!machine || !date) return null;
    return { machine, date };
  }, [deepLinkContext?.date, deepLinkContext?.machine, deepLinkContext?.openCell, poolMachineCode]);

  const deepLinkMatrixFocus = useMemo<WorkbenchMatrixFocusRequest | null>(() => {
    const materialId = String(deepLinkContext?.materialId || '').trim();
    const contractNo = String(deepLinkContext?.contractNo || '').trim();
    const context = String(deepLinkContext?.context || '').trim();
    const machine = String(deepLinkContext?.machine || '').trim();
    const dateRaw = String(deepLinkContext?.date || '').trim();
    const date = dayjs(dateRaw).isValid() ? dateRaw : undefined;

    // D2/风险问题跳转：优先展示“对应材料清单”（合同搜索），不直接进入单材料定位态。
    if (context === 'orders' && contractNo) {
      return {
        machine: machine || undefined,
        date,
        searchText: contractNo,
        contractNo,
        materialId: materialId || undefined,
        mode: 'SEARCH',
        nonce: `${Date.now()}-${contractNo}`,
      };
    }

    if (materialId) {
      return {
        machine: machine || undefined,
        date,
        materialId,
        contractNo: contractNo || undefined,
        mode: 'MATERIAL_LOCATE',
        nonce: `${Date.now()}-${materialId}`,
      };
    }

    if (!contractNo) return null;

    return {
      machine: machine || undefined,
      date,
      searchText: contractNo,
      contractNo,
      mode: 'SEARCH',
      nonce: `${Date.now()}-${contractNo}`,
    };
  }, [deepLinkContext?.context, deepLinkContext?.contractNo, deepLinkContext?.date, deepLinkContext?.machine, deepLinkContext?.materialId]);

  const [calendarOpenCellRequest, setCalendarOpenCellRequest] = useState<WorkbenchGanttAutoOpenCell | null>(null);
  const autoOpenCell = calendarOpenCellRequest || deepLinkAutoOpenCell;

  const openGanttCellDetail = useCallback(
    (machine: string, date: string, source: string) => {
      const machineCode = String(machine || '').trim();
      const d = dayjs(date);
      if (!machineCode || !d.isValid()) return;
      const dateKey = formatDate(d);
      setWorkbenchViewMode('CALENDAR');
      setCalendarOpenCellRequest({ machine: machineCode, date: dateKey, nonce: Date.now(), source });
      setScheduleFocus({ machine: machineCode, date: dateKey, source });
    },
    [setWorkbenchViewMode]
  );

  const navigateToMatrix = useCallback(
    (machine: string, date: string) => {
      const machineCode = String(machine || '').trim();
      const d = dayjs(date);
      if (!machineCode || !d.isValid()) return;
      const dateKey = formatDate(d);
      setWorkbenchViewMode('MATRIX');
      setMatrixFocusRequest({ machine: machineCode, date: dateKey, nonce: Date.now() });
      setScheduleFocus({ machine: machineCode, date: dateKey, source: 'matrixJump' });
    },
    [setWorkbenchViewMode]
  );

  return {
    scheduleFocus,
    setScheduleFocus,
    matrixFocusRequest: matrixFocusRequest || deepLinkMatrixFocus,
    focusedDate,
    autoOpenCell,
    openGanttCellDetail,
    navigateToMatrix,
  };
}

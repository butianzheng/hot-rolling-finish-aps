import React from 'react';
import dayjs from 'dayjs';

import DailyRhythmManagerModal from './DailyRhythmManagerModal';
import ConditionalSelectModal from './ConditionalSelectModal';
import MoveMaterialsModal from './MoveMaterialsModal';
import { MaterialInspector } from '../MaterialInspector';
import PathOverrideConfirmModal from '../path-override-confirm/PathOverrideConfirmModal';
import PathOverridePendingCenterModal from '../path-override-confirm/PathOverridePendingCenterModal';
import { formatDate } from '../../utils/formatters';
import type { MaterialPoolMaterial } from './MaterialPool';
import type { MoveModalState, MoveModalActions } from '../../pages/workbench/hooks/useWorkbenchMoveModal';
import type { WorkbenchModalState, WorkbenchModalKey } from '../../pages/workbench/hooks/useWorkbenchModalState';
import type {
  MaterialOperationType,
  WorkbenchPathOverrideState,
  WorkbenchScheduleFocus,
} from '../../pages/workbench/types';

/**
 * WorkbenchModals Props（Phase 2 重构）
 *
 * 原来：46 个散列 props
 * 重构后：20 个 props（2 个聚合对象 + 18 个独立 props）
 */
const WorkbenchModals: React.FC<{
  activeVersionId: string;
  currentUser: string;
  machineOptions: string[];
  poolMachineCode: string | null;
  scheduleFocus: WorkbenchScheduleFocus | null;

  /** 【新增】弹窗状态聚合对象（4 个弹窗） */
  modals: WorkbenchModalState;
  /** 关闭弹窗的统一操作 */
  closeModal: (key: WorkbenchModalKey) => void;

  /** Path Override 状态（包含 context, refetch, recalc 等） */
  pathOverride: WorkbenchPathOverrideState;

  /** Conditional Select 相关 */
  materials: MaterialPoolMaterial[];
  selectedMaterialIds: string[];
  setSelectedMaterialIds: (ids: string[]) => void;
  runMaterialOperation: (materialIds: string[], type: MaterialOperationType) => void;
  runForceReleaseOperation: (materialIds: string[]) => void;

  /** 【新增】Move Modal 聚合对象 */
  moveModalState: MoveModalState;
  moveModalActions: MoveModalActions;
  /** 排程数据加载状态（来自 PlanningWorkbench，不在 MoveModalState 中） */
  planItemsLoading: boolean;

  /** Inspector 相关 */
  inspectorOpen: boolean;
  setInspectorOpen: (open: boolean) => void;
  inspectedMaterial: Parameters<typeof MaterialInspector>[0]['material'];
}> = ({
  activeVersionId,
  currentUser,
  machineOptions,
  poolMachineCode,
  scheduleFocus,

  modals,
  closeModal,

  pathOverride,

  materials,
  selectedMaterialIds,
  setSelectedMaterialIds,
  runMaterialOperation,
  runForceReleaseOperation,

  moveModalState,
  moveModalActions,
  planItemsLoading,

  inspectorOpen,
  setInspectorOpen,
  inspectedMaterial,
}) => {
  return (
    <>
      <DailyRhythmManagerModal
        open={modals.rhythm}
        onClose={() => closeModal('rhythm')}
        versionId={activeVersionId}
        machineOptions={machineOptions}
        defaultMachineCode={scheduleFocus?.machine || poolMachineCode || machineOptions[0] || null}
        defaultPlanDate={scheduleFocus?.date || formatDate(dayjs())}
        operator={currentUser || 'system'}
      />

      <PathOverrideConfirmModal
        open={modals.pathOverrideConfirm}
        onClose={() => closeModal('pathOverrideConfirm')}
        versionId={activeVersionId}
        machineCode={pathOverride.context.machineCode}
        planDate={pathOverride.context.planDate}
        operator={currentUser || 'system'}
        onConfirmed={async ({ confirmedCount, autoRecalc }) => {
          if (confirmedCount <= 0) return;
          pathOverride.pendingRefetch();
          pathOverride.summaryRefetch();
          if (autoRecalc) {
            closeModal('pathOverrideConfirm');
            await pathOverride.recalcAfterPathOverride(pathOverride.context.planDate || '');
          }
        }}
      />

      <PathOverridePendingCenterModal
        open={modals.pathOverrideCenter}
        onClose={() => closeModal('pathOverrideCenter')}
        versionId={activeVersionId}
        planDateFrom={pathOverride.summaryRange.from}
        planDateTo={pathOverride.summaryRange.to}
        machineOptions={machineOptions}
        operator={currentUser || 'system'}
        onConfirmed={async ({ confirmedCount, autoRecalc, recalcBaseDate }) => {
          if (confirmedCount <= 0) return;
          pathOverride.pendingRefetch();
          pathOverride.summaryRefetch();
          if (autoRecalc) {
            closeModal('pathOverrideCenter');
            await pathOverride.recalcAfterPathOverride(recalcBaseDate || '');
          }
        }}
      />

      <ConditionalSelectModal
        open={modals.conditionalSelect}
        onClose={() => closeModal('conditionalSelect')}
        defaultMachine={poolMachineCode || 'all'}
        machineOptions={machineOptions}
        materials={materials}
        selectedMaterialIds={selectedMaterialIds}
        onSelectedMaterialIdsChange={setSelectedMaterialIds}
        onMaterialOperation={runMaterialOperation}
        onForceReleaseOperation={runForceReleaseOperation}
      />

      <MoveMaterialsModal
        state={moveModalState}
        actions={moveModalActions}
        planItemsLoading={planItemsLoading}
        selectedMaterialIds={selectedMaterialIds}
        machineOptions={machineOptions}
      />

      <MaterialInspector
        visible={inspectorOpen}
        material={inspectedMaterial}
        onClose={() => setInspectorOpen(false)}
        onLock={(id) => runMaterialOperation([id], 'lock')}
        onUnlock={(id) => runMaterialOperation([id], 'unlock')}
        onSetUrgent={(id) => runMaterialOperation([id], 'urgent_on')}
        onClearUrgent={(id) => runMaterialOperation([id], 'urgent_off')}
      />
    </>
  );
};

export default React.memo(WorkbenchModals);

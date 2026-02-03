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
import type {
  MoveImpactPreview,
  MoveRecommendSummary,
  MoveSeqMode,
  MoveValidationMode,
  SelectedPlanItemStats,
} from '../../pages/workbench/types';
import type {
  MaterialOperationType,
  WorkbenchPathOverrideState,
  WorkbenchScheduleFocus,
} from '../../pages/workbench/types';

const WorkbenchModals: React.FC<{
  activeVersionId: string;
  currentUser: string;
  machineOptions: string[];
  poolMachineCode: string | null;
  scheduleFocus: WorkbenchScheduleFocus | null;

  rhythmModalOpen: boolean;
  setRhythmModalOpen: (open: boolean) => void;

  pathOverrideModalOpen: boolean;
  setPathOverrideModalOpen: (open: boolean) => void;
  pathOverrideCenterOpen: boolean;
  setPathOverrideCenterOpen: (open: boolean) => void;
  pathOverride: WorkbenchPathOverrideState;

  conditionalSelectOpen: boolean;
  setConditionalSelectOpen: (open: boolean) => void;
  materials: MaterialPoolMaterial[];
  selectedMaterialIds: string[];
  setSelectedMaterialIds: (ids: string[]) => void;
  runMaterialOperation: (materialIds: string[], type: MaterialOperationType) => void;
  runForceReleaseOperation: (materialIds: string[]) => void;

  moveModalOpen: boolean;
  setMoveModalOpen: (open: boolean) => void;
  submitMove: () => Promise<void>;
  moveSubmitting: boolean;
  planItemsLoading: boolean;
  selectedPlanItemStats: SelectedPlanItemStats;
  moveTargetMachine: string | null;
  setMoveTargetMachine: (v: string | null) => void;
  moveTargetDate: dayjs.Dayjs | null;
  setMoveTargetDate: (v: dayjs.Dayjs | null) => void;
  moveSeqMode: MoveSeqMode;
  setMoveSeqMode: (v: MoveSeqMode) => void;
  moveStartSeq: number;
  setMoveStartSeq: (v: number) => void;
  moveValidationMode: MoveValidationMode;
  setMoveValidationMode: (v: MoveValidationMode) => void;
  moveReason: string;
  setMoveReason: (v: string) => void;
  recommendMoveTarget: () => Promise<void>;
  moveRecommendLoading: boolean;
  moveRecommendSummary: MoveRecommendSummary | null;
  strategyLabel: string;
  moveImpactPreview: MoveImpactPreview | null;

  inspectorOpen: boolean;
  setInspectorOpen: (open: boolean) => void;
  inspectedMaterial: Parameters<typeof MaterialInspector>[0]['material'];
}> = ({
  activeVersionId,
  currentUser,
  machineOptions,
  poolMachineCode,
  scheduleFocus,

  rhythmModalOpen,
  setRhythmModalOpen,

  pathOverrideModalOpen,
  setPathOverrideModalOpen,
  pathOverrideCenterOpen,
  setPathOverrideCenterOpen,
  pathOverride,

  conditionalSelectOpen,
  setConditionalSelectOpen,
  materials,
  selectedMaterialIds,
  setSelectedMaterialIds,
  runMaterialOperation,
  runForceReleaseOperation,

  moveModalOpen,
  setMoveModalOpen,
  submitMove,
  moveSubmitting,
  planItemsLoading,
  selectedPlanItemStats,
  moveTargetMachine,
  setMoveTargetMachine,
  moveTargetDate,
  setMoveTargetDate,
  moveSeqMode,
  setMoveSeqMode,
  moveStartSeq,
  setMoveStartSeq,
  moveValidationMode,
  setMoveValidationMode,
  moveReason,
  setMoveReason,
  recommendMoveTarget,
  moveRecommendLoading,
  moveRecommendSummary,
  strategyLabel,
  moveImpactPreview,

  inspectorOpen,
  setInspectorOpen,
  inspectedMaterial,
}) => {
  return (
    <>
      <DailyRhythmManagerModal
        open={rhythmModalOpen}
        onClose={() => setRhythmModalOpen(false)}
        versionId={activeVersionId}
        machineOptions={machineOptions}
        defaultMachineCode={scheduleFocus?.machine || poolMachineCode || machineOptions[0] || null}
        defaultPlanDate={scheduleFocus?.date || formatDate(dayjs())}
        operator={currentUser || 'system'}
      />

      <PathOverrideConfirmModal
        open={pathOverrideModalOpen}
        onClose={() => setPathOverrideModalOpen(false)}
        versionId={activeVersionId}
        machineCode={pathOverride.context.machineCode}
        planDate={pathOverride.context.planDate}
        operator={currentUser || 'system'}
        onConfirmed={async ({ confirmedCount, autoRecalc }) => {
          if (confirmedCount <= 0) return;
          pathOverride.pendingRefetch();
          pathOverride.summaryRefetch();
          if (autoRecalc) {
            setPathOverrideModalOpen(false);
            await pathOverride.recalcAfterPathOverride(pathOverride.context.planDate || '');
          }
        }}
      />

      <PathOverridePendingCenterModal
        open={pathOverrideCenterOpen}
        onClose={() => setPathOverrideCenterOpen(false)}
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
            setPathOverrideCenterOpen(false);
            await pathOverride.recalcAfterPathOverride(recalcBaseDate || '');
          }
        }}
      />

      <ConditionalSelectModal
        open={conditionalSelectOpen}
        onClose={() => setConditionalSelectOpen(false)}
        defaultMachine={poolMachineCode || 'all'}
        machineOptions={machineOptions}
        materials={materials}
        selectedMaterialIds={selectedMaterialIds}
        onSelectedMaterialIdsChange={setSelectedMaterialIds}
        onMaterialOperation={runMaterialOperation}
        onForceReleaseOperation={runForceReleaseOperation}
      />

      <MoveMaterialsModal
        open={moveModalOpen}
        onClose={() => setMoveModalOpen(false)}
        onSubmit={submitMove}
        submitting={moveSubmitting}
        planItemsLoading={planItemsLoading}
        selectedMaterialIds={selectedMaterialIds}
        machineOptions={machineOptions}
        selectedPlanItemStats={selectedPlanItemStats}
        moveTargetMachine={moveTargetMachine}
        setMoveTargetMachine={setMoveTargetMachine}
        moveTargetDate={moveTargetDate}
        setMoveTargetDate={setMoveTargetDate}
        moveSeqMode={moveSeqMode}
        setMoveSeqMode={setMoveSeqMode}
        moveStartSeq={moveStartSeq}
        setMoveStartSeq={setMoveStartSeq}
        moveValidationMode={moveValidationMode}
        setMoveValidationMode={setMoveValidationMode}
        moveReason={moveReason}
        setMoveReason={setMoveReason}
        recommendMoveTarget={() => void recommendMoveTarget()}
        moveRecommendLoading={moveRecommendLoading}
        moveRecommendSummary={moveRecommendSummary}
        strategyLabel={strategyLabel}
        moveImpactPreview={moveImpactPreview}
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

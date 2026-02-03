import React, { useCallback, useMemo, useState } from 'react';
import { Space, Tag, message } from 'antd';
import { useNavigate, useSearchParams } from 'react-router-dom';
import dayjs from 'dayjs';
import ErrorBoundary from '../components/ErrorBoundary';
import NoActiveVersionGuide from '../components/NoActiveVersionGuide';
import {
  useActiveVersionId,
  useAdminOverrideMode,
  useCurrentUser,
  useGlobalActions,
  useGlobalStore,
  useUserPreferences,
} from '../stores/use-global-store';
import type { PlanItemStatusFilter } from '../utils/planItemStatus';
import type { MaterialPoolFilters, MaterialPoolSelection } from '../components/workbench/MaterialPool';
import WorkbenchTopToolbar from '../components/workbench/WorkbenchTopToolbar';
import WorkbenchStatusBar from '../components/workbench/WorkbenchStatusBar';
import DecisionFlowGuide from '../components/flow/DecisionFlowGuide';
import WorkbenchAlerts from '../components/workbench/WorkbenchAlerts';
import WorkbenchMainLayout from '../components/workbench/WorkbenchMainLayout';
import WorkbenchModals from '../components/workbench/WorkbenchModals';
import { useWorkbenchAutoDateRange } from './workbench/hooks/useWorkbenchAutoDateRange';
import { useWorkbenchDeepLink } from './workbench/hooks/useWorkbenchDeepLink';
import { useWorkbenchBatchOperations } from './workbench/hooks/useWorkbenchBatchOperations';
import { useWorkbenchInspector } from './workbench/hooks/useWorkbenchInspector';
import { useWorkbenchMaterials } from './workbench/hooks/useWorkbenchMaterials';
import { useWorkbenchMoveModal } from './workbench/hooks/useWorkbenchMoveModal';
import { useWorkbenchPlanItems } from './workbench/hooks/useWorkbenchPlanItems';
import { useWorkbenchPathOverride } from './workbench/hooks/useWorkbenchPathOverride';
import { useWorkbenchDerivedProps } from './workbench/hooks/useWorkbenchDerivedProps';
import { useWorkbenchRefreshActions } from './workbench/hooks/useWorkbenchRefreshActions';
import { useWorkbenchScheduleNavigation } from './workbench/hooks/useWorkbenchScheduleNavigation';
import { useWorkbenchModalState } from './workbench/hooks/useWorkbenchModalState';
import type { WorkbenchDateRangeMode } from './workbench/types';

const PlanningWorkbench: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const activeVersionId = useActiveVersionId();
  const currentUser = useCurrentUser();
  const adminOverrideMode = useAdminOverrideMode();
  const workbenchViewMode = useGlobalStore((state) => state.workbenchViewMode);
  const workbenchFilters = useGlobalStore((state) => state.workbenchFilters);
  const preferences = useUserPreferences();
  const { setRecalculating, setActiveVersion, setWorkbenchViewMode, setWorkbenchFilters } = useGlobalActions();

  // 【Phase 2 重构】使用 useWorkbenchModalState 聚合 4 个弹窗状态
  const { modals, openModal, closeModal } = useWorkbenchModalState();

  const [poolSelection, setPoolSelection] = useState<MaterialPoolSelection>(() => ({
    machineCode: workbenchFilters.machineCode,
    schedState: null,
  }));
  const [selectedMaterialIds, setSelectedMaterialIds] = useState<string[]>([]);
  const [scheduleStatusFilter, setScheduleStatusFilter] = useState<PlanItemStatusFilter>('ALL');

  const [dateRangeMode, setDateRangeMode] = useState<WorkbenchDateRangeMode>(() => {
    const d = searchParams.get('date');
    const focusDate = d ? dayjs(d) : null;
    return focusDate && focusDate.isValid() ? 'PINNED' : 'AUTO';
  });
  const [workbenchDateRange, setWorkbenchDateRange] = useState<[dayjs.Dayjs, dayjs.Dayjs]>(() => {
    const d = searchParams.get('date');
    const focusDate = d ? dayjs(d) : null;
    if (focusDate && focusDate.isValid()) {
      return [focusDate.subtract(3, 'day'), focusDate.add(3, 'day')];
    }
    return [dayjs().subtract(3, 'day'), dayjs().add(10, 'day')];
  });

  const { materialsQuery, materials } = useWorkbenchMaterials({ machineCode: poolSelection.machineCode });

  const openRhythmModal = useCallback(() => openModal('rhythm'), [openModal]);
  const openConditionalSelect = useCallback(() => openModal('conditionalSelect'), [openModal]);
  const clearSelection = useCallback(() => setSelectedMaterialIds([]), []);
  const openPathOverrideConfirm = useCallback(() => openModal('pathOverrideConfirm'), [openModal]);
  const openPathOverrideCenter = useCallback(() => openModal('pathOverrideCenter'), [openModal]);
  const navigateToImport = useCallback(() => navigate('/import'), [navigate]);
  const navigateToComparison = useCallback(() => navigate('/comparison'), [navigate]);
  const navigateToDraftComparison = useCallback(() => navigate('/comparison?tab=draft'), [navigate]);
  const navigateToOverview = useCallback(() => navigate('/overview'), [navigate]);

  const { inspectorOpen, setInspectorOpen, setInspectedMaterialId, inspectedMaterial, openInspector } =
    useWorkbenchInspector({ materials });

  const { deepLinkContext, deepLinkContextLabel } = useWorkbenchDeepLink({
    searchParams,
    globalMachineCode: workbenchFilters.machineCode,
    setPoolSelection,
    setWorkbenchFilters,
    setWorkbenchViewMode,
    setDateRangeMode,
    setWorkbenchDateRange,
    setInspectorOpen,
    setInspectedMaterialId,
  });

  const {
    scheduleFocus,
    setScheduleFocus,
    matrixFocusRequest,
    focusedDate: ganttFocusedDate,
    autoOpenCell: ganttAutoOpenCell,
    openGanttCellDetail,
    navigateToMatrix,
  } = useWorkbenchScheduleNavigation({
    deepLinkContext,
    poolMachineCode: poolSelection.machineCode,
    setWorkbenchViewMode,
  });

  const { planItemsQuery, planItems } = useWorkbenchPlanItems({ activeVersionId });

  const { refreshAll, refreshPlanItems, refreshMaterials } = useWorkbenchRefreshActions();

  // AUTO 日期范围（基于当前机组的排程数据）
  const { autoDateRange, applyWorkbenchDateRange, resetWorkbenchDateRangeToAuto } = useWorkbenchAutoDateRange({
    planItems,
    machineCode: poolSelection.machineCode,
    dateRangeMode,
    setDateRangeMode,
    setWorkbenchDateRange,
  });
  const pathOverride = useWorkbenchPathOverride({
    activeVersionId,
    scheduleFocus,
    poolMachineCode: poolSelection.machineCode,
    autoDateRange,
    currentUser,
    defaultStrategy: preferences.defaultStrategy,
    setRecalculating,
    setActiveVersion,
  });

  const applyWorkbenchMachineCode = useCallback((machineCode: string | null) => {
    setPoolSelection((prev) => {
      if (prev.machineCode === machineCode) return prev;
      return { machineCode, schedState: null };
    });
    setWorkbenchFilters({ machineCode });
  }, [setWorkbenchFilters]);

  const { poolFilters, selectedTotalWeight, machineOptions } = useWorkbenchDerivedProps({
    materials,
    selectedMaterialIds,
    urgencyLevel: workbenchFilters.urgencyLevel,
    lockStatus: workbenchFilters.lockStatus,
  });

  const handlePoolSelectionChange = useCallback(
    (next: MaterialPoolSelection) => {
      setPoolSelection(next);
      setWorkbenchFilters({ machineCode: next.machineCode });
    },
    [setWorkbenchFilters]
  );

  const handlePoolFiltersChange = useCallback(
    (next: Partial<MaterialPoolFilters>) => {
      setWorkbenchFilters(next);
    },
    [setWorkbenchFilters]
  );

  const handleAfterRollCycleReset = useCallback(() => {
    void refreshAll();
    message.info('已重置换辊周期：建议执行"一键优化/重算"以刷新排程结果');
  }, [refreshAll]);

  const handleBeforeOptimize = useCallback(() => setRecalculating(true), [setRecalculating]);
  const handleAfterOptimize = useCallback(() => {
    setRecalculating(false);
    void refreshAll();
  }, [refreshAll, setRecalculating]);

  const {
    moveModalState,
    moveModalActions,
    openMoveModal,
    openMoveModalAt,
    openMoveModalWithRecommend,
  } = useWorkbenchMoveModal({
    activeVersionId,
    operator: currentUser || 'admin',
    deepLinkDate: deepLinkContext?.date || null,
    poolMachineCode: poolSelection.machineCode,
    machineOptions,
    defaultStrategy: preferences.defaultStrategy,
    workbenchDateRange,
    planItems,
    selectedMaterialIds,
    setSelectedMaterialIds,
  });

  const { runMaterialOperation, runForceReleaseOperation } = useWorkbenchBatchOperations({
    adminOverrideMode,
    currentUser,
    materials,
    setSelectedMaterialIds,
  });

  const statusBarHandlers = useMemo(
    () => ({
      onLock: () => runMaterialOperation(selectedMaterialIds, 'lock'),
      onUnlock: () => runMaterialOperation(selectedMaterialIds, 'unlock'),
      onSetUrgent: () => runMaterialOperation(selectedMaterialIds, 'urgent_on'),
      onClearUrgent: () => runMaterialOperation(selectedMaterialIds, 'urgent_off'),
      onForceRelease: () => runForceReleaseOperation(selectedMaterialIds),
    }),
    [runForceReleaseOperation, runMaterialOperation, selectedMaterialIds]
  );

  const decisionPrimaryAction = useMemo(
    () => ({
      label: '去策略草案对比',
      onClick: navigateToDraftComparison,
    }),
    [navigateToDraftComparison]
  );
  const decisionSecondaryAction = useMemo(
    () => ({
      label: '回风险概览',
      onClick: navigateToOverview,
    }),
    [navigateToOverview]
  );

  if (!activeVersionId) {
    return (
      <NoActiveVersionGuide
        title="尚无激活的排产版本"
        description="计划工作台需要一个激活的排产版本作为基础"
        onNavigateToImport={navigateToImport}
        onNavigateToPlan={navigateToComparison}
      />
    );
  }

  return (
    <ErrorBoundary>
      <div style={{ height: '100%', display: 'flex', flexDirection: 'column', gap: 12 }}>
        <DecisionFlowGuide
          stage="workbench"
          title="下一步：去策略草案对比生成多方案预览"
          tags={
            <Space wrap size={6}>
              {workbenchFilters.machineCode ? <Tag color="blue">机组 {workbenchFilters.machineCode}</Tag> : null}
              {workbenchFilters.urgencyLevel ? <Tag color="volcano">紧急 {workbenchFilters.urgencyLevel}</Tag> : null}
              {workbenchFilters.lockStatus && workbenchFilters.lockStatus !== 'ALL' ? (
                <Tag color="geekblue">
                  {workbenchFilters.lockStatus === 'LOCKED' ? '仅锁定' : '仅未锁定'}
                </Tag>
              ) : null}
            </Space>
          }
          description="建议先在工作台处理 P0/P1 物料（移位/锁定/紧急/强制放行），再去草案对比选择推荐方案发布并激活。"
          primaryAction={decisionPrimaryAction}
          secondaryAction={decisionSecondaryAction}
        />

        {/* 工具栏 */}
        <WorkbenchTopToolbar
          activeVersionId={activeVersionId}
          currentUser={currentUser}
          selectedMaterialIds={selectedMaterialIds}
          onRefresh={refreshAll}
          onOpenRhythmModal={openRhythmModal}
          onOpenConditionalSelect={openConditionalSelect}
          onClearSelection={clearSelection}
          openMoveModal={openMoveModal}
          runMaterialOperation={runMaterialOperation}
          runForceReleaseOperation={runForceReleaseOperation}
          onBeforeOptimize={handleBeforeOptimize}
          onAfterOptimize={handleAfterOptimize}
        />

        <WorkbenchAlerts
          activeVersionId={activeVersionId}
          pathOverride={pathOverride}
          onOpenPathOverrideCenter={openPathOverrideCenter}
          onOpenPathOverrideConfirm={openPathOverrideConfirm}
          materialsIsLoading={materialsQuery.isLoading}
          materialsError={materialsQuery.error}
          materialsCount={materials.length}
          planItemsIsLoading={planItemsQuery.isLoading}
          planItemsError={planItemsQuery.error}
          planItemsData={planItemsQuery.data}
        />

        <WorkbenchMainLayout
          activeVersionId={activeVersionId}
          currentUser={currentUser}
          machineOptions={machineOptions}
          deepLinkDate={deepLinkContext?.date || null}
          deepLinkMachine={deepLinkContext?.machine || null}
          deepLinkLabel={deepLinkContextLabel || null}
          showResetAutoRange={dateRangeMode !== 'AUTO'}
          onResetDateRangeToAuto={resetWorkbenchDateRangeToAuto}
          materials={materials}
          materialsLoading={materialsQuery.isLoading}
          materialsError={materialsQuery.error}
          onRetryMaterials={refreshMaterials}
          poolSelection={poolSelection}
          onPoolSelectionChange={handlePoolSelectionChange}
          poolFilters={poolFilters}
          onPoolFiltersChange={handlePoolFiltersChange}
          selectedMaterialIds={selectedMaterialIds}
          onSelectedMaterialIdsChange={setSelectedMaterialIds}
          onInspectMaterialId={openInspector}
          onAfterRollCycleReset={handleAfterRollCycleReset}
          workbenchDateRange={workbenchDateRange}
          autoDateRange={autoDateRange}
          onMachineCodeChange={applyWorkbenchMachineCode}
          onDateRangeChange={applyWorkbenchDateRange}
          viewMode={workbenchViewMode}
          onViewModeChange={setWorkbenchViewMode}
          scheduleFocus={scheduleFocus}
          setScheduleFocus={setScheduleFocus}
          matrixFocusRequest={matrixFocusRequest}
          scheduleStatusFilter={scheduleStatusFilter}
          setScheduleStatusFilter={setScheduleStatusFilter}
          focusedDate={ganttFocusedDate}
          autoOpenCell={ganttAutoOpenCell}
          openGanttCellDetail={openGanttCellDetail}
          navigateToMatrix={navigateToMatrix}
          pathOverridePendingCount={pathOverride.pendingCount}
          pathOverrideContextMachineCode={pathOverride.context.machineCode}
          pathOverrideIsFetching={pathOverride.pendingIsFetching}
          onOpenPathOverrideModal={openPathOverrideConfirm}
          planItemsData={planItemsQuery.data}
          planItemsLoading={planItemsQuery.isLoading}
          planItemsError={planItemsQuery.error}
          onRetryPlanItems={refreshPlanItems}
          onRequestMoveToCell={openMoveModalAt}
        />

        {/* 状态栏 */}
        <WorkbenchStatusBar
          selectedMaterialCount={selectedMaterialIds.length}
          selectedTotalWeight={selectedTotalWeight}
          disabled={selectedMaterialIds.length === 0}
          onLock={statusBarHandlers.onLock}
          onUnlock={statusBarHandlers.onUnlock}
          onSetUrgent={statusBarHandlers.onSetUrgent}
          onClearUrgent={statusBarHandlers.onClearUrgent}
          onForceRelease={statusBarHandlers.onForceRelease}
          onOpenMoveRecommend={openMoveModalWithRecommend}
          onOpenMoveModal={openMoveModal}
          onClearSelection={clearSelection}
        />

        <WorkbenchModals
          activeVersionId={activeVersionId}
          currentUser={currentUser}
          machineOptions={machineOptions}
          poolMachineCode={poolSelection.machineCode}
          scheduleFocus={scheduleFocus}
          modals={modals}
          closeModal={closeModal}
          pathOverride={pathOverride}
          materials={materials}
          selectedMaterialIds={selectedMaterialIds}
          setSelectedMaterialIds={setSelectedMaterialIds}
          runMaterialOperation={runMaterialOperation}
          runForceReleaseOperation={runForceReleaseOperation}
          moveModalState={moveModalState}
          moveModalActions={moveModalActions}
          planItemsLoading={planItemsQuery.isLoading}
          inspectorOpen={inspectorOpen}
          setInspectorOpen={setInspectorOpen}
          inspectedMaterial={inspectedMaterial}
        />
      </div>
    </ErrorBoundary>
  );
};

export default PlanningWorkbench;

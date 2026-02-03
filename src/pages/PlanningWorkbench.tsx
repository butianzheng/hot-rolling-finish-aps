import React, { useMemo, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  Space,
  Tag,
  message,
} from 'antd';
import { InfoCircleOutlined, SettingOutlined } from '@ant-design/icons';
import { useNavigate, useSearchParams } from 'react-router-dom';
import dayjs from 'dayjs';
import ErrorBoundary from '../components/ErrorBoundary';
import PageSkeleton from '../components/PageSkeleton';
import NoActiveVersionGuide from '../components/NoActiveVersionGuide';
import {
  useActiveVersionId,
  useAdminOverrideMode,
  useCurrentUser,
  useGlobalActions,
  useGlobalStore,
  useUserPreferences,
} from '../stores/use-global-store';
import { formatDate } from '../utils/formatters';
import type { PlanItemStatusFilter } from '../utils/planItemStatus';
import MaterialPool, { type MaterialPoolSelection } from '../components/workbench/MaterialPool';
import ScheduleCardView from '../components/workbench/ScheduleCardView';
import ScheduleGanttView from '../components/workbench/ScheduleGanttView';
import DailyRhythmManagerModal from '../components/workbench/DailyRhythmManagerModal';
import ConditionalSelectModal from '../components/workbench/ConditionalSelectModal';
import WorkbenchScheduleViewToolbar from '../components/workbench/WorkbenchScheduleViewToolbar';
import WorkbenchTopToolbar from '../components/workbench/WorkbenchTopToolbar';
import WorkbenchStatusBar from '../components/workbench/WorkbenchStatusBar';
import PathOverrideConfirmModal from '../components/path-override-confirm/PathOverrideConfirmModal';
import PathOverridePendingCenterModal from '../components/path-override-confirm/PathOverridePendingCenterModal';
import RollCycleAnchorCard from '../components/roll-cycle-anchor/RollCycleAnchorCard';
import { CapacityTimelineContainer } from '../components/CapacityTimelineContainer';
import { MaterialInspector } from '../components/MaterialInspector';
import DecisionFlowGuide from '../components/flow/DecisionFlowGuide';
import { useWorkbenchAutoDateRange } from './workbench/hooks/useWorkbenchAutoDateRange';
import { useWorkbenchDeepLink } from './workbench/hooks/useWorkbenchDeepLink';
import { useWorkbenchBatchOperations } from './workbench/hooks/useWorkbenchBatchOperations';
import { useWorkbenchInspector } from './workbench/hooks/useWorkbenchInspector';
import { useWorkbenchMaterials } from './workbench/hooks/useWorkbenchMaterials';
import { useWorkbenchMoveModal } from './workbench/hooks/useWorkbenchMoveModal';
import { useWorkbenchPlanItems } from './workbench/hooks/useWorkbenchPlanItems';
import { useWorkbenchPathOverride } from './workbench/hooks/useWorkbenchPathOverride';
import { useWorkbenchScheduleNavigation } from './workbench/hooks/useWorkbenchScheduleNavigation';
import MoveMaterialsModal from '../components/workbench/MoveMaterialsModal';
import type { WorkbenchDateRangeMode } from './workbench/types';

const PlanItemVisualization = React.lazy(() => import('../components/PlanItemVisualization'));

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
  const [refreshSignal, setRefreshSignal] = useState(0);
  const bumpRefreshSignal = React.useCallback(() => setRefreshSignal((v) => v + 1), []);

  const [pathOverrideModalOpen, setPathOverrideModalOpen] = useState(false);
  const [pathOverrideCenterOpen, setPathOverrideCenterOpen] = useState(false);

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

  const [conditionalSelectOpen, setConditionalSelectOpen] = useState(false);

  const [rhythmModalOpen, setRhythmModalOpen] = useState(false);

  const { materialsQuery, materials } = useWorkbenchMaterials({ machineCode: poolSelection.machineCode });

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

	  const { planItemsQuery, planItems } = useWorkbenchPlanItems({ activeVersionId, refreshSignal });

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
    refreshSignal,
    currentUser,
    defaultStrategy: preferences.defaultStrategy,
    setRecalculating,
    setActiveVersion,
    bumpRefreshSignal,
    materialsRefetch: materialsQuery.refetch,
  });

	  const applyWorkbenchMachineCode = (machineCode: string | null) => {
	    setPoolSelection((prev) => {
	      if (prev.machineCode === machineCode) return prev;
	      return { machineCode, schedState: null };
	    });
	    setWorkbenchFilters({ machineCode });
	  };

  const selectedMaterials = useMemo(() => {
    const set = new Set(selectedMaterialIds);
    return materials.filter((m) => set.has(m.material_id));
  }, [materials, selectedMaterialIds]);

  const selectedTotalWeight = useMemo(() => {
    return selectedMaterials.reduce((sum, m) => sum + Number(m.weight_t || 0), 0);
  }, [selectedMaterials]);

  const machineOptions = useMemo(() => {
    const codes = new Set<string>();
    materials.forEach((m) => {
      const code = String(m.machine_code || '').trim();
      if (code) codes.add(code);
    });
    return Array.from(codes).sort();
  }, [materials]);

  const {
    moveModalOpen,
    setMoveModalOpen,
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
    moveSubmitting,
    moveReason,
    setMoveReason,
    moveRecommendLoading,
    moveRecommendSummary,
    strategyLabel,
    selectedPlanItemStats,
    moveImpactPreview,
    recommendMoveTarget,
    openMoveModal,
    openMoveModalAt,
    openMoveModalWithRecommend,
    submitMove,
  } = useWorkbenchMoveModal({
    activeVersionId,
    operator: currentUser || 'admin',
    deepLinkDate: deepLinkContext?.date || null,
    poolMachineCode: poolSelection.machineCode,
    machineOptions,
	    defaultStrategy: preferences.defaultStrategy,
	    workbenchDateRange,
	    planItems,
	    planItemsRefetch: planItemsQuery.refetch,
	    selectedMaterialIds,
	    setSelectedMaterialIds,
	    bumpRefreshSignal,
    materialsRefetch: materialsQuery.refetch,
  });

  const { runMaterialOperation, runForceReleaseOperation } = useWorkbenchBatchOperations({
    adminOverrideMode,
    currentUser,
    materials,
    setSelectedMaterialIds,
    bumpRefreshSignal,
    materialsRefetch: materialsQuery.refetch,
    planItemsRefetch: planItemsQuery.refetch,
  });

  if (!activeVersionId) {
    return (
      <NoActiveVersionGuide
        title="尚无激活的排产版本"
        description="计划工作台需要一个激活的排产版本作为基础"
        onNavigateToImport={() => navigate('/import')}
        onNavigateToPlan={() => navigate('/comparison')}
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
          primaryAction={{
            label: '去策略草案对比',
            onClick: () => navigate('/comparison?tab=draft'),
          }}
          secondaryAction={{
            label: '回风险概览',
            onClick: () => navigate('/overview'),
          }}
        />

	        {/* 工具栏 */}
	        <WorkbenchTopToolbar
	          activeVersionId={activeVersionId}
	          currentUser={currentUser}
	          selectedMaterialIds={selectedMaterialIds}
	          onRefresh={() => {
	            setRefreshSignal((v) => v + 1);
	            materialsQuery.refetch();
	          }}
	          onOpenRhythmModal={() => setRhythmModalOpen(true)}
	          onOpenConditionalSelect={() => setConditionalSelectOpen(true)}
	          onClearSelection={() => setSelectedMaterialIds([])}
	          openMoveModal={openMoveModal}
	          runMaterialOperation={runMaterialOperation}
	          runForceReleaseOperation={runForceReleaseOperation}
	          onBeforeOptimize={() => setRecalculating(true)}
	          onAfterOptimize={() => {
	            setRecalculating(false);
	            setRefreshSignal((v) => v + 1);
	            materialsQuery.refetch();
	            planItemsQuery.refetch();
	          }}
	        />

        <DailyRhythmManagerModal
          open={rhythmModalOpen}
          onClose={() => setRhythmModalOpen(false)}
          versionId={activeVersionId}
          machineOptions={machineOptions}
          defaultMachineCode={scheduleFocus?.machine || poolSelection.machineCode || machineOptions[0] || null}
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

        {pathOverride.pendingTotalCount > 0 && activeVersionId ? (
          <Alert
            type="warning"
            showIcon
            message={`路径规则待确认（跨日期/跨机组）：${pathOverride.pendingTotalCount} 条`}
	            description={`范围 ${pathOverride.summaryRange.from} ~ ${pathOverride.summaryRange.to}（确认后建议重算生成新版本）`}
	            action={
	              <Space>
	                <Button
	                  size="small"
	                  type="primary"
	                  icon={<InfoCircleOutlined />}
	                  loading={pathOverride.summaryIsFetching}
	                  onClick={() => setPathOverrideCenterOpen(true)}
	                >
	                  待确认中心
	                </Button>
	                <Button
	                  size="small"
	                  icon={<SettingOutlined />}
	                  onClick={() => navigate('/settings?tab=path_rule')}
	                >
	                  路径规则设置
	                </Button>
	              </Space>
	            }
	          />
	        ) : null}

        {pathOverride.pendingCount > 0 && pathOverride.context.machineCode && pathOverride.context.planDate ? (
          <Alert
            type="warning"
            showIcon
            message={`路径规则待确认：${pathOverride.pendingCount} 条`}
	            description={`机组 ${pathOverride.context.machineCode} · 日期 ${pathOverride.context.planDate}（确认后建议重算生成新版本）`}
	            action={
	              <Space>
	                <Button
	                  size="small"
	                  type="primary"
	                  icon={<InfoCircleOutlined />}
	                  loading={pathOverride.pendingIsFetching}
	                  onClick={() => setPathOverrideModalOpen(true)}
	                >
	                  去确认
	                </Button>
	                <Button
	                  size="small"
	                  icon={<SettingOutlined />}
	                  onClick={() => navigate('/settings?tab=path_rule')}
	                >
	                  路径规则设置
	                </Button>
	              </Space>
	            }
	          />
	        ) : null}

        {!materialsQuery.isLoading && !materialsQuery.error && materials.length === 0 ? (
          <Alert
            type="info"
            showIcon
            message="暂无物料数据"
            description="请先在“数据导入”导入材料CSV；导入成功后再返回工作台进行排程与干预。"
            action={
              <Button size="small" type="primary" onClick={() => navigate('/import')}>
                去导入
              </Button>
            }
          />
        ) : null}

        {!planItemsQuery.isLoading &&
        !planItemsQuery.error &&
        Array.isArray(planItemsQuery.data) &&
        planItemsQuery.data.length === 0 ? (
          <Alert
            type="info"
            showIcon
            message="当前版本暂无排程明细"
            description="可点击右上角“一键优化”执行重算生成排程，然后再使用矩阵/甘特图视图进行调整。"
          />
        ) : null}

        {/* 主体：左物料池 + 右视图 */}
        <div style={{ flex: 1, minHeight: 0, display: 'flex', gap: 12 }}>
          <div style={{ flex: '0 0 380px', minWidth: 320, minHeight: 0 }}>
            {deepLinkContext?.date ? (
              <div style={{ marginBottom: 8 }}>
                <Space wrap size={6}>
                  <Tag color="blue">
                    定位：{deepLinkContext.machine || poolSelection.machineCode || '全部机组'} / {deepLinkContext.date}
                  </Tag>
                  {deepLinkContextLabel ? <Tag>来源：{deepLinkContextLabel}</Tag> : null}
                  {dateRangeMode !== 'AUTO' ? (
                    <Button size="small" onClick={resetWorkbenchDateRangeToAuto}>
                      恢复自动范围
                    </Button>
                  ) : null}
                </Space>
              </div>
            ) : null}
            <MaterialPool
              materials={materials}
              loading={materialsQuery.isLoading}
              error={materialsQuery.error}
              onRetry={() => materialsQuery.refetch()}
              selection={poolSelection}
              onSelectionChange={(next) => {
                setPoolSelection(next);
                setWorkbenchFilters({ machineCode: next.machineCode });
              }}
              filters={{
                urgencyLevel: workbenchFilters.urgencyLevel,
                lockStatus: workbenchFilters.lockStatus,
              }}
              onFiltersChange={(next) => setWorkbenchFilters(next)}
              selectedMaterialIds={selectedMaterialIds}
              onSelectedMaterialIdsChange={setSelectedMaterialIds}
              onInspectMaterial={(m) => openInspector(m.material_id)}
            />
          </div>

          <div
            style={{
              flex: 1,
              minWidth: 0,
              minHeight: 0,
              display: 'flex',
              flexDirection: 'column',
              gap: 12,
            }}
          >
            <RollCycleAnchorCard
              versionId={activeVersionId}
              machineCode={poolSelection.machineCode}
              operator={currentUser || 'system'}
              refreshSignal={refreshSignal}
              onAfterReset={() => {
                setRefreshSignal((v) => v + 1);
                pathOverride.pendingRefetch();
                message.info('已重置换辊周期：建议执行“一键优化/重算”以刷新排程结果');
              }}
            />

            <Card size="small" title="产能概览" bodyStyle={{ padding: 12 }}>
              <div style={{ maxHeight: 260, overflow: 'auto' }}>
                <CapacityTimelineContainer
                  machineCode={poolSelection.machineCode}
                  dateRange={workbenchDateRange}
                  onMachineCodeChange={applyWorkbenchMachineCode}
                  onDateRangeChange={applyWorkbenchDateRange}
                  onResetDateRange={resetWorkbenchDateRangeToAuto}
                  onOpenScheduleCell={(machine, date, _materialIds, options) => {
                    if (options?.statusFilter) {
                      setScheduleStatusFilter(options.statusFilter);
                    }
                    openGanttCellDetail(machine, date, 'capacity');
                  }}
                  selectedMaterialIds={selectedMaterialIds}
                  materials={materials}
                />
              </div>
            </Card>

	            <Card
	              size="small"
	              title="排程视图"
	              extra={
	                <WorkbenchScheduleViewToolbar
	                  machineCode={poolSelection.machineCode}
	                  machineOptions={machineOptions}
	                  onMachineCodeChange={applyWorkbenchMachineCode}
	                  scheduleFocus={scheduleFocus}
	                  pathOverridePendingCount={pathOverride.pendingCount}
	                  pathOverrideContextMachineCode={pathOverride.context.machineCode}
	                  pathOverrideIsFetching={pathOverride.pendingIsFetching}
	                  onOpenPathOverrideModal={() => setPathOverrideModalOpen(true)}
	                  viewMode={workbenchViewMode}
	                  onViewModeChange={setWorkbenchViewMode}
	                />
	              }
	              style={{ flex: 1, minHeight: 0 }}
	              bodyStyle={{
	                height: '100%',
                minHeight: 0,
                display: 'flex',
                flexDirection: 'column',
              }}
            >
              <div style={{ flex: 1, minHeight: 0, height: '100%' }}>
                {workbenchViewMode === 'CARD' ? (
                  <ScheduleCardView
                    machineCode={poolSelection.machineCode}
                    urgentLevel={workbenchFilters.urgencyLevel}
                    dateRange={workbenchDateRange}
                    statusFilter={scheduleStatusFilter}
                    onStatusFilterChange={setScheduleStatusFilter}
                    refreshSignal={refreshSignal}
                    selectedMaterialIds={selectedMaterialIds}
                    onSelectedMaterialIdsChange={setSelectedMaterialIds}
                    onInspectMaterialId={(id) => openInspector(id)}
                  />
	                ) : workbenchViewMode === 'GANTT' ? (
	                  <ScheduleGanttView
	                    machineCode={poolSelection.machineCode}
	                    urgentLevel={workbenchFilters.urgencyLevel}
	                    dateRange={workbenchDateRange}
	                    suggestedDateRange={autoDateRange}
	                    onDateRangeChange={applyWorkbenchDateRange}
	                    focusedDate={ganttFocusedDate}
	                    autoOpenCell={ganttAutoOpenCell}
	                    statusFilter={scheduleStatusFilter}
	                    onStatusFilterChange={setScheduleStatusFilter}
	                    onFocusChange={setScheduleFocus}
	                    focus={scheduleFocus}
	                    onNavigateToMatrix={navigateToMatrix}
	                    planItems={planItemsQuery.data}
	                    loading={planItemsQuery.isLoading}
	                    error={planItemsQuery.error}
                    onRetry={() => planItemsQuery.refetch()}
                    selectedMaterialIds={selectedMaterialIds}
                    onSelectedMaterialIdsChange={setSelectedMaterialIds}
                    onInspectMaterialId={(id) => openInspector(id)}
                    onRequestMoveToCell={(machine, date) => openMoveModalAt(machine, date)}
                  />
                ) : (
                    <React.Suspense fallback={<PageSkeleton />}>
                      <PlanItemVisualization
                        machineCode={poolSelection.machineCode}
                        urgentLevel={workbenchFilters.urgencyLevel}
                        statusFilter={scheduleStatusFilter}
                        onStatusFilterChange={setScheduleStatusFilter}
                        focusRequest={matrixFocusRequest}
                        refreshSignal={refreshSignal}
                        selectedMaterialIds={selectedMaterialIds}
                        onSelectedMaterialIdsChange={setSelectedMaterialIds}
                      />
                    </React.Suspense>
                )}
              </div>
            </Card>
          </div>
        </div>

	        {/* 状态栏 */}
	        <WorkbenchStatusBar
	          selectedMaterialCount={selectedMaterialIds.length}
	          selectedTotalWeight={selectedTotalWeight}
	          disabled={selectedMaterialIds.length === 0}
	          onLock={() => runMaterialOperation(selectedMaterialIds, 'lock')}
	          onUnlock={() => runMaterialOperation(selectedMaterialIds, 'unlock')}
	          onSetUrgent={() => runMaterialOperation(selectedMaterialIds, 'urgent_on')}
	          onClearUrgent={() => runMaterialOperation(selectedMaterialIds, 'urgent_off')}
	          onForceRelease={() => runForceReleaseOperation(selectedMaterialIds)}
	          onOpenMoveRecommend={openMoveModalWithRecommend}
	          onOpenMoveModal={openMoveModal}
	          onClearSelection={() => setSelectedMaterialIds([])}
	        />

        <ConditionalSelectModal
          open={conditionalSelectOpen}
          onClose={() => setConditionalSelectOpen(false)}
          defaultMachine={poolSelection.machineCode || 'all'}
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
          planItemsLoading={planItemsQuery.isLoading}
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

        {/* 物料 Inspector */}
        <MaterialInspector
          visible={inspectorOpen}
          material={inspectedMaterial}
          onClose={() => setInspectorOpen(false)}
          onLock={(id) => runMaterialOperation([id], 'lock')}
          onUnlock={(id) => runMaterialOperation([id], 'unlock')}
          onSetUrgent={(id) => runMaterialOperation([id], 'urgent_on')}
          onClearUrgent={(id) => runMaterialOperation([id], 'urgent_off')}
        />
      </div>
    </ErrorBoundary>
  );
};

export default PlanningWorkbench;

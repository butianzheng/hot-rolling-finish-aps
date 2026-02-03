import React from 'react';
import type { Dayjs } from 'dayjs';
import { Button, Card, Space, Tag } from 'antd';

import RollCycleAnchorCard from '../roll-cycle-anchor/RollCycleAnchorCard';
import { CapacityTimelineContainer } from '../CapacityTimelineContainer';
import PageSkeleton from '../PageSkeleton';
import MaterialPool, { type MaterialPoolFilters, type MaterialPoolMaterial, type MaterialPoolSelection } from './MaterialPool';
import ScheduleCardView from './ScheduleCardView';
import ScheduleGanttView from './ScheduleGanttView';
import WorkbenchScheduleViewToolbar from './WorkbenchScheduleViewToolbar';
import type { WorkbenchViewMode } from '../../stores/use-global-store';
import type { PlanItemStatusFilter } from '../../utils/planItemStatus';

const PlanItemVisualization = React.lazy(() => import('../PlanItemVisualization'));

type ScheduleFocus = Parameters<typeof WorkbenchScheduleViewToolbar>[0]['scheduleFocus'];
type GanttAutoOpenCell = { machine: string; date: string; nonce?: string | number; source?: string } | null;

const WorkbenchMainLayout: React.FC<{
  activeVersionId: string;
  currentUser: string;
  machineOptions: string[];

  // 左侧：定位提示（深链接）
  deepLinkDate: string | null;
  deepLinkMachine: string | null;
  deepLinkLabel: string | null;
  showResetAutoRange: boolean;
  onResetDateRangeToAuto: () => void;

  // 左侧：物料池
  materials: MaterialPoolMaterial[];
  materialsLoading: boolean;
  materialsError: unknown;
  onRetryMaterials: () => void;
  poolSelection: MaterialPoolSelection;
  onPoolSelectionChange: (next: MaterialPoolSelection) => void;
  poolFilters: MaterialPoolFilters;
  onPoolFiltersChange: (next: Partial<MaterialPoolFilters>) => void;
  selectedMaterialIds: string[];
  onSelectedMaterialIdsChange: (ids: string[]) => void;
  onInspectMaterialId: (id: string) => void;

  // 右侧：基础联动
  refreshSignal: number;
  onAfterRollCycleReset: () => void;
  workbenchDateRange: [Dayjs, Dayjs];
  autoDateRange: [Dayjs, Dayjs];
  onMachineCodeChange: (machineCode: string | null) => void;
  onDateRangeChange: (next: [Dayjs, Dayjs]) => void;

  // 右侧：视图与聚焦
  viewMode: WorkbenchViewMode;
  onViewModeChange: (mode: WorkbenchViewMode) => void;
  scheduleFocus: ScheduleFocus;
  setScheduleFocus: (focus: ScheduleFocus) => void;
  matrixFocusRequest: { machine?: string; date: string; nonce: string | number } | null;

  // 状态筛选
  scheduleStatusFilter: PlanItemStatusFilter;
  setScheduleStatusFilter: (next: PlanItemStatusFilter) => void;

  // 甘特图定位/跳转
  focusedDate: string | null;
  autoOpenCell: GanttAutoOpenCell;
  openGanttCellDetail: (machine: string, date: string, source: string) => void;
  navigateToMatrix: (machine: string, date: string) => void;

  // 路径规则待确认
  pathOverridePendingCount: number;
  pathOverrideContextMachineCode: string | null;
  pathOverrideIsFetching: boolean;
  onOpenPathOverrideModal: () => void;

  // 排程明细加载状态（给甘特图/矩阵用）
  planItemsData: unknown;
  planItemsLoading: boolean;
  planItemsError: unknown;
  onRetryPlanItems: () => void;

  // 移位弹窗入口：从甘特图单元格一键打开
  onRequestMoveToCell: (machine: string, date: string) => void;
}> = ({
  activeVersionId,
  currentUser,
  machineOptions,

  deepLinkDate,
  deepLinkMachine,
  deepLinkLabel,
  showResetAutoRange,
  onResetDateRangeToAuto,

  materials,
  materialsLoading,
  materialsError,
  onRetryMaterials,
  poolSelection,
  onPoolSelectionChange,
  poolFilters,
  onPoolFiltersChange,
  selectedMaterialIds,
  onSelectedMaterialIdsChange,
  onInspectMaterialId,

  refreshSignal,
  onAfterRollCycleReset,
  workbenchDateRange,
  autoDateRange,
  onMachineCodeChange,
  onDateRangeChange,

  viewMode,
  onViewModeChange,
  scheduleFocus,
  setScheduleFocus,
  matrixFocusRequest,

  scheduleStatusFilter,
  setScheduleStatusFilter,

  focusedDate,
  autoOpenCell,
  openGanttCellDetail,
  navigateToMatrix,

  pathOverridePendingCount,
  pathOverrideContextMachineCode,
  pathOverrideIsFetching,
  onOpenPathOverrideModal,

  planItemsData,
  planItemsLoading,
  planItemsError,
  onRetryPlanItems,

  onRequestMoveToCell,
}) => {
  return (
    <div style={{ flex: 1, minHeight: 0, display: 'flex', gap: 12 }}>
      <div style={{ flex: '0 0 380px', minWidth: 320, minHeight: 0 }}>
        {deepLinkDate ? (
          <div style={{ marginBottom: 8 }}>
            <Space wrap size={6}>
              <Tag color="blue">
                定位：{deepLinkMachine || poolSelection.machineCode || '全部机组'} / {deepLinkDate}
              </Tag>
              {deepLinkLabel ? <Tag>来源：{deepLinkLabel}</Tag> : null}
              {showResetAutoRange ? (
                <Button size="small" onClick={onResetDateRangeToAuto}>
                  恢复自动范围
                </Button>
              ) : null}
            </Space>
          </div>
        ) : null}

        <MaterialPool
          materials={materials}
          loading={materialsLoading}
          error={materialsError}
          onRetry={onRetryMaterials}
          selection={poolSelection}
          onSelectionChange={onPoolSelectionChange}
          filters={poolFilters}
          onFiltersChange={onPoolFiltersChange}
          selectedMaterialIds={selectedMaterialIds}
          onSelectedMaterialIdsChange={onSelectedMaterialIdsChange}
          onInspectMaterial={(m) => onInspectMaterialId(m.material_id)}
        />
      </div>

      <div style={{ flex: 1, minWidth: 0, minHeight: 0, display: 'flex', flexDirection: 'column', gap: 12 }}>
        <RollCycleAnchorCard
          versionId={activeVersionId}
          machineCode={poolSelection.machineCode}
          operator={currentUser || 'system'}
          refreshSignal={refreshSignal}
          onAfterReset={onAfterRollCycleReset}
        />

        <Card size="small" title="产能概览" bodyStyle={{ padding: 12 }}>
          <div style={{ maxHeight: 260, overflow: 'auto' }}>
            <CapacityTimelineContainer
              machineCode={poolSelection.machineCode}
              dateRange={workbenchDateRange}
              onMachineCodeChange={onMachineCodeChange}
              onDateRangeChange={onDateRangeChange}
              onResetDateRange={onResetDateRangeToAuto}
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
              onMachineCodeChange={onMachineCodeChange}
              scheduleFocus={scheduleFocus}
              pathOverridePendingCount={pathOverridePendingCount}
              pathOverrideContextMachineCode={pathOverrideContextMachineCode}
              pathOverrideIsFetching={pathOverrideIsFetching}
              onOpenPathOverrideModal={onOpenPathOverrideModal}
              viewMode={viewMode}
              onViewModeChange={onViewModeChange}
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
            {viewMode === 'CARD' ? (
              <ScheduleCardView
                machineCode={poolSelection.machineCode}
                urgentLevel={poolFilters.urgencyLevel}
                dateRange={workbenchDateRange}
                statusFilter={scheduleStatusFilter}
                onStatusFilterChange={setScheduleStatusFilter}
                refreshSignal={refreshSignal}
                selectedMaterialIds={selectedMaterialIds}
                onSelectedMaterialIdsChange={onSelectedMaterialIdsChange}
                onInspectMaterialId={onInspectMaterialId}
              />
            ) : viewMode === 'GANTT' ? (
              <ScheduleGanttView
                machineCode={poolSelection.machineCode}
                urgentLevel={poolFilters.urgencyLevel}
                dateRange={workbenchDateRange}
                suggestedDateRange={autoDateRange}
                onDateRangeChange={onDateRangeChange}
                focusedDate={focusedDate}
                autoOpenCell={autoOpenCell}
                statusFilter={scheduleStatusFilter}
                onStatusFilterChange={setScheduleStatusFilter}
                onFocusChange={setScheduleFocus}
                focus={scheduleFocus}
                onNavigateToMatrix={navigateToMatrix}
                planItems={planItemsData}
                loading={planItemsLoading}
                error={planItemsError}
                onRetry={onRetryPlanItems}
                selectedMaterialIds={selectedMaterialIds}
                onSelectedMaterialIdsChange={onSelectedMaterialIdsChange}
                onInspectMaterialId={onInspectMaterialId}
                onRequestMoveToCell={onRequestMoveToCell}
              />
            ) : (
              <React.Suspense fallback={<PageSkeleton />}>
                <PlanItemVisualization
                  machineCode={poolSelection.machineCode}
                  urgentLevel={poolFilters.urgencyLevel}
                  statusFilter={scheduleStatusFilter}
                  onStatusFilterChange={setScheduleStatusFilter}
                  focusRequest={matrixFocusRequest}
                  refreshSignal={refreshSignal}
                  selectedMaterialIds={selectedMaterialIds}
                  onSelectedMaterialIdsChange={onSelectedMaterialIdsChange}
                />
              </React.Suspense>
            )}
          </div>
        </Card>
      </div>
    </div>
  );
};

export default React.memo(WorkbenchMainLayout);


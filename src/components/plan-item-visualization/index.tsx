/**
 * 排产明细可视化 - 主组件
 *
 * 重构后：922 行 → ~180 行 (-80%)
 */

import React, { useMemo } from 'react';
import { Button, Card, Col, Dropdown, Row, Space, Table, Tag, Typography, message } from 'antd';
import { DownloadOutlined, ReloadOutlined } from '@ant-design/icons';
import { DndContext, closestCenter } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import { useNavigate } from 'react-router-dom';
import { exportCSV, exportJSON } from '../../utils/exportUtils';
import { formatWeight } from '../../utils/formatters';
import { getErrorMessage } from '../../utils/errorUtils';
import type { PlanItemStatusFilter } from '../../utils/planItemStatus';
import { PLAN_ITEM_STATUS_FILTER_META, isPlanItemForceReleased } from '../../utils/planItemStatus';
import { tableFilterEmptyConfig } from '../CustomEmpty';
import NoActiveVersionGuide from '../NoActiveVersionGuide';

import { usePlanItemVisualization } from './usePlanItemVisualization';
import { createPlanItemColumns } from './planItemColumns';
import { DraggableRow } from './DraggableRow';
import { StatisticsCards } from './StatisticsCards';
import { FilterBar } from './FilterBar';
import { BatchOperationBar } from './BatchOperationBar';
import { PlanItemDetailModal } from './PlanItemDetailModal';
import { ForceReleaseModal } from './ForceReleaseModal';
import type { PlanItemVisualizationProps } from './types';

const { Text } = Typography;

const PlanItemVisualization: React.FC<PlanItemVisualizationProps> = (props) => {
  const { onNavigateToPlan } = props;
  const navigate = useNavigate();
  const navigateToPlan = onNavigateToPlan || (() => navigate('/comparison'));

  const state = usePlanItemVisualization(props);
  const statusFilter = props.statusFilter || 'ALL';
  const onStatusFilterChange = props.onStatusFilterChange;

  // 表格列配置
  const columns = useMemo(
    () =>
      createPlanItemColumns({
        machineOptions: state.machineOptions,
        onViewDetail: state.handleViewDetail,
      }),
    [state.machineOptions, state.handleViewDetail]
  );

  // 导出处理
  const handleExportCSV = () => {
    try {
      const data = state.filteredItems.map((item) => ({
        材料号: item.material_id,
        机组: item.machine_code,
        计划日期: item.plan_date,
        序号: item.seq_no,
        重量: formatWeight(item.weight_t),
        钢种: item.steel_grade || '-',
        合同号: item.contract_no || '-',
        交期: item.due_date || '-',
        紧急等级: item.urgent_level || '-',
        来源: item.source_type,
        锁定: item.locked_in_plan ? '是' : '否',
        强制放行: isPlanItemForceReleased(item) ? '是' : '否',
        排产状态: item.sched_state || '-',
        当前方案排程日期: item.scheduled_date || '-',
        当前方案排程机组: item.scheduled_machine_code || '-',
        原因: item.assign_reason || '-',
      }));
      exportCSV(data, '排产明细');
      message.success('导出成功');
    } catch (error: unknown) {
      message.error(`导出失败: ${getErrorMessage(error)}`);
    }
  };

  const handleExportJSON = () => {
    try {
      exportJSON(state.filteredItems, '排产明细');
      message.success('导出成功');
    } catch (error: unknown) {
      message.error(`导出失败: ${getErrorMessage(error)}`);
    }
  };

  // 没有激活版本时显示引导
  if (!state.activeVersionId) {
    return (
      <NoActiveVersionGuide
        title="尚无激活的排产版本"
        description="排产明细可视化需要一个激活的排产版本作为基础"
        onNavigateToPlan={navigateToPlan}
      />
    );
  }

  return (
    <div style={{ padding: '24px' }}>
      {/* 标题和操作栏 */}
      <Row justify="space-between" align="middle" style={{ marginBottom: 16 }}>
        <Col>
          <h2 style={{ margin: 0 }}>排产明细可视化</h2>
        </Col>
        <Col>
          <Space>
            <Button icon={<ReloadOutlined />} onClick={() => state.loadPlanItems()}>
              刷新
            </Button>
            <Dropdown
              menu={{
                items: [
                  { label: '导出为表格文件', key: 'csv', onClick: handleExportCSV },
                  { label: '导出为数据文件', key: 'json', onClick: handleExportJSON },
                ],
              }}
            >
              <Button icon={<DownloadOutlined />}>导出</Button>
            </Dropdown>
          </Space>
        </Col>
      </Row>

      {/* 统计卡片 */}
      <StatisticsCards statistics={state.statistics} />

      {/* 状态快速筛选（与工作台其他视图一致） */}
      <Card size="small" style={{ marginBottom: 16 }}>
        <Space wrap size={6} style={{ justifyContent: 'space-between', width: '100%' }}>
          <Space wrap size={6}>
            <Tag
              color={PLAN_ITEM_STATUS_FILTER_META.ALL.color}
              style={{
                cursor: onStatusFilterChange ? 'pointer' : undefined,
                boxShadow: statusFilter === 'ALL' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
                userSelect: 'none',
              }}
              onClick={() => onStatusFilterChange?.('ALL')}
              title={`已排 ${state.statusSummary.totalCount} 件 / ${formatWeight(state.statusSummary.totalWeightT)}`}
            >
              已排 {state.statusSummary.totalCount}
            </Tag>
            <Tag
              color={PLAN_ITEM_STATUS_FILTER_META.LOCKED.color}
              style={{
                cursor:
                  onStatusFilterChange &&
                  (state.statusSummary.lockedInPlanCount > 0 || statusFilter === 'LOCKED')
                    ? 'pointer'
                    : 'not-allowed',
                opacity:
                  onStatusFilterChange && state.statusSummary.lockedInPlanCount === 0 && statusFilter !== 'LOCKED'
                    ? 0.35
                    : 1,
                boxShadow: statusFilter === 'LOCKED' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
                userSelect: 'none',
              }}
              onClick={() => {
                if (!onStatusFilterChange) return;
                if (state.statusSummary.lockedInPlanCount === 0 && statusFilter !== 'LOCKED') return;
                onStatusFilterChange(statusFilter === 'LOCKED' ? 'ALL' : ('LOCKED' as PlanItemStatusFilter));
              }}
              title={`冻结 ${state.statusSummary.lockedInPlanCount} 件 / ${formatWeight(state.statusSummary.lockedInPlanWeightT)}`}
            >
              冻结 {state.statusSummary.lockedInPlanCount}
            </Tag>
            <Tag
              color={PLAN_ITEM_STATUS_FILTER_META.FORCE_RELEASE.color}
              style={{
                cursor:
                  onStatusFilterChange &&
                  (state.statusSummary.forceReleaseCount > 0 || statusFilter === 'FORCE_RELEASE')
                    ? 'pointer'
                    : 'not-allowed',
                opacity:
                  onStatusFilterChange && state.statusSummary.forceReleaseCount === 0 && statusFilter !== 'FORCE_RELEASE'
                    ? 0.35
                    : 1,
                boxShadow: statusFilter === 'FORCE_RELEASE' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
                userSelect: 'none',
              }}
              onClick={() => {
                if (!onStatusFilterChange) return;
                if (state.statusSummary.forceReleaseCount === 0 && statusFilter !== 'FORCE_RELEASE') return;
                onStatusFilterChange(statusFilter === 'FORCE_RELEASE' ? 'ALL' : ('FORCE_RELEASE' as PlanItemStatusFilter));
              }}
              title={`强制放行 ${state.statusSummary.forceReleaseCount} 件 / ${formatWeight(state.statusSummary.forceReleaseWeightT)}`}
            >
              强放 {state.statusSummary.forceReleaseCount}
            </Tag>
            <Tag
              color={PLAN_ITEM_STATUS_FILTER_META.ADJUSTABLE.color}
              style={{
                cursor:
                  onStatusFilterChange &&
                  (state.statusSummary.adjustableCount > 0 || statusFilter === 'ADJUSTABLE')
                    ? 'pointer'
                    : 'not-allowed',
                opacity:
                  onStatusFilterChange && state.statusSummary.adjustableCount === 0 && statusFilter !== 'ADJUSTABLE'
                    ? 0.35
                    : 1,
                boxShadow: statusFilter === 'ADJUSTABLE' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
                userSelect: 'none',
              }}
              onClick={() => {
                if (!onStatusFilterChange) return;
                if (state.statusSummary.adjustableCount === 0 && statusFilter !== 'ADJUSTABLE') return;
                onStatusFilterChange(statusFilter === 'ADJUSTABLE' ? 'ALL' : ('ADJUSTABLE' as PlanItemStatusFilter));
              }}
              title={`可调（非冻结）${state.statusSummary.adjustableCount} 件 / ${formatWeight(state.statusSummary.adjustableWeightT)}`}
            >
              可调 {state.statusSummary.adjustableCount}
            </Tag>
            <Tag
              color={PLAN_ITEM_STATUS_FILTER_META.READY.color}
              style={{
                cursor: onStatusFilterChange ? 'pointer' : undefined,
                boxShadow: statusFilter === 'READY' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
                userSelect: 'none',
              }}
              onClick={() => onStatusFilterChange?.(statusFilter === 'READY' ? 'ALL' : ('READY' as PlanItemStatusFilter))}
            >
              就绪
            </Tag>
            <Tag
              color={PLAN_ITEM_STATUS_FILTER_META.PENDING_MATURE.color}
              style={{
                cursor: onStatusFilterChange ? 'pointer' : undefined,
                boxShadow: statusFilter === 'PENDING_MATURE' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
                userSelect: 'none',
              }}
              onClick={() =>
                onStatusFilterChange?.(
                  statusFilter === 'PENDING_MATURE' ? 'ALL' : ('PENDING_MATURE' as PlanItemStatusFilter)
                )
              }
            >
              待成熟
            </Tag>
            <Tag
              color={PLAN_ITEM_STATUS_FILTER_META.BLOCKED.color}
              style={{
                cursor: onStatusFilterChange ? 'pointer' : undefined,
                boxShadow: statusFilter === 'BLOCKED' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
                userSelect: 'none',
              }}
              onClick={() => onStatusFilterChange?.(statusFilter === 'BLOCKED' ? 'ALL' : ('BLOCKED' as PlanItemStatusFilter))}
            >
              阻断
            </Tag>
            <Tag
              color={PLAN_ITEM_STATUS_FILTER_META.URGENT_L3.color}
              style={{
                cursor: onStatusFilterChange ? 'pointer' : undefined,
                boxShadow: statusFilter === 'URGENT_L3' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
                userSelect: 'none',
              }}
              onClick={() =>
                onStatusFilterChange?.(statusFilter === 'URGENT_L3' ? 'ALL' : ('URGENT_L3' as PlanItemStatusFilter))
              }
            >
              L3
            </Tag>
            <Tag
              color={PLAN_ITEM_STATUS_FILTER_META.URGENT_L2.color}
              style={{
                cursor: onStatusFilterChange ? 'pointer' : undefined,
                boxShadow: statusFilter === 'URGENT_L2' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
                userSelect: 'none',
              }}
              onClick={() =>
                onStatusFilterChange?.(statusFilter === 'URGENT_L2' ? 'ALL' : ('URGENT_L2' as PlanItemStatusFilter))
              }
            >
              L2
            </Tag>
          </Space>

          <Text type="secondary" style={{ fontSize: 12 }}>
            当前状态筛选：{PLAN_ITEM_STATUS_FILTER_META[statusFilter]?.label || '已排'}
          </Text>
        </Space>
      </Card>

      {/* 筛选栏 */}
      <FilterBar
        machineOptions={state.machineOptions}
        selectedMachine={state.selectedMachine}
        onMachineChange={state.setSelectedMachine}
        selectedUrgentLevel={state.selectedUrgentLevel}
        onUrgentLevelChange={state.setSelectedUrgentLevel}
        selectedDate={state.selectedDate}
        onDateChange={state.setSelectedDate}
        dateRange={state.dateRange}
        onDateRangeChange={state.setDateRange}
        searchText={state.searchText}
        onSearchChange={state.setSearchText}
        onClearFilters={state.clearFilters}
      />

      {/* 批量操作栏 */}
      <BatchOperationBar
        selectedCount={state.selectedMaterialIds.length}
        onForceRelease={state.openForceReleaseModal}
        onClearForceRelease={state.handleBatchClearForceRelease}
        onCancelSelection={() => state.setSelectedMaterialIds([])}
      />

      {/* 排产明细表格 */}
      <Card>
        <DndContext
          sensors={state.sensors}
          collisionDetection={closestCenter}
          onDragEnd={state.handleDragEnd}
        >
          <SortableContext
            items={state.filteredItems.map((item) => item.key)}
            strategy={verticalListSortingStrategy}
          >
            <Table
              columns={columns}
              dataSource={state.filteredItems}
              loading={state.loading}
              locale={tableFilterEmptyConfig}
              rowKey="material_id"
              pagination={{
                pageSize: 20,
                showSizeChanger: true,
                showTotal: (total) => `共 ${total} 条记录`,
              }}
              scroll={{ x: 1400 }}
              size="small"
              rowSelection={{
                type: 'checkbox',
                selectedRowKeys: state.selectedMaterialIds,
                onChange: (selectedKeys) => {
                  state.setSelectedMaterialIds(selectedKeys as string[]);
                },
                getCheckboxProps: (record) => ({
                  disabled: record.locked_in_plan,
                }),
              }}
              components={{
                body: { row: DraggableRow },
              }}
            />
          </SortableContext>
        </DndContext>
      </Card>

      {/* 详情模态框 */}
      <PlanItemDetailModal
        open={state.showDetailModal}
        item={state.selectedItem}
        onClose={state.closeDetailModal}
      />

      {/* 强制放行模态框 */}
      <ForceReleaseModal
        open={state.forceReleaseModalVisible}
        selectedCount={state.selectedMaterialIds.length}
        reason={state.forceReleaseReason}
        onReasonChange={state.setForceReleaseReason}
        mode={state.forceReleaseMode}
        onModeChange={state.setForceReleaseMode}
        loading={state.loading}
        onOk={state.handleBatchForceRelease}
        onCancel={state.closeForceReleaseModal}
      />
    </div>
  );
};

export default PlanItemVisualization;

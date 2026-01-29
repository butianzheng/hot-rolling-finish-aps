/**
 * 排产明细可视化 - 主组件
 *
 * 重构后：922 行 → ~180 行 (-80%)
 */

import React, { useMemo } from 'react';
import { Button, Card, Col, Dropdown, Row, Space, Table, message } from 'antd';
import { DownloadOutlined, ReloadOutlined } from '@ant-design/icons';
import { DndContext, closestCenter } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import { useNavigate } from 'react-router-dom';
import { exportCSV, exportJSON } from '../../utils/exportUtils';
import { formatWeight } from '../../utils/formatters';
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

const PlanItemVisualization: React.FC<PlanItemVisualizationProps> = (props) => {
  const { onNavigateToPlan } = props;
  const navigate = useNavigate();
  const navigateToPlan = onNavigateToPlan || (() => navigate('/comparison'));

  const state = usePlanItemVisualization(props);

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
        紧急等级: item.urgent_level || '-',
        来源: item.source_type,
        锁定: item.locked_in_plan ? '是' : '否',
        强制放行: item.force_release_in_plan ? '是' : '否',
        排产状态: item.sched_state || '-',
        原因: item.assign_reason || '-',
      }));
      exportCSV(data, '排产明细');
      message.success('导出成功');
    } catch (error: any) {
      message.error(`导出失败: ${error.message}`);
    }
  };

  const handleExportJSON = () => {
    try {
      exportJSON(state.filteredItems, '排产明细');
      message.success('导出成功');
    } catch (error: any) {
      message.error(`导出失败: ${error.message}`);
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
                  { label: '导出为 CSV', key: 'csv', onClick: handleExportCSV },
                  { label: '导出为 JSON', key: 'json', onClick: handleExportJSON },
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
                  disabled: record.locked_in_plan || record.force_release_in_plan,
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

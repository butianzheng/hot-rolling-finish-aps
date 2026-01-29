/**
 * 产能池管理 - 主组件
 *
 * 重构后：594 行 → ~130 行 (-78%)
 */

import React, { useMemo } from 'react';
import { Alert, Button, Card, Col, Row, Space, Table } from 'antd';
import { EditOutlined, ReloadOutlined } from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import { tableEmptyConfig } from '../CustomEmpty';
import NoActiveVersionGuide from '../NoActiveVersionGuide';

import { useCapacityPoolManagement } from './useCapacityPoolManagement';
import { createCapacityPoolColumns } from './capacityPoolColumns';
import { StatisticsCards } from './StatisticsCards';
import { FilterBar } from './FilterBar';
import { EditCapacityModal } from './EditCapacityModal';
import { BatchEditModal } from './BatchEditModal';
import type { CapacityPool, CapacityPoolManagementProps } from './types';

const CapacityPoolManagement: React.FC<CapacityPoolManagementProps> = ({ onNavigateToPlan }) => {
  const navigate = useNavigate();
  const navigateToPlan = onNavigateToPlan || (() => navigate('/comparison'));

  const state = useCapacityPoolManagement();

  // 表格列配置
  const columns = useMemo(
    () => createCapacityPoolColumns({ onEdit: state.handleEdit }),
    [state.handleEdit]
  );

  // 没有激活版本时显示引导
  if (!state.activeVersionId) {
    return (
      <NoActiveVersionGuide
        title="尚无激活的排产版本"
        description="产能池管理需要一个激活的排产版本作为基础"
        onNavigateToPlan={navigateToPlan}
      />
    );
  }

  return (
    <div style={{ padding: '24px' }}>
      <Row justify="space-between" align="middle" style={{ marginBottom: 16 }}>
        <Col>
          <h2 style={{ margin: 0 }}>产能池管理</h2>
        </Col>
        <Col>
          <Space>
            <Button icon={<ReloadOutlined />} onClick={state.loadCapacityPools}>
              刷新
            </Button>
            <Button
              icon={<EditOutlined />}
              disabled={state.selectedPools.length === 0}
              onClick={state.openBatchModal}
            >
              批量调整{state.selectedPools.length > 0 ? `(${state.selectedPools.length})` : ''}
            </Button>
          </Space>
        </Col>
      </Row>

      {state.loadError && (
        <Alert
          type="error"
          showIcon
          message="产能池加载失败"
          description={state.loadError}
          action={
            <Button size="small" onClick={state.loadCapacityPools}>
              重试
            </Button>
          }
          style={{ marginBottom: 16 }}
        />
      )}

      {/* 统计卡片 */}
      <StatisticsCards stats={state.totalStats} />

      {/* 筛选栏 */}
      <FilterBar
        machineOptions={state.machineOptions}
        selectedMachines={state.selectedMachines}
        onMachinesChange={state.setSelectedMachines}
        dateRange={state.dateRange}
        onDateRangeChange={state.setDateRange}
        loading={state.loading}
        onQuery={state.loadCapacityPools}
      />

      {/* 产能池表格 */}
      <Card>
        <Table
          columns={columns}
          dataSource={state.capacityPools}
          loading={state.loading}
          rowSelection={{
            selectedRowKeys: state.selectedRowKeys,
            onChange: (keys, rows) => {
              state.setSelectedRowKeys(keys);
              state.setSelectedPools(rows as CapacityPool[]);
            },
          }}
          rowKey={(record) => `${record.machine_code}-${record.plan_date}`}
          locale={tableEmptyConfig}
          virtual
          pagination={{
            pageSize: 20,
            showSizeChanger: true,
            showTotal: (total) => `共 ${total} 条记录`,
          }}
          scroll={{ x: 1000, y: 520 }}
          size="small"
        />
      </Card>

      {/* 编辑模态框 */}
      <EditCapacityModal
        open={state.editModalVisible}
        pool={state.editingPool}
        targetCapacity={state.targetCapacity}
        onTargetCapacityChange={state.setTargetCapacity}
        limitCapacity={state.limitCapacity}
        onLimitCapacityChange={state.setLimitCapacity}
        reason={state.updateReason}
        onReasonChange={state.setUpdateReason}
        loading={state.loading}
        onOk={state.handleUpdate}
        onCancel={state.closeEditModal}
      />

      {/* 批量调整模态框 */}
      <BatchEditModal
        open={state.batchModalVisible}
        selectedCount={state.selectedPools.length}
        targetCapacity={state.batchTargetCapacity}
        onTargetCapacityChange={state.setBatchTargetCapacity}
        limitCapacity={state.batchLimitCapacity}
        onLimitCapacityChange={state.setBatchLimitCapacity}
        reason={state.batchReason}
        onReasonChange={state.setBatchReason}
        loading={state.loading}
        onOk={state.handleBatchUpdate}
        onCancel={state.closeBatchModal}
      />
    </div>
  );
};

export default CapacityPoolManagement;

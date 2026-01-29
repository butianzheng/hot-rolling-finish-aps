/**
 * 操作日志查询 - 主组件
 *
 * 重构后：503 行 → ~100 行 (-80%)
 */

import React, { useMemo } from 'react';
import { Button, Card, Col, Dropdown, message, Row, Space, Table } from 'antd';
import { DownloadOutlined, ReloadOutlined } from '@ant-design/icons';
import { tableFilterEmptyConfig } from '../CustomEmpty';
import { exportCSV, exportJSON } from '../../utils/exportUtils';

import { useActionLogQuery } from './useActionLogQuery';
import { createActionLogColumns } from './actionLogColumns';
import { FilterBar } from './FilterBar';
import { LogDetailModal } from './LogDetailModal';
import { actionTypeLabels } from './types';

const ActionLogQuery: React.FC = () => {
  const state = useActionLogQuery();

  // 表格列配置
  const columns = useMemo(
    () => createActionLogColumns({ onViewDetail: state.handleViewDetail }),
    [state.handleViewDetail]
  );

  // 导出处理
  const handleExportCSV = () => {
    try {
      const data = state.filteredLogs.map((log) => ({
        操作时间: log.action_ts,
        操作类型: actionTypeLabels[log.action_type]?.text || log.action_type,
        操作人: log.actor,
        版本ID: log.version_id,
        机组: log.machine_code || '-',
        操作详情: log.detail || '-',
      }));
      exportCSV(data, '操作日志');
      message.success('导出成功');
    } catch (error: any) {
      message.error(`导出失败: ${error.message}`);
    }
  };

  const handleExportJSON = () => {
    try {
      exportJSON(state.filteredLogs, '操作日志');
      message.success('导出成功');
    } catch (error: any) {
      message.error(`导出失败: ${error.message}`);
    }
  };

  return (
    <div style={{ padding: '24px' }}>
      {/* 标题和操作栏 */}
      <Row justify="space-between" align="middle" style={{ marginBottom: 16 }}>
        <Col>
          <h2 style={{ margin: 0 }}>操作日志查询</h2>
        </Col>
        <Col>
          <Space>
            <Button icon={<ReloadOutlined />} onClick={() => state.loadActionLogs()}>
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

      {/* 筛选栏 */}
      <FilterBar
        loadError={state.loadError}
        timeRange={state.timeRange}
        onTimeRangeChange={state.setTimeRange}
        selectedActionType={state.selectedActionType}
        onActionTypeChange={state.setSelectedActionType}
        selectedActor={state.selectedActor}
        onActorChange={state.setSelectedActor}
        selectedVersion={state.selectedVersion}
        onVersionChange={state.setSelectedVersion}
        searchText={state.searchText}
        onSearchTextChange={state.setSearchText}
        uniqueActors={state.uniqueActors}
        uniqueVersions={state.uniqueVersions}
        onClearFilters={state.clearFilters}
        onRetry={() => state.loadActionLogs()}
      />

      {/* 操作日志表格 */}
      <Card>
        <Table
          columns={columns}
          dataSource={state.filteredLogs}
          loading={state.loading}
          rowKey="action_id"
          locale={tableFilterEmptyConfig}
          virtual
          pagination={{
            pageSize: 20,
            showSizeChanger: true,
            showTotal: (total) => `共 ${total} 条记录`,
          }}
          scroll={{ x: 1200, y: 520 }}
          size="small"
        />
      </Card>

      {/* 详情模态框 */}
      <LogDetailModal
        open={state.showDetailModal}
        log={state.selectedLog}
        onClose={state.closeDetailModal}
      />
    </div>
  );
};

export default ActionLogQuery;

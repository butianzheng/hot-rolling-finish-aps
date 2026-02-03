/**
 * 配置管理 - 主组件
 *
 * 重构后：452 行 → ~90 行 (-80%)
 */

import React, { useMemo } from 'react';
import { Alert, Button, Card, Col, Row, Space, Table } from 'antd';
import {
  DownloadOutlined,
  ReloadOutlined,
  SettingOutlined,
  UploadOutlined,
} from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import { tableEmptyConfig } from '../CustomEmpty';

import { useConfigManagement } from './useConfigManagement';
import { createConfigColumns } from './configColumns';
import { StatisticsCards } from './StatisticsCards';
import { FilterBar } from './FilterBar';
import { EditConfigModal } from './EditConfigModal';

const ConfigManagement: React.FC = () => {
  const state = useConfigManagement();
  const navigate = useNavigate();

  // 表格列配置
  const columns = useMemo(
    () =>
      createConfigColumns({
        onEdit: state.handleEdit,
        onOpenPathRulePanel: () => navigate('/settings?tab=path_rule'),
      }),
    [navigate, state.handleEdit]
  );

  return (
    <div style={{ padding: '24px' }}>
      <Row justify="space-between" align="middle" style={{ marginBottom: 16 }}>
        <Col>
          <h2 style={{ margin: 0 }}>
            <SettingOutlined /> 配置管理
          </h2>
        </Col>
        <Col>
          <Space>
            <Button icon={<ReloadOutlined />} onClick={state.loadConfigs}>
              刷新
            </Button>
            <Button icon={<DownloadOutlined />} onClick={state.handleExportSnapshot}>
              导出快照
            </Button>
            <Button icon={<UploadOutlined />} onClick={state.handleImportSnapshot}>
              导入快照
            </Button>
          </Space>
        </Col>
      </Row>

      {state.loadError && (
        <Alert
          type="error"
          showIcon
          message="配置加载失败"
          description={state.loadError}
          action={
            <Button size="small" onClick={state.loadConfigs}>
              重试
            </Button>
          }
          style={{ marginBottom: 16 }}
        />
      )}

      {/* 统计卡片 */}
      <StatisticsCards
        totalCount={state.configs.length}
        scopeTypeCounts={state.scopeTypeCounts}
      />

      {/* 筛选栏 */}
      <FilterBar
        searchText={state.searchText}
        onSearchTextChange={state.setSearchText}
        selectedScopeType={state.selectedScopeType}
        onScopeTypeChange={state.setSelectedScopeType}
        onClearFilters={state.clearFilters}
      />

      {/* 配置表格 */}
      <Card>
        <Table
          columns={columns}
          dataSource={state.filteredConfigs}
          loading={state.loading}
          rowKey={(record) => `${record.scope_id}-${record.key}`}
          locale={tableEmptyConfig}
          virtual
          pagination={{
            pageSize: 20,
            showSizeChanger: true,
            showTotal: (total) => `共 ${total} 条配置`,
          }}
          scroll={{ x: 1000, y: 520 }}
          size="small"
        />
      </Card>

      {/* 编辑模态框 */}
      <EditConfigModal
        open={state.editModalVisible}
        config={state.editingConfig}
        editValue={state.editValue}
        onEditValueChange={state.setEditValue}
        updateReason={state.updateReason}
        onUpdateReasonChange={state.setUpdateReason}
        loading={state.loading}
        onOk={state.handleUpdate}
        onCancel={state.closeEditModal}
      />
    </div>
  );
};

export default ConfigManagement;

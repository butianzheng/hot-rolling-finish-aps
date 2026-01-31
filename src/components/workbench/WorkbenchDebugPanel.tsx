/**
 * 工作台联动调试面板
 *
 * 功能：
 * - 实时显示联动状态
 * - 显示状态变化日志
 * - 提供快捷测试按钮
 *
 * 使用方法：
 * <WorkbenchDebugPanel syncState={syncState} syncApi={syncApi} />
 */

import React, { useState, useEffect, useCallback } from 'react';
import { Card, Tag, Space, Button, Collapse, Switch, Statistic, Row, Col } from 'antd';
import {
  BugOutlined,
  HistoryOutlined,
  UndoOutlined,
  RedoOutlined,
  ClearOutlined,
  EyeOutlined,
} from '@ant-design/icons';
import { WorkbenchSyncState, WorkbenchSyncAPI } from '@/hooks/useWorkbenchSync';
import dayjs from 'dayjs';

const { Panel } = Collapse;

interface WorkbenchDebugPanelProps {
  syncState: WorkbenchSyncState;
  syncApi: WorkbenchSyncAPI;
  visible?: boolean;
  onVisibleChange?: (visible: boolean) => void;
}

interface StateChangeLog {
  timestamp: string;
  action: string;
  changes: Record<string, any>;
}

export const WorkbenchDebugPanel: React.FC<WorkbenchDebugPanelProps> = ({
  syncState,
  syncApi,
  visible: controlledVisible,
  onVisibleChange,
}) => {
  const [logs, setLogs] = useState<StateChangeLog[]>([]);
  const [localVisible, setLocalVisible] = useState(false);
  const [prevState, setPrevState] = useState<WorkbenchSyncState>(syncState);

  const visible = controlledVisible !== undefined ? controlledVisible : localVisible;

  // 监听状态变化并记录日志
  useEffect(() => {
    const changes: Record<string, any> = {};

    // 对比状态变化
    if (prevState.machineCode !== syncState.machineCode) {
      changes.machineCode = `${prevState.machineCode || 'null'} → ${syncState.machineCode || 'null'}`;
    }

    if (prevState.selectedMaterialIds.length !== syncState.selectedMaterialIds.length) {
      changes.selectedMaterialIds = `${prevState.selectedMaterialIds.length} → ${syncState.selectedMaterialIds.length} 个`;
    }

    if (!prevState.dateRange[0].isSame(syncState.dateRange[0], 'day') ||
        !prevState.dateRange[1].isSame(syncState.dateRange[1], 'day')) {
      changes.dateRange = `${syncState.dateRange[0].format('YYYY-MM-DD')} ~ ${syncState.dateRange[1].format('YYYY-MM-DD')}`;
    }

    if (prevState.focusedMaterialId !== syncState.focusedMaterialId) {
      changes.focusedMaterial = `${prevState.focusedMaterialId || 'null'} → ${syncState.focusedMaterialId || 'null'}`;
    }

    if (Object.keys(changes).length > 0) {
      setLogs(prev => [
        {
          timestamp: new Date().toLocaleTimeString(),
          action: '状态变化',
          changes,
        },
        ...prev.slice(0, 49), // 保留最近 50 条
      ]);
    }

    setPrevState(syncState);
  }, [syncState, prevState]);

  const toggleVisible = useCallback(() => {
    const newVisible = !visible;
    setLocalVisible(newVisible);
    onVisibleChange?.(newVisible);
  }, [visible, onVisibleChange]);

  const clearLogs = useCallback(() => {
    setLogs([]);
  }, []);

  if (!visible) {
    return (
      <div
        style={{
          position: 'fixed',
          bottom: 20,
          right: 20,
          zIndex: 1000,
        }}
      >
        <Button
          type="primary"
          icon={<BugOutlined />}
          size="large"
          shape="circle"
          onClick={toggleVisible}
          title="打开联动调试面板"
        />
      </div>
    );
  }

  return (
    <div
      style={{
        position: 'fixed',
        bottom: 20,
        right: 20,
        width: 600,
        maxHeight: '80vh',
        overflowY: 'auto',
        zIndex: 1000,
        boxShadow: '0 4px 12px rgba(0,0,0,0.15)',
        borderRadius: 8,
      }}
    >
      <Card
        title={
          <Space>
            <BugOutlined />
            工作台联动调试
            <Switch
              checked={syncState.debugMode}
              onChange={() => syncApi.toggleDebugMode()}
              checkedChildren="Console 日志"
              unCheckedChildren="Console 日志"
              size="small"
            />
          </Space>
        }
        extra={
          <Button
            type="text"
            size="small"
            onClick={toggleVisible}
            icon={<EyeOutlined />}
          >
            最小化
          </Button>
        }
        size="small"
      >
        <Collapse defaultActiveKey={['state', 'history']} ghost>
          {/* 当前状态 */}
          <Panel header="当前状态" key="state">
            <Space direction="vertical" style={{ width: '100%' }} size="small">
              <Row gutter={16}>
                <Col span={12}>
                  <Statistic
                    title="选中机组"
                    value={syncState.machineCode || '全部'}
                    valueStyle={{ fontSize: 16 }}
                  />
                </Col>
                <Col span={12}>
                  <Statistic
                    title="选中物料"
                    value={syncState.selectedMaterialIds.length}
                    suffix="个"
                    valueStyle={{ fontSize: 16 }}
                  />
                </Col>
              </Row>

              <Row gutter={16}>
                <Col span={24}>
                  <Statistic
                    title="日期范围"
                    value={`${syncState.dateRange[0].format('YYYY-MM-DD')} ~ ${syncState.dateRange[1].format('YYYY-MM-DD')}`}
                    valueStyle={{ fontSize: 14 }}
                  />
                </Col>
              </Row>

              <Row gutter={[8, 8]}>
                <Col span={12}>
                  <Tag color={syncState.autoDateRange ? 'green' : 'orange'}>
                    {syncState.autoDateRange ? '自动日期范围' : '手动日期范围'}
                  </Tag>
                </Col>
                <Col span={12}>
                  <Tag color={syncState.focusedMaterialId ? 'blue' : 'default'}>
                    {syncState.focusedMaterialId ? `聚焦: ${syncState.focusedMaterialId.slice(0, 8)}...` : '无聚焦'}
                  </Tag>
                </Col>
              </Row>
            </Space>
          </Panel>

          {/* 历史操作 */}
          <Panel header={`历史操作 (${syncState.historyStack.length})`} key="history">
            <Space direction="vertical" style={{ width: '100%' }} size="small">
              <Space>
                <Button
                  size="small"
                  icon={<UndoOutlined />}
                  disabled={!syncApi.canUndo()}
                  onClick={() => syncApi.undo()}
                >
                  撤销
                </Button>
                <Button
                  size="small"
                  icon={<RedoOutlined />}
                  disabled={!syncApi.canRedo()}
                  onClick={() => syncApi.redo()}
                >
                  重做
                </Button>
                <Tag>可撤销: {syncState.historyStack.length}</Tag>
                <Tag>可重做: {syncState.futureStack.length}</Tag>
              </Space>
            </Space>
          </Panel>

          {/* 变化日志 */}
          <Panel
            header={
              <Space>
                <HistoryOutlined />
                变化日志 ({logs.length})
                <Button
                  type="link"
                  size="small"
                  icon={<ClearOutlined />}
                  onClick={(e) => {
                    e.stopPropagation();
                    clearLogs();
                  }}
                >
                  清空
                </Button>
              </Space>
            }
            key="logs"
          >
            <div style={{ maxHeight: 300, overflowY: 'auto' }}>
              <Space direction="vertical" style={{ width: '100%' }} size="small">
                {logs.length === 0 && (
                  <div style={{ textAlign: 'center', color: '#999', padding: '20px 0' }}>
                    暂无日志
                  </div>
                )}

                {logs.map((log, index) => (
                  <Card
                    key={index}
                    size="small"
                    style={{ backgroundColor: '#fafafa' }}
                  >
                    <div style={{ fontSize: 12, marginBottom: 4 }}>
                      <Tag color="blue" style={{ marginRight: 8 }}>
                        {log.timestamp}
                      </Tag>
                      {log.action}
                    </div>
                    {Object.entries(log.changes).map(([key, value]) => (
                      <div key={key} style={{ fontSize: 11, color: '#666', marginLeft: 8 }}>
                        <strong>{key}:</strong> {value}
                      </div>
                    ))}
                  </Card>
                ))}
              </Space>
            </div>
          </Panel>

          {/* 快捷测试 */}
          <Panel header="快捷测试" key="test">
            <Space wrap>
              <Button
                size="small"
                onClick={() => syncApi.selectMachine('H031')}
              >
                选择 H031
              </Button>
              <Button
                size="small"
                onClick={() => syncApi.selectMachine(null)}
              >
                取消机组选择
              </Button>
              <Button
                size="small"
                onClick={() => syncApi.clearSelection()}
              >
                清空物料选择
              </Button>
              <Button
                size="small"
                onClick={() => syncApi.resetDateRangeToAuto()}
              >
                重置日期范围
              </Button>
            </Space>
          </Panel>
        </Collapse>
      </Card>
    </div>
  );
};

export default WorkbenchDebugPanel;

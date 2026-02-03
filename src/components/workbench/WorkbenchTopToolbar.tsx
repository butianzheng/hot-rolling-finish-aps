import React from 'react';
import { Button, Card, Dropdown, Space, Typography } from 'antd';
import { DownOutlined, ReloadOutlined, SettingOutlined } from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';

import BatchOperationToolbar from './BatchOperationToolbar';
import OneClickOptimizeMenu from './OneClickOptimizeMenu';
import type { MaterialOperationType } from '../../pages/workbench/types';

const WorkbenchTopToolbar: React.FC<{
  activeVersionId: string;
  currentUser: string;
  selectedMaterialIds: string[];
  onRefresh: () => void;
  onOpenRhythmModal: () => void;
  onOpenConditionalSelect: () => void;
  onClearSelection: () => void;
  openMoveModal: () => void;
  runMaterialOperation: (materialIds: string[], type: MaterialOperationType) => void;
  runForceReleaseOperation: (materialIds: string[]) => void;
  onBeforeOptimize: () => void;
  onAfterOptimize: () => void;
}> = ({
  activeVersionId,
  currentUser,
  selectedMaterialIds,
  onRefresh,
  onOpenRhythmModal,
  onOpenConditionalSelect,
  onClearSelection,
  openMoveModal,
  runMaterialOperation,
  runForceReleaseOperation,
  onBeforeOptimize,
  onAfterOptimize,
}) => {
  const navigate = useNavigate();

  const selectionDisabled = selectedMaterialIds.length === 0;

  return (
    <Card size="small">
      <Space wrap align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Space wrap>
          <Typography.Text type="secondary">当前版本</Typography.Text>
          <Typography.Text code>{activeVersionId || '-'}</Typography.Text>
          <Button size="small" icon={<ReloadOutlined />} onClick={onRefresh}>
            刷新
          </Button>
        </Space>

        <Space wrap>
          <Button onClick={() => navigate('/comparison')}>版本管理</Button>
          <Button onClick={() => navigate('/comparison?tab=draft')}>生成策略对比方案</Button>
          <Button onClick={onOpenRhythmModal}>每日节奏</Button>
          <BatchOperationToolbar
            disabled={selectionDisabled}
            onLock={() => runMaterialOperation(selectedMaterialIds, 'lock')}
            onUnlock={() => runMaterialOperation(selectedMaterialIds, 'unlock')}
            onSetUrgent={() => runMaterialOperation(selectedMaterialIds, 'urgent_on')}
            onClearUrgent={() => runMaterialOperation(selectedMaterialIds, 'urgent_off')}
            onForceRelease={() => runForceReleaseOperation(selectedMaterialIds)}
            onMove={openMoveModal}
            onConditional={onOpenConditionalSelect}
            onClear={onClearSelection}
          />
          <Dropdown
            menu={{
              onClick: ({ key }) => navigate(`/settings?tab=${key}`),
              items: [
                { key: 'materials', label: '物料管理（表格）' },
                { key: 'machine', label: '机组配置（产能池）' },
                { type: 'divider' },
                { key: 'system', label: '系统配置' },
                { key: 'path_rule', label: '路径规则（v0.6）' },
                { key: 'logs', label: '操作日志' },
              ],
            }}
          >
            <Button icon={<SettingOutlined />}>
              设置/工具 <DownOutlined />
            </Button>
          </Dropdown>
          <OneClickOptimizeMenu
            activeVersionId={activeVersionId}
            operator={currentUser}
            onBeforeExecute={onBeforeOptimize}
            onAfterExecute={onAfterOptimize}
          />
        </Space>
      </Space>
    </Card>
  );
};

export default React.memo(WorkbenchTopToolbar);

/**
 * MaterialInspector - 主组件
 *
 * 重构后：335 行 → ~90 行 (-73%)
 */

import React from 'react';
import { Drawer, Space, Button, Typography, Divider, Empty } from 'antd';
import { CloseOutlined } from '@ant-design/icons';
import { MaterialStatusIcons } from '../MaterialStatusIcons';
import type { MaterialInspectorProps } from './types';
import { useMaterialInspector } from './useMaterialInspector';
import { BasicInfoSection } from './BasicInfoSection';
import { StatusInfoSection } from './StatusInfoSection';
import { EngineReasonSection } from './EngineReasonSection';
import { ActionHistorySection } from './ActionHistorySection';

const { Title, Text } = Typography;

export const MaterialInspector: React.FC<MaterialInspectorProps> = ({
  visible,
  material,
  onClose,
  onLock,
  onUnlock,
  onSetUrgent,
  onClearUrgent,
}) => {
  const { actionLogs, loadingLogs } = useMaterialInspector(visible, material);

  if (!material) {
    return (
      <Drawer
        title="材料详情"
        placement="right"
        width={480}
        onClose={onClose}
        open={visible}
        closeIcon={<CloseOutlined />}
      >
        <Empty description="未选择材料" />
      </Drawer>
    );
  }

  return (
    <Drawer
      title={
        <Space>
          <Text strong>材料详情</Text>
          <MaterialStatusIcons
            lockFlag={material.lock_flag}
            schedState={material.sched_state}
            tempIssue={material.temp_issue || !material.is_mature}
          />
        </Space>
      }
      placement="right"
      width={480}
      onClose={onClose}
      open={visible}
      closeIcon={<CloseOutlined />}
      extra={
        <Space>
          {material.lock_flag ? (
            <Button size="small" onClick={() => onUnlock?.(material.material_id)}>
              解锁
            </Button>
          ) : (
            <Button size="small" onClick={() => onLock?.(material.material_id)}>
              锁定
            </Button>
          )}
          {material.manual_urgent_flag ? (
            <Button
              size="small"
              onClick={() => onClearUrgent?.(material.material_id)}
              disabled={!onClearUrgent}
            >
              取消紧急
            </Button>
          ) : (
            <Button
              size="small"
              type="primary"
              danger
              onClick={() => onSetUrgent?.(material.material_id)}
              disabled={!onSetUrgent}
            >
              设为紧急
            </Button>
          )}
        </Space>
      }
    >
      {/* 基本信息 */}
      <Title level={5}>基本信息</Title>
      <BasicInfoSection material={material} />

      <Divider />

      {/* 状态信息 */}
      <Title level={5}>状态信息</Title>
      <StatusInfoSection material={material} />

      <Divider />

      {/* 引擎推理原因 */}
      <EngineReasonSection material={material} />

      <Divider />

      {/* 操作历史 */}
      <ActionHistorySection actionLogs={actionLogs} loading={loadingLogs} />
    </Drawer>
  );
};

export default MaterialInspector;

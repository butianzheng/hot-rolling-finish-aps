// ==========================================
// 材料状态图标组件
// ==========================================
// 显示锁定、温度等状态图标
// 使用 React.memo 优化性能
// ==========================================

import React from 'react';
import { Tooltip, Space } from 'antd';
import { LockOutlined, FireOutlined, CheckCircleOutlined } from '@ant-design/icons';
import { STATE_COLORS } from '../theme';

interface MaterialStatusIconsProps {
  lockFlag: boolean;
  schedState: string;
  tempIssue?: boolean;
}

const MaterialStatusIconsComponent: React.FC<MaterialStatusIconsProps> = ({
  lockFlag,
  schedState,
  tempIssue = false,
}) => {
  return (
    <Space size={4}>
      {lockFlag && (
        <Tooltip title="已锁定 - 不可自动调整">
          <LockOutlined style={{ color: STATE_COLORS.FROZEN_LOCKED, fontSize: 14 }} />
        </Tooltip>
      )}

      {tempIssue && (
        <Tooltip title="温度问题 - 需要冷却">
          <FireOutlined style={{ color: STATE_COLORS.TEMP_ISSUE, fontSize: 14 }} />
        </Tooltip>
      )}

      {schedState === 'Scheduled' && (
        <Tooltip title="已排产">
          <CheckCircleOutlined style={{ color: STATE_COLORS.READY, fontSize: 14 }} />
        </Tooltip>
      )}
    </Space>
  );
};

// 使用 React.memo 优化，只在 props 改变时重新渲染
export const MaterialStatusIcons = React.memo(MaterialStatusIconsComponent);

/**
 * 操作状态徽章组件
 *
 * 显示物料的可操作性状态（🟢就绪可动、🟡已排需谨慎等）
 */

import React from 'react';
import { Badge, Tooltip } from 'antd';
import type { OperabilityStatus } from '../../utils/operabilityStatus';
import { getOperabilityConfig } from '../../utils/operabilityStatus';

export interface OperabilityBadgeProps {
  status: OperabilityStatus;
  showEmoji?: boolean;
  showLabel?: boolean;
  size?: 'small' | 'default';
}

export const OperabilityBadge: React.FC<OperabilityBadgeProps> = ({
  status,
  showEmoji = true,
  showLabel = true,
  size = 'default',
}) => {
  const config = getOperabilityConfig(status);

  const content = (
    <Badge
      color={config.color}
      text={
        <span style={{ fontSize: size === 'small' ? 12 : 14, fontWeight: 500 }}>
          {showEmoji && `${config.emoji} `}
          {showLabel && config.label}
        </span>
      }
    />
  );

  return (
    <Tooltip title={config.description}>
      {content}
    </Tooltip>
  );
};

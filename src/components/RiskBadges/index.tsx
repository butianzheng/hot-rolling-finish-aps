/**
 * 风险标记组件
 *
 * 显示物料的风险标记列表（超产能、L3紧急、轧辊换辊等）
 */

import React from 'react';
import { Tag, Tooltip, Space } from 'antd';
import {
  WarningOutlined,
  LockOutlined,
  FireOutlined,
  ThunderboltOutlined,
  ToolOutlined,
  ClockCircleOutlined,
} from '@ant-design/icons';
import type { RiskBadge, RiskBadgeType } from '../../utils/operabilityStatus';
import { getRiskSeverityColor } from '../../utils/operabilityStatus';

export interface RiskBadgesProps {
  badges: RiskBadge[];
  maxDisplay?: number;
  size?: 'small' | 'default';
}

const BADGE_ICONS: Record<RiskBadgeType, React.ReactNode> = {
  CAPACITY_OVERFLOW: <WarningOutlined />,
  L3_URGENT: <ThunderboltOutlined />,
  L2_URGENT: <FireOutlined />,
  ROLL_CHANGE_RISK: <ToolOutlined />,
  TEMP_ISSUE: <ClockCircleOutlined />,
  AGE_WARNING: <ClockCircleOutlined />,
  LOCKED_MANUAL: <LockOutlined />,
  FROZEN_ZONE: <LockOutlined />,
};

export const RiskBadges: React.FC<RiskBadgesProps> = ({
  badges,
  maxDisplay = 3,
  size = 'small',
}) => {
  if (badges.length === 0) {
    return null;
  }

  const displayBadges = badges.slice(0, maxDisplay);
  const hiddenCount = badges.length - maxDisplay;

  return (
    <Space size={4}>
      {displayBadges.map((badge, index) => (
        <Tooltip key={index} title={badge.tooltip || badge.label}>
          <Tag
            color={getRiskSeverityColor(badge.severity)}
            icon={BADGE_ICONS[badge.type]}
            style={{
              fontSize: size === 'small' ? 11 : 12,
              margin: 0,
              padding: size === 'small' ? '0 4px' : '2px 7px',
            }}
          >
            {badge.label}
          </Tag>
        </Tooltip>
      ))}
      {hiddenCount > 0 && (
        <Tooltip
          title={
            <div>
              其他风险：
              <ul style={{ margin: '4px 0', paddingLeft: 16 }}>
                {badges.slice(maxDisplay).map((badge, index) => (
                  <li key={index}>{badge.label}</li>
                ))}
              </ul>
            </div>
          }
        >
          <Tag style={{ fontSize: size === 'small' ? 11 : 12, margin: 0 }}>
            +{hiddenCount}
          </Tag>
        </Tooltip>
      )}
    </Space>
  );
};

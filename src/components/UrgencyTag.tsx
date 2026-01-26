// ==========================================
// 紧急度标签组件
// ==========================================
// 显示 L0-L3 紧急度等级，带 Tooltip 说明
// 使用 React.memo 优化性能
// ==========================================

import React, { useMemo } from 'react';
import { Tag, Tooltip } from 'antd';
import { URGENCY_COLORS } from '../theme';

interface UrgencyTagProps {
  level: string;
  reason?: string;
}

const URGENCY_CONFIG = {
  L3: {
    label: 'L3',
    color: URGENCY_COLORS.L3_EMERGENCY,
    description: '紧急/红线',
  },
  L2: {
    label: 'L2',
    color: URGENCY_COLORS.L2_HIGH,
    description: '高优先级',
  },
  L1: {
    label: 'L1',
    color: URGENCY_COLORS.L1_MEDIUM,
    description: '中等优先级',
  },
  L0: {
    label: 'L0',
    color: URGENCY_COLORS.L0_NORMAL,
    description: '正常',
  },
};

const UrgencyTagComponent: React.FC<UrgencyTagProps> = ({ level, reason }) => {
  const config = useMemo(
    () => URGENCY_CONFIG[level as keyof typeof URGENCY_CONFIG] || URGENCY_CONFIG.L0,
    [level]
  );

  const tooltipContent = useMemo(
    () => (
      <div>
        <div style={{ fontWeight: 'bold', marginBottom: 4 }}>
          {config.label} - {config.description}
        </div>
        {reason && (
          <div style={{ fontSize: 12, fontStyle: 'italic', opacity: 0.85 }}>
            原因: {reason}
          </div>
        )}
      </div>
    ),
    [config, reason]
  );

  return (
    <Tooltip title={tooltipContent}>
      <Tag
        color={config.color}
        style={{
          fontWeight: 'bold',
          fontSize: 12,
          padding: '2px 8px',
          border: 'none',
          cursor: 'help',
        }}
      >
        {config.label}
      </Tag>
    </Tooltip>
  );
};

// 使用 React.memo 优化，只在 props 改变时重新渲染
export const UrgencyTag = React.memo(UrgencyTagComponent);

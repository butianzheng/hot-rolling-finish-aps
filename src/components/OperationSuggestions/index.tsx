/**
 * 操作建议组件
 *
 * 显示物料的可执行操作列表
 */

import React from 'react';
import { Button, Space, Tooltip } from 'antd';
import {
  CalendarOutlined,
  ArrowRightOutlined,
  LockOutlined,
  UnlockOutlined,
  ThunderboltOutlined,
  CheckCircleOutlined,
  EyeOutlined,
} from '@ant-design/icons';
import type { Operation, OperationType } from '../../utils/operabilityStatus';

export interface OperationSuggestionsProps {
  operations: Operation[];
  onOperationClick?: (operation: Operation) => void;
  compact?: boolean;
  maxDisplay?: number;
}

const OPERATION_ICONS: Record<OperationType, React.ReactNode> = {
  SCHEDULE_TO: <CalendarOutlined />,
  MOVE_TO: <ArrowRightOutlined />,
  SET_URGENT: <ThunderboltOutlined />,
  LOCK: <LockOutlined />,
  UNLOCK: <UnlockOutlined />,
  FORCE_RELEASE: <CheckCircleOutlined />,
  CANCEL_FORCE: <CheckCircleOutlined />,
  VIEW_DETAILS: <EyeOutlined />,
};

export const OperationSuggestions: React.FC<OperationSuggestionsProps> = ({
  operations,
  onOperationClick,
  compact = false,
  maxDisplay = 3,
}) => {
  if (operations.length === 0) {
    return null;
  }

  const displayOperations = compact ? operations.slice(0, maxDisplay) : operations;

  return (
    <Space size={4} wrap>
      {displayOperations.map((op, index) => {
        const button = (
          <Button
            key={index}
            type={op.priority === 'primary' ? 'primary' : 'default'}
            danger={op.priority === 'danger'}
            size="small"
            icon={OPERATION_ICONS[op.type]}
            disabled={op.disabled}
            onClick={() => onOperationClick?.(op)}
            style={{ fontSize: 12 }}
          >
            {compact ? null : op.label}
          </Button>
        );

        const tooltipTitle = op.warning ? (
          <div>
            <div>{op.tooltip || op.label}</div>
            <div style={{ color: '#faad14', marginTop: 4 }}>
              ⚠️ {op.warning}
            </div>
          </div>
        ) : (
          op.tooltip || op.label
        );

        return op.tooltip || op.warning ? (
          <Tooltip key={index} title={tooltipTitle}>
            {button}
          </Tooltip>
        ) : (
          button
        );
      })}
    </Space>
  );
};

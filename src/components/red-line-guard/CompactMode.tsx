/**
 * RedLineGuard 紧凑模式组件
 */

import React from 'react';
import { Space, Tag, Tooltip } from 'antd';
import type { RedLineViolation } from './types';
import { RED_LINE_META } from './types';

interface CompactModeProps {
  violations: RedLineViolation[];
}

export const CompactMode: React.FC<CompactModeProps> = ({ violations }) => {
  return (
    <Space size="small" wrap>
      {violations.map((violation, index) => {
        const meta = RED_LINE_META[violation.type];
        return (
          <Tooltip
            key={index}
            title={
              <div>
                <div style={{ fontWeight: 'bold', marginBottom: '4px' }}>
                  {meta.label}
                </div>
                <div>{violation.message}</div>
                {violation.details && (
                  <div style={{ marginTop: '4px', fontSize: '12px' }}>
                    {violation.details}
                  </div>
                )}
              </div>
            }
          >
            <Tag
              color={violation.severity === 'error' ? 'red' : 'orange'}
              icon={meta.icon}
            >
              {meta.label}
            </Tag>
          </Tooltip>
        );
      })}
    </Space>
  );
};

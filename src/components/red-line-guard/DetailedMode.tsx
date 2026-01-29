/**
 * RedLineGuard 详细模式组件
 */

import React from 'react';
import { Alert, Space, Tag } from 'antd';
import { InfoCircleOutlined, WarningOutlined } from '@ant-design/icons';
import type { RedLineViolation } from './types';
import { RED_LINE_META } from './types';

interface DetailedModeProps {
  violations: RedLineViolation[];
  closable?: boolean;
  onClose?: () => void;
}

export const DetailedMode: React.FC<DetailedModeProps> = ({
  violations,
  closable = false,
  onClose,
}) => {
  return (
    <Space direction="vertical" style={{ width: '100%' }}>
      {violations.map((violation, index) => {
        const meta = RED_LINE_META[violation.type];
        return (
          <Alert
            key={index}
            type={violation.severity === 'error' ? 'error' : 'warning'}
            showIcon
            icon={
              violation.severity === 'error' ? (
                <WarningOutlined />
              ) : (
                <InfoCircleOutlined />
              )
            }
            message={
              <Space>
                {meta.icon}
                <span style={{ fontWeight: 'bold' }}>
                  工业红线：{meta.label}
                </span>
              </Space>
            }
            description={
              <div>
                <div style={{ marginBottom: '8px' }}>{violation.message}</div>
                {violation.details && (
                  <div
                    style={{
                      marginBottom: '8px',
                      fontSize: '12px',
                      color: '#8c8c8c',
                    }}
                  >
                    {violation.details}
                  </div>
                )}
                {violation.affectedEntities &&
                  violation.affectedEntities.length > 0 && (
                    <div>
                      <div
                        style={{
                          fontSize: '12px',
                          marginBottom: '4px',
                          color: '#8c8c8c',
                        }}
                      >
                        受影响实体:
                      </div>
                      <Space size="small" wrap>
                        {violation.affectedEntities.map((entity, idx) => (
                          <Tag key={idx} color="default">
                            {entity}
                          </Tag>
                        ))}
                      </Space>
                    </div>
                  )}
                <div
                  style={{
                    marginTop: '8px',
                    fontSize: '12px',
                    fontStyle: 'italic',
                    color: '#8c8c8c',
                  }}
                >
                  {meta.description}
                </div>
              </div>
            }
            closable={closable}
            onClose={onClose}
            style={{ marginBottom: index < violations.length - 1 ? '12px' : 0 }}
          />
        );
      })}
    </Space>
  );
};

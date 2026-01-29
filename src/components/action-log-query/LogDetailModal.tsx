/**
 * 操作日志详情模态框
 */

import React from 'react';
import { Button, Card, Descriptions, Modal, Tag } from 'antd';
import type { ActionLog } from './types';
import { actionTypeLabels } from './types';

export interface LogDetailModalProps {
  open: boolean;
  log: ActionLog | null;
  onClose: () => void;
}

export const LogDetailModal: React.FC<LogDetailModalProps> = ({
  open,
  log,
  onClose,
}) => {
  return (
    <Modal
      title="操作日志详情"
      open={open}
      onCancel={onClose}
      footer={[
        <Button key="close" onClick={onClose}>
          关闭
        </Button>,
      ]}
      width={800}
    >
      {log && (
        <div>
          <Descriptions bordered column={2} size="small">
            <Descriptions.Item label="操作ID" span={2}>
              {log.action_id}
            </Descriptions.Item>
            <Descriptions.Item label="操作时间" span={2}>
              {log.action_ts}
            </Descriptions.Item>
            <Descriptions.Item label="操作类型">
              <Tag color={actionTypeLabels[log.action_type]?.color || 'default'}>
                {actionTypeLabels[log.action_type]?.text || log.action_type}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label="操作人">
              {log.actor}
            </Descriptions.Item>
            <Descriptions.Item label="版本ID">
              {log.version_id}
            </Descriptions.Item>
            <Descriptions.Item label="机组">
              {log.machine_code || '-'}
            </Descriptions.Item>
            <Descriptions.Item label="操作详情" span={2}>
              {log.detail}
            </Descriptions.Item>
          </Descriptions>

          {/* Payload JSON */}
          {log.payload_json && (
            <Card title="操作参数 (Payload)" size="small" style={{ marginTop: 16 }}>
              <pre style={{ maxHeight: '200px', overflow: 'auto', fontSize: '12px' }}>
                {JSON.stringify(log.payload_json, null, 2)}
              </pre>
            </Card>
          )}

          {/* Impact Summary JSON */}
          {log.impact_summary_json && (
            <Card title="影响摘要 (Impact Summary)" size="small" style={{ marginTop: 16 }}>
              <pre style={{ maxHeight: '200px', overflow: 'auto', fontSize: '12px' }}>
                {JSON.stringify(log.impact_summary_json, null, 2)}
              </pre>
            </Card>
          )}
        </div>
      )}
    </Modal>
  );
};

export default LogDetailModal;

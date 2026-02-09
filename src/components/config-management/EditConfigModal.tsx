/**
 * 编辑配置模态框
 */

import React from 'react';
import { Descriptions, Input, Modal, Space, Tag } from 'antd';
import type { ConfigItem } from './types';
import { scopeTypeColors, scopeTypeLabels, configDescriptions } from './types';

const FRONTEND_RUNTIME_CONFIG_HINTS: Record<string, string> = {
  latest_run_ttl_ms: '建议范围 5000~900000（毫秒）',
  stale_plan_rev_toast_cooldown_ms: '建议范围 1000~60000（毫秒）',
};

export interface EditConfigModalProps {
  open: boolean;
  config: ConfigItem | null;
  editValue: string;
  onEditValueChange: (value: string) => void;
  updateReason: string;
  onUpdateReasonChange: (reason: string) => void;
  loading: boolean;
  onOk: () => void;
  onCancel: () => void;
}

export const EditConfigModal: React.FC<EditConfigModalProps> = ({
  open,
  config,
  editValue,
  onEditValueChange,
  updateReason,
  onUpdateReasonChange,
  loading,
  onOk,
  onCancel,
}) => {
  return (
    <Modal
      title="编辑配置"
      open={open}
      onOk={onOk}
      onCancel={onCancel}
      confirmLoading={loading}
      width={600}
    >
      {config && (
        <Space direction="vertical" style={{ width: '100%' }}>
          <Descriptions bordered column={1} size="small">
            <Descriptions.Item label="作用域类型">
              <Tag color={scopeTypeColors[config.scope_type]}>
                {scopeTypeLabels[config.scope_type] || config.scope_type}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label="作用域编号">
              {config.scope_id}
            </Descriptions.Item>
            <Descriptions.Item label="配置键">
              {config.key}
            </Descriptions.Item>
            <Descriptions.Item label="配置说明">
              {configDescriptions[config.key] || '无描述'}
            </Descriptions.Item>
          </Descriptions>

          <div style={{ marginTop: 16 }}>
            <label>配置值：</label>
            <Input
              style={{ marginTop: 8 }}
              value={editValue}
              onChange={(e) => onEditValueChange(e.target.value)}
              placeholder="请输入配置值"
            />
            {FRONTEND_RUNTIME_CONFIG_HINTS[config.key] ? (
              <div style={{ marginTop: 6, color: '#8c8c8c', fontSize: 12 }}>
                {FRONTEND_RUNTIME_CONFIG_HINTS[config.key]}
              </div>
            ) : null}
          </div>

          <div>
            <label>修改原因（必填）：</label>
            <Input.TextArea
              style={{ marginTop: 8 }}
              placeholder="请输入修改原因"
              value={updateReason}
              onChange={(e) => onUpdateReasonChange(e.target.value)}
              rows={3}
            />
          </div>
        </Space>
      )}
    </Modal>
  );
};

export default EditConfigModal;

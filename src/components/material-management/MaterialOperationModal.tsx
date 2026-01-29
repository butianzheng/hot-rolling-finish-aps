/**
 * 操作确认模态框
 */

import React from 'react';
import { Alert, Input, Modal, Space } from 'antd';
import { WarningOutlined } from '@ant-design/icons';
import type { OperationType } from './materialTypes';
import { getOperationModalTitle } from './materialTypes';

const { TextArea } = Input;

export interface MaterialOperationModalProps {
  open: boolean;
  modalType: OperationType;
  selectedCount: number;
  reason: string;
  adminOverrideMode: boolean;
  onReasonChange: (reason: string) => void;
  onOk: () => void;
  onCancel: () => void;
}

export const MaterialOperationModal: React.FC<MaterialOperationModalProps> = ({
  open,
  modalType,
  selectedCount,
  reason,
  adminOverrideMode,
  onReasonChange,
  onOk,
  onCancel,
}) => {
  return (
    <Modal
      title={getOperationModalTitle(modalType, selectedCount)}
      open={open}
      onOk={onOk}
      onCancel={onCancel}
      okText="确认"
      cancelText="取消"
      okButtonProps={{
        danger: adminOverrideMode,
      }}
    >
      <Space direction="vertical" style={{ width: '100%' }} size={16}>
        {/* 管理员覆盖模式警告 */}
        {adminOverrideMode && (
          <div
            style={{
              padding: 12,
              backgroundColor: '#fff2e8',
              border: '1px solid #ffbb96',
              borderRadius: 4,
            }}
          >
            <Space>
              <WarningOutlined style={{ color: '#ff4d4f', fontSize: 16 }} />
              <div>
                <div style={{ fontWeight: 'bold', color: '#ff4d4f' }}>管理员覆盖模式已启用</div>
                <div style={{ fontSize: 12, color: '#8c8c8c', marginTop: 4 }}>
                  此操作将绕过 Red Line 保护规则。请确保您了解操作的影响。
                </div>
              </div>
            </Space>
          </div>
        )}

        {!adminOverrideMode && modalType === 'forceRelease' && (
          <Alert
            type="info"
            showIcon
            message="提示"
            description={'严格模式下，若存在未适温材料会阻止"强制放行"。如需放行未适温材料，请开启"管理员覆盖模式"。'}
          />
        )}

        <div>
          <div style={{ marginBottom: 8, fontWeight: 'bold' }}>操作原因:</div>
          <TextArea
            rows={4}
            placeholder="请输入操作原因（必填）"
            value={reason}
            onChange={(e) => onReasonChange(e.target.value)}
            maxLength={500}
            showCount
          />
        </div>
      </Space>
    </Modal>
  );
};

export default MaterialOperationModal;

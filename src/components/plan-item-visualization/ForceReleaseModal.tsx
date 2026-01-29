/**
 * 强制放行模态框
 */

import React from 'react';
import { Input, Modal, Select, Space } from 'antd';

const { Option } = Select;

export interface ForceReleaseModalProps {
  open: boolean;
  selectedCount: number;
  reason: string;
  onReasonChange: (reason: string) => void;
  mode: 'AutoFix' | 'Strict';
  onModeChange: (mode: 'AutoFix' | 'Strict') => void;
  loading: boolean;
  onOk: () => void;
  onCancel: () => void;
}

export const ForceReleaseModal: React.FC<ForceReleaseModalProps> = ({
  open,
  selectedCount,
  reason,
  onReasonChange,
  mode,
  onModeChange,
  loading,
  onOk,
  onCancel,
}) => {
  return (
    <Modal
      title="批量强制放行"
      open={open}
      onOk={onOk}
      onCancel={onCancel}
      confirmLoading={loading}
    >
      <Space direction="vertical" style={{ width: '100%' }}>
        <p>即将强制放行 {selectedCount} 个材料</p>
        <Space wrap>
          <span>校验模式</span>
          <Select
            value={mode}
            onChange={(v) => onModeChange(v as 'AutoFix' | 'Strict')}
            style={{ width: 220 }}
          >
            <Option value="AutoFix">AUTO_FIX（允许未适温）</Option>
            <Option value="Strict">STRICT（未适温则失败）</Option>
          </Select>
        </Space>
        <p style={{ margin: 0, fontSize: 12, color: '#8c8c8c' }}>
          提示：STRICT 遇到未适温材料会失败；AUTO_FIX 允许放行并记录警告（可审计）。
        </p>
        <Input.TextArea
          placeholder="请输入强制放行原因(必填)"
          value={reason}
          onChange={(e) => onReasonChange(e.target.value)}
          rows={4}
        />
      </Space>
    </Modal>
  );
};

export default ForceReleaseModal;

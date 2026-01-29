/**
 * 批量调整产能池模态框
 */

import React from 'react';
import { Alert, Input, InputNumber, Modal, Space } from 'antd';

export interface BatchEditModalProps {
  open: boolean;
  selectedCount: number;
  targetCapacity: number | null;
  onTargetCapacityChange: (value: number | null) => void;
  limitCapacity: number | null;
  onLimitCapacityChange: (value: number | null) => void;
  reason: string;
  onReasonChange: (reason: string) => void;
  loading: boolean;
  onOk: () => void;
  onCancel: () => void;
}

export const BatchEditModal: React.FC<BatchEditModalProps> = ({
  open,
  selectedCount,
  targetCapacity,
  onTargetCapacityChange,
  limitCapacity,
  onLimitCapacityChange,
  reason,
  onReasonChange,
  loading,
  onOk,
  onCancel,
}) => {
  return (
    <Modal
      title={`批量调整产能池（已选 ${selectedCount} 条）`}
      open={open}
      onOk={onOk}
      onCancel={onCancel}
      okButtonProps={{ disabled: selectedCount === 0 }}
      confirmLoading={loading}
    >
      <Space direction="vertical" style={{ width: '100%' }}>
        <Alert
          type="info"
          showIcon
          message="提示"
          description="留空表示保持原值；批量提交后会自动触发一次决策刷新（可在Header看到刷新状态）。"
        />
        <div>
          <label>目标产能(吨)：</label>
          <InputNumber
            style={{ width: '100%', marginTop: 8 }}
            min={0}
            max={10000}
            value={targetCapacity}
            placeholder="留空表示不改"
            onChange={(val) => onTargetCapacityChange(typeof val === 'number' ? val : null)}
          />
        </div>
        <div>
          <label>极限产能(吨)：</label>
          <InputNumber
            style={{ width: '100%', marginTop: 8 }}
            min={0}
            max={10000}
            value={limitCapacity}
            placeholder="留空表示不改"
            onChange={(val) => onLimitCapacityChange(typeof val === 'number' ? val : null)}
          />
        </div>
        <div>
          <label>调整原因(必填)：</label>
          <Input.TextArea
            style={{ marginTop: 8 }}
            placeholder="请输入调整原因"
            value={reason}
            onChange={(e) => onReasonChange(e.target.value)}
            rows={3}
          />
        </div>
      </Space>
    </Modal>
  );
};

export default BatchEditModal;

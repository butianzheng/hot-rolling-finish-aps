/**
 * 编辑产能池模态框
 */

import React from 'react';
import { Input, InputNumber, Modal, Space } from 'antd';
import type { CapacityPool } from './types';

export interface EditCapacityModalProps {
  open: boolean;
  pool: CapacityPool | null;
  targetCapacity: number;
  onTargetCapacityChange: (value: number) => void;
  limitCapacity: number;
  onLimitCapacityChange: (value: number) => void;
  reason: string;
  onReasonChange: (reason: string) => void;
  loading: boolean;
  onOk: () => void;
  onCancel: () => void;
}

export const EditCapacityModal: React.FC<EditCapacityModalProps> = ({
  open,
  pool,
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
      title="调整产能池"
      open={open}
      onOk={onOk}
      onCancel={onCancel}
      confirmLoading={loading}
    >
      {pool && (
        <Space direction="vertical" style={{ width: '100%' }}>
          <div>
            <strong>机组:</strong> {pool.machine_code}
          </div>
          <div>
            <strong>日期:</strong> {pool.plan_date}
          </div>
          <div>
            <label>目标产能(吨):</label>
            <InputNumber
              style={{ width: '100%', marginTop: 8 }}
              min={0}
              max={10000}
              value={targetCapacity}
              onChange={(val) => onTargetCapacityChange(val || 0)}
            />
          </div>
          <div>
            <label>极限产能(吨):</label>
            <InputNumber
              style={{ width: '100%', marginTop: 8 }}
              min={0}
              max={10000}
              value={limitCapacity}
              onChange={(val) => onLimitCapacityChange(val || 0)}
            />
          </div>
          <div>
            <label>调整原因(必填):</label>
            <Input.TextArea
              style={{ marginTop: 8 }}
              placeholder="请输入调整原因"
              value={reason}
              onChange={(e) => onReasonChange(e.target.value)}
              rows={3}
            />
          </div>
        </Space>
      )}
    </Modal>
  );
};

export default EditCapacityModal;

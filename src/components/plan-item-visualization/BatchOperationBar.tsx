/**
 * 批量操作栏组件
 */

import React from 'react';
import { Button, Card, Space } from 'antd';

export interface BatchOperationBarProps {
  selectedCount: number;
  onForceRelease: () => void;
  onCancelSelection: () => void;
}

export const BatchOperationBar: React.FC<BatchOperationBarProps> = ({
  selectedCount,
  onForceRelease,
  onCancelSelection,
}) => {
  if (selectedCount === 0) return null;

  return (
    <Card style={{ marginBottom: 16, backgroundColor: '#e6f7ff' }}>
      <Space>
        <span>已选择 {selectedCount} 个材料</span>
        <Button type="primary" onClick={onForceRelease}>
          批量强制放行
        </Button>
        <Button onClick={onCancelSelection}>取消选择</Button>
      </Space>
    </Card>
  );
};

export default BatchOperationBar;

import React from 'react';
import { Button, Card, Space, Typography } from 'antd';

const WorkbenchStatusBar: React.FC<{
  selectedMaterialCount: number;
  selectedTotalWeight: number;
  disabled: boolean;
  onLock: () => void;
  onUnlock: () => void;
  onSetUrgent: () => void;
  onClearUrgent: () => void;
  onForceRelease: () => void;
  onOpenMoveRecommend: () => void;
  onOpenMoveModal: () => void;
  onClearSelection: () => void;
}> = ({
  selectedMaterialCount,
  selectedTotalWeight,
  disabled,
  onLock,
  onUnlock,
  onSetUrgent,
  onClearUrgent,
  onForceRelease,
  onOpenMoveRecommend,
  onOpenMoveModal,
  onClearSelection,
}) => {
  return (
    <Card size="small">
      <Space wrap align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Space wrap>
          <Typography.Text>已选: {selectedMaterialCount} 个物料</Typography.Text>
          <Typography.Text type="secondary">总重: {selectedTotalWeight.toFixed(2)}t</Typography.Text>
        </Space>

        <Space wrap>
          <Button disabled={disabled} onClick={onLock}>
            锁定
          </Button>
          <Button disabled={disabled} onClick={onUnlock}>
            解锁
          </Button>
          <Button type="primary" danger disabled={disabled} onClick={onSetUrgent}>
            设为紧急
          </Button>
          <Button disabled={disabled} onClick={onClearUrgent}>
            取消紧急
          </Button>
          <Button danger disabled={disabled} onClick={onForceRelease}>
            强制放行
          </Button>
          <Button disabled={disabled} onClick={onOpenMoveRecommend}>
            最近可行
          </Button>
          <Button disabled={disabled} onClick={onOpenMoveModal}>
            移动到...
          </Button>
          <Button disabled={disabled} onClick={onClearSelection}>
            清空选择
          </Button>
        </Space>
      </Space>
    </Card>
  );
};

export default React.memo(WorkbenchStatusBar);


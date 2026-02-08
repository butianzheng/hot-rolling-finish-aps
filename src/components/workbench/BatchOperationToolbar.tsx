import React from 'react';
import { Button, Dropdown, Modal } from 'antd';
import { DownOutlined } from '@ant-design/icons';

type BatchOpKey =
  | 'lock'
  | 'unlock'
  | 'urgent'
  | 'clear_urgent'
  | 'force_release'
  | 'clear_force_release'
  | 'move'
  | 'conditional'
  | 'clear';

interface BatchOperationToolbarProps {
  disabled: boolean;
  onLock: () => void;
  onUnlock: () => void;
  onSetUrgent: () => void;
  onClearUrgent?: () => void;
  onForceRelease?: () => void;
  onClearForceRelease?: () => void;
  onMove: () => void;
  onConditional?: () => void;
  onClear: () => void;
}

const BatchOperationToolbar: React.FC<BatchOperationToolbarProps> = ({
  disabled,
  onLock,
  onUnlock,
  onSetUrgent,
  onClearUrgent,
  onForceRelease,
  onClearForceRelease,
  onMove,
  onConditional,
  onClear,
}) => {
  return (
    <Dropdown
      disabled={disabled}
      menu={{
        onClick: ({ key }) => {
          const k = key as BatchOpKey;
          if (k === 'lock') return onLock();
          if (k === 'unlock') return onUnlock();
          if (k === 'urgent') return onSetUrgent();
          if (k === 'clear_urgent') {
            if (onClearUrgent) return onClearUrgent();
            Modal.info({ title: '取消紧急', content: '当前页面未提供取消紧急入口。' });
          }
          if (k === 'force_release') {
            if (onForceRelease) return onForceRelease();
            Modal.info({ title: '强制放行', content: '当前页面未提供强制放行入口。' });
          }
          if (k === 'clear_force_release') {
            if (onClearForceRelease) return onClearForceRelease();
            Modal.info({ title: '取消强放', content: '当前页面未提供取消强放入口。' });
          }
          if (k === 'move') return onMove();
          if (k === 'clear') return onClear();
          if (k === 'conditional') {
            if (onConditional) return onConditional();
            Modal.info({ title: '按条件操作', content: '当前页面未提供按条件操作入口。' });
          }
        },
        items: [
          { key: 'lock', label: '锁定' },
          { key: 'unlock', label: '解锁' },
          { key: 'urgent', label: '设为紧急' },
          { key: 'clear_urgent', label: '取消紧急' },
          { key: 'force_release', label: '强制放行' },
          { key: 'clear_force_release', label: '取消强放' },
          { key: 'move', label: '移动到...' },
          { type: 'divider' },
          { key: 'conditional', label: '按条件选中...' },
          { type: 'divider' },
          { key: 'clear', label: '清空选择' },
        ],
      }}
    >
      <Button>
        批量操作 <DownOutlined />
      </Button>
    </Dropdown>
  );
};

export default React.memo(BatchOperationToolbar);

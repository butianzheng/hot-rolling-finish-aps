/**
 * 创建后操作弹窗组件
 */

import React from 'react';
import { Alert, Button, Modal, Space, Typography } from 'antd';

interface PostCreateModalProps {
  open: boolean;
  createdVersionId: string | null;
  postActionLoading: 'switch' | 'activate' | null;
  onClose: () => void;
  onSwitch: () => void;
  onActivate: () => void;
}

export const PostCreateModal: React.FC<PostCreateModalProps> = ({
  open,
  createdVersionId,
  postActionLoading,
  onClose,
  onSwitch,
  onActivate,
}) => {
  return (
    <Modal
      title="已生成新版本"
      open={open}
      onCancel={onClose}
      footer={[
        <Button key="later" onClick={onClose}>
          稍后
        </Button>,
        <Button
          key="switch"
          disabled={!createdVersionId}
          loading={postActionLoading === 'switch'}
          onClick={onSwitch}
        >
          切换到新版本
        </Button>,
        <Button
          key="activate"
          type="primary"
          disabled={!createdVersionId}
          loading={postActionLoading === 'activate'}
          onClick={onActivate}
        >
          切换并激活
        </Button>,
      ]}
    >
      <Space direction="vertical" style={{ width: '100%' }} size={10}>
        <Alert
          type="success"
          showIcon
          message="重算已完成"
          description={
            <Space direction="vertical" size={6}>
              <Typography.Text type="secondary">新版本ID</Typography.Text>
              <Typography.Text code>{createdVersionId || '-'}</Typography.Text>
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                切换后工作台将加载该版本排程；激活会归档当前激活版本。
              </Typography.Text>
            </Space>
          }
        />
      </Space>
    </Modal>
  );
};

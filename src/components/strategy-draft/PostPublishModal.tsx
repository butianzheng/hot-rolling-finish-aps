/**
 * 发布后操作弹窗
 * 草案发布成功后的操作选择
 */

import React from 'react';
import { Alert, Button, Modal, Space, Typography } from 'antd';

const { Text } = Typography;

export interface PostPublishModalProps {
  open: boolean;
  createdVersionId: string | null;
  postActionLoading: 'switch' | 'activate' | null;
  onClose: () => void;
  onGoHistorical: () => void;
  onSwitch: () => Promise<void>;
  onActivate: () => Promise<void>;
}

export const PostPublishModal: React.FC<PostPublishModalProps> = ({
  open,
  createdVersionId,
  postActionLoading,
  onClose,
  onGoHistorical,
  onSwitch,
  onActivate,
}) => {
  return (
    <Modal
      title="草案已发布"
      open={open}
      onCancel={onClose}
      footer={[
        <Button key="later" onClick={onClose}>
          稍后
        </Button>,
        <Button key="historical" onClick={onGoHistorical}>
          去历史版本对比
        </Button>,
        <Button
          key="switch"
          disabled={!createdVersionId}
          loading={postActionLoading === 'switch'}
          onClick={onSwitch}
        >
          去工作台继续
        </Button>,
        <Button
          key="activate"
          type="primary"
          disabled={!createdVersionId}
          loading={postActionLoading === 'activate'}
          onClick={onActivate}
        >
          去工作台并激活
        </Button>,
      ]}
    >
      <Space direction="vertical" style={{ width: '100%' }} size={10}>
        <Alert
          type="success"
          showIcon
          message="已生成正式版本"
          description={
            <Space direction="vertical" size={6}>
              <Text type="secondary">新版本ID</Text>
              <Text code>{createdVersionId || '-'}</Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                你可以先切换到该版本继续在工作台做人工微调；也可以直接激活使其成为当前生效版本。
              </Text>
            </Space>
          }
        />
      </Space>
    </Modal>
  );
};

export default PostPublishModal;

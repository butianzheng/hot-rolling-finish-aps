/**
 * 原始数据查看 Modal
 * 展示导入冲突的原始 JSON 数据
 */

import React, { useMemo } from 'react';
import { Button, Modal, message } from 'antd';

export interface RawDataModalProps {
  open: boolean;
  title: string;
  content: string;
  onClose: () => void;
}

export const RawDataModal: React.FC<RawDataModalProps> = ({ open, title, content, onClose }) => {
  // 格式化 JSON
  const formattedContent = useMemo(() => {
    try {
      const obj = JSON.parse(content || '{}');
      return JSON.stringify(obj, null, 2);
    } catch {
      return content || '';
    }
  }, [content]);

  // 复制到剪贴板
  const handleCopy = () => {
    navigator.clipboard.writeText(content || '').then(
      () => message.success('已复制'),
      () => message.error('复制失败'),
    );
  };

  return (
    <Modal
      title={title}
      open={open}
      onCancel={onClose}
      footer={[
        <Button key="copy" onClick={handleCopy}>
          复制
        </Button>,
        <Button key="close" type="primary" onClick={onClose}>
          关闭
        </Button>,
      ]}
      width={900}
      destroyOnClose
    >
      <pre style={{ maxHeight: 520, overflow: 'auto', margin: 0 }}>{formattedContent}</pre>
    </Modal>
  );
};

export default RawDataModal;

import React from 'react';
import { Button, Modal, Space, Spin, Alert, Empty, Tag, Typography } from 'antd';
import type { MaterialDetailModalProps } from './types';
import { DraftInfoSection } from './DraftInfoSection';
import { MaterialInfoSection } from './MaterialInfoSection';
import { StateReasonSection } from './StateReasonSection';
import { ActionLogsSection } from './ActionLogsSection';

const { Text } = Typography;

export const MaterialDetailModal: React.FC<MaterialDetailModalProps> = ({
  open,
  loading,
  context,
  data,
  error,
  logsLoading,
  logsError,
  logs,
  range,
  onClose,
  onGoWorkbench,
}) => {
  const windowStart = range[0].format('YYYY-MM-DD');
  const windowEnd = range[1].format('YYYY-MM-DD');

  return (
    <Modal
      title={
        <Space>
          <span>物料详情</span>
          {context ? <Text code>{context.material_id}</Text> : null}
          {context?.change_type ? (
            <Tag
              color={
                String(context.change_type) === 'ADDED'
                  ? 'green'
                  : String(context.change_type) === 'SQUEEZED_OUT'
                    ? 'red'
                    : 'blue'
              }
            >
              {String(context.change_type) === 'ADDED'
                ? '新增'
                : String(context.change_type) === 'SQUEEZED_OUT'
                  ? '挤出'
                  : '移动'}
            </Tag>
          ) : null}
        </Space>
      }
      open={open}
      onCancel={onClose}
      footer={[
        <Button
          key="to-workbench"
          type="primary"
          disabled={!context?.material_id}
          onClick={() => {
            const id = String(context?.material_id ?? '').trim();
            if (id) onGoWorkbench(id);
          }}
        >
          去工作台查看
        </Button>,
        <Button key="close" onClick={onClose}>
          关闭
        </Button>,
      ]}
      width={760}
      destroyOnClose
    >
      {loading ? (
        <div style={{ padding: 24, textAlign: 'center' }}>
          <Spin tip="加载中…" />
        </div>
      ) : error ? (
        <Alert type="error" showIcon message="加载失败" description={error} />
      ) : data ? (
        <Space direction="vertical" style={{ width: '100%' }} size={12}>
          {context ? <DraftInfoSection context={context} data={data} windowStart={windowStart} windowEnd={windowEnd} /> : null}
          <MaterialInfoSection data={data} />
          <StateReasonSection data={data} />
          <ActionLogsSection logsLoading={logsLoading} logsError={logsError} logs={logs} />
        </Space>
      ) : (
        <Empty description="暂无数据" />
      )}
    </Modal>
  );
};

export default MaterialDetailModal;

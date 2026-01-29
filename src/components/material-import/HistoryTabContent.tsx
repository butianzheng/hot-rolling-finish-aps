/**
 * 导入历史 Tab 内容组件
 * 展示本机导入历史记录和操作
 */

import React from 'react';
import { Alert, Button, Card, Modal, Space, Table, Tag } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { ReloadOutlined } from '@ant-design/icons';
import { formatMs } from '../../utils/importFormatters';
import type { ImportHistoryItem } from '../../types/import';

export interface HistoryTabContentProps {
  // 数据
  importHistory: ImportHistoryItem[];

  // 回调
  onRefresh: () => void;
  onClearHistory: () => void;
  onViewConflicts: (batchId: string) => Promise<void>;
  onCopyId: (id: string) => void;
}

export const HistoryTabContent: React.FC<HistoryTabContentProps> = ({
  importHistory,
  onRefresh,
  onClearHistory,
  onViewConflicts,
  onCopyId,
}) => {
  // 历史表格列定义
  const historyColumns: ColumnsType<ImportHistoryItem> = [
    {
      title: '时间',
      dataIndex: 'created_at',
      width: 180,
      render: (v) => (
        <span style={{ fontFamily: 'monospace' }}>{String(v).replace('T', ' ').slice(0, 19)}</span>
      ),
    },
    { title: '导入人', dataIndex: 'operator', width: 100 },
    {
      title: '批次ID',
      dataIndex: 'id',
      width: 200,
      ellipsis: true,
      render: (v) => <span style={{ fontFamily: 'monospace' }}>{String(v)}</span>,
    },
    {
      title: '文件',
      dataIndex: 'file_path',
      ellipsis: true,
      render: (v) => <span style={{ fontFamily: 'monospace' }}>{String(v || '-')}</span>,
    },
    {
      title: '结果',
      key: 'result',
      width: 200,
      render: (_, r) => (
        <Space size={8}>
          <Tag color="green">导入 {r.imported}</Tag>
          <Tag color="blue">更新 {r.updated}</Tag>
          <Tag color={r.conflicts > 0 ? 'red' : 'default'}>冲突 {r.conflicts}</Tag>
        </Space>
      ),
    },
    {
      title: '耗时',
      dataIndex: 'elapsed_ms',
      width: 100,
      render: (v) => (
        <span style={{ fontFamily: 'monospace' }}>
          {formatMs(v == null ? undefined : Number(v))}
        </span>
      ),
    },
    {
      title: '操作',
      key: 'actions',
      width: 160,
      render: (_, r) => (
        <Space>
          <Button
            size="small"
            disabled={!r.import_batch_id}
            onClick={async () => {
              if (r.import_batch_id) {
                await onViewConflicts(r.import_batch_id);
              }
            }}
          >
            查看冲突
          </Button>
          <Button size="small" onClick={() => onCopyId(r.id)}>
            复制ID
          </Button>
        </Space>
      ),
    },
  ];

  return (
    <Card
      title="最近导入记录（本机）"
      extra={
        <Space>
          <Button icon={<ReloadOutlined />} onClick={onRefresh}>
            刷新
          </Button>
          <Button
            danger
            onClick={() => {
              Modal.confirm({
                title: '清空导入历史？',
                content: '仅清除本机 localStorage 记录，不影响后端数据。',
                okText: '清空',
                okButtonProps: { danger: true },
                cancelText: '取消',
                onOk: onClearHistory,
              });
            }}
            disabled={importHistory.length === 0}
          >
            清空
          </Button>
        </Space>
      }
    >
      <Alert
        type="info"
        showIcon
        message="说明"
        description="导入历史暂存于本机 localStorage，用于快速定位批次与冲突；如需全局导入审计/查询，需要后端提供 import_batch 列表接口。"
        style={{ marginBottom: 12 }}
      />

      <Table<ImportHistoryItem>
        rowKey={(r) => r.id}
        pagination={{ pageSize: 10 }}
        dataSource={importHistory}
        virtual
        columns={historyColumns}
        scroll={{ x: 980, y: 520 }}
      />
    </Card>
  );
};

export default HistoryTabContent;

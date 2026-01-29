import React from 'react';
import { Alert, Empty, Spin, Table, Tag, Typography } from 'antd';
import type { ActionLogRow } from '../../types/strategy-draft';

const { Text } = Typography;

export interface ActionLogsSectionProps {
  logsLoading: boolean;
  logsError: string | null;
  logs: ActionLogRow[];
}

export const ActionLogsSection: React.FC<ActionLogsSectionProps> = ({
  logsLoading,
  logsError,
  logs,
}) => (
  <div style={{ marginTop: 12 }}>
    <Text type="secondary">最近相关操作（30天）</Text>
    <div style={{ marginTop: 6 }}>
      {logsLoading ? (
        <div style={{ padding: 12, textAlign: 'center' }}>
          <Spin size="small" tip="加载操作历史…" />
        </div>
      ) : logsError ? (
        <Alert type="warning" showIcon message="操作历史加载失败" description={logsError} />
      ) : logs.length ? (
        <Table
          size="small"
          rowKey="action_id"
          pagination={false}
          dataSource={logs}
          columns={[
            {
              title: '时间',
              dataIndex: 'action_ts',
              width: 160,
              render: (v: string) => <Text>{String(v || '')}</Text>,
            },
            {
              title: '类型',
              dataIndex: 'action_type',
              width: 160,
              render: (v: string) => <Tag>{String(v || '')}</Tag>,
            },
            {
              title: '操作人',
              dataIndex: 'actor',
              width: 120,
              render: (v: string) => <Text>{String(v || '')}</Text>,
            },
            {
              title: '详情',
              dataIndex: 'detail',
              render: (v: string) => (
                <Text ellipsis={{ tooltip: v ? String(v) : '' }}>{v ? String(v) : '—'}</Text>
              ),
            },
          ]}
        />
      ) : (
        <Empty description="暂无相关操作" image={Empty.PRESENTED_IMAGE_SIMPLE} />
      )}
    </div>
  </div>
);

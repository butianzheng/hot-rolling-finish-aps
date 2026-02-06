/**
 * 自定义策略表格组件
 */

import React from 'react';
import { Button, Card, Space, Table, Tag, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { CopyOutlined, EditOutlined } from '@ant-design/icons';
import type { CustomStrategyProfile } from './types';
import { BASE_STRATEGY_LABEL, makeCustomStrategyKey } from './types';
import { tableEmptyConfig } from '../CustomEmpty';

const { Text } = Typography;

interface CustomProfilesTableProps {
  profiles: CustomStrategyProfile[];
  loading: boolean;
  onEdit: (profile: CustomStrategyProfile) => void;
  onCopy: (profile: CustomStrategyProfile) => void;
  onNavigateToDraft: (strategyKey: string) => void;
}

export const CustomProfilesTable: React.FC<CustomProfilesTableProps> = ({
  profiles,
  loading,
  onEdit,
  onCopy,
  onNavigateToDraft,
}) => {
  const columns: ColumnsType<CustomStrategyProfile> = [
    {
      title: '名称',
      dataIndex: 'title',
      key: 'title',
      width: 220,
      render: (v: string) => <Text strong>{v}</Text>,
    },
    {
      title: '策略编号',
      dataIndex: 'strategy_id',
      key: 'strategy_id',
      width: 200,
      render: (v: string) => <Text code>{v}</Text>,
    },
    {
      title: '基于预设',
      dataIndex: 'base_strategy',
      key: 'base_strategy',
      width: 140,
      render: (v: string) => <Tag color="blue">{BASE_STRATEGY_LABEL[String(v)] || String(v)}</Tag>,
    },
    {
      title: '参数摘要',
      key: 'params',
      width: 320,
      render: (_, r) => {
        const p = r?.parameters || {};
        const parts: string[] = [];
        if (p.urgent_weight != null) parts.push(`urgent=${p.urgent_weight}`);
        if (p.capacity_weight != null) parts.push(`capacity=${p.capacity_weight}`);
        if (p.cold_stock_weight != null) parts.push(`cold=${p.cold_stock_weight}`);
        if (p.due_date_weight != null) parts.push(`due=${p.due_date_weight}`);
        if (p.rolling_output_age_weight != null) parts.push(`roll_age=${p.rolling_output_age_weight}`);
        if (p.cold_stock_age_threshold_days != null) parts.push(`cold_days>=${p.cold_stock_age_threshold_days}`);
        if (p.overflow_tolerance_pct != null) parts.push(`overflow<=${Math.round(p.overflow_tolerance_pct * 100)}%`);

        return (
          <Text type="secondary">
            {parts.length ? parts.join(' · ') : '—'}
          </Text>
        );
      },
    },
    {
      title: '说明',
      dataIndex: 'description',
      key: 'description',
      ellipsis: true,
      render: (v: any) => <Text type="secondary">{String(v || '') || '—'}</Text>,
    },
    {
      title: '操作',
      key: 'actions',
      width: 240,
      render: (_, r) => (
        <Space size={6}>
          <Button size="small" icon={<EditOutlined />} onClick={() => onEdit(r)}>
            编辑
          </Button>
          <Button size="small" icon={<CopyOutlined />} onClick={() => onCopy(r)}>
            复制
          </Button>
          <Button
            size="small"
            onClick={() => {
              const key = makeCustomStrategyKey(r.strategy_id);
              onNavigateToDraft(key);
            }}
          >
            去草案对比
          </Button>
        </Space>
      ),
    },
  ];

  return (
    <Card
      size="small"
      title={
        <Space>
          <span>自定义策略</span>
          <Tag color="gold">{profiles.length}</Tag>
        </Space>
      }
    >
      <Table
        rowKey="strategy_id"
        size="small"
        loading={loading}
        columns={columns}
        dataSource={profiles}
        pagination={{ pageSize: 10, showSizeChanger: true }}
        locale={tableEmptyConfig}
      />
    </Card>
  );
};

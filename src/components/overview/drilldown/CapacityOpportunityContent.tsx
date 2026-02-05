/**
 * 容量优化机会内容
 */

import React from 'react';
import { Button, Descriptions, Space, Table, Tag, Typography, theme } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { CapacityOpportunity } from '../../../types/decision';
import {
  OPPORTUNITY_TYPE_COLORS,
  OPPORTUNITY_TYPE_LABELS,
  parseOpportunityType,
  ActionsList,
  getHighlightStyle,
  type WorkbenchCallback,
} from './shared';

const { Text } = Typography;

export interface CapacityOpportunityContentProps {
  opportunities: CapacityOpportunity[];
  machineCodeFilter?: string | null;
  planDateFilter?: string | null;
  onGoWorkbench?: WorkbenchCallback;
  onViewDetail: (record: CapacityOpportunity) => void;
}

export const CapacityOpportunityContent: React.FC<CapacityOpportunityContentProps> = ({
  opportunities,
  machineCodeFilter,
  planDateFilter,
  onGoWorkbench,
  onViewDetail,
}) => {
  const { token } = theme.useToken();

  const rows = [...opportunities].sort((a, b) => b.opportunitySpaceT - a.opportunitySpaceT);

  const selected =
    machineCodeFilter && planDateFilter
      ? opportunities.find((r) => r.machineCode === machineCodeFilter && r.planDate === planDateFilter) || null
      : null;

  const columns: ColumnsType<CapacityOpportunity> = [
    { title: '日期', dataIndex: 'planDate', key: 'planDate', width: 110 },
    { title: '机组', dataIndex: 'machineCode', key: 'machineCode', width: 90 },
    {
      title: '类型',
      dataIndex: 'opportunityType',
      key: 'opportunityType',
      width: 140,
      render: (v: string) => {
        const t = parseOpportunityType(v);
        return (
          <Tag color={OPPORTUNITY_TYPE_COLORS[t as keyof typeof OPPORTUNITY_TYPE_COLORS]}>
            {OPPORTUNITY_TYPE_LABELS[t as keyof typeof OPPORTUNITY_TYPE_LABELS] || t}
          </Tag>
        );
      },
    },
    {
      title: '当前利用率',
      dataIndex: 'currentUtilPct',
      key: 'currentUtilPct',
      width: 110,
      render: (v: number) => `${Number(v || 0).toFixed(1)}%`,
    },
    {
      title: '机会(吨)',
      dataIndex: 'opportunitySpaceT',
      key: 'opportunitySpaceT',
      width: 110,
      render: (v: number) => `${Number(v || 0).toFixed(2)}`,
    },
    {
      title: '描述',
      dataIndex: 'description',
      key: 'description',
      ellipsis: true,
    },
    {
      title: '首条建议',
      dataIndex: 'recommendedActions',
      key: 'recommendedActions',
      width: 260,
      ellipsis: true,
      render: (v: string[]) => (Array.isArray(v) && v.length > 0 ? v[0] : '-'),
    },
    {
      title: '操作',
      key: 'action',
      width: onGoWorkbench ? 160 : 90,
      render: (_, record) => (
        <Space size={8}>
          {onGoWorkbench ? (
            <Button
              size="small"
              type="primary"
              onClick={() => onGoWorkbench({ workbenchTab: 'capacity', machineCode: record.machineCode })}
            >
              处理
            </Button>
          ) : null}
          <Button size="small" onClick={() => onViewDetail(record)}>
            详情
          </Button>
        </Space>
      ),
    },
  ];

  return (
    <Space direction="vertical" size={12} style={{ width: '100%' }}>
      {selected ? (
        <>
          <Space wrap align="center">
            <Tag color="blue">{selected.machineCode}</Tag>
            <Text strong>
              {selected.planDate} · 机会 {selected.opportunitySpaceT.toFixed(2)}t
            </Text>
            {onGoWorkbench ? (
              <Button
                size="small"
                type="primary"
                onClick={() => onGoWorkbench({ workbenchTab: 'capacity', machineCode: selected.machineCode })}
              >
                去处理
              </Button>
            ) : null}
          </Space>

          <Descriptions column={4} bordered size="small">
            <Descriptions.Item label="当前利用率">{selected.currentUtilPct.toFixed(1)}%</Descriptions.Item>
            <Descriptions.Item label="优化后利用率">{selected.optimizedUtilPct.toFixed(1)}%</Descriptions.Item>
            <Descriptions.Item label="已用/目标">
              {selected.usedCapacityT.toFixed(2)} / {selected.targetCapacityT.toFixed(2)}t
            </Descriptions.Item>
            <Descriptions.Item label="机会空间">{selected.opportunitySpaceT.toFixed(2)}t</Descriptions.Item>
          </Descriptions>

          {selected.description ? (
            <div>
              <Text strong>描述</Text>
              <div style={{ marginTop: 6, color: token.colorTextSecondary }}>{selected.description}</div>
            </div>
          ) : null}

          <ActionsList
            title="建议操作"
            actions={selected.recommendedActions || []}
            colorTextSecondary={token.colorTextSecondary}
          />

          <ActionsList
            title="潜在收益"
            actions={selected.potentialBenefits || []}
            colorTextSecondary={token.colorTextSecondary}
          />
        </>
      ) : null}

      <Table
        rowKey={(r) => `${r.planDate}-${r.machineCode}`}
        size="small"
        columns={columns}
        dataSource={rows}
        pagination={{ pageSize: 20 }}
        onRow={(record) => ({
          style: getHighlightStyle(
            !!(
              machineCodeFilter &&
              planDateFilter &&
              record.machineCode === machineCodeFilter &&
              record.planDate === planDateFilter
            ),
            token
          ),
        })}
        scroll={{ x: 1100 }}
      />
    </Space>
  );
};

export default CapacityOpportunityContent;

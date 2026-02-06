/**
 * 风险摘要内容
 */

import React from 'react';
import { Button, Descriptions, Space, Table, Tag, Typography, theme } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { DaySummary } from '../../../types/decision';
import { getRiskLevelColor, getRiskLevelLabel, ReasonTable, getHighlightStyle, type WorkbenchCallback } from './shared';
import { formatNumber, formatWeight } from '../../../utils/formatters';

const { Text } = Typography;

export interface RiskDayContentProps {
  riskDays: DaySummary[];
  planDateFilter?: string | null;
  onGoWorkbench?: WorkbenchCallback;
  onViewDetail: (record: DaySummary) => void;
}

export const RiskDayContent: React.FC<RiskDayContentProps> = ({
  riskDays,
  planDateFilter,
  onGoWorkbench,
  onViewDetail,
}) => {
  const { token } = theme.useToken();

  const rows = [...riskDays].sort((a, b) => b.riskScore - a.riskScore);
  const selectedDay = planDateFilter ? riskDays.find((d) => d.planDate === planDateFilter) || null : null;

  const columns: ColumnsType<DaySummary> = [
    { title: '日期', dataIndex: 'planDate', key: 'planDate', width: 110 },
    {
      title: '等级',
      dataIndex: 'riskLevel',
      key: 'riskLevel',
      width: 120,
      render: (v: DaySummary['riskLevel']) => <Tag color={getRiskLevelColor(v)}>{getRiskLevelLabel(v)}</Tag>,
    },
    {
      title: '分数',
      dataIndex: 'riskScore',
      key: 'riskScore',
      width: 90,
      render: (v: number) => formatNumber(Number(v || 0), 2),
    },
    {
      title: '利用率',
      dataIndex: 'capacityUtilPct',
      key: 'capacityUtilPct',
      width: 100,
      render: (v: number) => `${formatNumber(Number(v || 0), 2)}%`,
    },
    {
      title: '超载（吨）',
      dataIndex: 'overloadWeightT',
      key: 'overloadWeightT',
      width: 130,
      render: (v: number) => formatWeight(v),
    },
    { title: '紧急失败', dataIndex: 'urgentFailureCount', key: 'urgentFailureCount', width: 90 },
    {
      title: '涉及机组',
      dataIndex: 'involvedMachines',
      key: 'involvedMachines',
      ellipsis: true,
      render: (v: string[]) => (Array.isArray(v) ? v.join(', ') : ''),
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
              onClick={() =>
                onGoWorkbench({
                  workbenchTab: 'capacity',
                  machineCode:
                    Array.isArray(record.involvedMachines) && record.involvedMachines.length > 0
                      ? record.involvedMachines[0]
                      : null,
                })
              }
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
      {selectedDay ? (
        <>
          <Space wrap align="center">
            <Tag color={getRiskLevelColor(selectedDay.riskLevel)}>
              {getRiskLevelLabel(selectedDay.riskLevel)} / {formatNumber(selectedDay.riskScore, 2)}
            </Tag>
            <Text strong>{selectedDay.planDate}</Text>
            {onGoWorkbench ? (
              <Button
                size="small"
                type="primary"
                onClick={() =>
                  onGoWorkbench({
                    workbenchTab: 'capacity',
                    machineCode:
                      Array.isArray(selectedDay.involvedMachines) && selectedDay.involvedMachines.length > 0
                        ? selectedDay.involvedMachines[0]
                        : null,
                  })
                }
              >
                去处理
              </Button>
            ) : null}
          </Space>

          <Descriptions column={4} bordered size="small">
            <Descriptions.Item label="风险分数">{formatNumber(selectedDay.riskScore, 2)}</Descriptions.Item>
            <Descriptions.Item label="容量利用率">{formatNumber(selectedDay.capacityUtilPct, 2)}%</Descriptions.Item>
            <Descriptions.Item label="超载">{formatWeight(selectedDay.overloadWeightT)}</Descriptions.Item>
            <Descriptions.Item label="紧急失败">{selectedDay.urgentFailureCount}</Descriptions.Item>
          </Descriptions>

          {Array.isArray(selectedDay.involvedMachines) && selectedDay.involvedMachines.length > 0 ? (
            <Space wrap>
              {selectedDay.involvedMachines.slice(0, 12).map((m) => (
                <Tag key={m} style={{ marginInlineEnd: 0 }}>
                  {m}
                </Tag>
              ))}
              {selectedDay.involvedMachines.length > 12 ? (
                <Text type="secondary">等 {selectedDay.involvedMachines.length} 个机组</Text>
              ) : null}
            </Space>
          ) : null}

          <ReasonTable reasons={selectedDay.topReasons || []} emptyText="暂无风险原因明细" />
        </>
      ) : null}

      <Table
        rowKey={(r) => r.planDate}
        size="small"
        columns={columns}
        dataSource={rows}
        pagination={{ pageSize: 20 }}
        onRow={(record) => ({
          style: getHighlightStyle(!!(planDateFilter && record.planDate === planDateFilter), token),
        })}
      />
    </Space>
  );
};

export default RiskDayContent;

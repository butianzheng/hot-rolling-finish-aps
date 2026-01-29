/**
 * 风险快照表格列配置
 */

import { Space, Tag } from 'antd';
import { WarningOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { formatNumber } from '../../utils/formatters';
import type { RiskDaySummary } from './types';
import { riskLevelColors } from './types';

export interface RiskSnapshotColumnsOptions {
  mostRiskyDate: string | null;
}

export function createRiskSnapshotColumns(
  options: RiskSnapshotColumnsOptions
): ColumnsType<RiskDaySummary> {
  const { mostRiskyDate } = options;

  return [
    {
      title: '日期',
      dataIndex: 'plan_date',
      key: 'plan_date',
      width: 120,
      render: (date: string) => {
        const isMostRisky = date === mostRiskyDate;
        return (
          <Space>
            {date}
            {isMostRisky && (
              <Tag color="red" icon={<WarningOutlined />}>
                最危险
              </Tag>
            )}
          </Space>
        );
      },
    },
    {
      title: '风险分数',
      dataIndex: 'risk_score',
      key: 'risk_score',
      width: 100,
      sorter: (a, b) => a.risk_score - b.risk_score,
      render: (score: number) => (
        <span style={{ fontWeight: 'bold', color: score > 60 ? '#cf1322' : '#52c41a' }}>
          {score}
        </span>
      ),
    },
    {
      title: '风险等级',
      dataIndex: 'risk_level',
      key: 'risk_level',
      width: 100,
      render: (level: string) => (
        <Tag color={riskLevelColors[level] || 'default'}>{level}</Tag>
      ),
    },
    {
      title: '产能利用率(%)',
      dataIndex: 'capacity_util_pct',
      key: 'capacity_util_pct',
      width: 100,
      render: (val: number) => formatNumber(val, 1),
    },
    {
      title: '超载吨数(t)',
      dataIndex: 'overload_weight_t',
      key: 'overload_weight_t',
      width: 100,
      render: (val: number) => formatNumber(val, 1),
    },
    {
      title: '紧急单失败数',
      dataIndex: 'urgent_failure_count',
      key: 'urgent_failure_count',
      width: 100,
    },
    {
      title: '涉及机组',
      dataIndex: 'involved_machines',
      key: 'involved_machines',
      width: 100,
      render: (machines: string[]) => (machines && machines.length > 0 ? machines.join(',') : '-'),
    },
  ];
}

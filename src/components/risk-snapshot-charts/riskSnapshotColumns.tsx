/**
 * 风险快照表格列配置
 */

import { Space, Tag } from 'antd';
import { WarningOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { formatNumber } from '../../utils/formatters';
import type { DaySummary } from '../../types/decision';
import { riskLevelColors } from './types';

export interface RiskSnapshotColumnsOptions {
  mostRiskyDate: string | null;
}

export function createRiskSnapshotColumns(
  options: RiskSnapshotColumnsOptions
): ColumnsType<DaySummary> {
  const { mostRiskyDate } = options;

  return [
    {
      title: '日期',
      dataIndex: 'planDate',
      key: 'planDate',
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
      dataIndex: 'riskScore',
      key: 'riskScore',
      width: 100,
      sorter: (a, b) => a.riskScore - b.riskScore,
      render: (score: number) => (
        <span style={{ fontWeight: 'bold', color: score > 60 ? '#cf1322' : '#52c41a' }}>
          {score}
        </span>
      ),
    },
    {
      title: '风险等级',
      dataIndex: 'riskLevel',
      key: 'riskLevel',
      width: 100,
      render: (level: string) => (
        <Tag color={riskLevelColors[level] || 'default'}>{level}</Tag>
      ),
    },
    {
      title: '产能利用率(%)',
      dataIndex: 'capacityUtilPct',
      key: 'capacityUtilPct',
      width: 100,
      render: (val: number) => formatNumber(val, 1),
    },
    {
      title: '超载吨数(t)',
      dataIndex: 'overloadWeightT',
      key: 'overloadWeightT',
      width: 100,
      render: (val: number) => formatNumber(val, 1),
    },
    {
      title: '紧急单失败数',
      dataIndex: 'urgentFailureCount',
      key: 'urgentFailureCount',
      width: 100,
    },
    {
      title: '涉及机组',
      dataIndex: 'involvedMachines',
      key: 'involvedMachines',
      width: 100,
      render: (machines: string[]) => (machines && machines.length > 0 ? machines.join(',') : '-'),
    },
  ];
}

/**
 * 换辊警报内容
 */

import React from 'react';
import { Button, Space, Table, Tag, theme } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { RollCampaignAlert } from '../../../types/decision';
import { getAlertLevelLabel, getAlertLevelColor, getHighlightStyle, type WorkbenchCallback } from './shared';

export interface RollAlertContentProps {
  alerts: RollCampaignAlert[];
  machineCodeFilter?: string | null;
  onGoWorkbench?: WorkbenchCallback;
  onViewDetail: (record: RollCampaignAlert) => void;
}

export const RollAlertContent: React.FC<RollAlertContentProps> = ({
  alerts,
  machineCodeFilter,
  onGoWorkbench,
  onViewDetail,
}) => {
  const { token } = theme.useToken();

  const rows = [...alerts].sort((a, b) => a.remainingTonnageT - b.remainingTonnageT);
  const selected = machineCodeFilter ? alerts.find((r) => r.machineCode === machineCodeFilter) || null : null;

  const columns: ColumnsType<RollCampaignAlert> = [
    { title: '机组', dataIndex: 'machineCode', key: 'machineCode', width: 90 },
    {
      title: '等级',
      dataIndex: 'alertLevel',
      key: 'alertLevel',
      width: 120,
      render: (v: string) => <Tag color={getAlertLevelColor(v)}>{getAlertLevelLabel(v)}</Tag>,
    },
    {
      title: '当前吨位',
      dataIndex: 'currentTonnageT',
      key: 'currentTonnageT',
      width: 100,
      render: (v: number) => v?.toFixed(3) || '-',
    },
    {
      title: '硬上限',
      dataIndex: 'hardLimitT',
      key: 'hardLimitT',
      width: 100,
      render: (v: number) => v?.toFixed(3) || '-',
    },
    {
      title: '剩余(距硬上限)',
      dataIndex: 'remainingTonnageT',
      key: 'remainingTonnageT',
      width: 140,
      render: (v: number) => v?.toFixed(3) || '-',
    },
    { title: '开始日', dataIndex: 'campaignStartDate', key: 'campaignStartDate', width: 110 },
    { title: '预计硬停止', dataIndex: 'estimatedHardStopDate', key: 'estimatedHardStopDate', width: 120 },
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
              onClick={() => onGoWorkbench({ workbenchTab: 'visualization', machineCode: record.machineCode })}
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
        <Space wrap align="center">
          <Tag color="blue">{selected.machineCode}</Tag>
          <Tag color={getAlertLevelColor(selected.alertLevel || '')}>
            {getAlertLevelLabel(selected.alertLevel || '')}
          </Tag>
          {onGoWorkbench ? (
            <Button
              size="small"
              type="primary"
              onClick={() => onGoWorkbench({ workbenchTab: 'visualization', machineCode: selected.machineCode })}
            >
              去处理
            </Button>
          ) : null}
        </Space>
      ) : null}

      <Table
        rowKey={(r) => `${r.machineCode}-${r.campaignId}`}
        size="small"
        columns={columns}
        dataSource={rows}
        pagination={{ pageSize: 20 }}
        onRow={(record) => ({
          style: getHighlightStyle(!!(machineCodeFilter && record.machineCode === machineCodeFilter), token),
        })}
      />
    </Space>
  );
};

export default RollAlertContent;

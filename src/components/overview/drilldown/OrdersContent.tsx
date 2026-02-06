/**
 * 订单失败集合内容
 */

import React from 'react';
import { Button, Space, Table } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { OrderFailure } from '../../../types/decision';
import { getUrgencyLevelColor, getUrgencyLevelLabel, TagWithColor, type WorkbenchCallback } from './shared';
import { formatNumber, formatWeight } from '../../../utils/formatters';

export interface OrdersContentProps {
  rows: OrderFailure[];
  urgencyFilter?: string | null;
  onGoWorkbench?: WorkbenchCallback;
  onViewDetail: (record: OrderFailure) => void;
}

export const OrdersContent: React.FC<OrdersContentProps> = ({
  rows,
  urgencyFilter,
  onGoWorkbench,
  onViewDetail,
}) => {
  const filteredRows = urgencyFilter ? rows.filter((o) => o.urgencyLevel === urgencyFilter) : rows;

  const columns: ColumnsType<OrderFailure> = [
    { title: '合同号', dataIndex: 'contractNo', key: 'contractNo', width: 140, ellipsis: true },
    {
      title: '紧急等级',
      dataIndex: 'urgencyLevel',
      key: 'urgencyLevel',
      width: 110,
      render: (v: OrderFailure['urgencyLevel']) => (
        <TagWithColor color={getUrgencyLevelColor(v)}>{getUrgencyLevelLabel(v)}</TagWithColor>
      ),
    },
    { title: '交期', dataIndex: 'dueDate', key: 'dueDate', width: 110 },
    {
      title: '完成率',
      dataIndex: 'completionRate',
      key: 'completionRate',
      width: 90,
      render: (v: number) => `${formatNumber(Number(v || 0), 2)}%`,
    },
    {
      title: '未排（吨）',
      dataIndex: 'unscheduledWeightT',
      key: 'unscheduledWeightT',
      width: 120,
      render: (v: number) => formatWeight(v),
    },
    { title: '机组', dataIndex: 'machineCode', key: 'machineCode', width: 90 },
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
                  workbenchTab: 'visualization',
                  machineCode: record.machineCode,
                  urgencyLevel: record.urgencyLevel,
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
    <Table
      rowKey={(r) => `${r.contractNo}-${r.dueDate}-${r.machineCode}`}
      size="small"
      columns={columns}
      dataSource={filteredRows}
      pagination={{ pageSize: 20 }}
    />
  );
};

export default OrdersContent;

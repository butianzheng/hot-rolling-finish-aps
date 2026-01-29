/**
 * 产能池表格列配置
 */

import { Button } from 'antd';
import { EditOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { formatCapacity, formatPercent } from '../../utils/formatters';
import type { CapacityPool } from './types';

export interface CapacityPoolColumnsOptions {
  onEdit: (record: CapacityPool) => void;
}

export function createCapacityPoolColumns(options: CapacityPoolColumnsOptions): ColumnsType<CapacityPool> {
  const { onEdit } = options;

  return [
    {
      title: '机组',
      dataIndex: 'machine_code',
      key: 'machine_code',
      width: 100,
      fixed: 'left',
    },
    {
      title: '日期',
      dataIndex: 'plan_date',
      key: 'plan_date',
      width: 120,
    },
    {
      title: '目标产能(吨)',
      dataIndex: 'target_capacity_t',
      key: 'target_capacity_t',
      width: 120,
      render: (value: number) => formatCapacity(value),
    },
    {
      title: '极限产能(吨)',
      dataIndex: 'limit_capacity_t',
      key: 'limit_capacity_t',
      width: 120,
      render: (value: number) => formatCapacity(value),
    },
    {
      title: '已用产能(吨)',
      dataIndex: 'used_capacity_t',
      key: 'used_capacity_t',
      width: 120,
      render: (value: number) => formatCapacity(value),
    },
    {
      title: '剩余产能(吨)',
      dataIndex: 'available_capacity_t',
      key: 'available_capacity_t',
      width: 120,
      render: (value: number) => (
        <span style={{ color: value < 100 ? '#cf1322' : '#52c41a' }}>
          {formatCapacity(value)}
        </span>
      ),
    },
    {
      title: '利用率',
      key: 'utilization',
      width: 100,
      render: (_, record) => {
        const target = record.target_capacity_t || 0;
        const used = record.used_capacity_t || 0;
        const rate = target > 0 ? (used / target) * 100 : 0;
        return (
          <span style={{ color: rate > 100 ? '#cf1322' : rate > 90 ? '#fa8c16' : '#52c41a' }}>
            {formatPercent(rate)}
          </span>
        );
      },
    },
    {
      title: '操作',
      key: 'action',
      width: 100,
      fixed: 'right',
      render: (_, record) => (
        <Button
          type="link"
          size="small"
          icon={<EditOutlined />}
          onClick={() => onEdit(record)}
        >
          调整
        </Button>
      ),
    },
  ];
}

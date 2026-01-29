/**
 * Dashboard 表格列配置
 */

import type { ColumnsType } from 'antd/es/table';
import { formatNumber, formatPercent } from '../../utils/formatters';
import type { OrderFailureRow, ColdStockBucketRow } from './types';

/**
 * 创建订单失败表格列配置
 */
export function createOrderFailureColumns(): ColumnsType<OrderFailureRow> {
  return [
    {
      title: '合同号',
      dataIndex: 'contract_no',
      key: 'contract_no',
    },
    {
      title: '紧急等级',
      dataIndex: 'urgency_level',
      key: 'urgency_level',
    },
    {
      title: '交期',
      dataIndex: 'due_date',
      key: 'due_date',
    },
    {
      title: '失败类型',
      dataIndex: 'fail_type',
      key: 'fail_type',
    },
    {
      title: '完成率',
      dataIndex: 'completion_rate',
      key: 'completion_rate',
      render: (val: number) => {
        const n = typeof val === 'number' ? val : 0;
        const pct = n <= 1 ? n * 100 : n; // 兼容 0-1 与 0-100 两种口径
        return formatPercent(pct || 0);
      },
    },
  ];
}

/**
 * 创建冷料表格列配置
 */
export function createColdStockColumns(): ColumnsType<ColdStockBucketRow> {
  return [
    {
      title: '机组',
      dataIndex: 'machine_code',
      key: 'machine_code',
    },
    {
      title: '库龄分桶',
      dataIndex: 'age_bin',
      key: 'age_bin',
    },
    {
      title: '压力等级',
      dataIndex: 'pressure_level',
      key: 'pressure_level',
    },
    {
      title: '数量',
      dataIndex: 'count',
      key: 'count',
      width: 80,
    },
    {
      title: '重量(吨)',
      dataIndex: 'weight_t',
      key: 'weight_t',
      render: (val: number) => formatNumber(val, 2),
    },
  ];
}

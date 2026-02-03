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
      dataIndex: 'contractNo',
      key: 'contractNo',
    },
    {
      title: '紧急等级',
      dataIndex: 'urgencyLevel',
      key: 'urgencyLevel',
    },
    {
      title: '交期',
      dataIndex: 'dueDate',
      key: 'dueDate',
    },
    {
      title: '失败类型',
      dataIndex: 'failType',
      key: 'failType',
    },
    {
      title: '完成率',
      dataIndex: 'completionRate',
      key: 'completionRate',
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
      dataIndex: 'machineCode',
      key: 'machineCode',
    },
    {
      title: '库龄分桶',
      dataIndex: 'ageBin',
      key: 'ageBin',
    },
    {
      title: '压力等级',
      dataIndex: 'pressureLevel',
      key: 'pressureLevel',
    },
    {
      title: '数量',
      dataIndex: 'count',
      key: 'count',
      width: 80,
    },
    {
      title: '重量(吨)',
      dataIndex: 'weightT',
      key: 'weightT',
      render: (val: number) => formatNumber(val, 2),
    },
  ];
}

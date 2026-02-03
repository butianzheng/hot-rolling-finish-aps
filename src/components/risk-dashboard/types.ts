/**
 * 风险仪表盘 - 类型定义
 */

import type { ColdStockBucket, OrderFailure } from '../../types/decision';

export type BlockedUrgentOrderRow = Pick<
  OrderFailure,
  | 'contractNo'
  | 'dueDate'
  | 'urgencyLevel'
  | 'failType'
  | 'completionRate'
  | 'daysToDue'
  | 'machineCode'
>;

export type ColdStockBucketRow = Pick<
  ColdStockBucket,
  | 'machineCode'
  | 'ageBin'
  | 'pressureLevel'
  | 'count'
  | 'weightT'
  | 'avgAgeDays'
  | 'maxAgeDays'
>;

// 风险等级颜色
export const getRiskColor = (level: string) => {
  switch (level) {
    case 'critical':
      return '#ff4d4f';
    case 'high':
      return '#faad14';
    case 'medium':
      return '#1677ff';
    default:
      return '#52c41a';
  }
};

// 轧辊状态颜色
export const getRollStatusColor = (status: string) => {
  switch (status) {
    case 'critical':
      return '#ff4d4f';
    case 'warning':
      return '#faad14';
    default:
      return '#52c41a';
  }
};

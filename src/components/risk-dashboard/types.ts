/**
 * 风险仪表盘 - 类型定义
 */

export interface BlockedUrgentOrderRow {
  contract_no: string;
  due_date: string;
  urgency_level: string;
  fail_type: string;
  completion_rate: number;
  days_to_due: number;
  machine_code: string;
}

export interface ColdStockBucketRow {
  machine_code: string;
  age_bin: string;
  pressure_level: string;
  count: number;
  weight_t: number;
  avg_age_days: number;
  max_age_days: number;
}

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

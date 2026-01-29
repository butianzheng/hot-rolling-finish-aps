/**
 * Dashboard 类型定义
 */

// D2: OrderFailureDto (snake_case)
export interface OrderFailureRow {
  contract_no: string;
  due_date: string;
  urgency_level: string;
  fail_type: string;
  completion_rate: number; // percent (0-100)
  total_weight_t: number;
  unscheduled_weight_t: number;
  machine_code: string;
}

export interface OrderFailureSetResponse {
  items: OrderFailureRow[];
  summary?: {
    total_failures: number;
    total_unscheduled_weight_t: number;
  };
}

// D3: ColdStockBucketDto (snake_case)
export interface ColdStockBucketRow {
  machine_code: string;
  age_bin: string;
  pressure_level: string;
  count: number;
  weight_t: number;
  avg_age_days: number;
  max_age_days: number;
}

export interface ColdStockProfileResponse {
  items: ColdStockBucketRow[];
  summary?: {
    total_cold_stock_count: number;
    total_cold_stock_weight_t: number;
  };
}

// D4: BottleneckPointDto (snake_case)
export interface BottleneckPointRow {
  machine_code: string;
  plan_date: string;
  bottleneck_score: number;
  bottleneck_level: string;
}

export interface MachineBottleneckProfileResponse {
  items: BottleneckPointRow[];
}

// 刷新间隔选项
export const REFRESH_INTERVAL_OPTIONS = [
  { value: 10000, label: '10 秒' },
  { value: 15000, label: '15 秒' },
  { value: 30000, label: '30 秒' },
  { value: 60000, label: '1 分钟' },
  { value: 300000, label: '5 分钟' },
] as const;

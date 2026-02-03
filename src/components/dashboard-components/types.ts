/**
 * Dashboard 类型定义
 */

// 统一使用决策层 D2-D4 的 camelCase 类型，避免重复定义与契约漂移
import type {
  OrderFailure,
  OrderFailureSummary,
  ColdStockBucket,
  ColdStockSummary,
  BottleneckPoint,
} from '../../types/decision';

export type OrderFailureRow = OrderFailure;
export type ColdStockBucketRow = ColdStockBucket;
export type BottleneckPointRow = BottleneckPoint;

export interface OrderFailureSetResponse {
  items: OrderFailureRow[];
  summary?: OrderFailureSummary;
}

export interface ColdStockProfileResponse {
  items: ColdStockBucketRow[];
  summary?: ColdStockSummary;
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

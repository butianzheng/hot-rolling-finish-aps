/**
 * OneClickOptimize 类型定义
 */

export type OptimizeMenuKey =
  | 'preview'
  | 'execute'
  | 'balanced'
  | 'urgent_first'
  | 'capacity_first'
  | 'cold_stock_first';

export type OptimizeStrategy = Exclude<OptimizeMenuKey, 'preview' | 'execute'>;

export interface OneClickOptimizeMenuProps {
  activeVersionId: string | null;
  operator: string;
  onBeforeExecute?: () => void;
  onAfterExecute?: () => void;
}

export interface SimulateResult {
  message?: string;
  plan_items_count?: number;
  frozen_items_count?: number;
  version_id?: string;
}

export const STRATEGY_OPTIONS = [
  { value: 'balanced' as const, label: '均衡方案' },
  { value: 'urgent_first' as const, label: '紧急优先' },
  { value: 'capacity_first' as const, label: '产能优先' },
  { value: 'cold_stock_first' as const, label: '冷料消化' },
];

export function getStrategyLabel(strategy: OptimizeStrategy): string {
  switch (strategy) {
    case 'urgent_first': return '紧急优先';
    case 'capacity_first': return '产能优先';
    case 'cold_stock_first': return '冷料消化';
    default: return '均衡方案';
  }
}

/**
 * 策略配置面板类型定义
 */

export type StrategyPresetRow = {
  strategy: string;
  title: string;
  description: string;
};

export type CustomStrategyProfile = {
  strategy_id: string;
  title: string;
  description?: string | null;
  base_strategy: string;
  parameters?: {
    urgent_weight?: number | null;
    capacity_weight?: number | null;
    cold_stock_weight?: number | null;
    due_date_weight?: number | null;
    rolling_output_age_weight?: number | null;
    cold_stock_age_threshold_days?: number | null;
    overflow_tolerance_pct?: number | null;
  } | null;
};

export type ModalMode = 'create' | 'edit' | 'copy';

export const BASE_STRATEGY_LABEL: Record<string, string> = {
  balanced: '均衡方案',
  urgent_first: '紧急优先',
  capacity_first: '产能优先',
  cold_stock_first: '冷坨消化',
};

export const DEFAULT_PRESETS: StrategyPresetRow[] = [
  { strategy: 'balanced', title: '均衡方案', description: '在交付/产能/库存之间保持均衡' },
  { strategy: 'urgent_first', title: '紧急优先', description: '优先保障 L3/L2 紧急订单' },
  { strategy: 'capacity_first', title: '产能优先', description: '优先提升产能利用率，减少溢出' },
  { strategy: 'cold_stock_first', title: '冷坨消化', description: '优先消化冷坨/压库物料' },
];

export function makeCustomStrategyKey(strategyId: string): string {
  return `custom:${String(strategyId || '').trim()}`;
}

export function suggestStrategyId(baseStrategy: string): string {
  const base = String(baseStrategy || 'balanced').trim();
  const suffix = Math.random().toString(36).slice(2, 8);
  return `custom_${base}_${suffix}`;
}

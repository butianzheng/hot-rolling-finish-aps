import type { BuiltinStrategyType } from './preferences';

export const BUILTIN_STRATEGY_OPTIONS: Array<{ value: BuiltinStrategyType; label: string }> = [
  { value: 'balanced', label: '均衡方案' },
  { value: 'urgent_first', label: '紧急优先' },
  { value: 'capacity_first', label: '产能优先' },
  { value: 'cold_stock_first', label: '冷料消化' },
  { value: 'manual', label: '手动调整' },
];

export function normalizeStrategyKey(strategy: string | null | undefined): string {
  const raw = String(strategy || '').trim();
  if (!raw) return 'balanced';
  if (raw.startsWith('custom:')) {
    const id = raw.slice('custom:'.length).trim();
    return id ? `custom:${id}` : 'balanced';
  }
  return raw;
}

export function getStrategyLabelByKey(strategy: string | null | undefined): string {
  const key = normalizeStrategyKey(strategy);
  if (key === 'urgent_first') return '紧急优先';
  if (key === 'capacity_first') return '产能优先';
  if (key === 'cold_stock_first') return '冷坯消化';
  if (key === 'manual') return '手动调整';
  if (key.startsWith('custom:')) {
    const id = key.slice('custom:'.length).trim();
    return id ? `自定义策略（${id}）` : '自定义策略';
  }
  return '均衡方案';
}


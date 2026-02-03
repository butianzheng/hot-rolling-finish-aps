import type { ForceReleaseViolation } from './types';

export function extractForceReleaseViolations(details: unknown): ForceReleaseViolation[] {
  if (!details || typeof details !== 'object') return [];
  const violations = (details as { violations?: unknown }).violations;
  if (!Array.isArray(violations)) return [];
  return violations.filter((v): v is ForceReleaseViolation => v != null && typeof v === 'object');
}

/**
 * 将策略 key 转换为中文标签
 */
export function getStrategyLabel(strategy: string | null | undefined): string {
  const v = String(strategy || 'balanced');
  if (v === 'urgent_first') return '紧急优先';
  if (v === 'capacity_first') return '产能优先';
  if (v === 'cold_stock_first') return '冷坯消化';
  if (v === 'manual') return '手动调整';
  return '均衡方案';
}


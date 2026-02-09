import type { ForceReleaseViolation } from './types';
import { getStrategyLabelByKey } from '../../types/strategy';

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
  return getStrategyLabelByKey(strategy);
}

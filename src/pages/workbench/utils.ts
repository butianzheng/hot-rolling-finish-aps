import type { ForceReleaseViolation } from './types';

export function extractForceReleaseViolations(details: unknown): ForceReleaseViolation[] {
  if (!details || typeof details !== 'object') return [];
  const violations = (details as { violations?: unknown }).violations;
  if (!Array.isArray(violations)) return [];
  return violations.filter((v): v is ForceReleaseViolation => v != null && typeof v === 'object');
}


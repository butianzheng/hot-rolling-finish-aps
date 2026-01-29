/**
 * RedLineGuard 工具函数
 */

import type { RedLineViolation } from './types';

/**
 * 创建冻结区保护违规
 */
export function createFrozenZoneViolation(
  materialNos: string[],
  message?: string
): RedLineViolation {
  return {
    type: 'FROZEN_ZONE_PROTECTION',
    message: message || '该操作涉及冻结材料，已被系统阻止',
    severity: 'error',
    details: '冻结材料不可自动调整或重排（工业红线1）',
    affectedEntities: materialNos,
  };
}

/**
 * 创建适温约束违规
 */
export function createMaturityViolation(
  materialNos: string[],
  daysToReady: number
): RedLineViolation {
  return {
    type: 'MATURITY_CONSTRAINT',
    message: '材料未适温，无法排产',
    severity: 'warning',
    details: `距离适温还需 ${daysToReady} 天`,
    affectedEntities: materialNos,
  };
}

/**
 * 创建容量约束违规
 */
export function createCapacityViolation(
  message: string,
  details?: string
): RedLineViolation {
  return {
    type: 'CAPACITY_FIRST',
    message,
    severity: 'error',
    details: details || '容量池约束优先于材料排序（工业红线4）',
  };
}

/**
 * 创建可解释性违规
 */
export function createExplainabilityViolation(
  message: string
): RedLineViolation {
  return {
    type: 'EXPLAINABILITY',
    message,
    severity: 'warning',
    details: '所有决策必须提供明确原因（工业红线5）',
  };
}

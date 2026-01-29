/**
 * 材料操作相关类型和工具函数
 */

import { createFrozenZoneViolation, createMaturityViolation } from '../guards/RedLineGuard';
import type { RedLineViolation } from '../guards/RedLineGuard';

export interface Material {
  material_id: string;
  machine_code: string;
  weight_t: number;
  steel_mark: string;
  sched_state: string;
  urgent_level: string;
  lock_flag: boolean;
  manual_urgent_flag: boolean;
  is_frozen?: boolean;
  is_mature?: boolean;
  temp_issue?: boolean;
  urgent_reason?: string;
  eligibility_reason?: string;
  priority_reason?: string;
}

export type OperationType = 'lock' | 'unlock' | 'urgent' | 'clearUrgent' | 'forceRelease';

/**
 * 检查冻结区违规
 */
export function checkFrozenViolation(
  material: Material,
  operation: string,
  adminOverrideMode: boolean
): RedLineViolation | null {
  if (adminOverrideMode) return null;

  if (
    material.is_frozen &&
    (operation === 'lock' || operation === 'unlock' || operation === 'urgent' || operation === 'clearUrgent')
  ) {
    return createFrozenZoneViolation([material.material_id], '该材料位于冻结区，不允许修改状态');
  }
  return null;
}

/**
 * 检查温度约束违规
 */
export function checkTempViolation(
  material: Material,
  operation: string,
  adminOverrideMode: boolean
): RedLineViolation | null {
  if (adminOverrideMode) return null;

  if (!material.is_mature && operation === 'urgent') {
    return createMaturityViolation([material.material_id], 1);
  }
  return null;
}

/**
 * 综合检查 Red Line 违规
 */
export function checkRedLineViolations(
  material: Material,
  operation: string,
  adminOverrideMode: boolean
): RedLineViolation[] {
  const violations: RedLineViolation[] = [];

  const frozenViolation = checkFrozenViolation(material, operation, adminOverrideMode);
  if (frozenViolation) violations.push(frozenViolation);

  const tempViolation = checkTempViolation(material, operation, adminOverrideMode);
  if (tempViolation) violations.push(tempViolation);

  return violations;
}

/**
 * 获取操作模态框标题
 */
export function getOperationModalTitle(modalType: OperationType, count: number): string {
  switch (modalType) {
    case 'lock':
      return `锁定材料 (${count} 件)`;
    case 'unlock':
      return `解锁材料 (${count} 件)`;
    case 'urgent':
      return `设置紧急标志 (${count} 件)`;
    case 'clearUrgent':
      return `取消紧急标志 (${count} 件)`;
    case 'forceRelease':
      return `强制放行 (${count} 件)`;
    default:
      return '操作';
  }
}

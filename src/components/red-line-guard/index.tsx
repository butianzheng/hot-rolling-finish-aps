/**
 * 工业红线防护组件
 *
 * 重构后：354 行 → ~30 行 (-92%)
 *
 * 用于在UI中展示工业红线约束违规信息，防止用户执行违规操作。
 *
 * @example
 * // 紧凑模式：仅显示违规标签
 * <RedLineGuard
 *   violations={[
 *     {
 *       type: 'FROZEN_ZONE_PROTECTION',
 *       message: '该材料已锁定，不可调整',
 *       severity: 'error',
 *     },
 *   ]}
 *   mode="compact"
 * />
 *
 * @example
 * // 详细模式：显示完整违规信息
 * <RedLineGuard
 *   violations={[
 *     {
 *       type: 'MATURITY_CONSTRAINT',
 *       message: '材料未适温，无法排产',
 *       severity: 'warning',
 *       details: '距离适温还需2天',
 *       affectedEntities: ['M12345678', 'M87654321'],
 *     },
 *   ]}
 *   mode="detailed"
 *   closable
 * />
 */

import React from 'react';
import type { RedLineGuardProps } from './types';
import { CompactMode } from './CompactMode';
import { DetailedMode } from './DetailedMode';

export const RedLineGuard: React.FC<RedLineGuardProps> = ({
  violations,
  mode = 'compact',
  closable = false,
  onClose,
}) => {
  // 无违规则不显示
  if (!violations || violations.length === 0) return null;

  // 紧凑模式
  if (mode === 'compact') {
    return <CompactMode violations={violations} />;
  }

  // 详细模式
  return (
    <DetailedMode
      violations={violations}
      closable={closable}
      onClose={onClose}
    />
  );
};

export default RedLineGuard;

// 导出类型和工具函数
export * from './types';
export * from './utils';

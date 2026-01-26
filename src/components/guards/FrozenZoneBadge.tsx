// ==========================================
// 冻结区标识组件
// ==========================================
// 职责: 可视化标识冻结材料，体现红线1：冻结区保护
// ==========================================

import React from 'react';
import { Tag, Tooltip } from 'antd';
import { LockOutlined } from '@ant-design/icons';

// ==========================================
// Props定义
// ==========================================

export interface FrozenZoneBadgeProps {
  /** 是否锁定（冻结） */
  locked: boolean;

  /** 自定义提示信息 */
  tooltipTitle?: string;

  /** 显示模式：badge（小标签）或 banner（横幅） */
  mode?: 'badge' | 'banner';

  /** 是否显示锁定原因 */
  lockReason?: string;
}

// ==========================================
// 主组件
// ==========================================

/**
 * 冻结区标识组件
 *
 * 用于在UI中标识被锁定（冻结）的材料。
 * 体现工业红线1：冻结区保护 - 冻结材料不可自动调整或重排。
 *
 * @example
 * // 基础用法
 * <FrozenZoneBadge locked={material.locked} />
 *
 * @example
 * // 带原因说明的横幅模式
 * <FrozenZoneBadge
 *   locked={material.locked}
 *   mode="banner"
 *   lockReason="已进入冻结区，不可调整"
 * />
 */
export const FrozenZoneBadge: React.FC<FrozenZoneBadgeProps> = ({
  locked,
  tooltipTitle = '冻结材料不可自动调整（红线保护）',
  mode = 'badge',
  lockReason,
}) => {
  // 未锁定则不显示
  if (!locked) return null;

  // Badge模式：小标签
  if (mode === 'badge') {
    return (
      <Tooltip title={lockReason || tooltipTitle}>
        <Tag color="red" icon={<LockOutlined />} style={{ margin: '0 4px' }}>
          冻结区
        </Tag>
      </Tooltip>
    );
  }

  // Banner模式：横幅提示
  return (
    <div
      style={{
        padding: '8px 16px',
        background: '#fff1f0',
        border: '1px solid #ffccc7',
        borderRadius: '4px',
        marginBottom: '16px',
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
      }}
    >
      <LockOutlined style={{ color: '#ff4d4f', fontSize: '16px' }} />
      <div style={{ flex: 1 }}>
        <div style={{ fontWeight: 'bold', color: '#ff4d4f' }}>
          冻结区保护（工业红线1）
        </div>
        <div style={{ fontSize: '12px', color: '#8c8c8c', marginTop: '4px' }}>
          {lockReason || tooltipTitle}
        </div>
      </div>
    </div>
  );
};

// ==========================================
// 默认导出
// ==========================================
export default FrozenZoneBadge;

/**
 * RedLineGuard 类型定义和配置
 */

import React from 'react';
import {
  LockOutlined,
  ClockCircleOutlined,
  ThunderboltOutlined,
  DatabaseOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';

/**
 * 工业红线类型
 */
export type RedLineType =
  | 'FROZEN_ZONE_PROTECTION' // 红线1: 冻结区保护
  | 'MATURITY_CONSTRAINT' // 红线2: 适温约束
  | 'LAYERED_URGENCY' // 红线3: 分层紧急度
  | 'CAPACITY_FIRST' // 红线4: 容量优先
  | 'EXPLAINABILITY'; // 红线5: 可解释性

/**
 * 红线违规信息
 */
export interface RedLineViolation {
  /** 红线类型 */
  type: RedLineType;

  /** 违规描述 */
  message: string;

  /** 违规等级：error（阻塞）或 warning（警告） */
  severity: 'error' | 'warning';

  /** 详细信息（可选） */
  details?: string;

  /** 受影响的实体（可选） */
  affectedEntities?: string[];
}

export interface RedLineGuardProps {
  /** 红线违规列表 */
  violations: RedLineViolation[];

  /** 显示模式：compact（紧凑）或 detailed（详细） */
  mode?: 'compact' | 'detailed';

  /** 是否可关闭 */
  closable?: boolean;

  /** 关闭回调 */
  onClose?: () => void;
}

export interface RedLineMeta {
  label: string;
  icon: React.ReactNode;
  color: string;
  description: string;
}

/**
 * 红线元数据配置
 */
export const RED_LINE_META: Record<RedLineType, RedLineMeta> = {
  FROZEN_ZONE_PROTECTION: {
    label: '冻结区保护',
    icon: React.createElement(LockOutlined),
    color: '#ff4d4f',
    description: '冻结材料不可自动调整或重排',
  },
  MATURITY_CONSTRAINT: {
    label: '适温约束',
    icon: React.createElement(ClockCircleOutlined),
    color: '#faad14',
    description: '未适温材料不可进入排产',
  },
  LAYERED_URGENCY: {
    label: '分层紧急度',
    icon: React.createElement(ThunderboltOutlined),
    color: '#1677ff',
    description: '紧急度采用四级分层制，非评分制',
  },
  CAPACITY_FIRST: {
    label: '容量优先',
    icon: React.createElement(DatabaseOutlined),
    color: '#722ed1',
    description: '容量池约束优先于材料排序',
  },
  EXPLAINABILITY: {
    label: '可解释性',
    icon: React.createElement(InfoCircleOutlined),
    color: '#13c2c2',
    description: '所有决策必须提供明确原因',
  },
};

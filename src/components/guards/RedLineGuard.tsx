// ==========================================
// 工业红线防护组件
// ==========================================
// 职责: 可视化展示5大工业红线约束，防止违规操作
// ==========================================

import React from 'react';
import { Alert, Space, Tag, Tooltip } from 'antd';
import {
  LockOutlined,
  ClockCircleOutlined,
  ThunderboltOutlined,
  DatabaseOutlined,
  InfoCircleOutlined,
  WarningOutlined,
} from '@ant-design/icons';

// ==========================================
// 红线类型定义
// ==========================================

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

// ==========================================
// Props定义
// ==========================================

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

// ==========================================
// 红线元数据配置
// ==========================================

const RED_LINE_META: Record<
  RedLineType,
  {
    label: string;
    icon: React.ReactNode;
    color: string;
    description: string;
  }
> = {
  FROZEN_ZONE_PROTECTION: {
    label: '冻结区保护',
    icon: <LockOutlined />,
    color: '#ff4d4f',
    description: '冻结材料不可自动调整或重排',
  },
  MATURITY_CONSTRAINT: {
    label: '适温约束',
    icon: <ClockCircleOutlined />,
    color: '#faad14',
    description: '未适温材料不可进入排产',
  },
  LAYERED_URGENCY: {
    label: '分层紧急度',
    icon: <ThunderboltOutlined />,
    color: '#1677ff',
    description: '紧急度采用L0-L3分层制，非评分制',
  },
  CAPACITY_FIRST: {
    label: '容量优先',
    icon: <DatabaseOutlined />,
    color: '#722ed1',
    description: '容量池约束优先于材料排序',
  },
  EXPLAINABILITY: {
    label: '可解释性',
    icon: <InfoCircleOutlined />,
    color: '#13c2c2',
    description: '所有决策必须提供明确原因',
  },
};

// ==========================================
// 主组件
// ==========================================

/**
 * 工业红线防护组件
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
export const RedLineGuard: React.FC<RedLineGuardProps> = ({
  violations,
  mode = 'compact',
  closable = false,
  onClose,
}) => {
  // 无违规则不显示
  if (!violations || violations.length === 0) return null;

  // ==========================================
  // 紧凑模式：仅显示标签
  // ==========================================

  if (mode === 'compact') {
    return (
      <Space size="small" wrap>
        {violations.map((violation, index) => {
          const meta = RED_LINE_META[violation.type];
          return (
            <Tooltip
              key={index}
              title={
                <div>
                  <div style={{ fontWeight: 'bold', marginBottom: '4px' }}>
                    {meta.label}
                  </div>
                  <div>{violation.message}</div>
                  {violation.details && (
                    <div style={{ marginTop: '4px', fontSize: '12px' }}>
                      {violation.details}
                    </div>
                  )}
                </div>
              }
            >
              <Tag
                color={violation.severity === 'error' ? 'red' : 'orange'}
                icon={meta.icon}
              >
                {meta.label}
              </Tag>
            </Tooltip>
          );
        })}
      </Space>
    );
  }

  // ==========================================
  // 详细模式：显示完整Alert
  // ==========================================

  return (
    <Space direction="vertical" style={{ width: '100%' }}>
      {violations.map((violation, index) => {
        const meta = RED_LINE_META[violation.type];
        return (
          <Alert
            key={index}
            type={violation.severity === 'error' ? 'error' : 'warning'}
            showIcon
            icon={
              violation.severity === 'error' ? (
                <WarningOutlined />
              ) : (
                <InfoCircleOutlined />
              )
            }
            message={
              <Space>
                {meta.icon}
                <span style={{ fontWeight: 'bold' }}>
                  工业红线：{meta.label}
                </span>
              </Space>
            }
            description={
              <div>
                <div style={{ marginBottom: '8px' }}>{violation.message}</div>
                {violation.details && (
                  <div
                    style={{
                      marginBottom: '8px',
                      fontSize: '12px',
                      color: '#8c8c8c',
                    }}
                  >
                    {violation.details}
                  </div>
                )}
                {violation.affectedEntities &&
                  violation.affectedEntities.length > 0 && (
                    <div>
                      <div
                        style={{
                          fontSize: '12px',
                          marginBottom: '4px',
                          color: '#8c8c8c',
                        }}
                      >
                        受影响实体:
                      </div>
                      <Space size="small" wrap>
                        {violation.affectedEntities.map((entity, idx) => (
                          <Tag key={idx} color="default">
                            {entity}
                          </Tag>
                        ))}
                      </Space>
                    </div>
                  )}
                <div
                  style={{
                    marginTop: '8px',
                    fontSize: '12px',
                    fontStyle: 'italic',
                    color: '#8c8c8c',
                  }}
                >
                  {meta.description}
                </div>
              </div>
            }
            closable={closable}
            onClose={onClose}
            style={{ marginBottom: index < violations.length - 1 ? '12px' : 0 }}
          />
        );
      })}
    </Space>
  );
};

// ==========================================
// 工具函数：创建红线违规对象
// ==========================================

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

// ==========================================
// 默认导出
// ==========================================
export default RedLineGuard;

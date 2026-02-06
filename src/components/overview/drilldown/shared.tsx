/**
 * DrilldownDrawer 共享组件和工具函数
 */

import React from 'react';
import { Button, Space, Table, Tag, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { ReasonItem } from '../../../types/decision';
import type { WorkbenchTabKey } from '../../../hooks/useRiskOverviewData';
import { formatNumber } from '../../../utils/formatters';

export type { WorkbenchTabKey };

export type WorkbenchCallback = (opts: {
  workbenchTab?: WorkbenchTabKey;
  machineCode?: string | null;
  urgencyLevel?: string | null;
}) => void;
import {
  BOTTLENECK_LEVEL_COLORS,
  BOTTLENECK_LEVEL_LABELS,
  OPPORTUNITY_TYPE_COLORS,
  OPPORTUNITY_TYPE_LABELS,
  getBottleneckTypeColor as _getBottleneckTypeColor,
  getBottleneckTypeLabel as _getBottleneckTypeLabel,
  getPressureLevelColor as _getPressureLevelColor,
  getPressureLevelLabel as _getPressureLevelLabel,
  getRiskLevelColor as _getRiskLevelColor,
  getRiskLevelLabel as _getRiskLevelLabel,
  getUrgencyLevelColor as _getUrgencyLevelColor,
  getUrgencyLevelLabel as _getUrgencyLevelLabel,
  parseAlertLevel as _parseAlertLevel,
  getAlertLevelLabel as _getAlertLevelLabel,
  getAlertLevelColor as _getAlertLevelColor,
} from '../../../types/decision';

const { Text } = Typography;

// Re-export color/label functions
export const getUrgencyLevelColor = _getUrgencyLevelColor;
export const getUrgencyLevelLabel = _getUrgencyLevelLabel;
export const getPressureLevelColor = _getPressureLevelColor;
export const getPressureLevelLabel = _getPressureLevelLabel;
export const getRiskLevelColor = _getRiskLevelColor;
export const getRiskLevelLabel = _getRiskLevelLabel;
export const getBottleneckTypeColor = _getBottleneckTypeColor;
export const getBottleneckTypeLabel = _getBottleneckTypeLabel;
export const parseAlertLevel = _parseAlertLevel;
export const getAlertLevelLabel = _getAlertLevelLabel;
export const getAlertLevelColor = _getAlertLevelColor;
export { BOTTLENECK_LEVEL_COLORS, BOTTLENECK_LEVEL_LABELS, OPPORTUNITY_TYPE_COLORS, OPPORTUNITY_TYPE_LABELS };

// Simple Tag component
export const TagWithColor: React.FC<{ color: string; children: React.ReactNode }> = ({ color, children }) => (
  <Tag color={color}>{children}</Tag>
);

/**
 * 原因代码中文翻译映射
 */
const REASON_CODE_LABELS: Record<string, string> = {
  CAPACITY_UTILIZATION: '产能利用率',
  LOW_REMAINING_CAPACITY: '剩余产能不足',
  HIGH_CAPACITY_PRESSURE: '产能压力高',
  STRUCTURE_GAP: '结构性缺口',
  COLD_STOCK_AGING: '冷料库龄',
  ROLL_CHANGE_CONFLICT: '换辊冲突',
  URGENCY_BACKLOG: '紧急订单积压',
  MATURITY_CONSTRAINT: '适温约束',
  OVERLOAD_RISK: '超载风险',
  SCHEDULING_CONFLICT: '排产冲突',
};

/**
 * 获取原因代码的中文标签
 */
function getReasonCodeLabel(code: string): string {
  return REASON_CODE_LABELS[code] || '其他原因';
}

// Reason table columns
export const reasonColumns: ColumnsType<ReasonItem> = [
  {
    title: '代码',
    dataIndex: 'code',
    key: 'code',
    width: 140,
    render: (v: string) => (
      <Tag color="blue" style={{ maxWidth: '130px', overflow: 'hidden', textOverflow: 'ellipsis' }}>
        {getReasonCodeLabel(v)}
      </Tag>
    ),
  },
  {
    title: '原因',
    dataIndex: 'msg',
    key: 'msg',
    ellipsis: { showTitle: true },
    width: 320,
  },
  {
    title: '权重',
    dataIndex: 'weight',
    key: 'weight',
    width: 90,
    render: (v: number) => `${formatNumber(Number(v || 0) * 100, 2)}%`,
  },
  {
    title: '影响数',
    dataIndex: 'affectedCount',
    key: 'affectedCount',
    width: 90,
    render: (v?: number) => (typeof v === 'number' ? v : '-'),
  },
];

// Reason table component
export interface ReasonTableProps {
  reasons: ReasonItem[];
  emptyText?: string;
}

export const ReasonTable: React.FC<ReasonTableProps> = ({ reasons, emptyText = '暂无原因明细' }) => {
  if (!Array.isArray(reasons) || reasons.length === 0) {
    return <Text type="secondary">{emptyText}</Text>;
  }
  return (
    <Table
      rowKey={(r) => r.code}
      size="small"
      columns={reasonColumns}
      dataSource={reasons}
      pagination={false}
    />
  );
};

// Summary header with action button
export interface SummaryHeaderProps {
  tags: React.ReactNode;
  title: string;
  onGoWorkbench?: () => void;
}

export const SummaryHeader: React.FC<SummaryHeaderProps> = ({ tags, title, onGoWorkbench }) => (
  <Space wrap align="center">
    {tags}
    <Text strong>{title}</Text>
    {onGoWorkbench ? (
      <Button size="small" type="primary" onClick={onGoWorkbench}>
        去处理
      </Button>
    ) : null}
  </Space>
);

// Actions list component
export interface ActionsListProps {
  title: string;
  actions: string[];
  maxItems?: number;
  colorTextSecondary?: string;
}

export const ActionsList: React.FC<ActionsListProps> = ({
  title,
  actions,
  maxItems = 6,
  colorTextSecondary = '#8c8c8c',
}) => {
  if (!Array.isArray(actions) || actions.length === 0) return null;
  return (
    <div>
      <Text strong>{title}</Text>
      <div style={{ marginTop: 6 }}>
        {actions.slice(0, maxItems).map((a, idx) => (
          <div key={`${idx}-${a}`} style={{ color: colorTextSecondary }}>
            · {a}
          </div>
        ))}
      </div>
    </div>
  );
};

// Parse opportunity type
export function parseOpportunityType(typeStr: string) {
  const upper = String(typeStr || '').toUpperCase().replace(/-/g, '_');
  if ((Object.keys(OPPORTUNITY_TYPE_COLORS) as string[]).includes(upper)) return upper;
  return 'UNDERUTILIZED';
}

// Common table row highlight style
export function getHighlightStyle(
  isHighlighted: boolean,
  token: { colorFillQuaternary: string }
): React.CSSProperties | undefined {
  return isHighlighted ? { background: token.colorFillQuaternary } : undefined;
}

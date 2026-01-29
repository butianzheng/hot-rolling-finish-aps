/**
 * DrilldownDrawer 共享组件和工具函数
 */

import React from 'react';
import { Button, Space, Table, Tag, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { ReasonItem } from '../../../types/decision';
import type { WorkbenchTabKey } from '../../../hooks/useRiskOverviewData';

export type { WorkbenchTabKey };

export type WorkbenchCallback = (opts: {
  workbenchTab?: WorkbenchTabKey;
  machineCode?: string | null;
  urgencyLevel?: string | null;
}) => void;
import {
  BOTTLENECK_LEVEL_COLORS,
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
export { BOTTLENECK_LEVEL_COLORS, OPPORTUNITY_TYPE_COLORS, OPPORTUNITY_TYPE_LABELS };

// Simple Tag component
export const TagWithColor: React.FC<{ color: string; children: React.ReactNode }> = ({ color, children }) => (
  <Tag color={color}>{children}</Tag>
);

// Reason table columns
export const reasonColumns: ColumnsType<ReasonItem> = [
  { title: '代码', dataIndex: 'code', key: 'code', width: 120, render: (v: string) => <Tag>{v}</Tag> },
  { title: '原因', dataIndex: 'msg', key: 'msg', ellipsis: true },
  {
    title: '权重',
    dataIndex: 'weight',
    key: 'weight',
    width: 90,
    render: (v: number) => `${(Number(v || 0) * 100).toFixed(1)}%`,
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

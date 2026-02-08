/**
 * 甘特图工具栏和图例
 */

import React from 'react';
import { Button, DatePicker, Space, Tag, Typography } from 'antd';
import type { Dayjs } from 'dayjs';
import { URGENCY_COLORS } from '../../theme/tokens';
import { formatWeight } from '../../utils/formatters';
import type { PlanItemStatusFilter, PlanItemStatusSummary } from '../../utils/planItemStatus';
import { PLAN_ITEM_STATUS_FILTER_META } from '../../utils/planItemStatus';

const { RangePicker } = DatePicker;
const { Text } = Typography;

interface GanttToolbarProps {
  range: [Dayjs, Dayjs];
  onRangeChange: (values: null | [Dayjs | null, Dayjs | null]) => void;
  onResetRange: () => void;
  onScrollToToday: () => void;
  dateKeysLength: number;
  machinesCount: number;
  filteredCount: number;
  filteredTotalWeight: number;
  dateRangeStart: string;
  dateRangeEnd: string;
  statusSummary: PlanItemStatusSummary;
  statusFilter: PlanItemStatusFilter;
  onStatusFilterChange?: (next: PlanItemStatusFilter) => void;
}

export const GanttToolbar: React.FC<GanttToolbarProps> = ({
  range,
  onRangeChange,
  onResetRange,
  onScrollToToday,
  dateKeysLength,
  machinesCount,
  filteredCount,
  filteredTotalWeight,
  dateRangeStart,
  dateRangeEnd,
  statusSummary,
  statusFilter,
  onStatusFilterChange,
}) => {
  const canChangeStatus = !!onStatusFilterChange;
  const toggleStatus = (next: PlanItemStatusFilter) => {
    if (!onStatusFilterChange) return;
    if (next === 'ALL') {
      onStatusFilterChange('ALL');
      return;
    }
    onStatusFilterChange(statusFilter === next ? 'ALL' : next);
  };

  const legend = (
    <Space size={8} wrap>
      <Tag color={URGENCY_COLORS.L3_EMERGENCY}>L3</Tag>
      <Tag color={URGENCY_COLORS.L2_HIGH} style={{ color: 'rgba(0, 0, 0, 0.85)' }}>
        L2
      </Tag>
      <Tag color={URGENCY_COLORS.L1_MEDIUM}>L1</Tag>
      <Tag color={URGENCY_COLORS.L0_NORMAL}>L0</Tag>
      <Tag color="red">超上限</Tag>
      <Tag color="orange">超目标</Tag>
      <Text type="secondary" style={{ fontSize: 12 }}>
        Ctrl/⌘+点击：切换选中 · 双击空白：同日明细
      </Text>
    </Space>
  );

  return (
    <Space wrap style={{ marginBottom: 8, justifyContent: 'space-between', width: '100%' }}>
      <Space wrap>
        <RangePicker
          size="small"
          value={range}
          onChange={(values) => onRangeChange(values as [Dayjs | null, Dayjs | null] | null)}
          allowClear
        />
        <Button size="small" onClick={onResetRange}>
          重置范围
        </Button>
        <Button size="small" onClick={onScrollToToday} disabled={dateKeysLength === 0}>
          定位今天
        </Button>

        <Space size={6} wrap>
          <Tag
            color={PLAN_ITEM_STATUS_FILTER_META.ALL.color}
            style={{
              cursor: canChangeStatus ? 'pointer' : undefined,
              boxShadow: statusFilter === 'ALL' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
              userSelect: 'none',
            }}
            onClick={() => canChangeStatus && toggleStatus('ALL')}
            title={`已排 ${statusSummary.totalCount} 件 / ${formatWeight(statusSummary.totalWeightT)}`}
          >
            已排 {statusSummary.totalCount}
          </Tag>
          <Tag
            color={PLAN_ITEM_STATUS_FILTER_META.LOCKED.color}
            style={{
              cursor:
                canChangeStatus && (statusSummary.lockedInPlanCount > 0 || statusFilter === 'LOCKED')
                  ? 'pointer'
                  : 'not-allowed',
              opacity:
                canChangeStatus && statusSummary.lockedInPlanCount === 0 && statusFilter !== 'LOCKED' ? 0.35 : 1,
              boxShadow: statusFilter === 'LOCKED' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
              userSelect: 'none',
            }}
            onClick={() => {
              if (!canChangeStatus) return;
              if (statusSummary.lockedInPlanCount === 0 && statusFilter !== 'LOCKED') return;
              toggleStatus('LOCKED');
            }}
            title={`冻结 ${statusSummary.lockedInPlanCount} 件 / ${formatWeight(statusSummary.lockedInPlanWeightT)}`}
          >
            冻结 {statusSummary.lockedInPlanCount}
          </Tag>
          <Tag
            color={PLAN_ITEM_STATUS_FILTER_META.FORCE_RELEASE.color}
            style={{
              cursor:
                canChangeStatus && (statusSummary.forceReleaseCount > 0 || statusFilter === 'FORCE_RELEASE')
                  ? 'pointer'
                  : 'not-allowed',
              opacity:
                canChangeStatus && statusSummary.forceReleaseCount === 0 && statusFilter !== 'FORCE_RELEASE' ? 0.35 : 1,
              boxShadow:
                statusFilter === 'FORCE_RELEASE' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
              userSelect: 'none',
            }}
            onClick={() => {
              if (!canChangeStatus) return;
              if (statusSummary.forceReleaseCount === 0 && statusFilter !== 'FORCE_RELEASE') return;
              toggleStatus('FORCE_RELEASE');
            }}
            title={`强制放行 ${statusSummary.forceReleaseCount} 件 / ${formatWeight(statusSummary.forceReleaseWeightT)}`}
          >
            强放 {statusSummary.forceReleaseCount}
          </Tag>
          <Tag
            color={PLAN_ITEM_STATUS_FILTER_META.ADJUSTABLE.color}
            style={{
              cursor:
                canChangeStatus && (statusSummary.adjustableCount > 0 || statusFilter === 'ADJUSTABLE')
                  ? 'pointer'
                  : 'not-allowed',
              opacity:
                canChangeStatus && statusSummary.adjustableCount === 0 && statusFilter !== 'ADJUSTABLE' ? 0.35 : 1,
              boxShadow:
                statusFilter === 'ADJUSTABLE' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
              userSelect: 'none',
            }}
            onClick={() => {
              if (!canChangeStatus) return;
              if (statusSummary.adjustableCount === 0 && statusFilter !== 'ADJUSTABLE') return;
              toggleStatus('ADJUSTABLE');
            }}
            title={`可调（非冻结）${statusSummary.adjustableCount} 件 / ${formatWeight(statusSummary.adjustableWeightT)}`}
          >
            可调 {statusSummary.adjustableCount}
          </Tag>
          <Tag
            color={PLAN_ITEM_STATUS_FILTER_META.READY.color}
            style={{
              cursor: canChangeStatus ? 'pointer' : undefined,
              boxShadow: statusFilter === 'READY' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
              userSelect: 'none',
            }}
            onClick={() => canChangeStatus && toggleStatus('READY')}
          >
            就绪
          </Tag>
          <Tag
            color={PLAN_ITEM_STATUS_FILTER_META.PENDING_MATURE.color}
            style={{
              cursor: canChangeStatus ? 'pointer' : undefined,
              boxShadow: statusFilter === 'PENDING_MATURE' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
              userSelect: 'none',
            }}
            onClick={() => canChangeStatus && toggleStatus('PENDING_MATURE')}
          >
            待成熟
          </Tag>
          <Tag
            color={PLAN_ITEM_STATUS_FILTER_META.BLOCKED.color}
            style={{
              cursor: canChangeStatus ? 'pointer' : undefined,
              boxShadow: statusFilter === 'BLOCKED' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
              userSelect: 'none',
            }}
            onClick={() => canChangeStatus && toggleStatus('BLOCKED')}
          >
            阻断
          </Tag>
          <Tag
            color={PLAN_ITEM_STATUS_FILTER_META.URGENT_L3.color}
            style={{
              cursor: canChangeStatus ? 'pointer' : undefined,
              boxShadow: statusFilter === 'URGENT_L3' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
              userSelect: 'none',
            }}
            onClick={() => canChangeStatus && toggleStatus('URGENT_L3')}
          >
            L3
          </Tag>
          <Tag
            color={PLAN_ITEM_STATUS_FILTER_META.URGENT_L2.color}
            style={{
              cursor: canChangeStatus ? 'pointer' : undefined,
              boxShadow: statusFilter === 'URGENT_L2' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
              userSelect: 'none',
            }}
            onClick={() => canChangeStatus && toggleStatus('URGENT_L2')}
          >
            L2
          </Tag>
        </Space>
      </Space>
      {legend}
      <Text type="secondary" style={{ fontSize: 12 }}>
        机组 {machinesCount} · 任务 {filteredCount} · 总重 {formatWeight(filteredTotalWeight)} · 范围 {dateRangeStart || '-'} ~{' '}
        {dateRangeEnd || '-'}
      </Text>
    </Space>
  );
};

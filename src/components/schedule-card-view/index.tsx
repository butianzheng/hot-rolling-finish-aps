import React, { useMemo } from 'react';
import { Alert, Empty, Skeleton, Space, Tag } from 'antd';
import { useActiveVersionId } from '../../stores/use-global-store';
import type { PlanItemStatusFilter } from '../../utils/planItemStatus';
import { PLAN_ITEM_STATUS_FILTER_META, matchPlanItemStatusFilter, summarizePlanItemStatus } from '../../utils/planItemStatus';
import { formatWeight } from '../../utils/formatters';
import type { ScheduleCardViewProps } from './types';
import { MACHINE_HEADER_HEIGHT, DATE_ROW_HEIGHT } from './types';
import { usePlanItems, normalizePlanItems } from './usePlanItems';
import { useFilteredPlanItems } from './useFilteredPlanItems';
import { useScheduleTree } from './useScheduleTree';
import { ScheduleCardRow } from './ScheduleCardRow';
import { CountInfo } from './CountInfo';

const ScheduleCardView: React.FC<ScheduleCardViewProps> = ({
  machineCode,
  urgentLevel,
  dateRange,
  statusFilter = 'ALL',
  onStatusFilterChange,
}) => {
  const activeVersionId = useActiveVersionId();
  const query = usePlanItems(machineCode, dateRange);

  const items = useMemo(() => {
    return normalizePlanItems(query.data);
  }, [query.data]);

  const inRange = useFilteredPlanItems(items, machineCode, urgentLevel, dateRange);
  const statusSummary = useMemo(() => summarizePlanItemStatus(inRange), [inRange]);
  const filtered = useMemo(() => {
    return statusFilter ? inRange.filter((it) => matchPlanItemStatusFilter(it, statusFilter)) : inRange;
  }, [inRange, statusFilter]);
  const filteredTotalWeight = useMemo(() => {
    return filtered.reduce((acc, it) => acc + Number(it.weight_t || 0), 0);
  }, [filtered]);

  // 树形分解图数据（机组 → 日期条形图）
  const { rows, toggleMachine } = useScheduleTree(filtered);

  if (!activeVersionId) {
    return (
      <Alert
        type="warning"
        showIcon
        message="尚无激活的排产版本"
        description='请先在"版本对比"页激活一个版本'
      />
    );
  }

  if (query.error) {
    return (
      <Alert
        type="error"
        showIcon
        message="排程数据加载失败"
        description={query.error instanceof Error ? query.error.message : String(query.error)}
        action={
          <a onClick={() => query.refetch()} style={{ padding: '0 8px' }}>
            重试
          </a>
        }
      />
    );
  }

  if (query.isLoading) {
    return <Skeleton active paragraph={{ rows: 10 }} />;
  }

  const showEmpty = rows.length === 0;

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', minHeight: 0 }}>
      <div style={{ marginBottom: 8, display: 'flex', justifyContent: 'space-between', gap: 8, flexWrap: 'wrap' }}>
        <Space size={6} wrap>
          <Tag
            color={PLAN_ITEM_STATUS_FILTER_META.ALL.color}
            style={{
              cursor: onStatusFilterChange ? 'pointer' : undefined,
              boxShadow: statusFilter === 'ALL' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
              userSelect: 'none',
            }}
            onClick={() => onStatusFilterChange?.('ALL')}
            title={`已排 ${statusSummary.totalCount} 件 / ${formatWeight(statusSummary.totalWeightT)}`}
          >
            已排 {statusSummary.totalCount}
          </Tag>
          <Tag
            color={PLAN_ITEM_STATUS_FILTER_META.LOCKED.color}
            style={{
              cursor:
                onStatusFilterChange &&
                (statusSummary.lockedInPlanCount > 0 || statusFilter === 'LOCKED')
                  ? 'pointer'
                  : 'not-allowed',
              opacity:
                onStatusFilterChange && statusSummary.lockedInPlanCount === 0 && statusFilter !== 'LOCKED' ? 0.35 : 1,
              boxShadow: statusFilter === 'LOCKED' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
              userSelect: 'none',
            }}
            onClick={() => {
              if (!onStatusFilterChange) return;
              if (statusSummary.lockedInPlanCount === 0 && statusFilter !== 'LOCKED') return;
              onStatusFilterChange(statusFilter === 'LOCKED' ? 'ALL' : ('LOCKED' as PlanItemStatusFilter));
            }}
            title={`冻结 ${statusSummary.lockedInPlanCount} 件 / ${formatWeight(statusSummary.lockedInPlanWeightT)}`}
          >
            冻结 {statusSummary.lockedInPlanCount}
          </Tag>
          <Tag
            color={PLAN_ITEM_STATUS_FILTER_META.FORCE_RELEASE.color}
            style={{
              cursor:
                onStatusFilterChange &&
                (statusSummary.forceReleaseCount > 0 || statusFilter === 'FORCE_RELEASE')
                  ? 'pointer'
                  : 'not-allowed',
              opacity:
                onStatusFilterChange && statusSummary.forceReleaseCount === 0 && statusFilter !== 'FORCE_RELEASE' ? 0.35 : 1,
              boxShadow: statusFilter === 'FORCE_RELEASE' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
              userSelect: 'none',
            }}
            onClick={() => {
              if (!onStatusFilterChange) return;
              if (statusSummary.forceReleaseCount === 0 && statusFilter !== 'FORCE_RELEASE') return;
              onStatusFilterChange(statusFilter === 'FORCE_RELEASE' ? 'ALL' : ('FORCE_RELEASE' as PlanItemStatusFilter));
            }}
            title={`强制放行 ${statusSummary.forceReleaseCount} 件 / ${formatWeight(statusSummary.forceReleaseWeightT)}`}
          >
            强放 {statusSummary.forceReleaseCount}
          </Tag>
          <Tag
            color={PLAN_ITEM_STATUS_FILTER_META.ADJUSTABLE.color}
            style={{
              cursor:
                onStatusFilterChange &&
                (statusSummary.adjustableCount > 0 || statusFilter === 'ADJUSTABLE')
                  ? 'pointer'
                  : 'not-allowed',
              opacity:
                onStatusFilterChange && statusSummary.adjustableCount === 0 && statusFilter !== 'ADJUSTABLE' ? 0.35 : 1,
              boxShadow: statusFilter === 'ADJUSTABLE' ? '0 0 0 2px rgba(22, 119, 255, 0.25)' : undefined,
              userSelect: 'none',
            }}
            onClick={() => {
              if (!onStatusFilterChange) return;
              if (statusSummary.adjustableCount === 0 && statusFilter !== 'ADJUSTABLE') return;
              onStatusFilterChange(statusFilter === 'ADJUSTABLE' ? 'ALL' : ('ADJUSTABLE' as PlanItemStatusFilter));
            }}
            title={`可调（非冻结）${statusSummary.adjustableCount} 件 / ${formatWeight(statusSummary.adjustableWeightT)}`}
          >
            可调 {statusSummary.adjustableCount}
          </Tag>
        </Space>
        <Space size={10} wrap>
          <CountInfo count={filtered.length} />
          <span style={{ color: '#8c8c8c', fontSize: 12 }}>
            总重 {formatWeight(filteredTotalWeight)}
          </span>
        </Space>
      </div>

      <div style={{ flex: 1, minHeight: 240, overflow: 'auto' }}>
        {showEmpty ? (
          <div style={{ padding: 24 }}>
            <Empty
              description={
                inRange.length === 0
                  ? '暂无排程数据（可先执行"一键优化/重算"生成排程）'
                  : '当前筛选下暂无排程数据（可切换状态标签）'
              }
            />
          </div>
        ) : (
          <div>
            {rows.map((row) => (
              <ScheduleCardRow
                key={row.type === 'machine' ? `m-${row.machineCode}` : `d-${row.machineCode}-${row.date}`}
                row={row}
                style={{ height: row.type === 'machine' ? MACHINE_HEADER_HEIGHT : DATE_ROW_HEIGHT }}
                onToggleMachine={toggleMachine}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default React.memo(ScheduleCardView);

import React, { useMemo } from 'react';
import { Alert, Empty, Skeleton } from 'antd';
import AutoSizer from 'react-virtualized-auto-sizer';
import { List } from 'react-window';
import { useActiveVersionId } from '../../stores/use-global-store';
import type { ScheduleCardViewProps } from './types';
import { ROW_HEIGHT } from './types';
import { usePlanItems, normalizePlanItems } from './usePlanItems';
import { useFilteredPlanItems } from './useFilteredPlanItems';
import { ScheduleCardRow } from './ScheduleCardRow';
import { CountInfo } from './CountInfo';

const ScheduleCardView: React.FC<ScheduleCardViewProps> = ({
  machineCode,
  urgentLevel,
  refreshSignal,
  selectedMaterialIds,
  onSelectedMaterialIdsChange,
  onInspectMaterialId,
}) => {
  const activeVersionId = useActiveVersionId();
  const query = usePlanItems(machineCode, refreshSignal);

  const items = useMemo(() => {
    return normalizePlanItems(query.data);
  }, [query.data]);

  const filtered = useFilteredPlanItems(items, machineCode, urgentLevel);

  const selectedSet = useMemo(() => new Set(selectedMaterialIds), [selectedMaterialIds]);

  const toggleSelection = (materialId: string, checked: boolean) => {
    const next = new Set(selectedSet);
    if (checked) next.add(materialId);
    else next.delete(materialId);
    onSelectedMaterialIdsChange(Array.from(next));
  };

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
        description={String((query.error as any)?.message || query.error)}
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

  if (filtered.length === 0) {
    return (
      <div style={{ padding: 24 }}>
        <Empty description='暂无排程数据（可先执行"一键优化/重算"生成排程）' />
      </div>
    );
  }

  return (
    <div style={{ height: '100%' }}>
      <div style={{ marginBottom: 8 }}>
        <CountInfo count={filtered.length} />
      </div>

      <div style={{ height: 'calc(100% - 24px)' }}>
        <AutoSizer>
          {({ height, width }) => (
            <List
              rowCount={filtered.length}
              rowHeight={ROW_HEIGHT}
              rowComponent={ScheduleCardRow}
              rowProps={{
                items: filtered,
                selected: selectedSet,
                onToggle: toggleSelection,
                onInspect: onInspectMaterialId,
              }}
              style={{ height, width }}
            />
          )}
        </AutoSizer>
      </div>
    </div>
  );
};

export default React.memo(ScheduleCardView);

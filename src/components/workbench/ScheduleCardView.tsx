import React, { useEffect, useMemo } from 'react';
import { Alert, Card, Checkbox, Empty, Skeleton, Space, Tag, Typography } from 'antd';
import { useQuery } from '@tanstack/react-query';
import AutoSizer from 'react-virtualized-auto-sizer';
import { List, type RowComponentProps } from 'react-window';
import { planApi } from '../../api/tauri';
import { useActiveVersionId } from '../../stores/use-global-store';
import { FONT_FAMILIES } from '../../theme';
import { formatWeight } from '../../utils/formatters';

const { Text } = Typography;

interface PlanItemRow {
  material_id: string;
  machine_code: string;
  plan_date: string;
  seq_no: number;
  weight_t: number;
  urgent_level?: string;
  locked_in_plan?: boolean;
  force_release_in_plan?: boolean;
}

interface ScheduleCardViewProps {
  machineCode?: string | null;
  urgentLevel?: string | null;
  refreshSignal?: number;
  selectedMaterialIds: string[];
  onSelectedMaterialIdsChange: (ids: string[]) => void;
  onInspectMaterialId?: (materialId: string) => void;
}

const ROW_HEIGHT = 92;

const ScheduleCardView: React.FC<ScheduleCardViewProps> = ({
  machineCode,
  urgentLevel,
  refreshSignal,
  selectedMaterialIds,
  onSelectedMaterialIdsChange,
  onInspectMaterialId,
}) => {
  const activeVersionId = useActiveVersionId();

  const query = useQuery({
    queryKey: ['planItems', activeVersionId, machineCode || 'all'],
    enabled: !!activeVersionId,
    queryFn: async () => {
      if (!activeVersionId) return [];
      const res = await planApi.listPlanItems(activeVersionId, {
        machine_code: machineCode && machineCode !== 'all' ? machineCode : undefined,
      });
      return Array.isArray(res) ? res : [];
    },
    staleTime: 30 * 1000,
  });

  useEffect(() => {
    if (!activeVersionId) return;
    if (refreshSignal == null) return;
    query.refetch();
  }, [activeVersionId, refreshSignal, query.refetch]);

  const items = useMemo<PlanItemRow[]>(() => {
    const raw = Array.isArray(query.data) ? query.data : [];
    return raw.map((it: any) => ({
      material_id: String(it?.material_id ?? ''),
      machine_code: String(it?.machine_code ?? ''),
      plan_date: String(it?.plan_date ?? ''),
      seq_no: Number(it?.seq_no ?? 0),
      weight_t: Number(it?.weight_t ?? 0),
      urgent_level: it?.urgent_level ? String(it.urgent_level) : undefined,
      locked_in_plan: !!it?.locked_in_plan,
      force_release_in_plan: !!it?.force_release_in_plan,
    }));
  }, [query.data]);

  const filtered = useMemo(() => {
    let list = items;
    if (machineCode && machineCode !== 'all') {
      list = list.filter((it) => it.machine_code === machineCode);
    }
    if (urgentLevel && urgentLevel !== 'all') {
      const want = String(urgentLevel).toUpperCase();
      list = list.filter((it) => String(it.urgent_level || 'L0').toUpperCase() === want);
    }
    return [...list].sort((a, b) => {
      if (a.plan_date !== b.plan_date) return a.plan_date.localeCompare(b.plan_date);
      if (a.machine_code !== b.machine_code) return a.machine_code.localeCompare(b.machine_code);
      return a.seq_no - b.seq_no;
    });
  }, [items, machineCode, urgentLevel]);

  const selectedSet = useMemo(() => new Set(selectedMaterialIds), [selectedMaterialIds]);

  const toggleSelection = (materialId: string, checked: boolean) => {
    const next = new Set(selectedSet);
    if (checked) next.add(materialId);
    else next.delete(materialId);
    onSelectedMaterialIdsChange(Array.from(next));
  };

  type RowData = {
    items: PlanItemRow[];
    selected: Set<string>;
    onToggle: (id: string, checked: boolean) => void;
    onInspect?: (id: string) => void;
  };

  const Row = ({ index, style, items, selected, onToggle, onInspect }: RowComponentProps<RowData>) => {
    const it = items[index];
    const checked = selected.has(it.material_id);
    const urgent = String(it.urgent_level || 'L0');

    const urgentColor =
      urgent === 'L3' ? 'red' : urgent === 'L2' ? 'orange' : urgent === 'L1' ? 'blue' : 'default';

    return (
      <div style={{ ...style, padding: '0 8px' }}>
        <Card
          size="small"
          style={{ cursor: 'pointer' }}
          onClick={() => onInspect?.(it.material_id)}
        >
          <Space align="start" style={{ width: '100%', justifyContent: 'space-between' }}>
            <Space align="start" size={10}>
              <Checkbox
                checked={checked}
                onClick={(e) => e.stopPropagation()}
                onChange={(e) => onToggle(it.material_id, e.target.checked)}
              />
              <Space direction="vertical" size={2} style={{ minWidth: 0 }}>
                <Text style={{ fontFamily: FONT_FAMILIES.MONOSPACE }} strong ellipsis>
                  {it.material_id}
                </Text>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {it.machine_code} · {it.plan_date} · 序{it.seq_no}
                </Text>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {formatWeight(it.weight_t)}
                </Text>
              </Space>
            </Space>

            <Space direction="vertical" size={4} align="end">
              <Tag color={urgentColor}>{urgent}</Tag>
              {it.locked_in_plan && <Tag color="purple">冻结</Tag>}
              {it.force_release_in_plan && <Tag color="orange">强制放行</Tag>}
            </Space>
          </Space>
        </Card>
      </div>
    );
  };

  if (!activeVersionId) {
    return (
      <Alert
        type="warning"
        showIcon
        message="尚无激活的排产版本"
        description="请先在“版本对比”页激活一个版本"
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
        <Empty description="暂无排程数据（可先执行“一键优化/重算”生成排程）" />
      </div>
    );
  }

  return (
    <div style={{ height: '100%' }}>
      <div style={{ marginBottom: 8 }}>
        <Text type="secondary" style={{ fontSize: 12 }}>
          共 {filtered.length} 条
        </Text>
      </div>

      <div style={{ height: 'calc(100% - 24px)' }}>
        <AutoSizer>
          {({ height, width }) => (
            <List
              rowCount={filtered.length}
              rowHeight={ROW_HEIGHT}
              rowComponent={Row}
              rowProps={{
                items: filtered,
                selected: selectedSet,
                onToggle: toggleSelection,
                onInspect: onInspectMaterialId,
              }}
              style={{ height, width }}
            >
            </List>
          )}
        </AutoSizer>
      </div>
    </div>
  );
};

export default React.memo(ScheduleCardView);

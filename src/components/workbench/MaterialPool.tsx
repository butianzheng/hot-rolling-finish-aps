import React, { useMemo, useState } from 'react';
import { DownOutlined, RightOutlined } from '@ant-design/icons';
import { Alert, Button, Card, Checkbox, Empty, Input, Select, Skeleton, Space, Tree, Typography } from 'antd';
import type { DataNode } from 'antd/es/tree';
import AutoSizer from 'react-virtualized-auto-sizer';
import { List, type RowComponentProps } from 'react-window';
import { MaterialStatusIcons } from '../MaterialStatusIcons';
import { UrgencyTag } from '../UrgencyTag';
import { FONT_FAMILIES } from '../../theme';
import type { WorkbenchLockStatusFilter } from '../../stores/use-global-store';
import { getSchedStateLabel, normalizeSchedState } from '../../utils/schedState';

const { Text } = Typography;

export interface MaterialPoolMaterial {
  material_id: string;
  machine_code: string;
  weight_t: number;
  steel_mark: string;
  sched_state: string;
  urgent_level: string;
  lock_flag: boolean;
  manual_urgent_flag: boolean;
  is_frozen?: boolean;
  is_mature?: boolean;
  temp_issue?: boolean;
}

export interface MaterialPoolSelection {
  machineCode: string | null;
  schedState: string | null;
}

export interface MaterialPoolFilters {
  urgencyLevel: string | null;
  lockStatus: WorkbenchLockStatusFilter;
}

interface MaterialPoolProps {
  materials: MaterialPoolMaterial[];
  loading?: boolean;
  error?: unknown;
  onRetry?: () => void;

  selection: MaterialPoolSelection;
  onSelectionChange: (next: MaterialPoolSelection) => void;

  filters?: MaterialPoolFilters;
  onFiltersChange?: (next: Partial<MaterialPoolFilters>) => void;

  selectedMaterialIds: string[];
  onSelectedMaterialIdsChange: (ids: string[]) => void;
  onInspectMaterial?: (material: MaterialPoolMaterial) => void;
}

function buildTreeData(materials: MaterialPoolMaterial[]): DataNode[] {
  const machineMap = new Map<string, Map<string, number>>();
  materials.forEach((m) => {
    const machine = String(m.machine_code || '').trim() || 'UNKNOWN';
    const state = normalizeSchedState(m.sched_state);
    if (!machineMap.has(machine)) machineMap.set(machine, new Map());
    const stateMap = machineMap.get(machine)!;
    stateMap.set(state, (stateMap.get(state) ?? 0) + 1);
  });

  const machines = Array.from(machineMap.keys()).sort();
  const preferredStates = ['READY', 'PENDING_MATURE', 'FORCE_RELEASE', 'LOCKED', 'SCHEDULED', 'BLOCKED'];

  return [
    {
      key: 'all',
      title: (
        <Space size={8}>
          <Text strong>全部机组</Text>
          <Text type="secondary">({materials.length})</Text>
        </Space>
      ),
      isLeaf: true,
    },
    ...machines.map((machine) => {
      const stateMap = machineMap.get(machine)!;
      const states = Array.from(stateMap.keys()).sort((a, b) => {
        const ai = preferredStates.indexOf(a);
        const bi = preferredStates.indexOf(b);
        if (ai !== -1 || bi !== -1) return (ai === -1 ? 99 : ai) - (bi === -1 ? 99 : bi);
        return a.localeCompare(b);
      });

      return {
        key: `machine:${machine}`,
        title: (
          <Space size={8}>
            <Text strong>{machine}</Text>
            <Text type="secondary">({states.reduce((sum, s) => sum + (stateMap.get(s) ?? 0), 0)})</Text>
          </Space>
        ),
        children: states.map((state) => {
          const count = stateMap.get(state) ?? 0;
          return {
            key: `machine:${machine}/state:${state}`,
            title: (
              <Space size={8}>
                <Text>{getSchedStateLabel(state)}</Text>
                <Text type="secondary">({count})</Text>
              </Space>
            ),
            isLeaf: true,
          };
        }),
      };
    }),
  ];
}

function parseTreeKey(key: string): MaterialPoolSelection {
  if (!key.startsWith('machine:')) return { machineCode: null, schedState: null };
  const rest = key.slice('machine:'.length);
  const [machineCode, statePart] = rest.split('/state:');
  if (!machineCode) return { machineCode: null, schedState: null };
  return { machineCode, schedState: statePart || null };
}

function selectionToTreeKey(selection: MaterialPoolSelection): string | null {
  if (!selection.machineCode) return 'all';
  if (selection.schedState) return `machine:${selection.machineCode}/state:${selection.schedState}`;
  return `machine:${selection.machineCode}`;
}

const ROW_HEIGHT = 56;

type UrgencyBucket = 'L3' | 'L2' | 'L1' | 'L0';
const URGENCY_ORDER: UrgencyBucket[] = ['L3', 'L2', 'L1', 'L0'];

function normalizeUrgencyLevel(level: string | null | undefined): UrgencyBucket {
  const v = String(level || '').toUpperCase().trim();
  if (v === 'L3') return 'L3';
  if (v === 'L2') return 'L2';
  if (v === 'L1') return 'L1';
  return 'L0';
}

type PoolRow =
  | { type: 'header'; level: UrgencyBucket; count: number; weight: number; collapsed: boolean }
  | { type: 'material'; material: MaterialPoolMaterial };

const MaterialPool: React.FC<MaterialPoolProps> = ({
  materials,
  loading,
  error,
  onRetry,
  selection,
  onSelectionChange,
  filters,
  onFiltersChange,
  selectedMaterialIds,
  onSelectedMaterialIdsChange,
  onInspectMaterial,
}) => {
  const [searchText, setSearchText] = useState('');
  const [groupByUrgency, setGroupByUrgency] = useState(true);
  const [collapsedUrgency, setCollapsedUrgency] = useState<Record<UrgencyBucket, boolean>>({
    L3: false,
    L2: false,
    L1: false,
    L0: false,
  });

  const treeData = useMemo(() => buildTreeData(materials), [materials]);
  const selectedTreeKey = selectionToTreeKey(selection);

  const filtered = useMemo(() => {
    let list = materials;
    if (selection.machineCode) {
      list = list.filter((m) => String(m.machine_code || '') === selection.machineCode);
    }
    if (selection.schedState) {
      const want = normalizeSchedState(selection.schedState);
      list = list.filter((m) => normalizeSchedState(m.sched_state) === want);
    }
    if (filters?.urgencyLevel) {
      const want = String(filters.urgencyLevel).toUpperCase();
      list = list.filter((m) => String(m.urgent_level || '').toUpperCase() === want);
    }
    if (filters?.lockStatus === 'LOCKED') {
      list = list.filter((m) => !!m.lock_flag);
    } else if (filters?.lockStatus === 'UNLOCKED') {
      list = list.filter((m) => !m.lock_flag);
    }
    if (searchText.trim()) {
      const q = searchText.trim().toLowerCase();
      list = list.filter((m) => String(m.material_id || '').toLowerCase().includes(q));
    }

    return [...list].sort((a, b) => {
      const ma = String(a.machine_code || '');
      const mb = String(b.machine_code || '');
      if (ma !== mb) return ma.localeCompare(mb);
      const ua = String(a.urgent_level || 'L0');
      const ub = String(b.urgent_level || 'L0');
      if (ua !== ub) return ub.localeCompare(ua);
      return String(a.material_id || '').localeCompare(String(b.material_id || ''));
    });
  }, [filters?.lockStatus, filters?.urgencyLevel, materials, searchText, selection.machineCode, selection.schedState]);

  const rows = useMemo<PoolRow[]>(() => {
    if (!groupByUrgency) {
      return filtered.map((m) => ({ type: 'material', material: m }));
    }

    const byLevel = new Map<UrgencyBucket, MaterialPoolMaterial[]>();
    for (const m of filtered) {
      const lvl = normalizeUrgencyLevel(m.urgent_level);
      const list = byLevel.get(lvl);
      if (list) list.push(m);
      else byLevel.set(lvl, [m]);
    }

    const out: PoolRow[] = [];
    for (const lvl of URGENCY_ORDER) {
      const list = byLevel.get(lvl);
      if (!list || list.length === 0) continue;
      const weight = list.reduce((sum, m) => sum + Number(m.weight_t || 0), 0);
      const collapsed = !!collapsedUrgency[lvl];
      out.push({ type: 'header', level: lvl, count: list.length, weight, collapsed });
      if (!collapsed) {
        list.forEach((m) => out.push({ type: 'material', material: m }));
      }
    }
    return out;
  }, [collapsedUrgency, filtered, groupByUrgency]);

  const selectedSet = useMemo(() => new Set(selectedMaterialIds), [selectedMaterialIds]);

  const toggleSelection = (materialId: string, checked: boolean) => {
    const next = new Set(selectedSet);
    if (checked) next.add(materialId);
    else next.delete(materialId);
    onSelectedMaterialIdsChange(Array.from(next));
  };

  const selectAllVisible = () => {
    const next = new Set(selectedSet);
    filtered.forEach((m) => next.add(m.material_id));
    onSelectedMaterialIdsChange(Array.from(next));
  };

  const clearSelection = () => onSelectedMaterialIdsChange([]);

  type RowData = {
    rows: PoolRow[];
    selected: Set<string>;
    onToggle: (id: string, checked: boolean) => void;
    onInspect?: (material: MaterialPoolMaterial) => void;
    onToggleUrgency: (level: UrgencyBucket) => void;
  };

  const Row = ({ index, style, rows, selected, onToggle, onInspect, onToggleUrgency }: RowComponentProps<RowData>) => {
    const row = rows[index];

    if (row.type === 'header') {
      return (
        <div
          style={{
            ...style,
            display: 'flex',
            alignItems: 'center',
            padding: '0 10px',
            borderBottom: '1px solid rgba(0,0,0,0.06)',
            background: 'rgba(0,0,0,0.02)',
            cursor: 'pointer',
            gap: 8,
          }}
          onClick={() => onToggleUrgency(row.level)}
        >
          <span style={{ width: 16, display: 'flex', justifyContent: 'center' }}>
            {row.collapsed ? <RightOutlined /> : <DownOutlined />}
          </span>
          <UrgencyTag level={row.level} />
          <Text style={{ fontWeight: 600 }}>{row.level}</Text>
          <Text type="secondary">({row.count})</Text>
          <Text type="secondary" style={{ marginLeft: 'auto', fontFamily: FONT_FAMILIES.MONOSPACE }}>
            {row.weight.toFixed(2)}t
          </Text>
        </div>
      );
    }

    const m = row.material;
    const checked = selected.has(m.material_id);

    return (
      <div
        style={{
          ...style,
          display: 'flex',
          alignItems: 'center',
          padding: '0 10px',
          borderBottom: '1px solid rgba(0,0,0,0.06)',
          cursor: 'pointer',
          gap: 8,
        }}
        onClick={() => onInspect?.(m)}
      >
        <Checkbox
          checked={checked}
          onClick={(e) => e.stopPropagation()}
          onChange={(e) => onToggle(m.material_id, e.target.checked)}
        />

        <div style={{ flex: 1, minWidth: 0 }}>
          <Space size={8} style={{ width: '100%', justifyContent: 'space-between' }}>
            <Text style={{ fontFamily: FONT_FAMILIES.MONOSPACE }} ellipsis>
              {m.material_id}
            </Text>
            <UrgencyTag level={m.urgent_level} />
          </Space>

          <Space size={8} style={{ width: '100%', justifyContent: 'space-between' }}>
            <Text type="secondary" style={{ fontSize: 12 }} ellipsis>
              {m.steel_mark || '-'} · {Number(m.weight_t || 0).toFixed(2)}t
            </Text>
            <MaterialStatusIcons
              lockFlag={!!m.lock_flag}
              schedState={String(m.sched_state || '')}
              tempIssue={!!m.temp_issue || m.is_mature === false}
            />
          </Space>
        </div>
      </div>
    );
  };

  const toggleUrgencyCollapse = (level: UrgencyBucket) => {
    setCollapsedUrgency((prev) => ({ ...prev, [level]: !prev[level] }));
  };

  return (
    <Card
      size="small"
      title="物料池"
      style={{ height: '100%' }}
      bodyStyle={{ height: '100%', display: 'flex', flexDirection: 'column', gap: 10 }}
      extra={
        <Space>
          <Button size="small" onClick={selectAllVisible} disabled={filtered.length === 0}>
            全选
          </Button>
          <Button size="small" onClick={clearSelection} disabled={selectedMaterialIds.length === 0}>
            清空
          </Button>
        </Space>
      }
    >
      {error ? (
        <Alert
          type="error"
          showIcon
          message="物料池加载失败"
          description={String((error as any)?.message || error)}
          action={
            onRetry ? (
              <Button size="small" onClick={onRetry}>
                重试
              </Button>
            ) : undefined
          }
        />
      ) : null}

      <Input.Search
        placeholder="搜索材料号"
        allowClear
        value={searchText}
        onChange={(e) => setSearchText(e.target.value)}
        onSearch={(v) => setSearchText(v)}
        disabled={loading}
      />

      {filters ? (
        <Space wrap size={8} style={{ width: '100%' }}>
          <Select
            size="small"
            style={{ width: 120 }}
            value={filters.urgencyLevel ?? 'ALL'}
            onChange={(value) => onFiltersChange?.({ urgencyLevel: value === 'ALL' ? null : value })}
            options={[
              { value: 'ALL', label: '全部紧急度' },
              { value: 'L3', label: 'L3' },
              { value: 'L2', label: 'L2' },
              { value: 'L1', label: 'L1' },
              { value: 'L0', label: 'L0' },
            ]}
          />
          <Select
            size="small"
            style={{ width: 120 }}
            value={filters.lockStatus}
            onChange={(value) => onFiltersChange?.({ lockStatus: value as WorkbenchLockStatusFilter })}
            options={[
              { value: 'ALL', label: '全部锁定' },
              { value: 'LOCKED', label: '已锁定' },
              { value: 'UNLOCKED', label: '未锁定' },
            ]}
          />
          <Button
            size="small"
            onClick={() => onFiltersChange?.({ urgencyLevel: null, lockStatus: 'ALL' })}
          >
            重置筛选
          </Button>
          <Checkbox
            checked={groupByUrgency}
            onChange={(e) => setGroupByUrgency(e.target.checked)}
            style={{ fontSize: 12 }}
          >
            按紧急度分组
          </Checkbox>
        </Space>
      ) : null}

      <div style={{ flex: '0 0 220px', overflow: 'auto' }}>
        <Tree
          showLine={{ showLeafIcon: false }}
          treeData={treeData}
          selectedKeys={selectedTreeKey ? [selectedTreeKey] : []}
          onSelect={(keys) => {
            const first = keys[0];
            if (!first) return;
            const next = parseTreeKey(String(first));
            onSelectionChange(next);
          }}
        />
      </div>

      <div style={{ flex: 1, minHeight: 260 }}>
        <Space style={{ width: '100%', justifyContent: 'space-between' }}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            显示 {filtered.length} 条
          </Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            已选 {selectedMaterialIds.length} 条
          </Text>
        </Space>

        <div style={{ height: '100%', marginTop: 8 }}>
          {loading ? (
            <Skeleton active paragraph={{ rows: 8 }} />
          ) : filtered.length === 0 ? (
            <div style={{ padding: 24 }}>
              <Empty
                description={
                  materials.length === 0
                    ? '暂无物料数据（请先在“数据导入”导入）'
                    : '当前筛选条件下暂无物料'
                }
              />
            </div>
          ) : (
            <AutoSizer>
              {({ height, width }) => (
                <List
                  rowCount={rows.length}
                  rowHeight={ROW_HEIGHT}
                  rowComponent={Row}
                  rowProps={{
                    rows,
                    selected: selectedSet,
                    onToggle: toggleSelection,
                    onInspect: onInspectMaterial,
                    onToggleUrgency: toggleUrgencyCollapse,
                  }}
                  style={{ height, width }}
                >
                </List>
              )}
            </AutoSizer>
          )}
        </div>
      </div>
    </Card>
  );
};

export default React.memo(MaterialPool);

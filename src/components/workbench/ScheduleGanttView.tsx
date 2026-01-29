import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Alert, Button, DatePicker, Modal, Space, Tag, Typography, message } from 'antd';
import type { Dayjs } from 'dayjs';
import dayjs from 'dayjs';
import { useQuery } from '@tanstack/react-query';
import AutoSizer from 'react-virtualized-auto-sizer';
import { List, type RowComponentProps, useListCallbackRef } from 'react-window';
import { capacityApi } from '../../api/tauri';
import { useActiveVersionId } from '../../stores/use-global-store';
import { FONT_FAMILIES } from '../../theme';
import { URGENCY_COLORS } from '../../theme/tokens';
import { formatCapacity, formatPercent, formatWeight } from '../../utils/formatters';

const { Text } = Typography;
const { RangePicker } = DatePicker;

type PlanItemRow = {
  material_id: string;
  machine_code: string;
  plan_date: string;
  seq_no: number;
  weight_t: number;
  urgent_level?: string;
  locked_in_plan?: boolean;
  force_release_in_plan?: boolean;
};

export interface ScheduleGanttViewProps {
  machineCode?: string | null;
  urgentLevel?: string | null;
  planItems?: unknown;
  loading?: boolean;
  error?: unknown;
  onRetry?: () => void;
  selectedMaterialIds: string[];
  onSelectedMaterialIdsChange: (ids: string[]) => void;
  onInspectMaterialId?: (materialId: string) => void;
  onRequestMoveToCell?: (machine: string, date: string) => void;
}

const LEFT_COL_WIDTH = 168;
const HEADER_HEIGHT = 44;
const ROW_HEIGHT = 72;
const COLUMN_WIDTH = 112;
const CELL_PADDING_X = 6;
const BAR_HEIGHT = 18;
const BAR_GAP = 4;
const MAX_ITEMS_PER_CELL = 2;
const MAX_DAYS = 60;

function normalizeDateKey(value: string): string {
  const trimmed = String(value || '').trim();
  if (!trimmed) return '';
  if (/^\d{4}-\d{2}-\d{2}/.test(trimmed)) return trimmed.slice(0, 10);
  return trimmed;
}

function urgencyToColor(level: string | undefined): { background: string; text: string } {
  const urgent = String(level || 'L0');
  if (urgent === 'L3') return { background: URGENCY_COLORS.L3_EMERGENCY, text: '#fff' };
  if (urgent === 'L2') return { background: URGENCY_COLORS.L2_HIGH, text: 'rgba(0, 0, 0, 0.85)' };
  if (urgent === 'L1') return { background: URGENCY_COLORS.L1_MEDIUM, text: '#fff' };
  return { background: URGENCY_COLORS.L0_NORMAL, text: '#fff' };
}

function computeSuggestedRange(dateKeys: string[]): [Dayjs, Dayjs] {
  const today = dayjs().startOf('day');
  if (dateKeys.length === 0) {
    return [today, today.add(13, 'day')];
  }
  const sorted = [...dateKeys].sort();
  const min = dayjs(sorted[0]).startOf('day');
  const max = dayjs(sorted[sorted.length - 1]).startOf('day');
  if (!min.isValid() || !max.isValid()) {
    return [today, today.add(13, 'day')];
  }
  const totalDays = max.diff(min, 'day') + 1;
  if (totalDays <= 14) {
    return [min, max];
  }

  let start = today.subtract(3, 'day');
  let end = start.add(13, 'day');
  if (end.isBefore(min)) {
    start = min;
    end = min.add(13, 'day');
  }
  if (start.isAfter(max)) {
    end = max;
    start = max.subtract(13, 'day');
  }
  if (start.isBefore(min)) start = min;
  if (end.isAfter(max)) end = max;
  if (end.isBefore(start)) {
    start = min;
    const candidate = min.add(13, 'day');
    end = candidate.isAfter(max) ? max : candidate;
  }
  return [start, end];
}

export default function ScheduleGanttView({
  machineCode,
  urgentLevel,
  planItems,
  loading,
  error,
  onRetry,
  selectedMaterialIds,
  onSelectedMaterialIdsChange,
  onInspectMaterialId,
  onRequestMoveToCell,
}: ScheduleGanttViewProps) {
  const activeVersionId = useActiveVersionId();
  const normalized = useMemo<PlanItemRow[]>(() => {
    const raw = Array.isArray(planItems) ? planItems : [];
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
  }, [planItems]);

  const availableDateKeys = useMemo(() => {
    const set = new Set<string>();
    normalized.forEach((it) => {
      const machine = String(it.machine_code || '').trim();
      if (!machine) return;
      if (machineCode && machineCode !== 'all' && machine !== machineCode) return;
      if (urgentLevel && urgentLevel !== 'all') {
        const want = String(urgentLevel).toUpperCase();
        if (String(it.urgent_level || 'L0').toUpperCase() !== want) return;
      }
      const key = normalizeDateKey(it.plan_date);
      if (!key) return;
      set.add(key);
    });
    return Array.from(set);
  }, [machineCode, normalized, urgentLevel]);

  const suggestedRange = useMemo(() => computeSuggestedRange(availableDateKeys), [availableDateKeys]);
  const didUserAdjustRangeRef = useRef(false);
  const lastMachineRef = useRef<string | null | undefined>(undefined);

  const [range, setRange] = useState<[Dayjs, Dayjs]>(() => suggestedRange);
  const [cellDetail, setCellDetail] = useState<{ machine: string; date: string } | null>(null);

  useEffect(() => {
    const machineKey = machineCode ?? null;
    if (lastMachineRef.current !== machineKey) {
      lastMachineRef.current = machineKey;
      didUserAdjustRangeRef.current = false;
    }
    if (!didUserAdjustRangeRef.current) {
      setRange(suggestedRange);
    }
  }, [machineCode, suggestedRange]);

  const dateKeys = useMemo(() => {
    const [start, end] = range;
    const startDay = start.startOf('day');
    const endDay = end.startOf('day');
    if (!startDay.isValid() || !endDay.isValid()) return [];
    const days = endDay.diff(startDay, 'day') + 1;
    const limited = Math.max(0, Math.min(days, MAX_DAYS));
    return Array.from({ length: limited }, (_, idx) => startDay.add(idx, 'day').format('YYYY-MM-DD'));
  }, [range]);

  const dateIndexByKey = useMemo(() => {
    const map = new Map<string, number>();
    dateKeys.forEach((k, idx) => map.set(k, idx));
    return map;
  }, [dateKeys]);

  const todayKey = useMemo(() => dayjs().format('YYYY-MM-DD'), []);
  const todayIndex = dateIndexByKey.get(todayKey) ?? -1;

  const { machines, itemsByMachineDate, filteredCount, filteredTotalWeight } = useMemo(() => {
    const byMachine = new Map<string, Map<string, PlanItemRow[]>>();
    const machineSet = new Set<string>();
    if (dateKeys.length === 0) {
      return { machines: [] as string[], itemsByMachineDate: byMachine, filteredCount: 0, filteredTotalWeight: 0 };
    }

    if (machineCode && machineCode !== 'all') {
      machineSet.add(machineCode);
    }

    const startKey = dateKeys[0];
    const endKey = dateKeys[dateKeys.length - 1];
    let count = 0;
    let totalWeight = 0;

    normalized.forEach((it) => {
      const machine = String(it.machine_code || '').trim();
      if (!machine) return;
      if (machineCode && machineCode !== 'all' && machine !== machineCode) return;
      if (urgentLevel && urgentLevel !== 'all') {
        const want = String(urgentLevel).toUpperCase();
        if (String(it.urgent_level || 'L0').toUpperCase() !== want) return;
      }
      const dateKey = normalizeDateKey(it.plan_date);
      if (!dateKey) return;
      if (dateKey < startKey || dateKey > endKey) return;

      machineSet.add(machine);
      let byDate = byMachine.get(machine);
      if (!byDate) {
        byDate = new Map();
        byMachine.set(machine, byDate);
      }
      const list = byDate.get(dateKey);
      if (list) list.push(it);
      else byDate.set(dateKey, [it]);
      count += 1;
      totalWeight += Number(it.weight_t || 0);
    });

    byMachine.forEach((byDate) => {
      byDate.forEach((list) => {
        list.sort((a, b) => a.seq_no - b.seq_no);
      });
    });

    return {
      machines: Array.from(machineSet).sort(),
      itemsByMachineDate: byMachine,
      filteredCount: count,
      filteredTotalWeight: totalWeight,
    };
  }, [dateKeys, machineCode, normalized, urgentLevel]);

  const selectedSet = useMemo(() => new Set(selectedMaterialIds), [selectedMaterialIds]);

  const toggleSelection = useCallback(
    (materialId: string, checked: boolean) => {
      const next = new Set(selectedSet);
      if (checked) next.add(materialId);
      else next.delete(materialId);
      onSelectedMaterialIdsChange(Array.from(next));
    },
    [onSelectedMaterialIdsChange, selectedSet]
  );

  const timelineWidth = useMemo(() => dateKeys.length * COLUMN_WIDTH, [dateKeys.length]);

  const headerInnerRef = useRef<HTMLDivElement | null>(null);
  const rafRef = useRef<number | null>(null);
  const [listApi, listRef] = useListCallbackRef(null);

  const syncHeader = useCallback((scrollLeft: number) => {
    if (!headerInnerRef.current) return;
    headerInnerRef.current.style.transform = `translateX(${-scrollLeft}px)`;
  }, []);

  const handleListScroll = useCallback(
    (e: React.UIEvent<HTMLDivElement>) => {
      const left = e.currentTarget.scrollLeft;
      if (rafRef.current != null) cancelAnimationFrame(rafRef.current);
      rafRef.current = requestAnimationFrame(() => syncHeader(left));
    },
    [syncHeader]
  );

  useEffect(() => {
    return () => {
      if (rafRef.current != null) cancelAnimationFrame(rafRef.current);
    };
  }, []);

  useEffect(() => {
    const el = listApi?.element;
    if (!el) return;
    syncHeader(el.scrollLeft);
  }, [dateKeys.length, listApi, syncHeader]);

  const onRangeChange = useCallback(
    (values: null | [Dayjs | null, Dayjs | null]) => {
      if (!values || !values[0] || !values[1]) {
        didUserAdjustRangeRef.current = false;
        setRange(suggestedRange);
        return;
      }
      let start = values[0].startOf('day');
      let end = values[1].startOf('day');
      if (end.isBefore(start)) {
        const tmp = start;
        start = end;
        end = tmp;
      }
      const days = end.diff(start, 'day') + 1;
      if (days > MAX_DAYS) {
        message.warning(`时间跨度过大，已限制为${MAX_DAYS}天`);
        end = start.add(MAX_DAYS - 1, 'day');
      }
      didUserAdjustRangeRef.current = true;
      setRange([start, end]);
    },
    [suggestedRange]
  );

  const resetRange = useCallback(() => {
    didUserAdjustRangeRef.current = false;
    setRange(suggestedRange);
  }, [suggestedRange]);

  const scrollToToday = useCallback(() => {
    const el = listApi?.element;
    if (!el) return;
    const idx = dateIndexByKey.get(todayKey);
    if (idx == null) {
      message.info('当前时间范围不包含今天');
      return;
    }
    const targetLeft = Math.max(0, idx * COLUMN_WIDTH - COLUMN_WIDTH);
    el.scrollTo({ left: targetLeft, behavior: 'smooth' });
  }, [dateIndexByKey, listApi, todayKey]);

  const capacityMachineCodes = useMemo(() => {
    if (machineCode && machineCode !== 'all') return [machineCode];
    return machines;
  }, [machineCode, machines]);

  const capacityQuery = useQuery({
    queryKey: [
      'ganttCapacityPools',
      activeVersionId,
      capacityMachineCodes.join(','),
      dateKeys[0] || '',
      dateKeys[dateKeys.length - 1] || '',
    ],
    enabled: !!activeVersionId && capacityMachineCodes.length > 0 && dateKeys.length > 0,
    queryFn: async () => {
      if (!activeVersionId) return [];
      const dateFrom = dateKeys[0];
      const dateTo = dateKeys[dateKeys.length - 1];
      const res = await capacityApi.getCapacityPools(capacityMachineCodes, dateFrom, dateTo, activeVersionId);
      return Array.isArray(res) ? res : [];
    },
    staleTime: 30 * 1000,
  });

  const capacityByMachineDate = useMemo(() => {
    const map = new Map<string, { target: number; limit: number; used: number }>();
    const raw = Array.isArray(capacityQuery.data) ? capacityQuery.data : [];
    raw.forEach((row: any) => {
      const machine = String(row?.machine_code ?? '').trim();
      const date = normalizeDateKey(String(row?.plan_date ?? ''));
      if (!machine || !date) return;
      const target = Number(row?.target_capacity_t ?? 0);
      const limit = Number(row?.limit_capacity_t ?? 0);
      const used = Number(row?.used_capacity_t ?? 0);

      map.set(`${machine}__${date}`, {
        target: Number.isFinite(target) ? target : 0,
        limit: Number.isFinite(limit) ? limit : 0,
        used: Number.isFinite(used) ? used : 0,
      });
    });
    return map;
  }, [capacityQuery.data]);

  type RowData = {
    machines: string[];
    itemsByMachineDate: Map<string, Map<string, PlanItemRow[]>>;
    dateKeys: string[];
    dateIndexByKey: Map<string, number>;
    capacityByMachineDate: Map<string, { target: number; limit: number; used: number }>;
    selected: Set<string>;
    onToggle: (id: string, checked: boolean) => void;
    onInspect?: (id: string) => void;
    onOpenCell?: (machine: string, date: string) => void;
    timelineWidth: number;
    todayIndex: number;
  };

  const Row = ({
    index,
    style,
    machines,
    itemsByMachineDate,
    dateKeys,
    dateIndexByKey,
    capacityByMachineDate,
    selected,
    onToggle,
    onInspect,
    onOpenCell,
    timelineWidth,
    todayIndex,
  }: RowComponentProps<RowData>) => {
    const machine = machines[index] || '';
    const byDate = itemsByMachineDate.get(machine) ?? null;
    let total = 0;
    let totalWeight = 0;
    if (byDate) {
      byDate.forEach((list) => {
        total += list.length;
        list.forEach((it) => {
          totalWeight += Number(it.weight_t || 0);
        });
      });
    }

    const bars: React.ReactNode[] = [];
    if (byDate) {
      const entries = Array.from(byDate.entries()).sort((a, b) => a[0].localeCompare(b[0]));
      entries.forEach(([dateKey, list]) => {
        const colIndex = dateIndexByKey.get(dateKey);
        if (colIndex == null) return;

        const visible = list.slice(0, MAX_ITEMS_PER_CELL);
        const overflow = Math.max(0, list.length - visible.length);
        const left = colIndex * COLUMN_WIDTH + CELL_PADDING_X;
        const width = COLUMN_WIDTH - CELL_PADDING_X * 2;

        visible.forEach((it, lane) => {
          const urgent = String(it.urgent_level || 'L0');
          const { background, text } = urgencyToColor(urgent);
          const checked = selected.has(it.material_id);
          const top = 8 + lane * (BAR_HEIGHT + BAR_GAP);
          const title = `${it.material_id} · ${it.machine_code} · ${normalizeDateKey(it.plan_date)} · 序${it.seq_no} · ${formatWeight(
            it.weight_t
          )}${it.locked_in_plan ? ' · 冻结' : ''}${it.force_release_in_plan ? ' · 强制放行' : ''}\n点击查看详情，Ctrl/⌘+点击切换选中`;

          bars.push(
            <div
              key={`${it.material_id}__${dateKey}__${lane}`}
              title={title}
              onClick={(e) => {
                e.stopPropagation();
                if (e.ctrlKey || e.metaKey) {
                  onToggle(it.material_id, !checked);
                  return;
                }
                onInspect?.(it.material_id);
              }}
              onDoubleClick={(e) => e.stopPropagation()}
              style={{
                position: 'absolute',
                left,
                top,
                width,
                height: BAR_HEIGHT,
                display: 'flex',
                alignItems: 'center',
                gap: 6,
                padding: '0 6px',
                borderRadius: 4,
                background,
                color: text,
                cursor: 'pointer',
                boxShadow: [
                  checked ? '0 0 0 2px rgba(22, 119, 255, 0.55)' : '0 0 0 1px rgba(0, 0, 0, 0.08)',
                  it.locked_in_plan ? 'inset 3px 0 0 rgba(114, 46, 209, 0.95)' : '',
                ]
                  .filter(Boolean)
                  .join(', '),
              }}
            >
              <input
                type="checkbox"
                aria-label={`选择 ${it.material_id}`}
                checked={checked}
                onClick={(ev) => ev.stopPropagation()}
                onDoubleClick={(ev) => ev.stopPropagation()}
                onChange={(ev) => onToggle(it.material_id, ev.target.checked)}
                style={{ margin: 0 }}
              />
              <span
                style={{
                  fontFamily: FONT_FAMILIES.MONOSPACE,
                  fontSize: 12,
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              >
                {it.material_id}
              </span>
              {it.force_release_in_plan && (
                <span
                  style={{
                    marginLeft: 'auto',
                    fontSize: 11,
                    opacity: 0.9,
                    padding: '0 4px',
                    borderRadius: 3,
                    background: 'rgba(255, 255, 255, 0.35)',
                  }}
                >
                  放
                </span>
              )}
            </div>
          );
        });

        if (overflow > 0) {
          const top = 8 + MAX_ITEMS_PER_CELL * (BAR_HEIGHT + BAR_GAP);
          bars.push(
            <div
              key={`${machine}__${dateKey}__more`}
              title={`${machine} · ${dateKey} 还有 ${overflow} 条未展示（点击查看）`}
              onClick={(e) => {
                e.stopPropagation();
                onOpenCell?.(machine, dateKey);
              }}
              onDoubleClick={(e) => e.stopPropagation()}
              style={{
                position: 'absolute',
                left,
                top,
                width,
                height: BAR_HEIGHT,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontSize: 12,
                color: '#8c8c8c',
                borderRadius: 4,
                border: '1px dashed #d9d9d9',
                background: '#fafafa',
                cursor: 'pointer',
              }}
            >
              +{overflow}
            </div>
          );
        }
      });
    }

    return (
      <div
        style={{
          ...style,
          minWidth: LEFT_COL_WIDTH + timelineWidth,
          display: 'flex',
          borderBottom: '1px solid #f0f0f0',
          background: '#fff',
        }}
      >
        <div
          style={{
            width: LEFT_COL_WIDTH,
            flex: `0 0 ${LEFT_COL_WIDTH}px`,
            position: 'sticky',
            left: 0,
            zIndex: 2,
            background: '#fff',
            borderRight: '1px solid #f0f0f0',
            padding: '6px 8px',
            display: 'flex',
            flexDirection: 'column',
            justifyContent: 'center',
            gap: 2,
          }}
        >
          <Text strong ellipsis title={machine} style={{ maxWidth: LEFT_COL_WIDTH - 16 }}>
            {machine}
          </Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {total} 条
          </Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {formatWeight(totalWeight)}
          </Text>
        </div>

        <div
          style={{
            position: 'relative',
            width: timelineWidth,
            height: ROW_HEIGHT,
            backgroundImage: `repeating-linear-gradient(to right, rgba(0, 0, 0, 0.06) 0, rgba(0, 0, 0, 0.06) 1px, transparent 1px, transparent ${COLUMN_WIDTH}px)`,
          }}
          onDoubleClick={(e) => {
            const rect = e.currentTarget.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const col = Math.floor(x / COLUMN_WIDTH);
            const dateKey = dateKeys[col];
            if (!dateKey) return;
            onOpenCell?.(machine, dateKey);
          }}
        >
          {dateKeys.map((key, colIndex) => {
            const cap = capacityByMachineDate.get(`${machine}__${key}`);
            if (!cap) return null;
            const { target, limit, used } = cap;
            const overLimit = limit > 0 && used > limit;
            const overTarget = !overLimit && target > 0 && used > target;
            if (!overLimit && !overTarget) return null;
            return (
              <div
                key={`cap__${machine}__${key}`}
                style={{
                  position: 'absolute',
                  left: colIndex * COLUMN_WIDTH,
                  top: 0,
                  bottom: 0,
                  width: COLUMN_WIDTH,
                  background: overLimit ? 'rgba(255, 77, 79, 0.10)' : 'rgba(250, 173, 20, 0.10)',
                  pointerEvents: 'none',
                }}
              />
            );
          })}
          {todayIndex >= 0 && (
            <div
              style={{
                position: 'absolute',
                left: todayIndex * COLUMN_WIDTH,
                top: 0,
                bottom: 0,
                width: COLUMN_WIDTH,
                background: 'rgba(22, 119, 255, 0.06)',
                pointerEvents: 'none',
              }}
            />
          )}
          {bars}
        </div>
      </div>
    );
  };

  const headerCells = useMemo(() => {
    return dateKeys.map((key) => {
      const d = dayjs(key);
      const isToday = key === todayKey;
      const isWeekend = d.day() === 0 || d.day() === 6;
      return (
        <div
          key={key}
          style={{
            width: COLUMN_WIDTH,
            flex: `0 0 ${COLUMN_WIDTH}px`,
            borderRight: '1px solid #f0f0f0',
            background: isToday ? '#e6f4ff' : isWeekend ? '#fafafa' : '#fff',
            padding: '6px 0',
            textAlign: 'center',
          }}
          title={key}
        >
          <div style={{ fontSize: 12, fontWeight: 600 }}>{d.format('MM-DD')}</div>
          <div style={{ fontSize: 11, color: '#8c8c8c' }}>{d.format('dd')}</div>
        </div>
      );
    });
  }, [dateKeys, todayKey]);

  const hasError = error != null;
  const showEmpty = !loading && !hasError && filteredCount === 0;

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

  const cellDetailItems = useMemo(() => {
    if (!cellDetail) return [];
    const byDate = itemsByMachineDate.get(cellDetail.machine);
    const list = byDate?.get(cellDetail.date) ?? [];
    return [...list].sort((a, b) => a.seq_no - b.seq_no);
  }, [cellDetail, itemsByMachineDate]);

  const cellCapacity = useMemo(() => {
    if (!cellDetail) return null;
    return capacityByMachineDate.get(`${cellDetail.machine}__${cellDetail.date}`) ?? null;
  }, [capacityByMachineDate, cellDetail]);

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Space wrap style={{ marginBottom: 8, justifyContent: 'space-between', width: '100%' }}>
        <Space wrap>
          <RangePicker
            size="small"
            value={range}
            onChange={(values) => onRangeChange(values as [Dayjs | null, Dayjs | null] | null)}
            allowClear
          />
          <Button size="small" onClick={resetRange}>
            重置范围
          </Button>
          <Button size="small" onClick={scrollToToday} disabled={dateKeys.length === 0}>
            定位今天
          </Button>
        </Space>
        {legend}
        <Text type="secondary" style={{ fontSize: 12 }}>
          机组 {machines.length} · 任务 {filteredCount} · 总重 {formatWeight(filteredTotalWeight)} · 范围 {dateKeys[0] || '-'} ~{' '}
          {dateKeys[dateKeys.length - 1] || '-'}
        </Text>
      </Space>

      {hasError && (
        <Alert
          type="error"
          showIcon
          message="甘特图数据加载失败"
          description={String((error as any)?.message || error)}
          action={
            onRetry ? (
              <a onClick={onRetry} style={{ padding: '0 8px' }}>
                重试
              </a>
            ) : null
          }
          style={{ marginBottom: 8 }}
        />
      )}

      {showEmpty && (
        <Alert
          type="info"
          showIcon
          message="当前范围内暂无排程数据"
          description="可调整时间范围，或切换机组查看"
          style={{ marginBottom: 8 }}
        />
      )}

      <div
        style={{
          flex: 1,
          minHeight: 0,
          border: '1px solid #f0f0f0',
          borderRadius: 6,
          overflow: 'hidden',
          background: '#fff',
        }}
      >
        <div
          style={{
            height: HEADER_HEIGHT,
            display: 'flex',
            borderBottom: '1px solid #f0f0f0',
            background: '#fafafa',
          }}
        >
          <div
            style={{
              width: LEFT_COL_WIDTH,
              flex: `0 0 ${LEFT_COL_WIDTH}px`,
              borderRight: '1px solid #f0f0f0',
              padding: '0 8px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              position: 'sticky',
              left: 0,
              zIndex: 3,
              background: '#fafafa',
            }}
          >
            <Text strong>机组</Text>
            <Text type="secondary" style={{ fontSize: 12 }}>
              日期
            </Text>
          </div>
          <div style={{ flex: 1, minWidth: 0, overflow: 'hidden' }}>
            <div
              ref={headerInnerRef}
              style={{
                width: timelineWidth,
                display: 'flex',
                willChange: 'transform',
              }}
            >
              {headerCells}
            </div>
          </div>
        </div>

        <div style={{ height: `calc(100% - ${HEADER_HEIGHT}px)`, minHeight: 0 }}>
          <AutoSizer>
            {({ height, width }) => (
              <List
                listRef={listRef}
                rowCount={machines.length}
                rowHeight={ROW_HEIGHT}
                rowComponent={Row}
                rowProps={{
                  machines,
                  itemsByMachineDate,
                  dateKeys,
                  dateIndexByKey,
                  capacityByMachineDate,
                  selected: selectedSet,
                  onToggle: toggleSelection,
                  onInspect: onInspectMaterialId,
                  onOpenCell: (machine, date) => setCellDetail({ machine, date }),
                  timelineWidth,
                  todayIndex,
                }}
                onScroll={handleListScroll}
                style={{
                  height,
                  width,
                  overflowX: 'auto',
                  overflowY: 'auto',
                }}
              />
            )}
          </AutoSizer>
        </div>
      </div>

      <Modal
        title={cellDetail ? `同日明细：${cellDetail.machine} / ${cellDetail.date}` : '同日明细'}
        open={!!cellDetail}
        onCancel={() => setCellDetail(null)}
        footer={
          cellDetail
            ? [
                onRequestMoveToCell && selectedMaterialIds.length > 0 ? (
                  <Button
                    key="move"
                    type="primary"
                    onClick={() => onRequestMoveToCell(cellDetail.machine, cellDetail.date)}
                  >
                    移动已选({selectedMaterialIds.length})到这里
                  </Button>
                ) : null,
                <Button key="close" onClick={() => setCellDetail(null)}>
                  关闭
                </Button>,
              ].filter(Boolean)
            : null
        }
        width={900}
        destroyOnClose
      >
        {cellCapacity ? (
          <Alert
            type={
              cellCapacity.limit > 0 && cellCapacity.used > cellCapacity.limit
                ? 'error'
                : cellCapacity.target > 0 && cellCapacity.used > cellCapacity.target
                ? 'warning'
                : 'info'
            }
            showIcon
            message={`产能 ${formatCapacity(cellCapacity.used)} / 目标 ${formatCapacity(cellCapacity.target)} / 上限 ${formatCapacity(
              cellCapacity.limit
            )}（利用率 ${formatPercent(
              (cellCapacity.used / Math.max(cellCapacity.limit || cellCapacity.target || 0, 1)) * 100
            )}）`}
            style={{ marginBottom: 12 }}
          />
        ) : (
          <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 12 }}>
            暂无产能池数据
          </Text>
        )}
        {cellDetailItems.length === 0 ? (
          <Alert type="info" showIcon message="该单元格暂无数据" />
        ) : (
          <div style={{ maxHeight: 560, overflow: 'auto' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse' }}>
              <thead>
                <tr style={{ textAlign: 'left', borderBottom: '1px solid #f0f0f0' }}>
                  <th style={{ width: 60, padding: '8px 6px' }}>选择</th>
                  <th style={{ width: 180, padding: '8px 6px' }}>物料</th>
                  <th style={{ width: 80, padding: '8px 6px' }}>序号</th>
                  <th style={{ width: 90, padding: '8px 6px' }}>紧急</th>
                  <th style={{ width: 120, padding: '8px 6px' }}>重量</th>
                  <th style={{ width: 90, padding: '8px 6px' }}>冻结</th>
                  <th style={{ width: 120, padding: '8px 6px' }}>操作</th>
                </tr>
              </thead>
              <tbody>
                {cellDetailItems.map((it) => {
                  const checked = selectedSet.has(it.material_id);
                  const urgent = String(it.urgent_level || 'L0');
                  const urgentColor =
                    urgent === 'L3'
                      ? 'red'
                      : urgent === 'L2'
                      ? 'orange'
                      : urgent === 'L1'
                      ? 'blue'
                      : 'default';

                  return (
                    <tr key={it.material_id} style={{ borderBottom: '1px solid #f5f5f5' }}>
                      <td style={{ padding: '8px 6px' }}>
                        <input
                          type="checkbox"
                          checked={checked}
                          onChange={(e) => toggleSelection(it.material_id, e.target.checked)}
                        />
                      </td>
                      <td style={{ padding: '8px 6px', fontFamily: FONT_FAMILIES.MONOSPACE }}>
                        {it.material_id}
                      </td>
                      <td style={{ padding: '8px 6px' }}>{it.seq_no}</td>
                      <td style={{ padding: '8px 6px' }}>
                        <Tag color={urgentColor}>{urgent}</Tag>
                      </td>
                      <td style={{ padding: '8px 6px' }}>{formatWeight(it.weight_t)}</td>
                      <td style={{ padding: '8px 6px' }}>{it.locked_in_plan ? '是' : '否'}</td>
                      <td style={{ padding: '8px 6px' }}>
                        <Button size="small" onClick={() => onInspectMaterialId?.(it.material_id)}>
                          查看
                        </Button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </Modal>
    </div>
  );
}

/**
 * 排程甘特图组件
 *
 * 重构后：935 行 → ~180 行 (-81%)
 */

import React, { useCallback, useEffect, useMemo, useRef } from 'react';
import { Alert, Tooltip, Typography } from 'antd';
import dayjs from 'dayjs';
import { useQuery } from '@tanstack/react-query';
import AutoSizer from 'react-virtualized-auto-sizer';
import { List, useListCallbackRef } from 'react-window';
import { capacityApi } from '../../api/tauri';
import { useActiveVersionId } from '../../stores/use-global-store';
import type { ScheduleGanttViewProps, CapacityData, PlanItemRow } from './types';
import { LEFT_COL_WIDTH, HEADER_HEIGHT, ROW_HEIGHT, COLUMN_WIDTH } from './types';
import { normalizeDateKey } from './utils';
import { useGanttData } from './useGanttData';
import { GanttRow } from './GanttRow';
import { GanttToolbar } from './GanttToolbar';
import { CellDetailModal } from './CellDetailModal';
import { URGENCY_COLORS } from '../../theme/tokens';
import {
  PLAN_ITEM_STATUS_FILTER_META,
  matchPlanItemStatusFilter,
  type PlanItemStatusFilter,
} from '../../utils/planItemStatus';

const { Text } = Typography;

export default function ScheduleGanttView({
  machineCode,
  urgentLevel,
  dateRange,
  suggestedDateRange,
  onDateRangeChange,
  focusedDate,
  autoOpenCell,
  statusFilter,
  onStatusFilterChange,
  onFocusChange,
  onNavigateToMatrix,
  focus,
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

  const ganttData = useGanttData({
    machineCode,
    urgentLevel,
    statusFilter,
    planItems,
    externalRange: dateRange ?? null,
    onExternalRangeChange: onDateRangeChange,
    externalSuggestedRange: suggestedDateRange,
    selectedMaterialIds,
    onSelectedMaterialIdsChange,
  });

  const {
    normalized,
    dateKeys,
    dateIndexByKey,
    todayKey,
    todayIndex,
    machines,
    itemsByMachineDate: itemsByMachineDateFiltered,
    filteredCount,
    filteredTotalWeight,
    statusSummary,
    selectedSet,
    toggleSelection,
    range,
    onRangeChange,
    resetRange,
    cellDetail,
    setCellDetail,
  } = ganttData;

  type HeaderDateSummary = {
    totalCount: number;
    totalWeightT: number;
    lockedCount: number;
    lockedWeightT: number;
    forceReleaseCount: number;
    forceReleaseWeightT: number;
    adjustableCount: number;
    adjustableWeightT: number;
    byUrgency: Record<'L3' | 'L2' | 'L1' | 'L0', { count: number; weightT: number }>;
    viewCount: number;
    viewWeightT: number;
  };

  const focusDateKey = useMemo(() => normalizeDateKey(String(focus?.date || '')), [focus?.date]);

  const headerDateSummaryByKey = useMemo(() => {
    const map = new Map<string, HeaderDateSummary>();
    if (dateKeys.length === 0) return map;
    const dateSet = new Set(dateKeys);
    const machineWant = machineCode && machineCode !== 'all' ? String(machineCode).trim() : '';
    const urgentWant =
      urgentLevel && urgentLevel !== 'all' ? String(urgentLevel).toUpperCase() : '';
    const viewStatus = (statusFilter || 'ALL') as PlanItemStatusFilter;

    const createEmpty = (): HeaderDateSummary => ({
      totalCount: 0,
      totalWeightT: 0,
      lockedCount: 0,
      lockedWeightT: 0,
      forceReleaseCount: 0,
      forceReleaseWeightT: 0,
      adjustableCount: 0,
      adjustableWeightT: 0,
      byUrgency: {
        L3: { count: 0, weightT: 0 },
        L2: { count: 0, weightT: 0 },
        L1: { count: 0, weightT: 0 },
        L0: { count: 0, weightT: 0 },
      },
      viewCount: 0,
      viewWeightT: 0,
    });

    normalized.forEach((it) => {
      const dateKey = normalizeDateKey(it.plan_date);
      if (!dateSet.has(dateKey)) return;
      const machine = String(it.machine_code || '').trim();
      if (machineWant && machine !== machineWant) return;

      let sum = map.get(dateKey);
      if (!sum) {
        sum = createEmpty();
        map.set(dateKey, sum);
      }

      const weight = Number(it.weight_t || 0);
      sum.totalCount += 1;
      sum.totalWeightT += weight;

      if (it.locked_in_plan) {
        sum.lockedCount += 1;
        sum.lockedWeightT += weight;
      } else {
        sum.adjustableCount += 1;
        sum.adjustableWeightT += weight;
      }

      if (it.force_release_in_plan) {
        sum.forceReleaseCount += 1;
        sum.forceReleaseWeightT += weight;
      }

      const u = String(it.urgent_level || 'L0').toUpperCase();
      const uKey = u === 'L3' || u === 'L2' || u === 'L1' || u === 'L0' ? u : 'L0';
      sum.byUrgency[uKey as 'L3' | 'L2' | 'L1' | 'L0'].count += 1;
      sum.byUrgency[uKey as 'L3' | 'L2' | 'L1' | 'L0'].weightT += weight;

      const viewUrgentOk = !urgentWant || uKey === urgentWant;
      const viewStatusOk = matchPlanItemStatusFilter(it, viewStatus);
      if (viewUrgentOk && viewStatusOk) {
        sum.viewCount += 1;
        sum.viewWeightT += weight;
      }
    });

    return map;
  }, [dateKeys, machineCode, normalized, statusFilter, urgentLevel]);

  // 同日明细使用“全量排程”数据（不受紧急度筛选影响），避免从产能概览点击后看不到明细
  const itemsByMachineDateForModal = useMemo(() => {
    const byMachine = new Map<string, Map<string, PlanItemRow[]>>();
    if (dateKeys.length === 0) return byMachine;
    const startKey = dateKeys[0];
    const endKey = dateKeys[dateKeys.length - 1];

    normalized.forEach((it) => {
      const machine = String(it.machine_code || '').trim();
      if (!machine) return;
      if (machineCode && machineCode !== 'all' && machine !== machineCode) return;
      const dateKey = normalizeDateKey(it.plan_date);
      if (!dateKey) return;
      if (dateKey < startKey || dateKey > endKey) return;
      let byDate = byMachine.get(machine);
      if (!byDate) {
        byDate = new Map();
        byMachine.set(machine, byDate);
      }
      const list = byDate.get(dateKey);
      if (list) list.push(it);
      else byDate.set(dateKey, [it]);
    });

    byMachine.forEach((byDate) => {
      byDate.forEach((list) => {
        list.sort((a, b) => Number(a.seq_no || 0) - Number(b.seq_no || 0));
      });
    });

    return byMachine;
  }, [dateKeys, machineCode, normalized]);

  // 稳定化 onOpenCell 回调以支持 React.memo 优化
  const handleOpenCell = useCallback(
    (machine: string, date: string) => {
      setCellDetail({ machine, date });
      onFocusChange?.({ machine, date, source: 'ganttCell' });
    },
    [onFocusChange, setCellDetail]
  );

  const timelineWidth = useMemo(() => dateKeys.length * COLUMN_WIDTH, [dateKeys.length]);

  // 容量查询
  const capacityMachineCodes = useMemo(() => {
    if (machineCode && machineCode !== 'all') return [machineCode];
    return machines;
  }, [machineCode, machines]);

  // H9修复：对机组列表排序后再join，确保queryKey稳定（避免数组顺序变化导致缓存失效）
  const capacityQuery = useQuery({
    queryKey: [
      'ganttCapacityPools',
      activeVersionId,
      capacityMachineCodes.slice().sort().join(','),
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
    staleTime: 2 * 60 * 1000, // 2分钟缓存（产能数据变化频率较低）
  });

  const capacityByMachineDate = useMemo(() => {
    const map = new Map<string, CapacityData>();
    const raw = Array.isArray(capacityQuery.data) ? capacityQuery.data : [];
    raw.forEach((row: unknown) => {
      const r = (row && typeof row === 'object' ? row : {}) as Record<string, unknown>;
      const machine = String(r.machine_code ?? '').trim();
      const date = normalizeDateKey(String(r.plan_date ?? ''));
      if (!machine || !date) return;
      const target = Number(r.target_capacity_t ?? 0);
      const limit = Number(r.limit_capacity_t ?? 0);
      const used = Number(r.used_capacity_t ?? 0);

      map.set(`${machine}__${date}`, {
        target: Number.isFinite(target) ? target : 0,
        limit: Number.isFinite(limit) ? limit : 0,
        used: Number.isFinite(used) ? used : 0,
      });
    });
    return map;
  }, [capacityQuery.data]);

  // 滚动同步
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

  const scrollToDateKey = useCallback(
    (dateKey: string, behavior: ScrollBehavior = 'smooth') => {
      const key = normalizeDateKey(dateKey);
      if (!key) return;
      const el = listApi?.element;
      if (!el) return;
      const idx = dateIndexByKey.get(key);
      if (idx == null) {
        return;
      }
      const targetLeft = Math.max(0, idx * COLUMN_WIDTH - COLUMN_WIDTH);
      el.scrollTo({ left: targetLeft, behavior });
    },
    [dateIndexByKey, listApi]
  );

  const scrollToToday = useCallback(() => {
    scrollToDateKey(todayKey);
  }, [scrollToDateKey, todayKey]);

  const focusedDateKey = useMemo(() => normalizeDateKey(String(focusedDate || '')), [focusedDate]);
  useEffect(() => {
    if (!focusedDateKey) return;
    scrollToDateKey(focusedDateKey, 'smooth');
  }, [focusedDateKey, scrollToDateKey]);

  const lastAutoOpenKeyRef = useRef<string>('');
  useEffect(() => {
    if (!autoOpenCell) return;
    const machine = String(autoOpenCell.machine || '').trim();
    const dateKey = normalizeDateKey(String(autoOpenCell.date || ''));
    if (!machine || !dateKey) return;
    const nonce = autoOpenCell.nonce != null ? String(autoOpenCell.nonce) : '';
    const openKey = `${machine}__${dateKey}__${nonce}`;
    if (lastAutoOpenKeyRef.current === openKey) return;
    lastAutoOpenKeyRef.current = openKey;
    setCellDetail({ machine, date: dateKey });
    onFocusChange?.({ machine, date: dateKey, source: String(autoOpenCell.source || 'auto') });
    const rowIndex = machines.indexOf(machine);
    if (rowIndex >= 0) {
      listApi?.scrollToRow({ index: rowIndex, behavior: 'auto' });
    }
    scrollToDateKey(dateKey, 'auto');
  }, [autoOpenCell, listApi, machines, onFocusChange, scrollToDateKey, setCellDetail]);

  // 表头
  const headerCells = useMemo(() => {
    return dateKeys.map((key) => {
      const d = dayjs(key);
      const isToday = key === todayKey;
      const isWeekend = d.day() === 0 || d.day() === 6;
      const isFocus = !!focusDateKey && key === focusDateKey;
      const summary = headerDateSummaryByKey.get(key) || null;
      const urgencyBaseTotal = summary
        ? summary.totalWeightT > 0
          ? summary.totalWeightT
          : summary.totalCount
        : 0;
      const urgencyValue = (u: 'L3' | 'L2' | 'L1' | 'L0') => {
        if (!summary) return 0;
        if (summary.totalWeightT > 0) return summary.byUrgency[u].weightT;
        return summary.byUrgency[u].count;
      };

      const tooltipTitle = (
        <div style={{ maxWidth: 420 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', gap: 12, marginBottom: 8 }}>
            <div style={{ fontSize: 13, fontWeight: 600, color: 'rgba(255, 255, 255, 0.92)' }}>{key}</div>
            <div style={{ fontSize: 12, color: 'rgba(255, 255, 255, 0.65)' }}>
              机组：{machineCode && machineCode !== 'all' ? String(machineCode) : '全部'}
            </div>
          </div>

          {summary ? (
            <>
              <div style={{ fontSize: 12, color: 'rgba(255, 255, 255, 0.85)', marginBottom: 8 }}>
                全量：{summary.totalCount} 件 / {summary.totalWeightT.toFixed(3)}t · 冻结 {summary.lockedCount} · 强放{' '}
                {summary.forceReleaseCount} · 可调 {summary.adjustableCount}
              </div>

              <div style={{ fontSize: 12, color: 'rgba(255, 255, 255, 0.65)', marginBottom: 6 }}>紧急度结构（全量）：</div>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6, marginBottom: 10 }}>
                {(['L3', 'L2', 'L1', 'L0'] as const).map((u) => (
                  <div
                    key={u}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 8,
                      padding: '6px 8px',
                      borderRadius: 6,
                      background: 'rgba(255, 255, 255, 0.06)',
                      border: '1px solid rgba(255, 255, 255, 0.10)',
                    }}
                  >
                    <span
                      style={{
                        width: 10,
                        height: 10,
                        borderRadius: 2,
                        background:
                          u === 'L3'
                            ? URGENCY_COLORS.L3_EMERGENCY
                            : u === 'L2'
                            ? URGENCY_COLORS.L2_HIGH
                            : u === 'L1'
                            ? URGENCY_COLORS.L1_MEDIUM
                            : URGENCY_COLORS.L0_NORMAL,
                      }}
                    />
                    <span style={{ fontSize: 12, color: 'rgba(255, 255, 255, 0.85)' }}>
                      {u} {summary.byUrgency[u].count} 件 / {summary.byUrgency[u].weightT.toFixed(3)}t
                    </span>
                  </div>
                ))}
              </div>

              <div style={{ fontSize: 12, color: 'rgba(255, 255, 255, 0.65)', marginBottom: 6 }}>当前筛选说明：</div>
              <div style={{ fontSize: 12, color: 'rgba(255, 255, 255, 0.85)' }}>
                <div>
                  紧急度：{urgentLevel && urgentLevel !== 'all' ? String(urgentLevel).toUpperCase() : '全部'} · 状态：
                  {PLAN_ITEM_STATUS_FILTER_META[(statusFilter || 'ALL') as PlanItemStatusFilter].label}
                </div>
                {urgentLevel && urgentLevel !== 'all' ? (
                  <div style={{ marginTop: 4, color: 'rgba(255, 255, 255, 0.65)' }}>
                    当前视图显示：{summary.viewCount} 件 / {summary.viewWeightT.toFixed(3)}t
                  </div>
                ) : statusFilter && statusFilter !== 'ALL' ? (
                  <div style={{ marginTop: 4, color: 'rgba(255, 255, 255, 0.65)' }}>
                    当前视图显示：{summary.viewCount} 件 / {summary.viewWeightT.toFixed(3)}t
                  </div>
                ) : null}
              </div>
            </>
          ) : (
            <div style={{ fontSize: 12, color: 'rgba(255, 255, 255, 0.65)' }}>该日期暂无排程</div>
          )}
        </div>
      );

      return (
        <Tooltip
          key={key}
          title={tooltipTitle}
          placement="bottom"
          color="#1f1f1f"
          overlayInnerStyle={{
            padding: 12,
            borderRadius: 10,
            boxShadow: '0 10px 28px rgba(0, 0, 0, 0.45)',
          }}
        >
          <div
            style={{
              width: COLUMN_WIDTH,
              flex: `0 0 ${COLUMN_WIDTH}px`,
              borderRight: '1px solid #f0f0f0',
              background: isToday ? '#e6f4ff' : isWeekend ? '#fafafa' : '#fff',
              padding: '6px 0',
              textAlign: 'center',
              cursor: onFocusChange ? 'pointer' : undefined,
              boxShadow: isFocus ? 'inset 0 0 0 2px rgba(22, 119, 255, 0.45)' : undefined,
            }}
            onClick={() => onFocusChange?.({ date: key, source: 'ganttHeader' })}
          >
            <div style={{ fontSize: 12, fontWeight: 600 }}>{d.format('MM-DD')}</div>
            <div style={{ fontSize: 11, color: '#8c8c8c' }}>{d.format('dd')}</div>

            {/* 紧急度结构：日期框格内用颜色区分展示 */}
            <div
              style={{
                width: 84,
                height: 6,
                margin: '4px auto 0',
                borderRadius: 3,
                overflow: 'hidden',
                background: 'rgba(0, 0, 0, 0.06)',
                display: 'flex',
              }}
              aria-label={`${key} 紧急度结构`}
              title={summary ? `${summary.totalCount} 件 / ${summary.totalWeightT.toFixed(3)}t` : '无排程'}
            >
              {(['L3', 'L2', 'L1', 'L0'] as const).map((u) => {
                if (!summary || urgencyBaseTotal <= 0) return null;
                const v = urgencyValue(u);
                const pct = Math.max(0, Math.min(100, (v / urgencyBaseTotal) * 100));
                if (pct <= 0) return null;
                const color =
                  u === 'L3'
                    ? URGENCY_COLORS.L3_EMERGENCY
                    : u === 'L2'
                    ? URGENCY_COLORS.L2_HIGH
                    : u === 'L1'
                    ? URGENCY_COLORS.L1_MEDIUM
                    : URGENCY_COLORS.L0_NORMAL;
                return <div key={u} style={{ width: `${pct}%`, background: color }} />;
              })}
            </div>
          </div>
        </Tooltip>
      );
    });
  }, [dateKeys, focusDateKey, headerDateSummaryByKey, machineCode, onFocusChange, statusFilter, todayKey, urgentLevel]);

  const hasError = error != null;
  const showEmpty = !loading && !hasError && filteredCount === 0;

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <GanttToolbar
        range={range}
        onRangeChange={onRangeChange}
        onResetRange={resetRange}
        onScrollToToday={scrollToToday}
        dateKeysLength={dateKeys.length}
        machinesCount={machines.length}
        filteredCount={filteredCount}
        filteredTotalWeight={filteredTotalWeight}
        dateRangeStart={dateKeys[0] || ''}
        dateRangeEnd={dateKeys[dateKeys.length - 1] || ''}
        statusSummary={statusSummary}
        statusFilter={statusFilter || 'ALL'}
        onStatusFilterChange={onStatusFilterChange}
      />

      {hasError && (
        <Alert
          type="error"
          showIcon
          message="甘特图数据加载失败"
          description={error instanceof Error ? error.message : String(error)}
          action={onRetry ? <a onClick={onRetry} style={{ padding: '0 8px' }}>重试</a> : null}
          style={{ marginBottom: 8 }}
        />
      )}

      {showEmpty && (
        <Alert
          type="info"
          showIcon
          message="当前范围/筛选下暂无排程数据"
          description="可调整时间范围、状态/紧急度筛选，或切换机组查看"
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
            <Text type="secondary" style={{ fontSize: 12 }}>日期</Text>
          </div>
          <div style={{ flex: 1, minWidth: 0, overflow: 'hidden' }}>
            <div
              ref={headerInnerRef}
              style={{ width: timelineWidth, display: 'flex', willChange: 'transform' }}
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
                rowComponent={GanttRow as any} // React.memo 包装后类型与 react-window 不兼容
                rowProps={{
                  machines,
                  itemsByMachineDate: itemsByMachineDateFiltered,
                  dateKeys,
                  dateIndexByKey,
                  capacityByMachineDate,
                  selected: selectedSet,
                  onToggle: toggleSelection,
                  onInspect: onInspectMaterialId,
                  onOpenCell: handleOpenCell,
                  timelineWidth,
                  todayIndex,
                }}
                onScroll={handleListScroll}
                style={{ height, width, overflowX: 'auto', overflowY: 'auto' }}
              />
            )}
          </AutoSizer>
        </div>
      </div>

      <CellDetailModal
        cellDetail={cellDetail}
        itemsByMachineDate={itemsByMachineDateForModal}
        capacityByMachineDate={capacityByMachineDate}
        selectedSet={selectedSet}
        selectedMaterialIds={selectedMaterialIds}
        onClose={() => setCellDetail(null)}
        onToggleSelection={toggleSelection}
        onInspectMaterialId={onInspectMaterialId}
        onRequestMoveToCell={onRequestMoveToCell}
        viewUrgentFilter={urgentLevel}
        viewStatusFilter={statusFilter || 'ALL'}
        onNavigateToMatrix={onNavigateToMatrix}
      />
    </div>
  );
}

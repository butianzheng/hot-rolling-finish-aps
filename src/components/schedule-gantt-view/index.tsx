/**
 * 排程甘特图组件
 *
 * 重构后：935 行 → ~180 行 (-81%)
 */

import React, { useCallback, useEffect, useMemo, useRef } from 'react';
import { Alert, Typography, message } from 'antd';
import dayjs from 'dayjs';
import { useQuery } from '@tanstack/react-query';
import AutoSizer from 'react-virtualized-auto-sizer';
import { List, useListCallbackRef } from 'react-window';
import { capacityApi } from '../../api/tauri';
import { useActiveVersionId } from '../../stores/use-global-store';
import type { ScheduleGanttViewProps, CapacityData } from './types';
import { LEFT_COL_WIDTH, HEADER_HEIGHT, ROW_HEIGHT, COLUMN_WIDTH } from './types';
import { normalizeDateKey } from './utils';
import { useGanttData } from './useGanttData';
import { GanttRow } from './GanttRow';
import { GanttToolbar } from './GanttToolbar';
import { CellDetailModal } from './CellDetailModal';

const { Text } = Typography;

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

  const ganttData = useGanttData({
    machineCode,
    urgentLevel,
    planItems,
    selectedMaterialIds,
    onSelectedMaterialIdsChange,
  });

  const {
    dateKeys,
    dateIndexByKey,
    todayKey,
    todayIndex,
    machines,
    itemsByMachineDate,
    filteredCount,
    filteredTotalWeight,
    selectedSet,
    toggleSelection,
    range,
    onRangeChange,
    resetRange,
    cellDetail,
    setCellDetail,
  } = ganttData;

  const timelineWidth = useMemo(() => dateKeys.length * COLUMN_WIDTH, [dateKeys.length]);

  // 容量查询
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
    const map = new Map<string, CapacityData>();
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

  // 表头
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
      />

      {hasError && (
        <Alert
          type="error"
          showIcon
          message="甘特图数据加载失败"
          description={String((error as any)?.message || error)}
          action={onRetry ? <a onClick={onRetry} style={{ padding: '0 8px' }}>重试</a> : null}
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
                rowComponent={GanttRow}
                rowProps={{
                  machines,
                  itemsByMachineDate,
                  dateKeys,
                  dateIndexByKey,
                  capacityByMachineDate,
                  selected: selectedSet,
                  onToggle: toggleSelection,
                  onInspect: onInspectMaterialId,
                  onOpenCell: (machine: string, date: string) => setCellDetail({ machine, date }),
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
        itemsByMachineDate={itemsByMachineDate}
        capacityByMachineDate={capacityByMachineDate}
        selectedSet={selectedSet}
        selectedMaterialIds={selectedMaterialIds}
        onClose={() => setCellDetail(null)}
        onToggleSelection={toggleSelection}
        onInspectMaterialId={onInspectMaterialId}
        onRequestMoveToCell={onRequestMoveToCell}
      />
    </div>
  );
}

/**
 * 甘特图行渲染组件
 */

import React from 'react';
import { Typography } from 'antd';
import type { RowComponentProps } from 'react-window';
import { FONT_FAMILIES } from '../../theme';
import { formatWeight } from '../../utils/formatters';
import type { PlanItemRow, CapacityData } from './types';
import {
  LEFT_COL_WIDTH,
  ROW_HEIGHT,
  COLUMN_WIDTH,
  CELL_PADDING_X,
  BAR_HEIGHT,
  BAR_GAP,
  MAX_ITEMS_PER_CELL,
} from './types';
import { normalizeDateKey, urgencyToColor } from './utils';

const { Text } = Typography;

export type GanttRowData = {
  machines: string[];
  itemsByMachineDate: Map<string, Map<string, PlanItemRow[]>>;
  dateKeys: string[];
  dateIndexByKey: Map<string, number>;
  capacityByMachineDate: Map<string, CapacityData>;
  selected: Set<string>;
  onToggle: (id: string, checked: boolean) => void;
  onInspect?: (id: string) => void;
  onOpenCell?: (machine: string, date: string) => void;
  timelineWidth: number;
  todayIndex: number;
};

export const GanttRow = React.memo(function GanttRow({
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
}: RowComponentProps<GanttRowData>) {
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
});

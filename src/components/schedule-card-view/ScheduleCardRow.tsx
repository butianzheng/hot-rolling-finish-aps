import React from 'react';
import { Tooltip, Typography } from 'antd';
import { RightOutlined, DownOutlined, ClusterOutlined, CalendarOutlined } from '@ant-design/icons';
import { FONT_FAMILIES } from '../../theme';
import { formatWeight } from '../../utils/formatters';
import type { ScheduleTreeRow, DateStatusSummary } from './types';

const { Text } = Typography;

export type ScheduleCardRowProps = {
  row: ScheduleTreeRow;
  style?: React.CSSProperties;
  onToggleMachine: (machineCode: string) => void;
};

/** 状态占比堆叠横条 */
const StatusBar: React.FC<{ status: DateStatusSummary; total: number }> = React.memo(({ status, total }) => {
  if (total === 0) return null;
  const segments: { count: number; color: string; label: string }[] = [
    { count: status.lockedCount, color: '#722ed1', label: '冻结' },
    { count: status.forceReleaseCount, color: '#f5222d', label: '强放' },
    { count: status.adjustableCount, color: '#1677ff', label: '可调' },
  ];

  return (
    <Tooltip
      title={segments.map((s) => `${s.label} ${s.count} 件 (${((s.count / total) * 100).toFixed(0)}%)`).join(' / ')}
    >
      <div
        style={{
          flex: 1,
          height: 16,
          display: 'flex',
          borderRadius: 3,
          overflow: 'hidden',
          minWidth: 60,
        }}
      >
        {segments.map((seg) => {
          if (seg.count === 0) return null;
          const pct = (seg.count / total) * 100;
          return (
            <div
              key={seg.label}
              style={{
                width: `${pct}%`,
                backgroundColor: seg.color,
                minWidth: seg.count > 0 ? 4 : 0,
                transition: 'width 0.2s',
              }}
            />
          );
        })}
      </div>
    </Tooltip>
  );
});

export const ScheduleCardRow: React.FC<ScheduleCardRowProps> = React.memo(({
  row,
  style,
  onToggleMachine,
}) => {
  // 机组分组头
  if (row.type === 'machine') {
    return (
      <div
        style={{
          ...style,
          padding: '0 8px',
          display: 'flex',
          alignItems: 'center',
        }}
      >
        <div
          onClick={() => onToggleMachine(row.machineCode)}
          style={{
            flex: 1,
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '8px 12px',
            background: '#fafafa',
            borderRadius: 6,
            cursor: 'pointer',
            userSelect: 'none',
            borderLeft: '3px solid #1677ff',
          }}
        >
          {row.collapsed ? (
            <RightOutlined style={{ fontSize: 12, color: '#8c8c8c' }} />
          ) : (
            <DownOutlined style={{ fontSize: 12, color: '#8c8c8c' }} />
          )}
          <ClusterOutlined style={{ fontSize: 14, color: '#1677ff' }} />
          <Text strong style={{ flex: 1 }}>
            {row.machineCode}
          </Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {row.count} 件
          </Text>
          <Text type="secondary" style={{ fontSize: 12, fontFamily: FONT_FAMILIES.MONOSPACE }}>
            {formatWeight(row.weightT)}
          </Text>
        </div>
      </div>
    );
  }

  // 日期行（含状态占比条形图）
  return (
    <div
      style={{
        ...style,
        padding: '0 8px 0 32px',
        display: 'flex',
        alignItems: 'center',
      }}
    >
      <div
        style={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '6px 12px',
          background: '#fff',
          borderRadius: 4,
          border: '1px solid #f0f0f0',
        }}
      >
        <CalendarOutlined style={{ fontSize: 12, color: '#8c8c8c', flexShrink: 0 }} />
        <Text style={{ fontSize: 13, flexShrink: 0, width: 80 }}>
          {row.date}
        </Text>
        <StatusBar status={row.status} total={row.count} />
        <Text type="secondary" style={{ fontSize: 12, flexShrink: 0, fontFamily: FONT_FAMILIES.MONOSPACE }}>
          {row.count}件 {formatWeight(row.weightT)}
        </Text>
      </div>
    </div>
  );
});

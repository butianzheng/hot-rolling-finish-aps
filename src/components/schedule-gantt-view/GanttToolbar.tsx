/**
 * 甘特图工具栏和图例
 */

import React from 'react';
import { Button, DatePicker, Space, Tag, Typography } from 'antd';
import type { Dayjs } from 'dayjs';
import { URGENCY_COLORS } from '../../theme/tokens';
import { formatWeight } from '../../utils/formatters';

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
}) => {
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
      </Space>
      {legend}
      <Text type="secondary" style={{ fontSize: 12 }}>
        机组 {machinesCount} · 任务 {filteredCount} · 总重 {formatWeight(filteredTotalWeight)} · 范围 {dateRangeStart || '-'} ~{' '}
        {dateRangeEnd || '-'}
      </Text>
    </Space>
  );
};

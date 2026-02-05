/**
 * 日历视图切换器
 * 职责：提供日/月视图切换和快捷日期范围选择
 */

import React from 'react';
import { Space, Radio, Button, DatePicker } from 'antd';
import dayjs, { Dayjs } from 'dayjs';
import type { ViewMode, DateRangePreset } from './types';

const { RangePicker } = DatePicker;

export interface CalendarViewSwitcherProps {
  viewMode: ViewMode;
  onViewModeChange: (mode: ViewMode) => void;
  dateRange: { dateFrom: string; dateTo: string };
  onDateRangeChange: (dateFrom: string, dateTo: string) => void;
}

// 快捷日期范围选项
const DATE_PRESETS: DateRangePreset[] = [
  {
    label: '近7天',
    getValue: () => {
      const today = dayjs();
      return {
        dateFrom: today.format('YYYY-MM-DD'),
        dateTo: today.add(6, 'day').format('YYYY-MM-DD'),
      };
    },
  },
  {
    label: '近30天',
    getValue: () => {
      const today = dayjs();
      return {
        dateFrom: today.format('YYYY-MM-DD'),
        dateTo: today.add(29, 'day').format('YYYY-MM-DD'),
      };
    },
  },
  {
    label: '本月',
    getValue: () => {
      const today = dayjs();
      return {
        dateFrom: today.startOf('month').format('YYYY-MM-DD'),
        dateTo: today.endOf('month').format('YYYY-MM-DD'),
      };
    },
  },
  {
    label: '本季度',
    getValue: () => {
      const today = dayjs();
      const month = today.month(); // 0-11
      const quarterStartMonth = Math.floor(month / 3) * 3; // 0, 3, 6, 9
      return {
        dateFrom: today.month(quarterStartMonth).startOf('month').format('YYYY-MM-DD'),
        dateTo: today.month(quarterStartMonth + 2).endOf('month').format('YYYY-MM-DD'),
      };
    },
  },
  {
    label: '全年',
    getValue: () => {
      const today = dayjs();
      return {
        dateFrom: today.startOf('year').format('YYYY-MM-DD'),
        dateTo: today.endOf('year').format('YYYY-MM-DD'),
      };
    },
  },
];

export const CalendarViewSwitcher: React.FC<CalendarViewSwitcherProps> = ({
  viewMode,
  onViewModeChange,
  dateRange,
  onDateRangeChange,
}) => {
  // 处理日期范围变更
  const handleRangeChange = (dates: null | [Dayjs | null, Dayjs | null]) => {
    if (dates && dates[0] && dates[1]) {
      onDateRangeChange(
        dates[0].format('YYYY-MM-DD'),
        dates[1].format('YYYY-MM-DD')
      );
    }
  };

  // 快捷选择
  const handlePresetClick = (preset: DateRangePreset) => {
    const { dateFrom, dateTo } = preset.getValue();
    onDateRangeChange(dateFrom, dateTo);
  };

  return (
    <Space size={8}>
      {/* 视图模式切换 */}
      <Radio.Group
        value={viewMode}
        onChange={(e) => onViewModeChange(e.target.value)}
        buttonStyle="solid"
        size="small"
      >
        <Radio.Button value="day">日</Radio.Button>
        <Radio.Button value="month">月</Radio.Button>
      </Radio.Group>

      {/* 日期范围选择器 */}
      <RangePicker
        value={[dayjs(dateRange.dateFrom), dayjs(dateRange.dateTo)]}
        onChange={handleRangeChange}
        format="YYYY-MM-DD"
        allowClear={false}
        size="small"
      />

      {/* 快捷选择按钮 */}
      {DATE_PRESETS.map((preset) => (
        <Button
          key={preset.label}
          size="small"
          onClick={() => handlePresetClick(preset)}
        >
          {preset.label}
        </Button>
      ))}
    </Space>
  );
};

export default CalendarViewSwitcher;

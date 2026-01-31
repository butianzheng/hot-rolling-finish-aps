/**
 * 工具栏组件
 */

import React from 'react';
import { Button, DatePicker, Select, Space } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import type dayjs from 'dayjs';
import type { DateRangeValue, MachineOption } from './types';

const { RangePicker } = DatePicker;

export interface ToolBarProps {
  dateRange: DateRangeValue;
  onDateRangeChange: (range: DateRangeValue) => void;
  onResetDateRange?: () => void;
  selectedMachine: string;
  onMachineChange: (machine: string) => void;
  machineOptions: MachineOption[];
  onRefresh: () => void;
  loading: boolean;
}

export const ToolBar: React.FC<ToolBarProps> = ({
  dateRange,
  onDateRangeChange,
  onResetDateRange,
  selectedMachine,
  onMachineChange,
  machineOptions,
  onRefresh,
  loading,
}) => {
  return (
    <Space style={{ marginBottom: 16 }} size={16}>
      <RangePicker
        value={dateRange}
        allowClear={!!onResetDateRange}
        onChange={(dates) => {
          if (!dates || !dates[0] || !dates[1]) {
            onResetDateRange?.();
            return;
          }
          onDateRangeChange(dates as [dayjs.Dayjs, dayjs.Dayjs]);
        }}
        format="YYYY-MM-DD"
      />
      {onResetDateRange ? (
        <Button onClick={onResetDateRange}>
          重置范围
        </Button>
      ) : null}
      <Select
        style={{ width: 150 }}
        value={selectedMachine}
        onChange={onMachineChange}
        options={[
          { label: '全部机组', value: 'all' },
          ...machineOptions,
        ]}
      />
      <Button icon={<ReloadOutlined />} onClick={onRefresh} loading={loading}>
        刷新
      </Button>
    </Space>
  );
};

export default ToolBar;

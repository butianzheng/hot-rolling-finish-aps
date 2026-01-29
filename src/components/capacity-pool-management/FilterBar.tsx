/**
 * 产能池筛选栏
 */

import React from 'react';
import { Button, Card, DatePicker, Select, Space } from 'antd';
import type { Dayjs } from 'dayjs';

const { RangePicker } = DatePicker;
const { Option } = Select;

export interface FilterBarProps {
  machineOptions: string[];
  selectedMachines: string[];
  onMachinesChange: (machines: string[]) => void;
  dateRange: [Dayjs, Dayjs];
  onDateRangeChange: (range: [Dayjs, Dayjs]) => void;
  loading: boolean;
  onQuery: () => void;
}

export const FilterBar: React.FC<FilterBarProps> = ({
  machineOptions,
  selectedMachines,
  onMachinesChange,
  dateRange,
  onDateRangeChange,
  loading,
  onQuery,
}) => {
  return (
    <Card style={{ marginBottom: 16 }}>
      <Space wrap>
        <Select
          mode="multiple"
          style={{ width: 300 }}
          placeholder="选择机组"
          value={selectedMachines}
          onChange={onMachinesChange}
        >
          {machineOptions.map((code) => (
            <Option key={code} value={code}>
              {code}
            </Option>
          ))}
        </Select>

        <RangePicker
          value={dateRange}
          onChange={(dates) => dates && onDateRangeChange(dates as [Dayjs, Dayjs])}
          format="YYYY-MM-DD"
        />

        <Button type="primary" onClick={onQuery} loading={loading}>
          查询
        </Button>
      </Space>
    </Card>
  );
};

export default FilterBar;

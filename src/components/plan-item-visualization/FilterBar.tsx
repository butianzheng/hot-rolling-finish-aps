/**
 * 筛选栏组件
 */

import React from 'react';
import { Button, Card, DatePicker, Input, Select, Space } from 'antd';
import { FilterOutlined } from '@ant-design/icons';
import type { Dayjs } from 'dayjs';

const { Option } = Select;
const { Search } = Input;
const { RangePicker } = DatePicker;

export interface FilterBarProps {
  machineOptions: string[];
  selectedMachine: string;
  onMachineChange: (value: string) => void;
  selectedUrgentLevel: string;
  onUrgentLevelChange: (value: string) => void;
  selectedDate: Dayjs | null;
  onDateChange: (date: Dayjs | null) => void;
  dateRange: [Dayjs, Dayjs] | null;
  onDateRangeChange: (range: [Dayjs, Dayjs] | null) => void;
  searchText: string;
  onSearchChange: (text: string) => void;
  onClearFilters: () => void;
}

export const FilterBar: React.FC<FilterBarProps> = ({
  machineOptions,
  selectedMachine,
  onMachineChange,
  selectedUrgentLevel,
  onUrgentLevelChange,
  selectedDate,
  onDateChange,
  dateRange,
  onDateRangeChange,
  searchText,
  onSearchChange,
  onClearFilters,
}) => {
  return (
    <Card style={{ marginBottom: 16 }}>
      <Space wrap>
        <Select
          style={{ width: 150 }}
          placeholder="选择机组"
          value={selectedMachine}
          onChange={onMachineChange}
        >
          <Option value="all">全部机组</Option>
          {machineOptions.map((code) => (
            <Option key={code} value={code}>
              {code}
            </Option>
          ))}
        </Select>

        <Select
          style={{ width: 150 }}
          placeholder="紧急等级"
          value={selectedUrgentLevel}
          onChange={onUrgentLevelChange}
        >
          <Option value="all">全部等级</Option>
          <Option value="L3">L3-超紧急</Option>
          <Option value="L2">L2-紧急</Option>
          <Option value="L1">L1-较紧急</Option>
          <Option value="L0">L0-正常</Option>
        </Select>

        <DatePicker
          placeholder="选择日期"
          value={selectedDate}
          onChange={onDateChange}
          format="YYYY-MM-DD"
        />

        <RangePicker
          placeholder={['开始日期', '结束日期']}
          value={dateRange as any}
          onChange={(dates) => {
            if (dates && dates[0] && dates[1]) {
              onDateRangeChange([dates[0], dates[1]]);
            } else {
              onDateRangeChange(null);
            }
          }}
          format="YYYY-MM-DD"
        />

        <Search
          placeholder="搜索材料编号或钢种"
          value={searchText}
          onChange={(e) => onSearchChange(e.target.value)}
          style={{ width: 250 }}
          allowClear
        />

        <Button icon={<FilterOutlined />} onClick={onClearFilters}>
          清除筛选
        </Button>
      </Space>
    </Card>
  );
};

export default FilterBar;

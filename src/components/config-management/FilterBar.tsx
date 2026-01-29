/**
 * 配置筛选栏
 */

import React from 'react';
import { Button, Card, Input, Select, Space } from 'antd';

const { Option } = Select;

export interface FilterBarProps {
  searchText: string;
  onSearchTextChange: (text: string) => void;
  selectedScopeType: string;
  onScopeTypeChange: (type: string) => void;
  onClearFilters: () => void;
}

export const FilterBar: React.FC<FilterBarProps> = ({
  searchText,
  onSearchTextChange,
  selectedScopeType,
  onScopeTypeChange,
  onClearFilters,
}) => {
  return (
    <Card style={{ marginBottom: 16 }}>
      <Space wrap>
        <Input
          placeholder="搜索配置键、值或作用域ID"
          value={searchText}
          onChange={(e) => onSearchTextChange(e.target.value)}
          style={{ width: 250 }}
          allowClear
        />

        <Select
          style={{ width: 200 }}
          placeholder="选择作用域类型"
          value={selectedScopeType}
          onChange={onScopeTypeChange}
        >
          <Option value="all">全部类型</Option>
          <Option value="GLOBAL">GLOBAL</Option>
          <Option value="MACHINE">MACHINE</Option>
          <Option value="STEEL_GRADE">STEEL_GRADE</Option>
          <Option value="VERSION">VERSION</Option>
        </Select>

        <Button onClick={onClearFilters}>
          清除筛选
        </Button>
      </Space>
    </Card>
  );
};

export default FilterBar;

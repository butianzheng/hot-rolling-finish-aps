/**
 * 配置筛选栏
 */

import React, { useState } from 'react';
import { Button, Card, Input, Select, Space } from 'antd';
import { useDebounce } from '../../hooks/useDebounce';

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
  // M3修复：添加防抖优化，减少搜索框频繁触发筛选
  const [localSearchText, setLocalSearchText] = useState(searchText);
  const debouncedSearchText = useDebounce(localSearchText, 400);

  // 同步外部searchText变化到本地状态
  React.useEffect(() => {
    setLocalSearchText(searchText);
  }, [searchText]);

  // 当防抖后的值变化时，触发外部回调
  React.useEffect(() => {
    if (debouncedSearchText !== searchText) {
      onSearchTextChange(debouncedSearchText);
    }
  }, [debouncedSearchText, onSearchTextChange, searchText]);

  return (
    <Card style={{ marginBottom: 16 }}>
      <Space wrap>
        <Input
          placeholder="搜索配置键、值或作用域ID"
          value={localSearchText}
          onChange={(e) => setLocalSearchText(e.target.value)}
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

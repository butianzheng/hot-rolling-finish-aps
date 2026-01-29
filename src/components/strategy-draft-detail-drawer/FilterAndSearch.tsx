import React from 'react';
import { Input, Segmented, Space, Typography } from 'antd';

const { Text } = Typography;

export interface FilterAndSearchProps {
  filter: 'ALL' | 'MOVED' | 'ADDED' | 'SQUEEZED_OUT';
  search: string;
  loading: boolean;
  itemCount: number;
  onFilterChange: (filter: 'ALL' | 'MOVED' | 'ADDED' | 'SQUEEZED_OUT') => void;
  onSearchChange: (search: string) => void;
}

export const FilterAndSearch: React.FC<FilterAndSearchProps> = ({
  filter,
  search,
  loading,
  itemCount,
  onFilterChange,
  onSearchChange,
}) => (
  <Space wrap>
    <Segmented
      value={filter}
      onChange={(v) => onFilterChange(v as any)}
      options={[
        { label: '全部', value: 'ALL' },
        { label: '移动', value: 'MOVED' },
        { label: '新增', value: 'ADDED' },
        { label: '挤出', value: 'SQUEEZED_OUT' },
      ]}
    />
    <Input
      allowClear
      placeholder="搜索 material_id"
      style={{ width: 240 }}
      value={search}
      onChange={(e) => onSearchChange(e.target.value)}
    />
    <Text type="secondary" style={{ fontSize: 12 }}>
      {loading ? '加载中…' : `共 ${itemCount} 条`}
    </Text>
  </Space>
);

import React, { useState } from 'react';
import { Input, Segmented, Space, Typography } from 'antd';
import { useDebounce } from '../../hooks/useDebounce';

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
}) => {
  // M3修复：添加防抖优化，减少搜索框频繁触发筛选
  const [localSearch, setLocalSearch] = useState(search);
  const debouncedSearch = useDebounce(localSearch, 300);

  // 同步外部search变化到本地状态
  React.useEffect(() => {
    setLocalSearch(search);
  }, [search]);

  // 当防抖后的值变化时，触发外部回调
  React.useEffect(() => {
    if (debouncedSearch !== search) {
      onSearchChange(debouncedSearch);
    }
  }, [debouncedSearch, onSearchChange, search]);

  return (
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
        value={localSearch}
        onChange={(e) => setLocalSearch(e.target.value)}
      />
      <Text type="secondary" style={{ fontSize: 12 }}>
        {loading ? '加载中…' : `共 ${itemCount} 条`}
      </Text>
    </Space>
  );
};

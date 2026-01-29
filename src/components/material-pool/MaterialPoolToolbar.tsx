/**
 * MaterialPool 工具栏组件（搜索和筛选）
 */

import React from 'react';
import { Button, Checkbox, Input, Select, Space } from 'antd';
import type { MaterialPoolFilters } from './types';
import type { WorkbenchLockStatusFilter } from '../../stores/use-global-store';

interface MaterialPoolToolbarProps {
  searchText: string;
  onSearchChange: (value: string) => void;
  loading?: boolean;

  filters?: MaterialPoolFilters;
  onFiltersChange?: (next: Partial<MaterialPoolFilters>) => void;

  groupByUrgency: boolean;
  onGroupByUrgencyChange: (value: boolean) => void;
}

export const MaterialPoolToolbar: React.FC<MaterialPoolToolbarProps> = ({
  searchText,
  onSearchChange,
  loading,
  filters,
  onFiltersChange,
  groupByUrgency,
  onGroupByUrgencyChange,
}) => {
  return (
    <>
      <Input.Search
        placeholder="搜索材料号"
        allowClear
        value={searchText}
        onChange={(e) => onSearchChange(e.target.value)}
        onSearch={(v) => onSearchChange(v)}
        disabled={loading}
      />

      {filters ? (
        <Space wrap size={8} style={{ width: '100%' }}>
          <Select
            size="small"
            style={{ width: 120 }}
            value={filters.urgencyLevel ?? 'ALL'}
            onChange={(value) => onFiltersChange?.({ urgencyLevel: value === 'ALL' ? null : value })}
            options={[
              { value: 'ALL', label: '全部紧急度' },
              { value: 'L3', label: 'L3' },
              { value: 'L2', label: 'L2' },
              { value: 'L1', label: 'L1' },
              { value: 'L0', label: 'L0' },
            ]}
          />
          <Select
            size="small"
            style={{ width: 120 }}
            value={filters.lockStatus}
            onChange={(value) => onFiltersChange?.({ lockStatus: value as WorkbenchLockStatusFilter })}
            options={[
              { value: 'ALL', label: '全部锁定' },
              { value: 'LOCKED', label: '已锁定' },
              { value: 'UNLOCKED', label: '未锁定' },
            ]}
          />
          <Button
            size="small"
            onClick={() => onFiltersChange?.({ urgencyLevel: null, lockStatus: 'ALL' })}
          >
            重置筛选
          </Button>
          <Checkbox
            checked={groupByUrgency}
            onChange={(e) => onGroupByUrgencyChange(e.target.checked)}
            style={{ fontSize: 12 }}
          >
            按紧急度分组
          </Checkbox>
        </Space>
      ) : null}
    </>
  );
};

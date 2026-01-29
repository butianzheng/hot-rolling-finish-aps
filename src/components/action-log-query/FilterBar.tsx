/**
 * 操作日志筛选栏
 */

import React from 'react';
import { Alert, Button, Card, DatePicker, Input, Select, Space } from 'antd';
import { FilterOutlined } from '@ant-design/icons';
import type { Dayjs } from 'dayjs';
import { actionTypeLabels } from './types';

const { Option } = Select;
const { RangePicker } = DatePicker;
const { Search } = Input;

export interface FilterBarProps {
  loadError: string | null;
  timeRange: [Dayjs, Dayjs] | null;
  onTimeRangeChange: (range: [Dayjs, Dayjs] | null) => void;
  selectedActionType: string;
  onActionTypeChange: (type: string) => void;
  selectedActor: string;
  onActorChange: (actor: string) => void;
  selectedVersion: string;
  onVersionChange: (version: string) => void;
  searchText: string;
  onSearchTextChange: (text: string) => void;
  uniqueActors: string[];
  uniqueVersions: string[];
  onClearFilters: () => void;
  onRetry: () => void;
}

export const FilterBar: React.FC<FilterBarProps> = ({
  loadError,
  timeRange,
  onTimeRangeChange,
  selectedActionType,
  onActionTypeChange,
  selectedActor,
  onActorChange,
  selectedVersion,
  onVersionChange,
  searchText,
  onSearchTextChange,
  uniqueActors,
  uniqueVersions,
  onClearFilters,
  onRetry,
}) => {
  return (
    <Card style={{ marginBottom: 16 }}>
      {loadError && (
        <Alert
          type="error"
          showIcon
          message="操作日志加载失败"
          description={loadError}
          action={
            <Button size="small" onClick={onRetry}>
              重试
            </Button>
          }
          style={{ marginBottom: 12 }}
        />
      )}
      <Space wrap size="middle">
        <RangePicker
          showTime
          placeholder={['开始时间', '结束时间']}
          value={timeRange as any}
          onChange={(dates) => {
            if (dates && dates[0] && dates[1]) {
              onTimeRangeChange([dates[0], dates[1]]);
            } else {
              onTimeRangeChange(null);
            }
          }}
          format="YYYY-MM-DD HH:mm:ss"
          style={{ width: 400 }}
        />

        <Select
          style={{ width: 150 }}
          placeholder="操作类型"
          value={selectedActionType}
          onChange={onActionTypeChange}
        >
          <Option value="all">全部类型</Option>
          {Object.entries(actionTypeLabels).map(([key, value]) => (
            <Option key={key} value={key}>
              {value.text}
            </Option>
          ))}
        </Select>

        <Select
          style={{ width: 120 }}
          placeholder="操作人"
          value={selectedActor}
          onChange={onActorChange}
        >
          <Option value="all">全部操作人</Option>
          {uniqueActors.map((actor) => (
            <Option key={actor} value={actor}>
              {actor}
            </Option>
          ))}
        </Select>

        <Select
          style={{ width: 120 }}
          placeholder="版本"
          value={selectedVersion}
          onChange={onVersionChange}
        >
          <Option value="all">全部版本</Option>
          {uniqueVersions.map((version) => (
            <Option key={version} value={version}>
              {version}
            </Option>
          ))}
        </Select>

        <Search
          placeholder="搜索操作ID或详情"
          value={searchText}
          onChange={(e) => onSearchTextChange(e.target.value)}
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

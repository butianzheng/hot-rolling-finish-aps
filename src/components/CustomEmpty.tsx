import React from 'react';
import { Empty } from 'antd';
import { EmptyProps } from 'antd/es/empty';

interface CustomEmptyProps extends EmptyProps {
  type?: 'data' | 'search' | 'filter';
}

const emptyDescriptions = {
  data: '暂无数据',
  search: '未找到匹配的结果',
  filter: '当前筛选条件下无数据',
};

export const CustomEmpty: React.FC<CustomEmptyProps> = ({ type = 'data', ...props }) => {
  return (
    <Empty
      image={Empty.PRESENTED_IMAGE_SIMPLE}
      description={emptyDescriptions[type]}
      {...props}
    />
  );
};

// 表格空态配置
export const tableEmptyConfig = {
  emptyText: <CustomEmpty type="data" />,
};

export const tableSearchEmptyConfig = {
  emptyText: <CustomEmpty type="search" />,
};

export const tableFilterEmptyConfig = {
  emptyText: <CustomEmpty type="filter" />,
};

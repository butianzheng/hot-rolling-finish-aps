/**
 * M2修复：统一的空数据状态组件
 * 用于决策看板和其他页面的空数据展示
 */

import React from 'react';
import { Empty } from 'antd';
import {
  InboxOutlined,
  DatabaseOutlined,
  FileTextOutlined,
  CalendarOutlined,
  WarningOutlined,
} from '@ant-design/icons';

export type EmptyType = 'data' | 'search' | 'order' | 'date' | 'error';

interface EmptyStateProps {
  /** 空状态类型 */
  type?: EmptyType;
  /** 自定义标题 */
  title?: string;
  /** 自定义描述 */
  description?: string;
  /** 自定义样式 */
  style?: React.CSSProperties;
}

const EMPTY_CONFIG: Record<EmptyType, { icon: React.ReactNode; title: string; description: string }> = {
  data: {
    icon: <DatabaseOutlined style={{ fontSize: 48, color: '#d9d9d9' }} />,
    title: '暂无数据',
    description: '当前条件下没有可显示的数据',
  },
  search: {
    icon: <InboxOutlined style={{ fontSize: 48, color: '#d9d9d9' }} />,
    title: '未找到结果',
    description: '请尝试调整筛选条件或搜索关键词',
  },
  order: {
    icon: <FileTextOutlined style={{ fontSize: 48, color: '#d9d9d9' }} />,
    title: '暂无订单',
    description: '当前条件下没有匹配的订单',
  },
  date: {
    icon: <CalendarOutlined style={{ fontSize: 48, color: '#d9d9d9' }} />,
    title: '暂无排产数据',
    description: '该日期范围内没有排产记录',
  },
  error: {
    icon: <WarningOutlined style={{ fontSize: 48, color: '#faad14' }} />,
    title: '数据加载失败',
    description: '请稍后重试或联系管理员',
  },
};

export const EmptyState: React.FC<EmptyStateProps> = ({
  type = 'data',
  title,
  description,
  style,
}) => {
  const config = EMPTY_CONFIG[type];

  return (
    <div style={{ textAlign: 'center', padding: '60px 20px', ...style }}>
      <Empty
        image={config.icon}
        imageStyle={{ height: 60 }}
        description={
          <div style={{ marginTop: 16 }}>
            <div style={{ fontSize: 16, color: '#595959', marginBottom: 8 }}>
              {title || config.title}
            </div>
            <div style={{ fontSize: 14, color: '#8c8c8c' }}>
              {description || config.description}
            </div>
          </div>
        }
      />
    </div>
  );
};

/**
 * 配置表格列配置
 */

import { Button, Tag, Tooltip } from 'antd';
import { EditOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import type { ConfigItem } from './types';
import { scopeTypeColors, configDescriptions } from './types';

export interface ConfigColumnsOptions {
  onEdit: (record: ConfigItem) => void;
}

export function createConfigColumns(
  options: ConfigColumnsOptions
): ColumnsType<ConfigItem> {
  const { onEdit } = options;

  return [
    {
      title: '作用域类型',
      dataIndex: 'scope_type',
      key: 'scope_type',
      width: 120,
      render: (type: string) => (
        <Tag color={scopeTypeColors[type] || 'default'}>{type}</Tag>
      ),
    },
    {
      title: '作用域ID',
      dataIndex: 'scope_id',
      key: 'scope_id',
      width: 150,
    },
    {
      title: '配置键',
      dataIndex: 'key',
      key: 'key',
      width: 200,
      render: (key: string) => (
        <Tooltip title={configDescriptions[key] || '无描述'}>
          <span style={{ cursor: 'help' }}>{key}</span>
        </Tooltip>
      ),
    },
    {
      title: '配置值',
      dataIndex: 'value',
      key: 'value',
      width: 150,
      render: (value: string) => (
        <span style={{ fontWeight: 'bold', color: '#1890ff' }}>{value}</span>
      ),
    },
    {
      title: '更新时间',
      dataIndex: 'updated_at',
      key: 'updated_at',
      width: 180,
    },
    {
      title: '操作',
      key: 'action',
      width: 100,
      fixed: 'right',
      render: (_, record) => (
        <Button
          type="link"
          size="small"
          icon={<EditOutlined />}
          onClick={() => onEdit(record)}
        >
          编辑
        </Button>
      ),
    },
  ];
}

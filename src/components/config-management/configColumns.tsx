/**
 * 配置表格列配置
 */

import { Button, Space, Tag, Tooltip } from 'antd';
import { EditOutlined, SettingOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import type { ConfigItem } from './types';
import { scopeTypeColors, scopeTypeLabels, scopeIdLabels, configKeyLabels, configDescriptions } from './types';

export interface ConfigColumnsOptions {
  onEdit: (record: ConfigItem) => void;
  onOpenPathRulePanel?: () => void;
}

export function createConfigColumns(
  options: ConfigColumnsOptions
): ColumnsType<ConfigItem> {
  const { onEdit, onOpenPathRulePanel } = options;

  return [
    {
      title: '作用域类型',
      dataIndex: 'scope_type',
      key: 'scope_type',
      width: 120,
      render: (type: string) => (
        <Tag color={scopeTypeColors[type] || 'default'}>
          {scopeTypeLabels[type] || type}
        </Tag>
      ),
    },
    {
      title: '作用域ID',
      dataIndex: 'scope_id',
      key: 'scope_id',
      width: 120,
      render: (id: string) => (
        <span>{scopeIdLabels[id] || id}</span>
      ),
    },
    {
      title: '配置键',
      dataIndex: 'key',
      key: 'key',
      width: 180,
      render: (key: string) => {
        const label = configKeyLabels[key] || key;
        const desc = configDescriptions[key];
        return (
          <Tooltip title={desc || '无描述'}>
            <span style={{ cursor: 'help' }}>{label}</span>
          </Tooltip>
        );
      },
    },
    {
      title: '配置值',
      dataIndex: 'value',
      key: 'value',
      width: 180,
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
      width: 160,
      fixed: 'right',
      render: (_, record) => (
        <Space size={4}>
          <Button
            type="link"
            size="small"
            icon={<EditOutlined />}
            onClick={() => onEdit(record)}
          >
            编辑
          </Button>
          {(record.key.startsWith('path_rule_') || record.key.startsWith('seed_s2_')) && onOpenPathRulePanel ? (
            <Tooltip title="打开路径规则设置面板">
              <Button
                type="link"
                size="small"
                icon={<SettingOutlined />}
                onClick={onOpenPathRulePanel}
              >
                面板
              </Button>
            </Tooltip>
          ) : null}
        </Space>
      ),
    },
  ];
}

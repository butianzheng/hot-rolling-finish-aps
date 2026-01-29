/**
 * 操作日志表格列配置
 */

import { Button, Tag } from 'antd';
import { EyeOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import type { ActionLog } from './types';
import { actionTypeLabels } from './types';

export interface ActionLogColumnsOptions {
  onViewDetail: (log: ActionLog) => void;
}

export function createActionLogColumns(
  options: ActionLogColumnsOptions
): ColumnsType<ActionLog> {
  const { onViewDetail } = options;

  return [
    {
      title: '操作时间',
      dataIndex: 'action_ts',
      key: 'action_ts',
      width: 180,
      sorter: (a, b) => a.action_ts.localeCompare(b.action_ts),
      defaultSortOrder: 'descend',
    },
    {
      title: '操作类型',
      dataIndex: 'action_type',
      key: 'action_type',
      width: 150,
      render: (type: string) => {
        const label = actionTypeLabels[type] || { text: type, color: 'default' };
        return <Tag color={label.color}>{label.text}</Tag>;
      },
    },
    {
      title: '操作人',
      dataIndex: 'actor',
      key: 'actor',
      width: 120,
    },
    {
      title: '版本ID',
      dataIndex: 'version_id',
      key: 'version_id',
      width: 120,
    },
    {
      title: '机组',
      dataIndex: 'machine_code',
      key: 'machine_code',
      width: 100,
      render: (code: string | null) => code || '-',
    },
    {
      title: '操作详情',
      dataIndex: 'detail',
      key: 'detail',
      ellipsis: true,
    },
    {
      title: '操作',
      key: 'action',
      width: 100,
      fixed: 'right',
      render: (_, record: ActionLog) => (
        <Button
          type="link"
          size="small"
          icon={<EyeOutlined />}
          onClick={() => onViewDetail(record)}
        >
          详情
        </Button>
      ),
    },
  ];
}

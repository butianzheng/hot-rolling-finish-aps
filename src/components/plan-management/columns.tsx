/**
 * PlanManagement 表格列定义
 */

import { Button, Space, Tag, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { Plan, Version } from '../comparison/types';
import { formatVersionLabelWithCode } from '../comparison/utils';

export const createPlanColumns = (
  onLoadVersions: (planId: string) => void,
  onCreateVersion: (planId: string) => void,
  onDeletePlan: (plan: Plan) => void
): ColumnsType<Plan> => [
  {
    title: '方案名称',
    dataIndex: 'plan_name',
    key: 'plan_name',
  },
  {
    title: '创建人',
    dataIndex: 'created_by',
    key: 'created_by',
  },
  {
    title: '创建时间',
    dataIndex: 'created_at',
    key: 'created_at',
  },
  {
    title: '操作',
    key: 'action',
    render: (_, record) => (
      <Space>
        <Button size="small" onClick={() => onLoadVersions(record.plan_id)}>
          查看版本
        </Button>
        <Button size="small" onClick={() => onCreateVersion(record.plan_id)}>
          创建版本
        </Button>
        <Button
          size="small"
          danger
          onClick={() => onDeletePlan(record)}
        >
          删除
        </Button>
      </Space>
    ),
  },
];

export const createVersionColumns = (
  onActivateVersion: (versionId: string) => void,
  onRecalc: (versionId: string) => void,
  onDeleteVersion: (version: Version) => void
): ColumnsType<Version> => [
  {
    title: '版本',
    key: 'version',
    render: (_: any, record) => {
      const label = formatVersionLabelWithCode(record);
      return (
        <Space size={6}>
          <Tag color={record.status === 'ACTIVE' ? 'green' : 'default'}>
            {record.status === 'ACTIVE' ? '激活' : `V${record.version_no}`}
          </Tag>
          <Typography.Text strong={record.status === 'ACTIVE'}>{label}</Typography.Text>
        </Space>
      );
    },
  },
  {
    title: '状态',
    dataIndex: 'status',
    key: 'status',
  },
  {
    title: '窗口天数',
    dataIndex: 'recalc_window_days',
    key: 'recalc_window_days',
  },
  {
    title: '创建时间',
    dataIndex: 'created_at',
    key: 'created_at',
  },
  {
    title: '操作',
    key: 'action',
    render: (_, record) => (
      <Space>
        <Button
          size="small"
          type="primary"
          disabled={record.status === 'ACTIVE'}
          onClick={() => onActivateVersion(record.version_id)}
        >
          {record.status === 'ACTIVE' ? '已激活' : '回滚/激活'}
        </Button>
        {record.status === 'ACTIVE' && (
          <Button
            size="small"
            type="default"
            onClick={() => onRecalc(record.version_id)}
          >
            一键重算
          </Button>
        )}
        {record.status !== 'ACTIVE' && (
          <Button
            size="small"
            danger
            onClick={() => onDeleteVersion(record)}
          >
            删除
          </Button>
        )}
      </Space>
    ),
  },
];

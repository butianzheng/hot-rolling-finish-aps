/**
 * 排产明细表格列配置
 */

import { Button, Space, Tag, Tooltip } from 'antd';
import { DragOutlined, HolderOutlined, LockOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { formatNumber, formatWeight } from '../../utils/formatters';
import { urgentLevelColors, sourceTypeLabels, type PlanItem } from './types';

export interface PlanItemColumnsOptions {
  machineOptions: string[];
  onViewDetail: (item: PlanItem) => void;
}

export function createPlanItemColumns(options: PlanItemColumnsOptions): ColumnsType<PlanItem> {
  const { machineOptions, onViewDetail } = options;

  return [
    {
      title: '',
      key: 'drag',
      width: 40,
      render: (_, record: PlanItem) =>
        !record.locked_in_plan ? (
          <HolderOutlined style={{ cursor: 'move', color: '#999' }} />
        ) : null,
    },
    {
      title: '序号',
      dataIndex: 'seq_no',
      key: 'seq_no',
      width: 70,
      sorter: (a, b) => a.seq_no - b.seq_no,
      render: (seq_no: number, record: PlanItem) => (
        <Space>
          {record.locked_in_plan && (
            <Tooltip title="冻结">
              <LockOutlined style={{ color: '#8c8c8c' }} />
            </Tooltip>
          )}
          <span>{seq_no}</span>
        </Space>
      ),
    },
    {
      title: '材料编号',
      dataIndex: 'material_id',
      key: 'material_id',
      width: 150,
      render: (text: string, record: PlanItem) => (
        <Button type="link" onClick={() => onViewDetail(record)}>
          {text}
        </Button>
      ),
    },
    {
      title: '钢种',
      dataIndex: 'steel_grade',
      key: 'steel_grade',
      width: 100,
    },
    {
      title: '厚度（毫米）',
      dataIndex: 'thickness_mm',
      key: 'thickness_mm',
      width: 100,
      align: 'right',
      sorter: (a, b) => Number(a.thickness_mm ?? 0) - Number(b.thickness_mm ?? 0),
      render: (value: number | null | undefined) =>
        value == null || !Number.isFinite(Number(value)) ? '-' : formatNumber(Number(value), 2, { useGrouping: false }),
    },
    {
      title: '宽度（毫米）',
      dataIndex: 'width_mm',
      key: 'width_mm',
      width: 100,
      align: 'right',
      sorter: (a, b) => Number(a.width_mm ?? 0) - Number(b.width_mm ?? 0),
      render: (value: number | null | undefined) =>
        value == null || !Number.isFinite(Number(value)) ? '-' : formatNumber(Number(value), 2, { useGrouping: false }),
    },
    {
      title: '吨位',
      dataIndex: 'weight_t',
      key: 'weight_t',
      width: 100,
      sorter: (a, b) => a.weight_t - b.weight_t,
      render: (value: number) => formatWeight(value),
    },
    {
      title: '机组',
      dataIndex: 'machine_code',
      key: 'machine_code',
      width: 100,
      filters: machineOptions.map((code) => ({ text: code, value: code })),
      onFilter: (value, record) => record.machine_code === value,
    },
    {
      title: '排产日期',
      dataIndex: 'plan_date',
      key: 'plan_date',
      width: 120,
      sorter: (a, b) => a.plan_date.localeCompare(b.plan_date),
    },
    {
      title: '紧急等级',
      dataIndex: 'urgent_level',
      key: 'urgent_level',
      width: 100,
      render: (level: string) => (
        <Tag color={urgentLevelColors[level] || 'default'}>{level}</Tag>
      ),
      filters: [
        { text: '三级-超紧急', value: 'L3' },
        { text: '二级-紧急', value: 'L2' },
        { text: '一级-较紧急', value: 'L1' },
        { text: '常规-正常', value: 'L0' },
      ],
      onFilter: (value, record) => record.urgent_level === value,
    },
    {
      title: '来源',
      dataIndex: 'source_type',
      key: 'source_type',
      width: 100,
      render: (type: string) => {
        const label = sourceTypeLabels[type] || { text: type, color: 'default' };
        return <Tag color={label.color}>{label.text}</Tag>;
      },
    },
    {
      title: '状态',
      key: 'status',
      width: 120,
      render: (_, record: PlanItem) => (
        <Space size={4}>
          {record.locked_in_plan && <Tag color="purple">冻结</Tag>}
          {record.force_release_in_plan && <Tag color="orange">强制放行</Tag>}
          {!record.locked_in_plan && !record.force_release_in_plan && (
            <Tag color="green">正常</Tag>
          )}
        </Space>
      ),
    },
    {
      title: '操作',
      key: 'action',
      width: 150,
      fixed: 'right',
      render: (_, record: PlanItem) => (
        <Space size="small">
          <Button type="link" size="small" onClick={() => onViewDetail(record)}>
            详情
          </Button>
          {!record.locked_in_plan && (
            <Tooltip title="拖拽调整顺序">
              <Button type="link" size="small" icon={<DragOutlined />} />
            </Tooltip>
          )}
        </Space>
      ),
    },
  ];
}

/**
 * 材料表格列配置
 */

import type { ProColumns } from '@ant-design/pro-components';
import { Button, Dropdown, Space, Tag, Tooltip } from 'antd';
import type { MenuProps } from 'antd';
import {
  LockOutlined,
  UnlockOutlined,
  FireOutlined,
  StopOutlined,
  MoreOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import { UrgencyTag } from '../UrgencyTag';
import { MaterialStatusIcons } from '../MaterialStatusIcons';
import { FrozenZoneBadge } from '../guards/FrozenZoneBadge';
import { FONT_FAMILIES } from '../../theme';
import { normalizeSchedState } from '../../utils/schedState';
import type { Material, OperationType } from './materialTypes';

const SCHED_STATE_CONFIG: Record<string, { color: string; text: string; tooltip: string }> = {
  READY: {
    color: '#52c41a',
    text: '就绪',
    tooltip: '就绪状态 - 材料已适温,可以进入产能池参与排产',
  },
  PENDING_MATURE: {
    color: '#8c8c8c',
    text: '未成熟',
    tooltip: '未成熟/冷料 - 材料尚未达到适温要求,不可排产',
  },
  FORCE_RELEASE: {
    color: '#1677ff',
    text: '强制放行',
    tooltip: '强制放行 - 绕过适温限制,允许参与排产',
  },
  SCHEDULED: {
    color: '#1677ff',
    text: '已排产',
    tooltip: '已排产 - 材料已分配到具体日期和机组,等待执行',
  },
  LOCKED: {
    color: '#faad14',
    text: '已锁定',
    tooltip: '已锁定 - 材料被人工锁定,不可自动调整位置',
  },
  BLOCKED: {
    color: '#ff4d4f',
    text: '阻断',
    tooltip: '阻断 - 数据质量问题导致材料不可排产,需要先修复数据',
  },
  UNKNOWN: {
    color: '#8c8c8c',
    text: '未知',
    tooltip: '未知状态 - 材料状态缺失或未被正确计算',
  },
};

export interface MaterialTableColumnsOptions {
  machineOptions: Array<{ label: string; value: string }>;
  loadMachineOptions: () => Promise<Array<{ label: string; value: string }>>;
  onViewDetail: (record: Material) => void;
  onOperation: (record: Material, type: OperationType) => void;
}

export function createMaterialTableColumns(options: MaterialTableColumnsOptions): ProColumns<Material>[] {
  const { machineOptions, loadMachineOptions, onViewDetail, onOperation } = options;

  return [
    {
      title: '状态',
      dataIndex: 'status_icons',
      key: 'status_icons',
      width: 120,
      align: 'center',
      search: false,
      render: (_, record) => (
        <Space direction="vertical" size={4} align="center">
          <MaterialStatusIcons
            lockFlag={record.lock_flag}
            schedState={record.sched_state}
            tempIssue={record.temp_issue || record.is_mature === false}
          />
          <FrozenZoneBadge locked={record.is_frozen || false} />
        </Space>
      ),
    },
    {
      title: '材料号',
      dataIndex: 'material_id',
      key: 'material_id',
      width: 160,
      copyable: true,
      ellipsis: true,
      render: (text) => (
        <span style={{ fontFamily: FONT_FAMILIES.MONOSPACE, fontSize: 13 }}>{text}</span>
      ),
    },
    {
      title: '机组',
      dataIndex: 'machine_code',
      key: 'machine_code',
      width: 100,
      valueType: 'select',
      request: async () => {
        if (machineOptions.length > 0) return machineOptions;
        try {
          return await loadMachineOptions();
        } catch {
          return [];
        }
      },
    },
    {
      title: '重量(吨)',
      dataIndex: 'weight_t',
      key: 'weight_t',
      width: 110,
      align: 'right',
      search: false,
      render: (val: any) => (
        <span style={{ fontFamily: FONT_FAMILIES.MONOSPACE }}>{val ? val.toFixed(2) : '-'}</span>
      ),
    },
    {
      title: '钢种',
      dataIndex: 'steel_mark',
      key: 'steel_mark',
      width: 120,
      ellipsis: true,
    },
    {
      title: '排产状态',
      dataIndex: 'sched_state',
      key: 'sched_state',
      width: 120,
      valueType: 'select',
      valueEnum: {
        READY: { text: '就绪', status: 'Success' },
        PENDING_MATURE: { text: '未成熟', status: 'Default' },
        FORCE_RELEASE: { text: '强制放行', status: 'Processing' },
        LOCKED: { text: '已锁定', status: 'Warning' },
        SCHEDULED: { text: '已排产', status: 'Processing' },
        BLOCKED: { text: '阻断', status: 'Error' },
        UNKNOWN: { text: '未知', status: 'Default' },
      },
      render: (_, record) => {
        const state = normalizeSchedState(record.sched_state);
        const config = SCHED_STATE_CONFIG[state] || SCHED_STATE_CONFIG.UNKNOWN;
        return (
          <Tooltip title={config.tooltip}>
            <Tag color={config.color} style={{ cursor: 'help' }}>
              {config.text}
            </Tag>
          </Tooltip>
        );
      },
    },
    {
      title: '紧急等级',
      dataIndex: 'urgent_level',
      key: 'urgent_level',
      width: 100,
      align: 'center',
      valueType: 'select',
      valueEnum: {
        L3: { text: 'L3', status: 'Error' },
        L2: { text: 'L2', status: 'Warning' },
        L1: { text: 'L1', status: 'Processing' },
        L0: { text: 'L0', status: 'Default' },
      },
      render: (_, record) => <UrgencyTag level={record.urgent_level} />,
    },
    {
      title: '人工紧急',
      dataIndex: 'manual_urgent_flag',
      key: 'manual_urgent_flag',
      width: 100,
      align: 'center',
      valueType: 'select',
      valueEnum: {
        true: { text: '是', status: 'Error' },
        false: { text: '否', status: 'Default' },
      },
      render: (_, record) =>
        record.manual_urgent_flag ? (
          <Tooltip title="人工红线 - 由调度员手动标记为紧急,优先级最高 (L3)">
            <span style={{ color: '#ff4d4f', fontWeight: 'bold', cursor: 'help' }}>是</span>
          </Tooltip>
        ) : (
          <Tooltip title="未标记人工紧急 - 紧急等级由系统引擎自动计算">
            <span style={{ color: '#8c8c8c', cursor: 'help' }}>否</span>
          </Tooltip>
        ),
    },
    {
      title: '锁定状态',
      dataIndex: 'lock_flag',
      key: 'lock_flag',
      width: 100,
      align: 'center',
      valueType: 'select',
      valueEnum: {
        true: { text: '已锁定', status: 'Warning' },
        false: { text: '未锁定', status: 'Default' },
      },
      render: (_, record) =>
        record.lock_flag ? (
          <Tooltip title="已锁定 - 材料位置被锁定,系统不会自动调整其排产顺序">
            <span style={{ color: '#faad14', fontWeight: 'bold', cursor: 'help' }}>已锁定</span>
          </Tooltip>
        ) : (
          <Tooltip title="未锁定 - 材料可以被系统自动调整排产顺序">
            <span style={{ color: '#8c8c8c', cursor: 'help' }}>未锁定</span>
          </Tooltip>
        ),
    },
    {
      title: '操作',
      key: 'action',
      width: 80,
      align: 'center',
      search: false,
      fixed: 'right',
      render: (_, record) => {
        const menuItems: MenuProps['items'] = [
          {
            key: 'view',
            label: '查看详情',
            onClick: () => onViewDetail(record),
          },
          {
            type: 'divider',
          },
          {
            key: 'lock',
            label: record.lock_flag ? '解锁' : '锁定',
            icon: record.lock_flag ? <UnlockOutlined /> : <LockOutlined />,
            onClick: () => onOperation(record, record.lock_flag ? 'unlock' : 'lock'),
          },
          {
            key: 'urgent',
            label: record.manual_urgent_flag ? '取消紧急' : '设为紧急',
            icon: record.manual_urgent_flag ? <StopOutlined /> : <FireOutlined />,
            danger: !record.manual_urgent_flag,
            onClick: () => onOperation(record, record.manual_urgent_flag ? 'clearUrgent' : 'urgent'),
          },
          {
            key: 'forceRelease',
            label: '强制放行',
            icon: <WarningOutlined />,
            danger: true,
            onClick: () => onOperation(record, 'forceRelease'),
          },
        ];

        return (
          <Dropdown menu={{ items: menuItems }} trigger={['click']}>
            <Button type="text" size="small" icon={<MoreOutlined />} />
          </Dropdown>
        );
      },
    },
  ];
}

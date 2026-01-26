// ==========================================
// 材料管理 - Scheduler Workbench
// ==========================================
// 使用 ProTable 的专业调度工作台
// ==========================================

import React, { useCallback, useEffect, useRef, useState } from 'react';
import { ProTable } from '@ant-design/pro-components';
import type { ProColumns, ActionType } from '@ant-design/pro-components';
import {
  Alert,
  Button,
  Collapse,
  DatePicker,
  Dropdown,
  Empty,
  Select,
  Space,
  Spin,
  Tag,
  Tooltip,
  message,
  Modal,
  Input,
} from 'antd';
import type { MenuProps } from 'antd';
import {
  ReloadOutlined,
  LockOutlined,
  UnlockOutlined,
  FireOutlined,
  MoreOutlined,
  WarningOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';
import dayjs, { type Dayjs } from 'dayjs';
import { capacityApi, materialApi, planApi } from '../api/tauri';
import { useEvent } from '../api/eventBus';
import { UrgencyTag } from './UrgencyTag';
import { MaterialStatusIcons } from './MaterialStatusIcons';
import { MaterialInspector } from './MaterialInspector';
import { CapacityTimeline } from './CapacityTimeline';
import { FrozenZoneBadge } from './guards/FrozenZoneBadge';
import { RedLineGuard, createFrozenZoneViolation, createMaturityViolation } from './guards/RedLineGuard';
import type { RedLineViolation } from './guards/RedLineGuard';
import { FONT_FAMILIES } from '../theme';
import { useActiveVersionId, useCurrentUser, useAdminOverrideMode } from '../stores/use-global-store';
import { formatDate } from '../utils/formatters';
import type { CapacityTimelineData } from '../types/capacity';

const { TextArea } = Input;

interface Material {
  material_id: string;
  machine_code: string;
  weight_t: number;
  steel_mark: string;
  sched_state: string;
  urgent_level: string;
  lock_flag: boolean;
  manual_urgent_flag: boolean;
  is_frozen?: boolean; // 是否在冻结区
  is_mature?: boolean; // 是否适温
  temp_issue?: boolean; // 是否有温度问题
  urgent_reason?: string; // 紧急等级判定原因
  eligibility_reason?: string; // 适温判定原因
  priority_reason?: string; // 优先级排序原因
}

const MaterialManagement: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const [selectedRowKeys, setSelectedRowKeys] = useState<React.Key[]>([]);
  const [selectedMaterial, setSelectedMaterial] = useState<Material | null>(null);
  const [inspectorVisible, setInspectorVisible] = useState(false);
  const [modalVisible, setModalVisible] = useState(false);
  const [modalType, setModalType] = useState<'lock' | 'unlock' | 'urgent'>('lock');
  const [reason, setReason] = useState('');
  const currentUser = useCurrentUser();
  const adminOverrideMode = useAdminOverrideMode();
  const activeVersionId = useActiveVersionId();

  // 产能时间线（来自实际排产版本+产能池）
  const [machineOptions, setMachineOptions] = useState<Array<{ label: string; value: string }>>([]);
  const [timelineMachine, setTimelineMachine] = useState<string | undefined>(undefined);
  const [timelineDate, setTimelineDate] = useState(() => dayjs());
  const [timelineData, setTimelineData] = useState<CapacityTimelineData | null>(null);
  const [timelineLoading, setTimelineLoading] = useState(false);
  const [timelineError, setTimelineError] = useState<string | null>(null);

  const loadMachineOptions = useCallback(async () => {
    const result = await materialApi.listMaterials({ limit: 0, offset: 0 });
    const codes = new Set<string>();
    (Array.isArray(result) ? result : []).forEach((m: any) => {
      const code = String(m?.machine_code ?? '').trim();
      if (code) codes.add(code);
    });
    const options = Array.from(codes)
      .sort()
      .map((code) => ({ label: code, value: code }));
    setMachineOptions(options);
    return options;
  }, []);

  const loadTimeline = useCallback(
    async (opts?: { machineCode?: string; date?: Dayjs }) => {
      const machineCode = opts?.machineCode ?? timelineMachine;
      const date = opts?.date ?? timelineDate;

      if (!machineCode) return;
      if (!activeVersionId) {
        setTimelineData(null);
        setTimelineError(null);
        return;
      }

      const dateStr = formatDate(date);
      setTimelineLoading(true);
      setTimelineError(null);
      try {
        const [capacityPools, itemsByDate] = await Promise.all([
          capacityApi.getCapacityPools([machineCode], dateStr, dateStr, activeVersionId),
          planApi.listItemsByDate(activeVersionId, dateStr),
        ]);

        const pools = Array.isArray(capacityPools) ? capacityPools : [];
        const pool = pools.find(
          (p: any) => String(p?.machine_code ?? '') === machineCode && String(p?.plan_date ?? '') === dateStr
        );

        const planItems = (Array.isArray(itemsByDate) ? itemsByDate : []).filter(
          (it: any) => String(it?.machine_code ?? '') === machineCode
        );

        const buckets: Record<'L0' | 'L1' | 'L2' | 'L3', { tonnage: number; count: number }> = {
          L0: { tonnage: 0, count: 0 },
          L1: { tonnage: 0, count: 0 },
          L2: { tonnage: 0, count: 0 },
          L3: { tonnage: 0, count: 0 },
        };

        planItems.forEach((it: any) => {
          const raw = String(it?.urgent_level ?? 'L0').toUpperCase();
          const level = (['L0', 'L1', 'L2', 'L3'].includes(raw) ? raw : 'L0') as
            | 'L0'
            | 'L1'
            | 'L2'
            | 'L3';
          const weight = Number(it?.weight_t ?? 0);
          if (!Number.isFinite(weight) || weight <= 0) return;
          buckets[level].tonnage += weight;
          buckets[level].count += 1;
        });

        const segmentTotal = (Object.keys(buckets) as Array<keyof typeof buckets>).reduce(
          (sum, k) => sum + buckets[k].tonnage,
          0
        );

        const poolUsed = Number(pool?.used_capacity_t ?? 0);
        const actualCapacity =
          Number.isFinite(segmentTotal) && segmentTotal > 0
            ? segmentTotal
            : Number.isFinite(poolUsed) && poolUsed > 0
            ? poolUsed
            : 0;

        if (segmentTotal <= 0 && actualCapacity > 0) {
          buckets.L0.tonnage = actualCapacity;
        }

        const target = Number(pool?.target_capacity_t ?? 0);
        const limit = Number(pool?.limit_capacity_t ?? 0);

        const targetCapacity =
          Number.isFinite(target) && target > 0 ? target : Math.max(actualCapacity, 1);
        const limitCapacity =
          Number.isFinite(limit) && limit > 0 ? limit : targetCapacity;

        const accumulated = Number(pool?.accumulated_tonnage_t ?? 0);

        setTimelineData({
          date: dateStr,
          machineCode,
          targetCapacity,
          limitCapacity,
          actualCapacity,
          segments: (['L3', 'L2', 'L1', 'L0'] as const).map((level) => ({
            urgencyLevel: level,
            tonnage: buckets[level].tonnage,
            materialCount: buckets[level].count,
          })),
          rollCampaignProgress: Number.isFinite(accumulated) ? accumulated : 0,
          rollChangeThreshold: 2500,
        });
      } catch (error: any) {
        setTimelineError(error?.message || String(error) || '加载失败');
        setTimelineData(null);
      } finally {
        setTimelineLoading(false);
      }
    },
    [activeVersionId, timelineDate, timelineMachine]
  );

  // 预加载机组列表（供筛选 + 时间线使用）
  useEffect(() => {
    loadMachineOptions()
      .then((options) => {
        setTimelineMachine((prev) => prev || options[0]?.value);
      })
      .catch((e) => {
        console.error('加载机组列表失败:', e);
      });
  }, [loadMachineOptions]);

  // 激活版本 / 选择变化时刷新时间线
  useEffect(() => {
    if (!activeVersionId || !timelineMachine) return;
    loadTimeline();
  }, [activeVersionId, timelineMachine, timelineDate, loadTimeline]);

  // 订阅事件：材料状态变化刷新列表，排产更新刷新时间线
  useEvent('material_state_changed', () => actionRef.current?.reload());
  useEvent('plan_updated', () => loadTimeline());

  // ==========================================
  // Red Line 检查函数
  // ==========================================

  // 检查是否违反冻结区保护
  const checkFrozenViolation = (material: Material, operation: string): RedLineViolation | null => {
    if (adminOverrideMode) return null; // 管理员覆盖模式下跳过检查

    if (material.is_frozen && (operation === 'lock' || operation === 'unlock' || operation === 'urgent')) {
      return createFrozenZoneViolation(
        [material.material_id],
        '该材料位于冻结区，不允许修改状态'
      );
    }
    return null;
  };

  // 检查是否违反温度约束
  const checkTempViolation = (material: Material, operation: string): RedLineViolation | null => {
    if (adminOverrideMode) return null; // 管理员覆盖模式下跳过检查

    if (!material.is_mature && operation === 'urgent') {
      // 假设材料距离适温还需要一定天数（这里简化为1天，实际应从材料数据读取）
      return createMaturityViolation([material.material_id], 1);
    }
    return null;
  };

  // 综合检查 Red Line 违规
  const checkRedLineViolations = (material: Material, operation: string): RedLineViolation[] => {
    const violations: RedLineViolation[] = [];

    const frozenViolation = checkFrozenViolation(material, operation);
    if (frozenViolation) violations.push(frozenViolation);

    const tempViolation = checkTempViolation(material, operation);
    if (tempViolation) violations.push(tempViolation);

    return violations;
  };

  // 定义列配置
  const columns: ProColumns<Material>[] = [
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
            tempIssue={record.temp_issue || !record.is_mature}
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
        <span style={{ fontFamily: FONT_FAMILIES.MONOSPACE, fontSize: 13 }}>
          {text}
        </span>
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
        <span style={{ fontFamily: FONT_FAMILIES.MONOSPACE }}>
          {val ? val.toFixed(2) : '-'}
        </span>
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
        Ready: { text: '就绪', status: 'Success' },
        Scheduled: { text: '已排产', status: 'Processing' },
        Locked: { text: '已锁定', status: 'Warning' },
        Frozen: { text: '冻结', status: 'Default' },
      },
      render: (_, record) => {
        const stateConfig: Record<string, { color: string; text: string; tooltip: string }> = {
          Ready: {
            color: '#52c41a',
            text: '就绪',
            tooltip: '就绪状态 - 材料已适温,可以进入产能池参与排产',
          },
          Scheduled: {
            color: '#1677ff',
            text: '已排产',
            tooltip: '已排产 - 材料已分配到具体日期和机组,等待执行',
          },
          Locked: {
            color: '#faad14',
            text: '已锁定',
            tooltip: '已锁定 - 材料被人工锁定,不可自动调整位置',
          },
          Frozen: {
            color: '#8c8c8c',
            text: '冻结',
            tooltip: '冻结区 - 材料位于冻结区,受 Red Line 保护,不可修改',
          },
        };
        const config = stateConfig[record.sched_state] || stateConfig.Ready;
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
            onClick: () => handleViewDetail(record),
          },
          {
            type: 'divider',
          },
          {
            key: 'lock',
            label: record.lock_flag ? '解锁' : '锁定',
            icon: record.lock_flag ? <UnlockOutlined /> : <LockOutlined />,
            onClick: () => handleSingleOperation(record, record.lock_flag ? 'unlock' : 'lock'),
          },
          {
            key: 'urgent',
            label: '设为紧急',
            icon: <FireOutlined />,
            danger: true,
            onClick: () => handleSingleOperation(record, 'urgent'),
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

  // 加载数据
  const loadMaterials = async (params: any) => {
    try {
      const result = await materialApi.listMaterials({
        machine_code: params.machine_code,
        limit: 0,
        offset: 0,
      });

      // 应用筛选
      let filtered = result;

      if (params.sched_state) {
        filtered = filtered.filter((m: Material) => m.sched_state === params.sched_state);
      }
      if (params.urgent_level) {
        filtered = filtered.filter((m: Material) => m.urgent_level === params.urgent_level);
      }
      if (params.manual_urgent_flag !== undefined) {
        const flag = params.manual_urgent_flag === 'true';
        filtered = filtered.filter((m: Material) => m.manual_urgent_flag === flag);
      }
      if (params.lock_flag !== undefined) {
        const flag = params.lock_flag === 'true';
        filtered = filtered.filter((m: Material) => m.lock_flag === flag);
      }
      if (params.material_id) {
        filtered = filtered.filter((m: Material) =>
          m.material_id.toLowerCase().includes(params.material_id.toLowerCase())
        );
      }
      if (params.steel_mark) {
        filtered = filtered.filter((m: Material) =>
          m.steel_mark.toLowerCase().includes(params.steel_mark.toLowerCase())
        );
      }

      return {
        data: filtered,
        success: true,
        total: filtered.length,
      };
    } catch (error: any) {
      message.error(`加载失败: ${error.message || error}`);
      return {
        data: [],
        success: false,
        total: 0,
      };
    }
  };

  // 查看详情
  const handleViewDetail = (record: Material) => {
    setSelectedMaterial(record);
    setInspectorVisible(true);
  };

  // 单个材料操作
  const handleSingleOperation = (record: Material, type: 'lock' | 'unlock' | 'urgent') => {
    // 检查 Red Line 违规
    const violations = checkRedLineViolations(record, type);
    if (violations.length > 0) {
      Modal.error({
        title: '工业红线保护',
        width: 600,
        content: (
          <Space direction="vertical" style={{ width: '100%' }} size={16}>
            <RedLineGuard violations={violations} mode="detailed" />
            {!adminOverrideMode && (
              <div
                style={{
                  padding: '12px',
                  background: '#fff7e6',
                  border: '1px solid #ffd591',
                  borderRadius: '4px',
                }}
              >
                <Space>
                  <InfoCircleOutlined style={{ color: '#faad14' }} />
                  <div>
                    <div style={{ fontWeight: 'bold', color: '#faad14' }}>提示</div>
                    <div style={{ fontSize: '12px', color: '#8c8c8c', marginTop: '4px' }}>
                      如需覆盖此保护，请启用"管理员覆盖模式"。
                    </div>
                  </div>
                </Space>
              </div>
            )}
          </Space>
        ),
      });
      return;
    }

    setSelectedRowKeys([record.material_id]);
    setModalType(type);
    setModalVisible(true);
  };

  // 批量操作
  const handleBatchOperation = async (type: 'lock' | 'unlock' | 'urgent') => {
    if (selectedRowKeys.length === 0) {
      message.warning('请先选择材料');
      return;
    }

    // 批量检查 Red Line 违规
    if (!adminOverrideMode) {
      const result = await materialApi.listMaterials({ limit: 0, offset: 0 });
      const selectedMaterials = result.filter((m: Material) =>
        selectedRowKeys.includes(m.material_id)
      );

      const allViolations: RedLineViolation[] = [];
      selectedMaterials.forEach((material: Material) => {
        const violations = checkRedLineViolations(material, type);
        allViolations.push(...violations);
      });

      if (allViolations.length > 0) {
        Modal.error({
          title: '工业红线保护',
          width: 700,
          content: (
            <Space direction="vertical" style={{ width: '100%' }} size={16}>
              <div>
                <div style={{ fontWeight: 'bold', marginBottom: '8px' }}>
                  以下材料违反工业红线保护规则:
                </div>
                <div style={{ maxHeight: '400px', overflowY: 'auto' }}>
                  <RedLineGuard violations={allViolations} mode="detailed" />
                </div>
              </div>
              <div
                style={{
                  padding: '12px',
                  background: '#fff7e6',
                  border: '1px solid #ffd591',
                  borderRadius: '4px',
                }}
              >
                <Space>
                  <InfoCircleOutlined style={{ color: '#faad14' }} />
                  <div>
                    <div style={{ fontWeight: 'bold', color: '#faad14' }}>提示</div>
                    <div style={{ fontSize: '12px', color: '#8c8c8c', marginTop: '4px' }}>
                      如需覆盖此保护，请启用"管理员覆盖模式"。
                    </div>
                  </div>
                </Space>
              </div>
            </Space>
          ),
        });
        return;
      }
    }

    setModalType(type);
    setModalVisible(true);
  };

  // 执行操作
  const executeOperation = async () => {
    if (!reason.trim()) {
      message.warning('请输入操作原因');
      return;
    }

    try {
      const materialIds = selectedRowKeys as string[];
      const operator = currentUser || 'admin';

      if (modalType === 'lock') {
        await materialApi.batchLockMaterials(materialIds, true, operator, reason);
        message.success('锁定成功');
      } else if (modalType === 'unlock') {
        await materialApi.batchLockMaterials(materialIds, false, operator, reason);
        message.success('解锁成功');
      } else if (modalType === 'urgent') {
        await materialApi.batchSetUrgent(materialIds, true, operator, reason);
        message.success('设置紧急标志成功');
      }

      setModalVisible(false);
      setReason('');
      setSelectedRowKeys([]);
      actionRef.current?.reload();
    } catch (error: any) {
      message.error(`操作失败: ${error.message || error}`);
    }
  };

  // 模态框标题
  const getModalTitle = () => {
    const count = selectedRowKeys.length;
    switch (modalType) {
      case 'lock':
        return `锁定材料 (${count} 件)`;
      case 'unlock':
        return `解锁材料 (${count} 件)`;
      case 'urgent':
        return `设置紧急标志 (${count} 件)`;
      default:
        return '操作';
    }
  };

  return (
    <>
      {/* 产能时间线 */}
      <Collapse
        defaultActiveKey={['capacity']}
        style={{ marginBottom: 16 }}
        items={[
          {
            key: 'capacity',
            label: '产能时间线',
            children: (
              <div>
                <Space style={{ marginBottom: 12 }} size={12} wrap>
                  <span>机组</span>
                  <Select
                    value={timelineMachine}
                    style={{ width: 160 }}
                    placeholder="请选择机组"
                    options={machineOptions}
                    showSearch
                    optionFilterProp="label"
                    onChange={(value) => setTimelineMachine(value)}
                  />
                  <span>日期</span>
                  <DatePicker
                    value={timelineDate}
                    onChange={(d) => d && setTimelineDate(d)}
                    format="YYYY-MM-DD"
                    allowClear={false}
                  />
                  <Button
                    icon={<ReloadOutlined />}
                    onClick={() => loadTimeline()}
                    loading={timelineLoading}
                  >
                    刷新
                  </Button>
                </Space>

                {!activeVersionId ? (
                  <Alert
                    message="产能时间线需要激活排产版本"
                    description="请先在“排产方案”中激活一个版本后再查看排产产能分布。"
                    type="warning"
                    showIcon
                  />
                ) : timelineError ? (
                  <Alert
                    message="产能时间线加载失败"
                    description={timelineError}
                    type="error"
                    showIcon
                  />
                ) : (
                  <Spin spinning={timelineLoading}>
                    <div style={{ minHeight: 80 }}>
                      {timelineData ? (
                        <CapacityTimeline data={timelineData} />
                      ) : (
                        <Empty description="暂无产能时间线数据" />
                      )}
                    </div>
                  </Spin>
                )}
              </div>
            ),
          },
        ]}
      />

      {/* 材料表格 */}
      <ProTable<Material>
        columns={columns}
        actionRef={actionRef}
        request={loadMaterials}
        rowKey="material_id"
        search={{
          labelWidth: 'auto',
          defaultCollapsed: false,
          optionRender: (_searchConfig, formProps, dom) => [
            ...dom,
            <Button
              key="reset"
              onClick={() => {
                formProps.form?.resetFields();
                formProps.form?.submit();
              }}
            >
              重置
            </Button>,
          ],
        }}
        pagination={{
          defaultPageSize: 50,
          showSizeChanger: true,
          showQuickJumper: true,
          showTotal: (total) => `共 ${total} 条`,
        }}
        rowSelection={{
          selectedRowKeys,
          onChange: setSelectedRowKeys,
          preserveSelectedRowKeys: true,
        }}
        tableAlertRender={({ selectedRowKeys }) => (
          <Space size={16}>
            <span>已选择 {selectedRowKeys.length} 项</span>
          </Space>
        )}
        tableAlertOptionRender={() => (
          <Space size={8}>
            <Button
              size="small"
              icon={<LockOutlined />}
              onClick={() => handleBatchOperation('lock')}
            >
              批量锁定
            </Button>
            <Button
              size="small"
              icon={<UnlockOutlined />}
              onClick={() => handleBatchOperation('unlock')}
            >
              批量解锁
            </Button>
            <Button
              size="small"
              type="primary"
              danger
              icon={<FireOutlined />}
              onClick={() => handleBatchOperation('urgent')}
            >
              批量设为紧急
            </Button>
            <Button size="small" onClick={() => setSelectedRowKeys([])}>
              取消选择
            </Button>
          </Space>
        )}
        toolbar={{
          actions: [
            <Button
              key="reload"
              icon={<ReloadOutlined />}
              onClick={() => actionRef.current?.reload()}
            >
              刷新
            </Button>,
          ],
        }}
        scroll={{ x: 1200 }}
        options={{
          density: true,
          fullScreen: false,
          reload: true,
          setting: true,
        }}
        onRow={(record) => ({
          onClick: () => handleViewDetail(record),
          style: {
            cursor: 'pointer',
            opacity: record.lock_flag ? 0.7 : 1,
          },
        })}
      />

      {/* Inspector 侧边栏 */}
      <MaterialInspector
        visible={inspectorVisible}
        material={selectedMaterial}
        onClose={() => setInspectorVisible(false)}
        onLock={() => handleSingleOperation(selectedMaterial!, 'lock')}
        onUnlock={() => handleSingleOperation(selectedMaterial!, 'unlock')}
        onSetUrgent={() => handleSingleOperation(selectedMaterial!, 'urgent')}
      />

      {/* 操作确认模态框 */}
      <Modal
        title={getModalTitle()}
        open={modalVisible}
        onOk={executeOperation}
        onCancel={() => {
          setModalVisible(false);
          setReason('');
        }}
        okText="确认"
        cancelText="取消"
        okButtonProps={{
          danger: adminOverrideMode,
        }}
      >
        <Space direction="vertical" style={{ width: '100%' }} size={16}>
          {/* 管理员覆盖模式警告 */}
          {adminOverrideMode && (
            <div
              style={{
                padding: 12,
                backgroundColor: '#fff2e8',
                border: '1px solid #ffbb96',
                borderRadius: 4,
              }}
            >
              <Space>
                <WarningOutlined style={{ color: '#ff4d4f', fontSize: 16 }} />
                <div>
                  <div style={{ fontWeight: 'bold', color: '#ff4d4f' }}>
                    管理员覆盖模式已启用
                  </div>
                  <div style={{ fontSize: 12, color: '#8c8c8c', marginTop: 4 }}>
                    此操作将绕过 Red Line 保护规则。请确保您了解操作的影响。
                  </div>
                </div>
              </Space>
            </div>
          )}

          <div>
            <div style={{ marginBottom: 8, fontWeight: 'bold' }}>操作原因:</div>
            <TextArea
              rows={4}
              placeholder="请输入操作原因（必填）"
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              maxLength={500}
              showCount
            />
          </div>
        </Space>
      </Modal>
    </>
  );
};

export default MaterialManagement;

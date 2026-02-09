import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Card, DatePicker, Form, Input, InputNumber, Modal, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs, { type Dayjs } from 'dayjs';
import { ReloadOutlined, SettingOutlined } from '@ant-design/icons';
import { configApi, rollApi } from '../../api/tauri';
import { useAllRollCampaignAlerts } from '../../hooks/queries/use-decision-queries';
import { useActiveVersionId, useCurrentUser } from '../../stores/use-global-store';
import type { RollCampaignAlert } from '../../types/decision';
import { calculateUtilization, getAlertLevelColor, getAlertLevelLabel } from '../../types/decision';
import { formatDateTime } from '../../utils/formatters';

type RollCampaignPlanRow = {
  machineCode: string;
  initialStartAt: string;
  nextChangeAt: string | null;
  downtimeMinutes: number | null;
  updatedAt: string;
  updatedBy: string | null;
};

type TableRow = {
  machineCode: string;
  alert: RollCampaignAlert | null;
  plan: RollCampaignPlanRow | null;
};

function parseToDayjs(value?: string | null): Dayjs | null {
  if (!value) return null;
  const d = dayjs(value);
  return d.isValid() ? d : null;
}

function isExpiredDateTime(value?: string | null): boolean {
  const d = parseToDayjs(value);
  return !!(d && d.isValid() && d.isBefore(dayjs()));
}

const SAMPLE_CONFIGS = [
  { key: 'roll_suggest_threshold_t', value: '2000' },
  { key: 'roll_hard_limit_t', value: '2300' },
  { key: 'roll_change_downtime_minutes', value: '45' },
] as const;

const RollCampaignManagementPanel: React.FC = () => {
  const versionId = useActiveVersionId();
  const currentUser = useCurrentUser() || 'admin';
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [plans, setPlans] = useState<RollCampaignPlanRow[]>([]);

  const [editOpen, setEditOpen] = useState(false);
  const [editingMachineCode, setEditingMachineCode] = useState<string | null>(null);
  const [form] = Form.useForm();

  const rollAlertsQuery = useAllRollCampaignAlerts(versionId);
  const rollAlerts = rollAlertsQuery.data?.items ?? [];

  const plansByMachine = useMemo(() => {
    const map: Record<string, RollCampaignPlanRow> = {};
    plans.forEach((p) => {
      map[p.machineCode] = p;
    });
    return map;
  }, [plans]);

  const rows = useMemo<TableRow[]>(() => {
    const machines = new Set<string>();
    rollAlerts.forEach((a) => machines.add(a.machineCode));
    Object.keys(plansByMachine).forEach((m) => machines.add(m));
    const machineList = Array.from(machines).sort();
    return machineList.map((machineCode) => ({
      machineCode,
      alert: rollAlerts.find((a) => a.machineCode === machineCode) || null,
      plan: plansByMachine[machineCode] || null,
    }));
  }, [plansByMachine, rollAlerts]);

  const expiredPlanMachineCodes = useMemo(
    () => rows.filter((row) => isExpiredDateTime(row.plan?.nextChangeAt)).map((row) => row.machineCode),
    [rows]
  );

  const loadPlans = async () => {
    if (!versionId) return;
    setLoading(true);
    setLoadError(null);
    try {
      const raw = await rollApi.listRollCampaignPlans(versionId);
      const next: RollCampaignPlanRow[] = Array.isArray(raw)
        ? raw
            .map((p: any) => ({
              machineCode: String(p?.machine_code ?? ''),
              initialStartAt: String(p?.initial_start_at ?? ''),
              nextChangeAt: p?.next_change_at != null ? String(p.next_change_at) : null,
              downtimeMinutes: p?.downtime_minutes != null ? Number(p.downtime_minutes) : null,
              updatedAt: String(p?.updated_at ?? ''),
              updatedBy: p?.updated_by != null ? String(p.updated_by) : null,
            }))
            .filter((p) => p.machineCode && p.initialStartAt)
        : [];
      setPlans(next);
    } catch (e: any) {
      const msg = String(e?.message || e || '加载失败');
      setLoadError(msg);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    setPlans([]);
    setLoadError(null);
    if (!versionId) return;
    void loadPlans();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [versionId]);

  const openEdit = (row: TableRow) => {
    const machineCode = row.machineCode;
    setEditingMachineCode(machineCode);

    const initialStartAt = row.plan?.initialStartAt || row.alert?.campaignStartAt || '';
    const nextChangeAt = row.plan?.nextChangeAt ?? row.alert?.plannedChangeAt ?? null;
    const downtimeMinutes = row.plan?.downtimeMinutes ?? row.alert?.plannedDowntimeMinutes ?? null;

    form.resetFields();
    form.setFieldsValue({
      initialStartAt: parseToDayjs(initialStartAt),
      nextChangeAt: parseToDayjs(nextChangeAt),
      downtimeMinutes: downtimeMinutes ?? undefined,
      reason: '',
    });
    setEditOpen(true);
  };

  const handleSave = async () => {
    if (!versionId || !editingMachineCode) return;
    const values = await form.validateFields();

    const reason = String(values?.reason || '').trim();
    if (!reason) {
      message.warning('请输入修改原因');
      return;
    }

    const initialStartAt: Dayjs | null = values?.initialStartAt ?? null;
    if (!initialStartAt || !initialStartAt.isValid()) {
      message.warning('请选择周期起点');
      return;
    }

    const nextChangeAt: Dayjs | null = values?.nextChangeAt ?? null;
    const downtimeMinutes: number | undefined = values?.downtimeMinutes;

    if (nextChangeAt && nextChangeAt.isValid()) {
      if (nextChangeAt.isBefore(initialStartAt)) {
        message.warning('计划换辊时刻不能早于周期起点');
        return;
      }
      if (nextChangeAt.isBefore(dayjs())) {
        message.warning('计划换辊时刻不能早于当前时间（过期时刻会被系统忽略）');
        return;
      }
    }

    setLoading(true);
    try {
      await rollApi.upsertRollCampaignPlan({
        versionId,
        machineCode: editingMachineCode,
        initialStartAt: initialStartAt.format('YYYY-MM-DD HH:mm:ss'),
        nextChangeAt: nextChangeAt && nextChangeAt.isValid() ? nextChangeAt.format('YYYY-MM-DD HH:mm:ss') : undefined,
        downtimeMinutes: typeof downtimeMinutes === 'number' ? downtimeMinutes : undefined,
        operator: currentUser,
        reason,
      });
      message.success('换辊监控计划已更新');
      setEditOpen(false);
      await loadPlans();
      await rollAlertsQuery.refetch();
    } catch (e: any) {
      message.error(e?.message || '保存失败');
    } finally {
      setLoading(false);
    }
  };

  const importSampleConfigs = async () => {
    if (!versionId) {
      message.warning('请先激活一个排产版本');
      return;
    }

    Modal.confirm({
      title: '导入示例配置（全局）',
      content: (
        <div>
          <div style={{ marginBottom: 8 }}>
            将写入以下配置项到 <Tag>global</Tag>（可在「系统配置-配置管理」中调整）：
          </div>
          <ul style={{ margin: 0, paddingInlineStart: 18 }}>
            {SAMPLE_CONFIGS.map((c) => (
              <li key={c.key}>
                <code>{c.key}</code> = <code>{c.value}</code>
              </li>
            ))}
          </ul>
        </div>
      ),
      okText: '确认写入',
      cancelText: '取消',
      onOk: async () => {
        setLoading(true);
        try {
          for (const c of SAMPLE_CONFIGS) {
            await configApi.updateConfig('global', c.key, c.value, currentUser, '导入换辊示例配置');
          }
          message.success('示例配置已写入，将自动触发刷新');
        } catch (e: any) {
          message.error(e?.message || '写入失败');
        } finally {
          setLoading(false);
        }
      },
    });
  };

  const columns: ColumnsType<TableRow> = [
    {
      title: '机组',
      dataIndex: 'machineCode',
      key: 'machineCode',
      width: 90,
      fixed: 'left',
      render: (v: string) => <Tag color="blue">{v}</Tag>,
    },
    {
      title: '状态',
      key: 'alertLevel',
      width: 120,
      render: (_, row) => {
        const level = row.alert?.alertLevel ?? 'NONE';
        return <Tag color={getAlertLevelColor(level)}>{getAlertLevelLabel(level)}</Tag>;
      },
    },
    {
      title: '当前累积',
      key: 'currentTonnageT',
      width: 120,
      render: (_, row) => (row.alert ? `${row.alert.currentTonnageT.toFixed(3)} 吨` : '-'),
    },
    {
      title: '软/硬阈值',
      key: 'limits',
      width: 140,
      render: (_, row) =>
        row.alert ? `${row.alert.softLimitT.toFixed(3)} / ${row.alert.hardLimitT.toFixed(3)}` : '-',
    },
    {
      title: '利用率(软)',
      key: 'utilization',
      width: 120,
      render: (_, row) => {
        if (!row.alert) return '-';
        return `${calculateUtilization(row.alert.currentTonnageT, row.alert.softLimitT)}%`;
      },
    },
    {
      title: '周期起点',
      key: 'campaignStartAt',
      width: 260,
      render: (_, row) => (
        <div>
          <div>
            <Typography.Text type="secondary">监控：{row.alert?.campaignStartAt || '-'}</Typography.Text>
          </div>
          <div>
            <Typography.Text>微调：{row.plan?.initialStartAt || '-'}</Typography.Text>
          </div>
        </div>
      ),
    },
    {
      title: '计划换辊时刻',
      key: 'plannedChangeAt',
      width: 300,
      render: (_, row) => {
        const expired = isExpiredDateTime(row.plan?.nextChangeAt);
        return (
          <div>
            <div>
              <Typography.Text type="secondary">监控：{row.alert?.plannedChangeAt || '-'}</Typography.Text>
            </div>
            <div>
              <Typography.Text>微调：{row.plan?.nextChangeAt || '-'}</Typography.Text>
              {expired ? <Tag color="warning" style={{ marginInlineStart: 8 }}>已过期</Tag> : null}
            </div>
          </div>
        );
      },
    },
    {
      title: '预计触达软/硬',
      key: 'estimatedReach',
      width: 220,
      render: (_, row) =>
        row.alert
          ? `${row.alert.estimatedSoftReachAt || '-'} / ${row.alert.estimatedHardReachAt || '-'}`
          : '-',
    },
    {
      title: '停机时长(分钟)',
      key: 'downtime',
      width: 220,
      render: (_, row) => (
        <div>
          <div>
            <Typography.Text type="secondary">
              监控：{row.alert?.plannedDowntimeMinutes != null ? row.alert.plannedDowntimeMinutes : '-'}
            </Typography.Text>
          </div>
          <div>
            <Typography.Text>微调：{row.plan?.downtimeMinutes != null ? row.plan.downtimeMinutes : '-'}</Typography.Text>
          </div>
        </div>
      ),
    },
    {
      title: '覆盖记录',
      key: 'override',
      width: 200,
      render: (_, row) => {
        if (!row.plan) return <span style={{ color: '#8c8c8c' }}>未微调</span>;
        const who = row.plan.updatedBy ? ` · ${row.plan.updatedBy}` : '';
        const when = row.plan.updatedAt ? formatDateTime(row.plan.updatedAt) : '';
        const expired = isExpiredDateTime(row.plan?.nextChangeAt);
        return (
          <Space size={4}>
            <span style={{ color: '#1677ff' }}>
              已微调{who}{when ? ` · ${when}` : ''}
            </span>
            {expired ? <Tag color="warning">换辊时刻过期</Tag> : null}
          </Space>
        );
      },
    },
    {
      title: '操作',
      key: 'action',
      width: 110,
      fixed: 'right',
      render: (_, row) => (
        <Button size="small" type="primary" onClick={() => openEdit(row)} disabled={!versionId}>
          微调
        </Button>
      ),
    },
  ];

  return (
    <Space direction="vertical" size={12} style={{ width: '100%' }}>
      {!versionId ? (
        <Alert
          type="warning"
          showIcon
          message="未检测到激活版本"
          description="换辊监控计划按“版本+机组”维护，请先激活一个排产版本。"
        />
      ) : null}

      <Card size="small" title="使用说明" extra={<Button icon={<SettingOutlined />} onClick={importSampleConfigs}>导入示例配置</Button>}>
        <Typography.Paragraph style={{ marginBottom: 8 }}>
          本模块用于<span style={{ fontWeight: 600 }}>设备换辊时间监控</span>与<span style={{ fontWeight: 600 }}>计划换辊时刻微调</span>：
        </Typography.Paragraph>
        <ul style={{ margin: 0, paddingInlineStart: 18 }}>
          <li>不直接影响排程结果，仅用于监控设备状态与生产效能。</li>
          <li>系统按版本计划项时间线估算累积吨位，并推算触达软/硬阈值的日期时间。</li>
          <li>你可以为某机组微调：周期起点、计划换辊时刻、停机时长（典型 30~60 分钟）。</li>
          <li>阈值/默认停机时长等参数请在“系统配置-配置管理”中维护。</li>
        </ul>
      </Card>

      {loadError ? (
        <Alert
          type="error"
          showIcon
          message="换辊计划加载失败"
          description={loadError}
          action={
            <Button size="small" onClick={loadPlans} icon={<ReloadOutlined />}>
              重试
            </Button>
          }
        />
      ) : null}

      <Card
        title={`换辊时间监控（${rows.length} 台机组）`}
        extra={
          <Space>
            <Button icon={<ReloadOutlined />} onClick={async () => {
              await loadPlans();
              await rollAlertsQuery.refetch();
            }}>
              刷新
            </Button>
          </Space>
        }
      >
        {expiredPlanMachineCodes.length > 0 ? (
          <Alert
            type="warning"
            showIcon
            style={{ marginBottom: 12 }}
            message="检测到过期的人工计划换辊时刻"
            description={`机组：${expiredPlanMachineCodes.join('、')}。过期时刻不会参与系统推算，建议重新微调或清空该字段。`}
          />
        ) : null}

        <Table<TableRow>
          rowKey={(r) => r.machineCode}
          size="small"
          loading={loading || rollAlertsQuery.isLoading}
          columns={columns}
          dataSource={rows}
          pagination={{ pageSize: 20, showSizeChanger: true }}
          scroll={{ x: 1900 }}
        />
      </Card>

      <Modal
        open={editOpen}
        title={editingMachineCode ? `微调换辊计划 · ${editingMachineCode}` : '微调换辊计划'}
        onCancel={() => setEditOpen(false)}
        onOk={handleSave}
        okText="保存"
        confirmLoading={loading}
        destroyOnClose
      >
        <Form layout="vertical" form={form}>
          <Form.Item
            label="周期起点"
            name="initialStartAt"
            rules={[{ required: true, message: '请选择周期起点' }]}
          >
            <DatePicker showTime style={{ width: '100%' }} />
          </Form.Item>

          <Form.Item label="计划换辊时刻（可选）" name="nextChangeAt">
            <DatePicker showTime style={{ width: '100%' }} allowClear />
          </Form.Item>

          <Form.Item label="停机时长（分钟，可选）" name="downtimeMinutes">
            <InputNumber min={1} max={1440} style={{ width: '100%' }} />
          </Form.Item>

          <Form.Item
            label="修改原因"
            name="reason"
            rules={[{ required: true, message: '请输入修改原因' }]}
          >
            <Input.TextArea rows={3} placeholder="例如：按现场点检/计划停机窗口调整" />
          </Form.Item>
        </Form>
      </Modal>
    </Space>
  );
};

export default RollCampaignManagementPanel;

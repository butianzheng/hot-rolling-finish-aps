import React, { useEffect, useMemo, useState } from 'react';
import { Button, Card, Form, Input, InputNumber, Modal, Select, Space, Table, Tag, Typography, message, Alert } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { CopyOutlined, EditOutlined, PlusOutlined, ReloadOutlined } from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import { configApi, planApi } from '../../api/tauri';
import { useCurrentUser } from '../../stores/use-global-store';
import { tableEmptyConfig } from '../CustomEmpty';

const { Text } = Typography;

type StrategyPresetRow = { strategy: string; title: string; description: string };

type CustomStrategyProfile = {
  strategy_id: string;
  title: string;
  description?: string | null;
  base_strategy: string;
  parameters?: {
    urgent_weight?: number | null;
    capacity_weight?: number | null;
    cold_stock_weight?: number | null;
    due_date_weight?: number | null;
    rolling_output_age_weight?: number | null;
    cold_stock_age_threshold_days?: number | null;
    overflow_tolerance_pct?: number | null;
  } | null;
};

const BASE_STRATEGY_LABEL: Record<string, string> = {
  balanced: '均衡方案',
  urgent_first: '紧急优先',
  capacity_first: '产能优先',
  cold_stock_first: '冷坨消化',
};

function makeCustomStrategyKey(strategyId: string): string {
  return `custom:${String(strategyId || '').trim()}`;
}

function suggestStrategyId(baseStrategy: string): string {
  const base = String(baseStrategy || 'balanced').trim();
  const suffix = Math.random().toString(36).slice(2, 8);
  return `custom_${base}_${suffix}`;
}

const StrategyProfilesPanel: React.FC = () => {
  const navigate = useNavigate();
  const currentUser = useCurrentUser();
  const [loading, setLoading] = useState(false);
  const [presets, setPresets] = useState<StrategyPresetRow[]>([]);
  const [customProfiles, setCustomProfiles] = useState<CustomStrategyProfile[]>([]);

  const [modalOpen, setModalOpen] = useState(false);
  const [modalMode, setModalMode] = useState<'create' | 'edit' | 'copy'>('create');
  const [saving, setSaving] = useState(false);
  const [form] = Form.useForm();

  const presetsByKey = useMemo(() => {
    const map: Record<string, StrategyPresetRow> = {};
    (presets || []).forEach((p) => {
      map[String(p.strategy)] = p;
    });
    return map;
  }, [presets]);

  const loadAll = async () => {
    setLoading(true);
    try {
      const [presetRes, customRes] = await Promise.all([
        planApi.getStrategyPresets().catch(() => null),
        configApi.listCustomStrategies().catch(() => null),
      ]);

      const nextPresets: StrategyPresetRow[] = Array.isArray(presetRes)
        ? presetRes
            .map((p: any) => ({
              strategy: String(p?.strategy ?? ''),
              title: String(p?.title ?? ''),
              description: String(p?.description ?? ''),
            }))
            .filter((p) => p.strategy && p.title)
        : [];

      const nextCustom: CustomStrategyProfile[] = Array.isArray(customRes)
        ? customRes
            .map((p: any) => ({
              strategy_id: String(p?.strategy_id ?? ''),
              title: String(p?.title ?? ''),
              description: p?.description != null ? String(p.description) : null,
              base_strategy: String(p?.base_strategy ?? ''),
              parameters: p?.parameters ?? null,
            }))
            .filter((p) => p.strategy_id && p.title && p.base_strategy)
        : [];

      setPresets(nextPresets);
      setCustomProfiles(nextCustom);
    } catch (e: any) {
      message.error(e?.message || '加载策略配置失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const openCreate = (baseStrategy: string = 'balanced') => {
    setModalMode('create');
    form.resetFields();
    form.setFieldsValue({
      strategy_id: suggestStrategyId(baseStrategy),
      title: '',
      description: '',
      base_strategy: baseStrategy,
      parameters: {},
      reason: '',
    });
    setModalOpen(true);
  };

  const openCopyFromPreset = (preset: StrategyPresetRow) => {
    setModalMode('copy');
    form.resetFields();
    form.setFieldsValue({
      strategy_id: suggestStrategyId(preset.strategy),
      title: `${preset.title}（自定义）`,
      description: preset.description,
      base_strategy: preset.strategy,
      parameters: {},
      reason: '',
    });
    setModalOpen(true);
  };

  const openEdit = (profile: CustomStrategyProfile) => {
    setModalMode('edit');
    form.resetFields();
    form.setFieldsValue({
      strategy_id: profile.strategy_id,
      title: profile.title,
      description: profile.description || '',
      base_strategy: profile.base_strategy,
      parameters: profile.parameters || {},
      reason: '',
    });
    setModalOpen(true);
  };

  const openCopyFromCustom = (profile: CustomStrategyProfile) => {
    setModalMode('copy');
    form.resetFields();
    form.setFieldsValue({
      strategy_id: suggestStrategyId(profile.base_strategy),
      title: `${profile.title}（复制）`,
      description: profile.description || '',
      base_strategy: profile.base_strategy,
      parameters: profile.parameters || {},
      reason: '',
    });
    setModalOpen(true);
  };

  const handleSave = async () => {
    const values = await form.validateFields();
    const reason = String(values?.reason || '').trim();
    if (!reason) {
      message.warning('请输入保存原因');
      return;
    }

    const payload: CustomStrategyProfile = {
      strategy_id: String(values?.strategy_id || '').trim(),
      title: String(values?.title || '').trim(),
      description: String(values?.description || '').trim() ? String(values.description).trim() : null,
      base_strategy: String(values?.base_strategy || '').trim(),
      parameters: values?.parameters || {},
    };

    setSaving(true);
    try {
      const resp = await configApi.saveCustomStrategy({
        strategy: payload,
        operator: currentUser || 'admin',
        reason,
      });
      message.success(resp?.message || '保存成功');
      setModalOpen(false);
      await loadAll();
    } catch (e: any) {
      message.error(e?.message || '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const baseStrategyOptions = useMemo(() => {
    const keys = Object.keys(presetsByKey);
    if (!keys.length) {
      return [
        { value: 'balanced', label: '均衡方案' },
        { value: 'urgent_first', label: '紧急优先' },
        { value: 'capacity_first', label: '产能优先' },
        { value: 'cold_stock_first', label: '冷坨消化' },
      ];
    }
    return keys.map((k) => ({
      value: k,
      label: presetsByKey[k]?.title || BASE_STRATEGY_LABEL[k] || k,
    }));
  }, [presetsByKey]);

  const columns: ColumnsType<CustomStrategyProfile> = [
    {
      title: '名称',
      dataIndex: 'title',
      key: 'title',
      width: 220,
      render: (v: string) => <Text strong>{v}</Text>,
    },
    {
      title: '策略ID',
      dataIndex: 'strategy_id',
      key: 'strategy_id',
      width: 200,
      render: (v: string) => <Text code>{v}</Text>,
    },
    {
      title: '基于预设',
      dataIndex: 'base_strategy',
      key: 'base_strategy',
      width: 140,
      render: (v: string) => <Tag color="blue">{BASE_STRATEGY_LABEL[String(v)] || String(v)}</Tag>,
    },
    {
      title: '参数摘要',
      key: 'params',
      width: 320,
      render: (_, r) => {
        const p = r?.parameters || {};
        const parts: string[] = [];
        if (p.urgent_weight != null) parts.push(`urgent=${p.urgent_weight}`);
        if (p.capacity_weight != null) parts.push(`capacity=${p.capacity_weight}`);
        if (p.cold_stock_weight != null) parts.push(`cold=${p.cold_stock_weight}`);
        if (p.due_date_weight != null) parts.push(`due=${p.due_date_weight}`);
        if (p.rolling_output_age_weight != null) parts.push(`roll_age=${p.rolling_output_age_weight}`);
        if (p.cold_stock_age_threshold_days != null) parts.push(`cold_days>=${p.cold_stock_age_threshold_days}`);
        if (p.overflow_tolerance_pct != null) parts.push(`overflow<=${Math.round(p.overflow_tolerance_pct * 100)}%`);

        return (
          <Text type="secondary">
            {parts.length ? parts.join(' · ') : '—'}
          </Text>
        );
      },
    },
    {
      title: '说明',
      dataIndex: 'description',
      key: 'description',
      ellipsis: true,
      render: (v: any) => <Text type="secondary">{String(v || '') || '—'}</Text>,
    },
    {
      title: '操作',
      key: 'actions',
      width: 240,
      render: (_, r) => (
        <Space size={6}>
          <Button size="small" icon={<EditOutlined />} onClick={() => openEdit(r)}>
            编辑
          </Button>
          <Button size="small" icon={<CopyOutlined />} onClick={() => openCopyFromCustom(r)}>
            复制
          </Button>
          <Button
            size="small"
            onClick={() => {
              const key = makeCustomStrategyKey(r.strategy_id);
              navigate(`/comparison?tab=draft&strategies=${encodeURIComponent(key)}`);
            }}
          >
            去草案对比
          </Button>
        </Space>
      ),
    },
  ];

  return (
    <Space direction="vertical" size={12} style={{ width: '100%' }}>
      <Alert
        type="info"
        showIcon
        message="策略配置说明"
        description={
          <div>
            <div>自定义策略用于沉淀“可复用的策略模板”（复制预设 → 调参 → 保存）。</div>
            <div>注意：当前版本自定义策略的参数仅做保存与展示；草案试算仍按“基于预设策略”执行（参数将在后续引擎支持后生效）。</div>
          </div>
        }
      />

      <Card
        size="small"
        title="预设策略（可复制为自定义）"
        extra={
          <Button size="small" icon={<ReloadOutlined />} loading={loading} onClick={loadAll}>
            刷新
          </Button>
        }
      >
        <Space wrap>
          {(presets.length ? presets : [
            { strategy: 'balanced', title: '均衡方案', description: '在交付/产能/库存之间保持均衡' },
            { strategy: 'urgent_first', title: '紧急优先', description: '优先保障 L3/L2 紧急订单' },
            { strategy: 'capacity_first', title: '产能优先', description: '优先提升产能利用率，减少溢出' },
            { strategy: 'cold_stock_first', title: '冷坨消化', description: '优先消化冷坨/压库物料' },
          ]).map((p) => (
            <Button key={`preset-${p.strategy}`} onClick={() => openCopyFromPreset(p)}>
              复制：{p.title}
            </Button>
          ))}

          <Button type="primary" icon={<PlusOutlined />} onClick={() => openCreate('balanced')}>
            新建自定义策略
          </Button>
        </Space>
      </Card>

      <Card
        size="small"
        title={
          <Space>
            <span>自定义策略</span>
            <Tag color="gold">{customProfiles.length}</Tag>
          </Space>
        }
      >
        <Table
          rowKey="strategy_id"
          size="small"
          loading={loading}
          columns={columns}
          dataSource={customProfiles}
          pagination={{ pageSize: 10, showSizeChanger: true }}
          locale={tableEmptyConfig}
        />
      </Card>

      <Modal
        title={modalMode === 'edit' ? '编辑自定义策略' : modalMode === 'copy' ? '复制自定义策略' : '新建自定义策略'}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleSave}
        okText="保存"
        cancelText="取消"
        confirmLoading={saving}
        width={720}
      >
        <Form form={form} layout="vertical">
          <Form.Item
            label="策略ID（仅字母/数字/_/-）"
            name="strategy_id"
            rules={[{ required: true, message: '请输入策略ID' }]}
          >
            <Input disabled={modalMode === 'edit'} placeholder="例如：custom_balanced_ab12cd" />
          </Form.Item>

          <Form.Item
            label="策略名称"
            name="title"
            rules={[{ required: true, message: '请输入策略名称' }]}
          >
            <Input placeholder="例如：均衡方案-偏紧急" />
          </Form.Item>

          <Form.Item label="说明" name="description">
            <Input.TextArea rows={2} placeholder="可选：说明该策略适用场景/注意事项" />
          </Form.Item>

          <Form.Item
            label="基于预设策略"
            name="base_strategy"
            rules={[{ required: true, message: '请选择基于策略' }]}
          >
            <Select options={baseStrategyOptions} style={{ width: 240 }} />
          </Form.Item>

          <Card size="small" title="参数（当前仅保存展示）" style={{ marginBottom: 12 }}>
            <Space wrap size={12}>
              <Form.Item label="urgent_weight" name={['parameters', 'urgent_weight']}>
                <InputNumber min={0} max={100} style={{ width: 140 }} />
              </Form.Item>
              <Form.Item label="capacity_weight" name={['parameters', 'capacity_weight']}>
                <InputNumber min={0} max={100} style={{ width: 140 }} />
              </Form.Item>
              <Form.Item label="cold_stock_weight" name={['parameters', 'cold_stock_weight']}>
                <InputNumber min={0} max={100} style={{ width: 160 }} />
              </Form.Item>
              <Form.Item label="due_date_weight" name={['parameters', 'due_date_weight']}>
                <InputNumber min={0} max={100} style={{ width: 150 }} />
              </Form.Item>
              <Form.Item label="rolling_output_age_weight" name={['parameters', 'rolling_output_age_weight']}>
                <InputNumber min={0} max={100} style={{ width: 210 }} />
              </Form.Item>
              <Form.Item label="cold_stock_age_threshold_days" name={['parameters', 'cold_stock_age_threshold_days']}>
                <InputNumber min={0} max={365} style={{ width: 240 }} />
              </Form.Item>
              <Form.Item label="overflow_tolerance_pct (0~1)" name={['parameters', 'overflow_tolerance_pct']}>
                <InputNumber min={0} max={1} step={0.01} style={{ width: 210 }} />
              </Form.Item>
            </Space>
          </Card>

          <Form.Item
            label="保存原因（用于审计）"
            name="reason"
            rules={[{ required: true, message: '请输入保存原因' }]}
          >
            <Input.TextArea rows={2} placeholder="例如：为保障 L3 订单，提升紧急权重" />
          </Form.Item>
        </Form>
      </Modal>
    </Space>
  );
};

export default StrategyProfilesPanel;

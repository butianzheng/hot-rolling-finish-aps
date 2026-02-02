import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Card, DatePicker, Divider, Input, InputNumber, Modal, Select, Space, Switch, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs, { type Dayjs } from 'dayjs';
import { ReloadOutlined } from '@ant-design/icons';
import { rhythmApi } from '../../api/tauri';
import { formatDate } from '../../utils/formatters';

type PresetRow = {
  presetId: string;
  presetName: string;
  target: Record<string, number>;
};

type ProfileRow = {
  category: string;
  scheduledWeightT: number;
  actualRatio: number;
  targetRatio: number | null;
  diffRatio: number | null;
};

type DailyProfile = {
  versionId: string;
  machineCode: string;
  planDate: string;
  totalScheduledWeightT: number;
  deviationThreshold: number;
  maxDeviation: number;
  isViolated: boolean;
  targetPresetId: string | null;
  targetUpdatedAt: string | null;
  targetUpdatedBy: string | null;
  categories: ProfileRow[];
};

function safeJsonObject(raw: unknown): Record<string, number> {
  if (typeof raw !== 'string') return {};
  try {
    const obj = JSON.parse(raw);
    if (!obj || typeof obj !== 'object' || Array.isArray(obj)) return {};
    const out: Record<string, number> = {};
    Object.entries(obj as Record<string, unknown>).forEach(([k, v]) => {
      const key = String(k || '').trim();
      const num = typeof v === 'number' ? v : Number(v);
      if (!key) return;
      if (!Number.isFinite(num)) return;
      if (num <= 0) return;
      out[key] = num;
    });
    return out;
  } catch {
    return {};
  }
}

function normalizeRatios(input: Record<string, number>): Record<string, number> {
  const entries = Object.entries(input).filter(([, v]) => Number.isFinite(v) && v > 0);
  const sum = entries.reduce((s, [, v]) => s + v, 0);
  if (sum <= 0) return {};
  const out: Record<string, number> = {};
  entries.forEach(([k, v]) => {
    out[k] = v / sum;
  });
  return out;
}

function mapProfile(raw: any): DailyProfile | null {
  if (!raw) return null;
  const rows: ProfileRow[] = Array.isArray(raw?.categories)
    ? raw.categories.map((r: any) => ({
        category: String(r?.category ?? ''),
        scheduledWeightT: Number(r?.scheduled_weight_t ?? 0),
        actualRatio: Number(r?.actual_ratio ?? 0),
        targetRatio: r?.target_ratio != null ? Number(r.target_ratio) : null,
        diffRatio: r?.diff_ratio != null ? Number(r.diff_ratio) : null,
      }))
    : [];

  return {
    versionId: String(raw?.version_id ?? ''),
    machineCode: String(raw?.machine_code ?? ''),
    planDate: String(raw?.plan_date ?? ''),
    totalScheduledWeightT: Number(raw?.total_scheduled_weight_t ?? 0),
    deviationThreshold: Number(raw?.deviation_threshold ?? 0.1),
    maxDeviation: Number(raw?.max_deviation ?? 0),
    isViolated: !!raw?.is_violated,
    targetPresetId: raw?.target_preset_id != null ? String(raw.target_preset_id) : null,
    targetUpdatedAt: raw?.target_updated_at != null ? String(raw.target_updated_at) : null,
    targetUpdatedBy: raw?.target_updated_by != null ? String(raw.target_updated_by) : null,
    categories: rows.filter((r) => !!r.category),
  };
}

export type DailyRhythmManagerModalProps = {
  open: boolean;
  onClose: () => void;
  versionId: string | null;
  machineOptions: string[];
  defaultMachineCode?: string | null;
  defaultPlanDate?: string | null;
  operator: string;
};

const DailyRhythmManagerModal: React.FC<DailyRhythmManagerModalProps> = ({
  open,
  onClose,
  versionId,
  machineOptions,
  defaultMachineCode,
  defaultPlanDate,
  operator,
}) => {
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const [presets, setPresets] = useState<PresetRow[]>([]);
  const [selectedMachine, setSelectedMachine] = useState<string | null>(null);
  const [selectedDate, setSelectedDate] = useState<Dayjs | null>(null);
  const [profile, setProfile] = useState<DailyProfile | null>(null);

  const [editingTarget, setEditingTarget] = useState<Record<string, number>>({});
  const [appliedPresetId, setAppliedPresetId] = useState<string | null>(null);
  const [targetDirty, setTargetDirty] = useState(false);
  const [saveReason, setSaveReason] = useState('');

  const [batchMachines, setBatchMachines] = useState<string[]>([]);
  const [batchRange, setBatchRange] = useState<[Dayjs, Dayjs] | null>(null);
  const [batchPresetId, setBatchPresetId] = useState<string | null>(null);
  const [batchOverwrite, setBatchOverwrite] = useState(true);
  const [batchReason, setBatchReason] = useState('');

  const presetOptions = useMemo(() => {
    return presets.map((p) => ({ label: p.presetName, value: p.presetId }));
  }, [presets]);

  const loadPresets = async () => {
    setLoadError(null);
    try {
      const raw = await rhythmApi.listRhythmPresets('PRODUCT_CATEGORY');
      const list: PresetRow[] = Array.isArray(raw)
        ? raw
            .map((p: any) => ({
              presetId: String(p?.preset_id ?? ''),
              presetName: String(p?.preset_name ?? ''),
              target: normalizeRatios(safeJsonObject(p?.target_json)),
            }))
            .filter((p) => p.presetId && p.presetName)
        : [];
      setPresets(list);
      if (!batchPresetId && list.length > 0) setBatchPresetId(list[0].presetId);
    } catch (e: any) {
      setLoadError(String(e?.message || e || '加载节奏模板失败'));
    }
  };

  const loadProfile = async (machine: string, date: Dayjs) => {
    if (!versionId) return;
    setLoadError(null);
    try {
      const raw = await rhythmApi.getDailyRhythmProfile(versionId, machine, formatDate(date));
      const p = mapProfile(raw);
      setProfile(p);
      const nextTarget: Record<string, number> = {};
      (p?.categories || []).forEach((r) => {
        if (!r.category) return;
        if (typeof r.targetRatio === 'number' && Number.isFinite(r.targetRatio) && r.targetRatio > 0) {
          nextTarget[r.category] = r.targetRatio;
        }
      });
      setEditingTarget(nextTarget);
      setAppliedPresetId(p?.targetPresetId || null);
      setTargetDirty(false);
      setSaveReason('');
    } catch (e: any) {
      setLoadError(String(e?.message || e || '加载节奏画像失败'));
      setProfile(null);
      setEditingTarget({});
    }
  };

  useEffect(() => {
    if (!open) return;
    const fallbackMachine = String(defaultMachineCode || '').trim() || machineOptions[0] || null;
    const fallbackDate = defaultPlanDate ? dayjs(defaultPlanDate) : dayjs();
    setSelectedMachine(fallbackMachine);
    setSelectedDate(fallbackDate.isValid() ? fallbackDate : dayjs());
    setBatchMachines(machineOptions.slice(0, 1));
    setBatchRange([dayjs().startOf('day'), dayjs().add(6, 'day').startOf('day')]);
    void loadPresets();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  useEffect(() => {
    if (!open) return;
    if (!versionId) return;
    if (!selectedMachine || !selectedDate || !selectedDate.isValid()) return;
    void loadProfile(selectedMachine, selectedDate);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, versionId, selectedMachine, selectedDate?.valueOf()]);

  const onApplyPresetToCurrent = (presetId: string) => {
    const preset = presets.find((p) => p.presetId === presetId);
    if (!preset) return;
    setEditingTarget(preset.target);
    setAppliedPresetId(presetId);
    setTargetDirty(false);
  };

  const updateTargetPct = (category: string, pct: number | null) => {
    const nextPct = Number(pct || 0);
    const ratio = nextPct > 0 ? nextPct / 100 : 0;
    setEditingTarget((prev) => {
      const next = { ...prev };
      if (ratio <= 0) {
        delete next[category];
      } else {
        next[category] = ratio;
      }
      return next;
    });
    setTargetDirty(true);
  };

  const saveCurrentTarget = async () => {
    if (!versionId || !selectedMachine || !selectedDate) return;
    const reason = saveReason.trim();
    if (!reason) {
      message.warning('请输入修改原因');
      return;
    }

    const normalized = normalizeRatios(editingTarget);
    const targetJson = JSON.stringify(normalized);
    setLoading(true);
    try {
      await rhythmApi.upsertRhythmTarget({
        versionId,
        machineCode: selectedMachine,
        planDate: formatDate(selectedDate),
        dimension: 'PRODUCT_CATEGORY',
        targetJson,
        presetId: !targetDirty ? appliedPresetId || undefined : undefined,
        operator,
        reason,
      });
      message.success('已更新当日节奏目标');
      await loadProfile(selectedMachine, selectedDate);
    } catch (e: any) {
      message.error(e?.message || '保存失败');
    } finally {
      setLoading(false);
    }
  };

  const applyBatch = async () => {
    if (!versionId) return;
    if (!batchPresetId) {
      message.warning('请选择模板');
      return;
    }
    if (!batchMachines.length) {
      message.warning('请选择机组');
      return;
    }
    if (!batchRange || !batchRange[0] || !batchRange[1]) {
      message.warning('请选择日期范围');
      return;
    }
    const reason = batchReason.trim();
    if (!reason) {
      message.warning('请输入批量应用原因');
      return;
    }

    setLoading(true);
    try {
      const res = await rhythmApi.applyRhythmPreset({
        versionId,
        dimension: 'PRODUCT_CATEGORY',
        presetId: batchPresetId,
        machineCodes: batchMachines,
        dateFrom: formatDate(batchRange[0]),
        dateTo: formatDate(batchRange[1]),
        overwrite: batchOverwrite,
        operator,
        reason,
      });
      const applied = Number(res?.applied ?? 0);
      message.success(`已批量应用模板（写入 ${applied} 条）`);
      if (selectedMachine && selectedDate) {
        await loadProfile(selectedMachine, selectedDate);
      }
    } catch (e: any) {
      message.error(e?.message || '批量应用失败');
    } finally {
      setLoading(false);
    }
  };

  const rows = profile?.categories || [];

  const columns: ColumnsType<ProfileRow> = [
    {
      title: '品种大类',
      dataIndex: 'category',
      key: 'category',
      width: 140,
      render: (v: string) => <Tag color="blue">{v}</Tag>,
    },
    {
      title: '已排(吨)',
      dataIndex: 'scheduledWeightT',
      key: 'scheduledWeightT',
      width: 110,
      render: (v: number) => (Number.isFinite(v) ? v.toFixed(1) : '-'),
    },
    {
      title: '实际占比',
      dataIndex: 'actualRatio',
      key: 'actualRatio',
      width: 110,
      render: (v: number) => `${((Number.isFinite(v) ? v : 0) * 100).toFixed(1)}%`,
    },
  ];

  columns.push(
    {
      title: '目标占比',
      key: 'targetRatio',
      width: 160,
      render: (_, row) => {
        const ratio = editingTarget[row.category] ?? (row.targetRatio ?? 0);
        return (
          <InputNumber
            min={0}
            max={100}
            precision={1}
            value={Number.isFinite(ratio) ? ratio * 100 : 0}
            onChange={(v) => updateTargetPct(row.category, typeof v === 'number' ? v : null)}
            style={{ width: '100%' }}
            addonAfter="%"
          />
        );
      },
    },
    {
      title: '偏差',
      key: 'diffRatio',
      width: 100,
      render: (_, row) => {
        const ratio = editingTarget[row.category] ?? row.targetRatio ?? null;
        const diff = ratio == null ? null : Math.abs((row.actualRatio || 0) - ratio);
        if (diff == null) return '-';
        const pct = diff * 100;
        return <span style={{ color: pct >= (profile?.deviationThreshold || 0.1) * 100 ? '#faad14' : undefined }}>{pct.toFixed(1)}%</span>;
      },
    }
  );

  const headerSummary = useMemo(() => {
    if (!profile) return null;
    const maxPct = (profile.maxDeviation || 0) * 100;
    const thPct = (profile.deviationThreshold || 0.1) * 100;
    return (
      <Space wrap size={8}>
        <Tag color={profile.isViolated ? 'red' : 'green'}>
          最大偏差 {maxPct.toFixed(1)}% / 阈值 {thPct.toFixed(1)}%
        </Tag>
        <Tag>当日已排 {profile.totalScheduledWeightT.toFixed(1)} 吨</Tag>
        {profile.targetPresetId ? <Tag>模板 {profile.targetPresetId}</Tag> : <Tag>未绑定模板</Tag>}
        {profile.targetUpdatedAt ? <Tag>更新 {profile.targetUpdatedAt}</Tag> : null}
      </Space>
    );
  }, [profile]);

  return (
    <Modal
      open={open}
      onCancel={onClose}
      title="每日生产节奏管理（品种大类）"
      width={980}
      footer={null}
      destroyOnClose
    >
      {!versionId ? (
        <Alert type="warning" showIcon message="请先激活一个排产版本" />
      ) : null}

      <Alert
        type="info"
        showIcon
        message="口径说明"
        description="节奏目标用于监控当日品种大类占比（目标 vs 实际），不直接改变排程结果；修改后会触发 D4 等读模型刷新。"
        style={{ marginBottom: 12 }}
      />

      {loadError ? <Alert type="error" showIcon message={loadError} style={{ marginBottom: 12 }} /> : null}

      <Space wrap style={{ width: '100%', justifyContent: 'space-between', marginBottom: 8 }}>
        <Space wrap>
          <Typography.Text type="secondary">机组</Typography.Text>
          <Select
            style={{ minWidth: 140 }}
            value={selectedMachine}
            options={machineOptions.map((m) => ({ label: m, value: m }))}
            onChange={(v) => setSelectedMachine(v)}
          />
          <Typography.Text type="secondary">日期</Typography.Text>
          <DatePicker
            value={selectedDate}
            onChange={(d) => setSelectedDate(d)}
            allowClear={false}
          />
          <Button
            icon={<ReloadOutlined />}
            onClick={() => {
              if (!selectedMachine || !selectedDate || !versionId) return;
              void loadProfile(selectedMachine, selectedDate);
            }}
          >
            刷新
          </Button>
        </Space>

        <Space wrap>
          <Typography.Text type="secondary">快速套用模板</Typography.Text>
          <Select
            style={{ minWidth: 220 }}
            value={appliedPresetId}
            options={presetOptions}
            placeholder={presets.length ? '选择模板' : '暂无模板'}
            onChange={(v) => onApplyPresetToCurrent(String(v))}
            allowClear
          />
        </Space>
      </Space>

      {headerSummary ? (
        <div style={{ marginBottom: 8 }}>{headerSummary}</div>
      ) : null}

      <Card size="small" style={{ marginBottom: 12 }} loading={loading}>
        <Table<ProfileRow>
          rowKey={(r) => r.category}
          size="small"
          pagination={false}
          columns={columns}
          dataSource={rows}
          locale={{ emptyText: '暂无排程数据（该机组/日期没有计划项）' }}
        />

        <Divider style={{ margin: '12px 0' }} />

        <Space wrap style={{ width: '100%', justifyContent: 'space-between' }}>
          <Space wrap>
            <Input
              value={saveReason}
              onChange={(e) => setSaveReason(e.target.value)}
              placeholder="请输入修改原因（必填）"
              style={{ width: 360 }}
            />
            <Button
              type="primary"
              disabled={!versionId || !selectedMachine || !selectedDate}
              loading={loading}
              onClick={saveCurrentTarget}
            >
              保存当日目标
            </Button>
          </Space>

          <Space wrap>
            <Button
              danger
              disabled={!Object.keys(editingTarget).length}
              onClick={() => {
                setEditingTarget({});
                setTargetDirty(true);
              }}
            >
              清空目标
            </Button>
          </Space>
        </Space>
      </Card>

      <Card size="small" title="批量应用模板（提效）" loading={loading}>
        <Space direction="vertical" style={{ width: '100%' }} size={8}>
          <Space wrap>
            <Typography.Text type="secondary">机组</Typography.Text>
            <Select
              mode="multiple"
              style={{ minWidth: 320 }}
              value={batchMachines}
              options={machineOptions.map((m) => ({ label: m, value: m }))}
              onChange={(v) => setBatchMachines(v)}
              placeholder="选择机组"
            />
            <Typography.Text type="secondary">日期范围</Typography.Text>
            <DatePicker.RangePicker
              value={batchRange}
              onChange={(v) => setBatchRange(v as any)}
              allowClear={false}
            />
          </Space>

          <Space wrap>
            <Typography.Text type="secondary">模板</Typography.Text>
            <Select
              style={{ minWidth: 240 }}
              value={batchPresetId}
              options={presetOptions}
              placeholder={presets.length ? '选择模板' : '暂无模板'}
              onChange={(v) => setBatchPresetId(String(v))}
              allowClear={false}
            />
            <Typography.Text type="secondary">覆盖</Typography.Text>
            <Switch checked={batchOverwrite} onChange={(v) => setBatchOverwrite(v)} />
            <Typography.Text type="secondary">原因</Typography.Text>
            <Input
              value={batchReason}
              onChange={(e) => setBatchReason(e.target.value)}
              placeholder="必填，如：调整本周节奏"
              style={{ width: 260 }}
            />
            <Button type="primary" onClick={applyBatch} loading={loading}>
              批量应用
            </Button>
          </Space>
        </Space>
      </Card>
    </Modal>
  );
};

export default DailyRhythmManagerModal;

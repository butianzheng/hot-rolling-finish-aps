import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Card, Input, InputNumber, Modal, Space, Switch, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { PlusOutlined, ReloadOutlined } from '@ant-design/icons';
import { rhythmApi } from '../../api/tauri';
import { useCurrentUser } from '../../stores/use-global-store';

type PresetRow = {
  presetId: string;
  presetName: string;
  dimension: string;
  targetJson: string;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
  updatedBy: string | null;
};

const DEFAULT_DIMENSION = 'PRODUCT_CATEGORY';

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

function formatTargetSummary(targetJson: string): string {
  const map = normalizeRatios(safeJsonObject(targetJson));
  const pairs = Object.entries(map)
    .sort((a, b) => (b[1] || 0) - (a[1] || 0))
    .slice(0, 6)
    .map(([k, v]) => `${k}:${(v * 100).toFixed(0)}%`);
  return pairs.length ? pairs.join('，') : '-';
}

const RhythmPresetManagementPanel: React.FC = () => {
  const currentUser = useCurrentUser() || 'admin';
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [showInactive, setShowInactive] = useState(false);
  const [presets, setPresets] = useState<PresetRow[]>([]);

  const [editOpen, setEditOpen] = useState(false);
  const [editing, setEditing] = useState<PresetRow | null>(null);
  const [editName, setEditName] = useState('');
  const [editIsActive, setEditIsActive] = useState(true);
  const [editTarget, setEditTarget] = useState<Record<string, number>>({});
  const [editReason, setEditReason] = useState('');
  const [newCategory, setNewCategory] = useState('');
  const [newPct, setNewPct] = useState<number | null>(null);

  const loadPresets = async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const raw = await rhythmApi.listRhythmPresets(DEFAULT_DIMENSION, !showInactive);
      const list: PresetRow[] = Array.isArray(raw)
        ? raw
            .map((p: any) => ({
              presetId: String(p?.preset_id ?? ''),
              presetName: String(p?.preset_name ?? ''),
              dimension: String(p?.dimension ?? ''),
              targetJson: String(p?.target_json ?? '{}'),
              isActive: !!p?.is_active,
              createdAt: String(p?.created_at ?? ''),
              updatedAt: String(p?.updated_at ?? ''),
              updatedBy: p?.updated_by != null ? String(p.updated_by) : null,
            }))
            .filter((p) => p.presetId && p.presetName)
        : [];
      setPresets(list);
    } catch (e: any) {
      setLoadError(String(e?.message || e || '加载失败'));
      setPresets([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadPresets();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [showInactive]);

  const openCreate = () => {
    setEditing(null);
    setEditName('');
    setEditIsActive(true);
    setEditTarget({});
    setEditReason('');
    setNewCategory('');
    setNewPct(null);
    setEditOpen(true);
  };

  const openEdit = (row: PresetRow) => {
    setEditing(row);
    setEditName(row.presetName);
    setEditIsActive(row.isActive);
    setEditTarget(safeJsonObject(row.targetJson));
    setEditReason('');
    setNewCategory('');
    setNewPct(null);
    setEditOpen(true);
  };

  const savePreset = async () => {
    const name = editName.trim();
    const reason = editReason.trim();
    if (!name) {
      message.warning('请输入模板名称');
      return;
    }
    if (!reason) {
      message.warning('请输入保存原因');
      return;
    }

    const normalized = normalizeRatios(editTarget);
    const targetJson = JSON.stringify(normalized);

    setLoading(true);
    try {
      await rhythmApi.upsertRhythmPreset({
        presetId: editing?.presetId || undefined,
        presetName: name,
        dimension: DEFAULT_DIMENSION,
        targetJson,
        isActive: editIsActive,
        operator: currentUser,
        reason,
      });
      message.success('节奏模板已保存');
      setEditOpen(false);
      await loadPresets();
    } catch (e: any) {
      message.error(e?.message || '保存失败');
    } finally {
      setLoading(false);
    }
  };

  const toggleActive = (row: PresetRow) => {
    let reason = '';
    const nextActive = !row.isActive;
    Modal.confirm({
      title: nextActive ? '启用节奏模板' : '停用节奏模板',
      content: (
        <div>
          <div style={{ marginBottom: 8 }}>
            模板：<Tag color="blue">{row.presetName}</Tag>
          </div>
          <div style={{ marginBottom: 8 }}>请输入操作原因：</div>
          <Input.TextArea rows={3} placeholder="必填" onChange={(e) => (reason = e.target.value)} />
        </div>
      ),
      okText: '确认',
      cancelText: '取消',
      onOk: async () => {
        if (!reason.trim()) {
          message.warning('请输入原因');
          return Promise.reject();
        }
        setLoading(true);
        try {
          await rhythmApi.setRhythmPresetActive(row.presetId, nextActive, currentUser, reason.trim());
          message.success(nextActive ? '已启用' : '已停用');
          await loadPresets();
        } catch (e: any) {
          message.error(e?.message || '操作失败');
          throw e;
        } finally {
          setLoading(false);
        }
      },
    });
  };

  const targetRows = useMemo(() => {
    const entries = Object.entries(editTarget || {}).sort((a, b) => (b[1] || 0) - (a[1] || 0));
    return entries.map(([category, ratio]) => ({
      key: category,
      category,
      pct: Number.isFinite(ratio) ? ratio * 100 : 0,
    }));
  }, [editTarget]);

  const addCategory = () => {
    const cat = newCategory.trim();
    const pct = Number(newPct || 0);
    if (!cat) {
      message.warning('请输入品种大类名称');
      return;
    }
    if (!Number.isFinite(pct) || pct <= 0) {
      message.warning('请输入大于 0 的占比%');
      return;
    }
    setEditTarget((prev) => ({ ...prev, [cat]: pct / 100 }));
    setNewCategory('');
    setNewPct(null);
  };

  const removeCategory = (category: string) => {
    setEditTarget((prev) => {
      const next = { ...prev };
      delete next[category];
      return next;
    });
  };

  const updatePct = (category: string, pct: number | null) => {
    const nextPct = Number(pct || 0);
    setEditTarget((prev) => {
      const next = { ...prev };
      if (!Number.isFinite(nextPct) || nextPct <= 0) {
        delete next[category];
      } else {
        next[category] = nextPct / 100;
      }
      return next;
    });
  };

  const columns: ColumnsType<PresetRow> = [
    {
      title: '模板名称',
      dataIndex: 'presetName',
      key: 'presetName',
      width: 220,
      render: (v: string, row) => (
        <Space size={6}>
          <Tag color={row.isActive ? 'green' : 'default'}>{row.isActive ? '启用' : '停用'}</Tag>
          <span>{v}</span>
        </Space>
      ),
    },
    {
      title: '维度',
      dataIndex: 'dimension',
      key: 'dimension',
      width: 140,
      render: (v: string) => <Tag>{v || '-'}</Tag>,
    },
    {
      title: '目标配比(摘要)',
      key: 'target',
      render: (_, row) => (
        <Typography.Text style={{ fontSize: 12 }} title={row.targetJson}>
          {formatTargetSummary(row.targetJson)}
        </Typography.Text>
      ),
    },
    {
      title: '更新',
      key: 'updatedAt',
      width: 200,
      render: (_, row) => (
        <div style={{ fontSize: 12 }}>
          <div>{row.updatedAt || row.createdAt || '-'}</div>
          <div style={{ color: '#666' }}>{row.updatedBy || '-'}</div>
        </div>
      ),
    },
    {
      title: '操作',
      key: 'actions',
      width: 180,
      render: (_, row) => (
        <Space>
          <Button size="small" onClick={() => openEdit(row)}>
            编辑
          </Button>
          <Button size="small" danger={row.isActive} onClick={() => toggleActive(row)}>
            {row.isActive ? '停用' : '启用'}
          </Button>
        </Space>
      ),
    },
  ];

  const editColumns: ColumnsType<{ key: string; category: string; pct: number }> = [
    {
      title: '品种大类',
      dataIndex: 'category',
      key: 'category',
      width: 220,
      render: (v: string) => <Tag color="blue">{v}</Tag>,
    },
    {
      title: '占比(%)',
      dataIndex: 'pct',
      key: 'pct',
      width: 140,
      render: (_: number, row) => (
        <InputNumber
          min={0}
          max={100}
          step={1}
          value={row.pct}
          onChange={(v) => updatePct(row.category, typeof v === 'number' ? v : null)}
        />
      ),
    },
    {
      title: '操作',
      key: 'op',
      width: 100,
      render: (_, row) => (
        <Button size="small" danger onClick={() => removeCategory(row.category)}>
          删除
        </Button>
      ),
    },
  ];

  return (
    <div>
      <Card size="small" style={{ marginBottom: 12 }}>
        <Typography.Title level={5} style={{ margin: 0 }}>
          每日节奏模板（品种大类）
        </Typography.Title>
        <Typography.Paragraph style={{ marginTop: 8, marginBottom: 0, color: '#666' }}>
          用于在「计划工作台-每日节奏管理」中快速套用常见的品种大类占比。模板仅影响监控评估（偏差提示），不直接修改排程结果。
          占比会在保存时自动归一化（不强制要求合计=100%）。
        </Typography.Paragraph>
      </Card>

      <Space style={{ marginBottom: 12 }}>
        <Button icon={<ReloadOutlined />} onClick={loadPresets} loading={loading}>
          刷新
        </Button>
        <Button type="primary" icon={<PlusOutlined />} onClick={openCreate} disabled={loading}>
          新建模板
        </Button>
        <Space size={6}>
          <Switch checked={showInactive} onChange={setShowInactive} />
          <span style={{ color: '#666' }}>显示停用</span>
        </Space>
      </Space>

      {loadError && (
        <Alert
          type="error"
          showIcon
          style={{ marginBottom: 12 }}
          message="加载失败"
          description={loadError}
        />
      )}

      <Table<PresetRow>
        rowKey="presetId"
        size="small"
        loading={loading}
        columns={columns}
        dataSource={presets}
        pagination={{ pageSize: 8, showSizeChanger: false }}
      />

      <Modal
        title={editing ? '编辑节奏模板' : '新建节奏模板'}
        open={editOpen}
        onCancel={() => setEditOpen(false)}
        onOk={savePreset}
        okText="保存"
        cancelText="取消"
        confirmLoading={loading}
        width={760}
      >
        <Space direction="vertical" style={{ width: '100%' }} size={12}>
          <Card size="small">
            <Space style={{ width: '100%' }} direction="vertical" size={10}>
              <Space>
                <span style={{ width: 90, color: '#666' }}>模板名称</span>
                <Input
                  value={editName}
                  onChange={(e) => setEditName(e.target.value)}
                  placeholder="例如：普板为主(30/30/40)"
                  style={{ width: 420 }}
                />
                <Space size={6}>
                  <Switch checked={editIsActive} onChange={setEditIsActive} />
                  <span style={{ color: '#666' }}>启用</span>
                </Space>
              </Space>

              <Space align="start" style={{ width: '100%' }}>
                <span style={{ width: 90, color: '#666', paddingTop: 6 }}>目标配比</span>
                <div style={{ flex: 1 }}>
                  <Space style={{ marginBottom: 8 }}>
                    <Input
                      value={newCategory}
                      onChange={(e) => setNewCategory(e.target.value)}
                      placeholder="品种大类名称"
                      style={{ width: 220 }}
                    />
                    <InputNumber
                      min={0}
                      max={100}
                      step={1}
                      value={newPct}
                      onChange={(v) => setNewPct(typeof v === 'number' ? v : null)}
                      placeholder="占比%"
                    />
                    <Button onClick={addCategory}>添加</Button>
                  </Space>
                  <Table
                    size="small"
                    rowKey="key"
                    columns={editColumns}
                    dataSource={targetRows}
                    pagination={false}
                    locale={{ emptyText: '请添加至少一个品种大类占比（也可保存为空，表示不启用目标）' }}
                  />
                </div>
              </Space>

              <Space>
                <span style={{ width: 90, color: '#666' }}>保存原因</span>
                <Input
                  value={editReason}
                  onChange={(e) => setEditReason(e.target.value)}
                  placeholder="必填，用于操作审计"
                />
              </Space>
            </Space>
          </Card>
        </Space>
      </Modal>
    </div>
  );
};

export default RhythmPresetManagementPanel;


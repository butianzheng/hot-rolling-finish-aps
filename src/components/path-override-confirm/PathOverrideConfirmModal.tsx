import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Checkbox, Input, Modal, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { ReloadOutlined } from '@ant-design/icons';
import { pathRuleApi } from '../../api/tauri';

type PendingRow = {
  material_id: string;
  material_no: string;
  width_mm: number;
  thickness_mm: number;
  urgent_level: string;
  violation_type: string;
  anchor_width_mm: number;
  anchor_thickness_mm: number;
  width_delta_mm: number;
  thickness_delta_mm: number;
};

const urgentLevelColors: Record<string, string> = {
  L3: 'red',
  L2: 'orange',
  L1: 'gold',
  L0: 'default',
};

const violationLabels: Record<string, { text: string; color: string }> = {
  WIDTH_EXCEEDED: { text: '宽度超限', color: 'volcano' },
  THICKNESS_EXCEEDED: { text: '厚度超限', color: 'magenta' },
  BOTH_EXCEEDED: { text: '宽厚均超限', color: 'red' },
  UNKNOWN: { text: '未知', color: 'default' },
};

function normalizeUrgentLevel(raw: unknown): string {
  const v = String(raw || '').trim().toUpperCase();
  if (!v) return 'L0';
  if (!/^L[0-3]$/.test(v)) return 'L0';
  return v;
}

function fmtNum(v: unknown, digits: number = 1): string {
  const n = typeof v === 'number' ? v : Number(v);
  if (!Number.isFinite(n)) return '-';
  return n.toFixed(digits);
}

export type PathOverrideConfirmModalProps = {
  open: boolean;
  onClose: () => void;
  versionId: string | null;
  machineCode: string | null;
  planDate: string | null; // YYYY-MM-DD
  operator: string;
  onConfirmed?: (result: { confirmedCount: number; autoRecalc: boolean }) => void;
};

const PathOverrideConfirmModal: React.FC<PathOverrideConfirmModalProps> = ({
  open,
  onClose,
  versionId,
  machineCode,
  planDate,
  operator,
  onConfirmed,
}) => {
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [rows, setRows] = useState<PendingRow[]>([]);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [reason, setReason] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [autoRecalc, setAutoRecalc] = useState(true);

  const canQuery = !!(versionId && machineCode && planDate);

  const anchorSummary = useMemo(() => {
    if (rows.length === 0) return null;
    const r = rows[0];
    return { w: r.anchor_width_mm, t: r.anchor_thickness_mm };
  }, [rows]);

  const loadPending = async () => {
    if (!canQuery) {
      setRows([]);
      return;
    }
    setLoading(true);
    setLoadError(null);
    try {
      const data = await pathRuleApi.listPathOverridePending({
        versionId: versionId!,
        machineCode: machineCode!,
        planDate: planDate!,
      });
      const list: PendingRow[] = Array.isArray(data)
        ? data
            .filter((r) => !!r.material_id)
            .map((r) => ({
              material_id: r.material_id,
              material_no: r.material_no || r.material_id,
              width_mm: r.width_mm,
              thickness_mm: r.thickness_mm,
              urgent_level: normalizeUrgentLevel(r.urgent_level),
              violation_type: r.violation_type?.toUpperCase() || 'UNKNOWN',
              anchor_width_mm: r.anchor_width_mm,
              anchor_thickness_mm: r.anchor_thickness_mm,
              width_delta_mm: r.width_delta_mm,
              thickness_delta_mm: r.thickness_delta_mm,
            }))
        : [];
      setRows(list);
      setSelectedIds((prev) => prev.filter((id) => list.some((r) => r.material_id === id)));
    } catch (e: unknown) {
      console.error('[PathOverrideConfirmModal] loadPending failed:', e);
      setRows([]);
      const errorMessage = e instanceof Error ? e.message : String(e);
      setLoadError(errorMessage || '加载待确认列表失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!open) return;
    setReason('');
    setSelectedIds([]);
    setAutoRecalc(true);
    void loadPending();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, versionId, machineCode, planDate]);

  const confirmSingle = async (materialId: string) => {
    if (!versionId) return;
    if (!materialId) return;
    const cleanReason = String(reason || '').trim();
    if (!cleanReason) {
      message.warning('请输入确认原因');
      return;
    }

    setSubmitting(true);
    try {
      await pathRuleApi.confirmPathOverride({
        versionId,
        materialId,
        confirmedBy: operator || 'system',
        reason: cleanReason,
      });
      message.success('已确认突破');
      onConfirmed?.({ confirmedCount: 1, autoRecalc });
      await loadPending();
    } catch (e: unknown) {
      console.error('[PathOverrideConfirmModal] confirmSingle failed:', e);
      const errorMessage = e instanceof Error ? e.message : String(e);
      message.error(errorMessage || '确认失败');
    } finally {
      setSubmitting(false);
    }
  };

  const confirmBatch = async (materialIds: string[]) => {
    if (!versionId) return;
    const ids = Array.from(new Set(materialIds.map((id) => String(id || '').trim()))).filter(Boolean);
    if (ids.length === 0) {
      message.warning('请选择要确认的物料');
      return;
    }
    const cleanReason = String(reason || '').trim();
    if (!cleanReason) {
      message.warning('请输入确认原因');
      return;
    }

    setSubmitting(true);
    try {
      const res = await pathRuleApi.batchConfirmPathOverride({
        versionId,
        materialIds: ids,
        confirmedBy: operator || 'system',
        reason: cleanReason,
      });
      const ok = Number(res?.success_count ?? 0);
      const fail = Number(res?.fail_count ?? 0);
      if (ok > 0) message.success(`已确认 ${ok} 条突破`);
      if (fail > 0) {
        const failed = Array.isArray(res?.failed_material_ids) ? res.failed_material_ids : [];
        Modal.info({
          title: '部分确认失败',
          content: (
            <div>
              <p style={{ marginBottom: 8 }}>成功 {ok} 条，失败 {fail} 条。</p>
              {failed.length > 0 ? (
                <pre style={{ whiteSpace: 'pre-wrap', margin: 0 }}>
                  {failed.filter(Boolean).join('\n')}
                </pre>
              ) : null}
            </div>
          ),
        });
      }
      onConfirmed?.({ confirmedCount: ok, autoRecalc });
      setSelectedIds([]);
      await loadPending();
    } catch (e: unknown) {
      console.error('[PathOverrideConfirmModal] confirmBatch failed:', e);
      const errorMessage = e instanceof Error ? e.message : String(e);
      message.error(errorMessage || '批量确认失败');
    } finally {
      setSubmitting(false);
    }
  };

  const columns: ColumnsType<PendingRow> = [
    {
      title: '物料',
      dataIndex: 'material_no',
      key: 'material_no',
      width: 160,
      render: (v) => <Typography.Text code>{String(v || '-')}</Typography.Text>,
    },
    {
      title: '紧急度',
      dataIndex: 'urgent_level',
      key: 'urgent_level',
      width: 90,
      render: (v) => {
        const level = normalizeUrgentLevel(v);
        return <Tag color={urgentLevelColors[level] || 'default'}>{level}</Tag>;
      },
    },
    {
      title: '宽/厚 (mm)',
      key: 'wt',
      width: 140,
      render: (_, r) => (
        <Space size={6}>
          <span>W {fmtNum(r.width_mm, 1)}</span>
          <span>T {fmtNum(r.thickness_mm, 2)}</span>
        </Space>
      ),
    },
    {
      title: '违规类型',
      dataIndex: 'violation_type',
      key: 'violation_type',
      width: 120,
      render: (v) => {
        const key = String(v || 'UNKNOWN').toUpperCase();
        const meta = violationLabels[key] || violationLabels.UNKNOWN;
        return <Tag color={meta.color}>{meta.text}</Tag>;
      },
    },
    {
      title: '锚点 (mm)',
      key: 'anchor',
      width: 150,
      render: (_, r) => (
        <Space size={6}>
          <span>W {fmtNum(r.anchor_width_mm, 1)}</span>
          <span>T {fmtNum(r.anchor_thickness_mm, 2)}</span>
        </Space>
      ),
    },
    {
      title: 'Δ宽/Δ厚 (mm)',
      key: 'delta',
      width: 150,
      render: (_, r) => (
        <Space size={6}>
          <Tag color="volcano">ΔW {fmtNum(r.width_delta_mm, 1)}</Tag>
          <Tag color="magenta">ΔT {fmtNum(r.thickness_delta_mm, 2)}</Tag>
        </Space>
      ),
    },
    {
      title: '操作',
      key: 'action',
      width: 90,
      fixed: 'right',
      render: (_, r) => (
        <Button
          size="small"
          type="primary"
          disabled={submitting}
          onClick={() => confirmSingle(r.material_id)}
        >
          确认
        </Button>
      ),
    },
  ];

  return (
    <Modal
      open={open}
      title="路径规则：人工确认突破"
      width={980}
      onCancel={onClose}
      footer={
        <Space style={{ width: '100%', justifyContent: 'space-between' }}>
          <Space>
            <Button icon={<ReloadOutlined />} onClick={loadPending} disabled={!canQuery || loading || submitting}>
              刷新列表
            </Button>
            <Button onClick={() => confirmBatch(selectedIds)} type="primary" disabled={selectedIds.length === 0 || submitting}>
              确认所选 ({selectedIds.length})
            </Button>
            <Button onClick={() => confirmBatch(rows.map((r) => r.material_id))} disabled={rows.length === 0 || submitting}>
              全部确认 ({rows.length})
            </Button>
          </Space>
          <Button onClick={onClose}>关闭</Button>
        </Space>
      }
    >
      {!versionId ? (
        <Alert type="warning" showIcon message="尚未选择版本" />
      ) : !machineCode ? (
        <Alert type="warning" showIcon message="请先在工作台选择机组" />
      ) : !planDate ? (
        <Alert type="warning" showIcon message="请先选择计划日期" />
      ) : null}

      {canQuery ? (
        <div style={{ marginTop: 12 }}>
          <Space wrap size={10}>
            <Typography.Text type="secondary">版本</Typography.Text>
            <Typography.Text code>{versionId}</Typography.Text>
            <Typography.Text type="secondary">机组</Typography.Text>
            <Tag color="blue">{machineCode}</Tag>
            <Typography.Text type="secondary">日期</Typography.Text>
            <Tag>{planDate}</Tag>
            {anchorSummary ? (
              <>
                <Typography.Text type="secondary">当前锚点</Typography.Text>
                <Tag color="geekblue">
                  W {fmtNum(anchorSummary.w, 1)} / T {fmtNum(anchorSummary.t, 2)}
                </Tag>
              </>
            ) : null}
          </Space>
        </div>
      ) : null}

      <div style={{ marginTop: 12 }}>
        <Space direction="vertical" style={{ width: '100%' }} size={10}>
          <Input.TextArea
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            rows={2}
            placeholder="确认原因（必填，例如：客户交付临期/现场指令/工艺例外审批等）"
            maxLength={200}
            showCount
          />
          <Checkbox checked={autoRecalc} onChange={(e) => setAutoRecalc(e.target.checked)}>
            确认后自动重算（生成新版本并切换）
          </Checkbox>

          {loadError ? <Alert type="error" showIcon message={loadError} /> : null}

          <Table
            rowKey="material_id"
            size="small"
            loading={loading}
            columns={columns}
            dataSource={rows}
            pagination={{ pageSize: 8, showSizeChanger: true }}
            rowSelection={{
              selectedRowKeys: selectedIds,
              onChange: (keys) => setSelectedIds(keys.map((k) => String(k))),
            }}
            scroll={{ x: 920 }}
          />
        </Space>
      </div>
    </Modal>
  );
};

export default PathOverrideConfirmModal;


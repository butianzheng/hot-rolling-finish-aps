import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Checkbox, Input, Modal, Select, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { ReloadOutlined } from '@ant-design/icons';
import { pathRuleApi } from '../../api/tauri';

type SummaryRow = {
  machine_code: string;
  plan_date: string; // YYYY-MM-DD
  pending_count: number;
};

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
  if (!/^L[0-3]$/.test(v)) return 'L0';
  return v;
}

function fmtNum(v: unknown, digits: number = 1): string {
  const n = typeof v === 'number' ? v : Number(v);
  if (!Number.isFinite(n)) return '-';
  return n.toFixed(digits);
}

export type PathOverridePendingCenterModalProps = {
  open: boolean;
  onClose: () => void;
  versionId: string | null;
  planDateFrom: string; // YYYY-MM-DD
  planDateTo: string; // YYYY-MM-DD
  machineOptions: string[];
  operator: string;
  onConfirmed?: (result: { confirmedCount: number; autoRecalc: boolean; recalcBaseDate: string }) => void;
};

const PathOverridePendingCenterModal: React.FC<PathOverridePendingCenterModalProps> = ({
  open,
  onClose,
  versionId,
  planDateFrom,
  planDateTo,
  machineOptions,
  operator,
  onConfirmed,
}) => {
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [rows, setRows] = useState<SummaryRow[]>([]);
  const [selectedMachineCodes, setSelectedMachineCodes] = useState<string[]>([]);

  const [selectedGroup, setSelectedGroup] = useState<{ machineCode: string; planDate: string } | null>(null);
  const [detailRows, setDetailRows] = useState<PendingRow[]>([]);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);

  const [reason, setReason] = useState('');
  const [autoRecalc, setAutoRecalc] = useState(true);
  const [submitting, setSubmitting] = useState(false);

  const canQuery = !!versionId;

  const totalPendingCount = useMemo(() => {
    return rows.reduce((sum, r) => sum + (Number(r.pending_count) || 0), 0);
  }, [rows]);

  const earliestPendingDate = useMemo(() => {
    if (rows.length === 0) return planDateFrom;
    const dates = rows.map((r) => String(r.plan_date || '').trim()).filter(Boolean).sort();
    return dates[0] || planDateFrom;
  }, [planDateFrom, rows]);

  const loadSummary = async () => {
    if (!canQuery) {
      setRows([]);
      return;
    }

    setLoading(true);
    setLoadError(null);
    try {
      const data = await pathRuleApi.listPathOverridePendingSummary({
        versionId: versionId!,
        planDateFrom,
        planDateTo,
        machineCodes: selectedMachineCodes.length > 0 ? selectedMachineCodes : undefined,
      });
      const list: SummaryRow[] = Array.isArray(data)
        ? data
            .map((r: any) => ({
              machine_code: String(r?.machine_code ?? '').trim(),
              plan_date: String(r?.plan_date ?? '').trim(),
              pending_count: Number(r?.pending_count ?? 0),
            }))
            .filter((r) => !!r.machine_code && !!r.plan_date)
        : [];
      list.sort((a, b) => {
        const d = String(a.plan_date).localeCompare(String(b.plan_date));
        if (d !== 0) return d;
        return String(a.machine_code).localeCompare(String(b.machine_code));
      });
      setRows(list);
    } catch (e: any) {
      console.error('[PathOverridePendingCenterModal] loadSummary failed:', e);
      setRows([]);
      setLoadError(String(e?.message || e || '加载待确认汇总失败'));
    } finally {
      setLoading(false);
    }
  };

  const loadDetail = async (machineCode: string, planDate: string) => {
    if (!versionId) return;
    const mc = String(machineCode || '').trim();
    const dt = String(planDate || '').trim();
    if (!mc || !dt) return;

    setDetailLoading(true);
    setDetailError(null);
    try {
      const data = await pathRuleApi.listPathOverridePending({
        versionId,
        machineCode: mc,
        planDate: dt,
      });
      const list: PendingRow[] = Array.isArray(data)
        ? data
            .map((r: any) => ({
              material_id: String(r?.material_id ?? ''),
              material_no: String(r?.material_no ?? r?.material_id ?? ''),
              width_mm: Number(r?.width_mm ?? 0),
              thickness_mm: Number(r?.thickness_mm ?? 0),
              urgent_level: normalizeUrgentLevel(r?.urgent_level),
              violation_type: String(r?.violation_type ?? 'UNKNOWN').toUpperCase() || 'UNKNOWN',
              anchor_width_mm: Number(r?.anchor_width_mm ?? 0),
              anchor_thickness_mm: Number(r?.anchor_thickness_mm ?? 0),
              width_delta_mm: Number(r?.width_delta_mm ?? 0),
              thickness_delta_mm: Number(r?.thickness_delta_mm ?? 0),
            }))
            .filter((r) => !!r.material_id)
        : [];
      setDetailRows(list);
    } catch (e: any) {
      console.error('[PathOverridePendingCenterModal] loadDetail failed:', e);
      setDetailRows([]);
      setDetailError(String(e?.message || e || '加载明细失败'));
    } finally {
      setDetailLoading(false);
    }
  };

  useEffect(() => {
    if (!open) return;
    setReason('');
    setAutoRecalc(true);
    setSelectedGroup(null);
    setDetailRows([]);
    setDetailError(null);
    void loadSummary();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, versionId, planDateFrom, planDateTo]);

  useEffect(() => {
    if (!open) return;
    void loadSummary();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedMachineCodes]);

  const confirmAll = async () => {
    if (!versionId) return;
    const cleanReason = String(reason || '').trim();
    if (!cleanReason) {
      message.warning('请输入确认原因');
      return;
    }

    setSubmitting(true);
    try {
      const res = await pathRuleApi.batchConfirmPathOverrideByRange({
        versionId,
        planDateFrom,
        planDateTo,
        machineCodes: selectedMachineCodes.length > 0 ? selectedMachineCodes : undefined,
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
                  {failed.map((id: any) => String(id || '')).filter(Boolean).join('\n')}
                </pre>
              ) : null}
            </div>
          ),
        });
      }

      onConfirmed?.({ confirmedCount: ok, autoRecalc, recalcBaseDate: earliestPendingDate });
      await loadSummary();

      if (selectedGroup) {
        await loadDetail(selectedGroup.machineCode, selectedGroup.planDate);
      }
    } catch (e: any) {
      console.error('[PathOverridePendingCenterModal] confirmAll failed:', e);
      message.error(String(e?.message || e || '批量确认失败'));
    } finally {
      setSubmitting(false);
    }
  };

  const summaryColumns: ColumnsType<SummaryRow> = [
    {
      title: '日期',
      dataIndex: 'plan_date',
      key: 'plan_date',
      width: 120,
      render: (v) => <Tag>{String(v || '-')}</Tag>,
    },
    {
      title: '机组',
      dataIndex: 'machine_code',
      key: 'machine_code',
      width: 100,
      render: (v) => <Tag color="blue">{String(v || '-')}</Tag>,
    },
    {
      title: '待确认数',
      dataIndex: 'pending_count',
      key: 'pending_count',
      width: 110,
      render: (v) => <Typography.Text strong>{Number(v || 0)}</Typography.Text>,
    },
    {
      title: '操作',
      key: 'action',
      width: 90,
      fixed: 'right',
      render: (_, r) => (
        <Button
          size="small"
          onClick={() => {
            const mc = String(r.machine_code || '').trim();
            const dt = String(r.plan_date || '').trim();
            if (!mc || !dt) return;
            setSelectedGroup({ machineCode: mc, planDate: dt });
            void loadDetail(mc, dt);
          }}
        >
          查看
        </Button>
      ),
    },
  ];

  const detailColumns: ColumnsType<PendingRow> = [
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
  ];

  return (
    <Modal
      open={open}
      title="路径规则：待确认中心（跨日期/跨机组）"
      width={1050}
      onCancel={onClose}
      footer={
        <Space style={{ width: '100%', justifyContent: 'space-between' }}>
          <Space>
            <Button icon={<ReloadOutlined />} onClick={loadSummary} disabled={!canQuery || loading || submitting}>
              刷新
            </Button>
            <Button
              type="primary"
              onClick={confirmAll}
              disabled={!canQuery || totalPendingCount === 0 || submitting}
              loading={submitting}
            >
              全部确认 ({totalPendingCount})
            </Button>
          </Space>
          <Button onClick={onClose}>关闭</Button>
        </Space>
      }
    >
      {!versionId ? <Alert type="warning" showIcon message="尚未选择版本" /> : null}

      <Space direction="vertical" style={{ width: '100%', marginTop: 12 }} size={10}>
        <Alert
          type="info"
          showIcon
          message="说明"
          description="此处展示“最近一次重算”落库的 PATH_OVERRIDE_REQUIRED 待确认清单（按首次遇到日期汇总）。确认后建议执行“一键优化/重算”生成新版本以生效。"
        />

        <Space wrap size={10}>
          <Typography.Text type="secondary">范围</Typography.Text>
          <Tag>{planDateFrom}</Tag>
          <Typography.Text type="secondary">~</Typography.Text>
          <Tag>{planDateTo}</Tag>
          <Typography.Text type="secondary">机组过滤</Typography.Text>
          <Select
            mode="multiple"
            allowClear
            placeholder="全部机组"
            style={{ width: 320 }}
            value={selectedMachineCodes}
            onChange={(v) => setSelectedMachineCodes(v)}
            options={machineOptions.map((c) => ({ label: c, value: c }))}
          />
          <Typography.Text type="secondary">建议重算起点</Typography.Text>
          <Tag color="geekblue">{earliestPendingDate}</Tag>
        </Space>

        {loadError ? <Alert type="error" showIcon message={loadError} /> : null}

        <Table
          rowKey={(r) => `${r.plan_date}-${r.machine_code}`}
          size="small"
          loading={loading}
          columns={summaryColumns}
          dataSource={rows}
          pagination={{ pageSize: 8, showSizeChanger: true }}
          scroll={{ x: 900 }}
        />

        {selectedGroup ? (
          <div>
            <Space wrap size={10} style={{ marginBottom: 8 }}>
              <Typography.Text type="secondary">明细</Typography.Text>
              <Tag color="blue">{selectedGroup.machineCode}</Tag>
              <Tag>{selectedGroup.planDate}</Tag>
              <Button
                size="small"
                onClick={() => loadDetail(selectedGroup.machineCode, selectedGroup.planDate)}
                loading={detailLoading}
                disabled={!canQuery || submitting}
              >
                刷新明细
              </Button>
            </Space>
            {detailError ? <Alert type="error" showIcon message={detailError} style={{ marginBottom: 8 }} /> : null}
            <Table
              rowKey="material_id"
              size="small"
              loading={detailLoading}
              columns={detailColumns}
              dataSource={detailRows}
              pagination={{ pageSize: 6, showSizeChanger: true }}
              scroll={{ x: 880 }}
            />
          </div>
        ) : null}

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
      </Space>
    </Modal>
  );
};

export default PathOverridePendingCenterModal;


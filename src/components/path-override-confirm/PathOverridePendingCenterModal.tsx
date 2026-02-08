import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Input, Modal, Select, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { ReloadOutlined, SettingOutlined } from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
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

function formatUrgencyLevel(level: string): string {
  const normalized = normalizeUrgentLevel(level);
  const mapping: Record<string, string> = {
    L3: '三级紧急',
    L2: '二级紧急',
    L1: '一级紧急',
    L0: '常规',
  };
  return mapping[normalized] || normalized;
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
  onRejected?: (result: { rejectedCount: number; autoRecalc: boolean; recalcBaseDate: string }) => void;
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
  onRejected,
}) => {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [rows, setRows] = useState<SummaryRow[]>([]);
  const [selectedMachineCodes, setSelectedMachineCodes] = useState<string[]>([]);

  const [selectedGroup, setSelectedGroup] = useState<{ machineCode: string; planDate: string } | null>(null);
  const [detailRows, setDetailRows] = useState<PendingRow[]>([]);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);

  const [reason, setReason] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [recalcFailed, setRecalcFailed] = useState(false);

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
            .filter((r) => !!r.machine_code && !!r.plan_date)
            .map((r) => ({
              machine_code: r.machine_code.trim(),
              plan_date: r.plan_date.trim(),
              pending_count: r.pending_count,
            }))
        : [];
      list.sort((a, b) => {
        const d = String(a.plan_date).localeCompare(String(b.plan_date));
        if (d !== 0) return d;
        return String(a.machine_code).localeCompare(String(b.machine_code));
      });
      setRows(list);
    } catch (e: unknown) {
      console.error('【路径放行待确认中心】加载摘要失败：', e);
      setRows([]);
      const errorMessage = e instanceof Error ? e.message : String(e);
      setLoadError(errorMessage || '加载待确认汇总失败');
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
      setDetailRows(list);
    } catch (e: unknown) {
      console.error('【路径放行待确认中心】加载明细失败：', e);
      setDetailRows([]);
      const errorMessage = e instanceof Error ? e.message : String(e);
      setDetailError(errorMessage || '加载明细失败');
    } finally {
      setDetailLoading(false);
    }
  };

  useEffect(() => {
    if (!open) return;
    setReason('');
    setRecalcFailed(false);
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

  const confirmAndRecalc = async () => {
    if (!versionId) return;
    const cleanReason = String(reason || '').trim();
    if (!cleanReason) {
      message.warning('请输入确认原因');
      return;
    }

    setSubmitting(true);
    setRecalcFailed(false);
    try {
      // Step 1: 批量确认
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
                  {failed.filter(Boolean).join('\n')}
                </pre>
              ) : null}
            </div>
          ),
        });
      }

      // Step 2: 自动触发重算
      onConfirmed?.({ confirmedCount: ok, autoRecalc: true, recalcBaseDate: earliestPendingDate });

      await loadSummary();
      if (selectedGroup) {
        await loadDetail(selectedGroup.machineCode, selectedGroup.planDate);
      }
    } catch (e: unknown) {
      console.error('【路径放行待确认中心】确认并重算失败：', e);
      const errorMessage = e instanceof Error ? e.message : String(e);
      message.error(errorMessage || '批量确认失败');
      setRecalcFailed(true);
    } finally {
      setSubmitting(false);
    }
  };

  const confirmAll = async () => {
    if (!versionId) return;
    const cleanReason = String(reason || '').trim();
    if (!cleanReason) {
      message.warning('请输入确认原因');
      return;
    }

    setSubmitting(true);
    setRecalcFailed(false);
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
                  {failed.filter(Boolean).join('\n')}
                </pre>
              ) : null}
            </div>
          ),
        });
      }

      onConfirmed?.({ confirmedCount: ok, autoRecalc: false, recalcBaseDate: earliestPendingDate });
      await loadSummary();

      if (selectedGroup) {
        await loadDetail(selectedGroup.machineCode, selectedGroup.planDate);
      }
    } catch (e: unknown) {
      console.error('【路径放行待确认中心】全部确认失败：', e);
      const errorMessage = e instanceof Error ? e.message : String(e);
      message.error(errorMessage || '批量确认失败');
    } finally {
      setSubmitting(false);
    }
  };

  const rejectByRange = async (autoRecalcAfterReject: boolean) => {
    if (!versionId) return;
    const cleanReason = String(reason || '').trim();
    if (!cleanReason) {
      message.warning('请输入拒绝原因');
      return;
    }

    setSubmitting(true);
    setRecalcFailed(false);
    try {
      const res = await pathRuleApi.batchRejectPathOverrideByRange({
        versionId,
        planDateFrom,
        planDateTo,
        machineCodes: selectedMachineCodes.length > 0 ? selectedMachineCodes : undefined,
        rejectedBy: operator || 'system',
        reason: cleanReason,
      });
      const ok = Number(res?.success_count ?? 0);
      const fail = Number(res?.fail_count ?? 0);
      if (ok > 0) message.success(`已拒绝 ${ok} 条突破`);
      if (fail > 0) {
        const failed = Array.isArray(res?.failed_material_ids) ? res.failed_material_ids : [];
        Modal.info({
          title: '部分拒绝失败',
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

      onRejected?.({ rejectedCount: ok, autoRecalc: autoRecalcAfterReject, recalcBaseDate: earliestPendingDate });
      await loadSummary();
      if (selectedGroup) {
        await loadDetail(selectedGroup.machineCode, selectedGroup.planDate);
      }
    } catch (e: unknown) {
      console.error('【路径放行待确认中心】范围拒绝失败：', e);
      const errorMessage = e instanceof Error ? e.message : String(e);
      message.error(errorMessage || '批量拒绝失败');
      setRecalcFailed(autoRecalcAfterReject);
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
        return <Tag color={urgentLevelColors[level] || 'default'}>{formatUrgencyLevel(level)}</Tag>;
      },
    },
    {
      title: '宽/厚（毫米）',
      key: 'wt',
      width: 140,
      render: (_, r) => (
        <Space size={6}>
          <span>宽 {fmtNum(r.width_mm, 1)}</span>
          <span>厚 {fmtNum(r.thickness_mm, 2)}</span>
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
      title: '锚点（毫米）',
      key: 'anchor',
      width: 150,
      render: (_, r) => (
        <Space size={6}>
          <span>宽 {fmtNum(r.anchor_width_mm, 1)}</span>
          <span>厚 {fmtNum(r.anchor_thickness_mm, 2)}</span>
        </Space>
      ),
    },
    {
      title: '宽差/厚差（毫米）',
      key: 'delta',
      width: 150,
      render: (_, r) => (
        <Space size={6}>
          <Tag color="volcano">宽差 {fmtNum(r.width_delta_mm, 1)}</Tag>
          <Tag color="magenta">厚差 {fmtNum(r.thickness_delta_mm, 2)}</Tag>
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
              onClick={confirmAndRecalc}
              disabled={!canQuery || totalPendingCount === 0 || submitting}
              loading={submitting}
            >
              确认并重算 ({totalPendingCount})
            </Button>
            <Button
              onClick={confirmAll}
              disabled={!canQuery || totalPendingCount === 0 || submitting}
            >
              仅确认（不重算）
            </Button>
            <Button
              danger
              onClick={() => rejectByRange(true)}
              disabled={!canQuery || totalPendingCount === 0 || submitting}
            >
              拒绝并重算 ({totalPendingCount})
            </Button>
            <Button
              danger
              type="dashed"
              onClick={() => rejectByRange(false)}
              disabled={!canQuery || totalPendingCount === 0 || submitting}
            >
              仅拒绝（不重算）
            </Button>
          </Space>
          <Space>
            <Button
              icon={<SettingOutlined />}
              onClick={() => {
                const params = new URLSearchParams({ tab: 'path_rule' });
                // 携带上下文：如果有选中的组合，使用它；否则使用范围的第一个日期
                const contextDate = selectedGroup?.planDate || earliestPendingDate;
                const contextMachine = selectedGroup?.machineCode || (rows.length > 0 ? rows[0].machine_code : undefined);
                if (contextMachine) params.set('machine_code', contextMachine);
                if (contextDate) params.set('plan_date', contextDate);
                navigate(`/settings?${params.toString()}`);
              }}
            >
              配置路径规则
            </Button>
            <Button onClick={onClose}>关闭</Button>
          </Space>
        </Space>
      }
    >
      {!versionId ? <Alert type="warning" showIcon message="尚未选择版本" /> : null}

      <Space direction="vertical" style={{ width: '100%', marginTop: 12 }} size={10}>
        <Alert
          type="info"
          showIcon
          message="说明"
          description={
            <div>
              <p style={{ marginBottom: 4 }}>
                此处展示“最近一次重算”落库的路径规则待确认清单（按首次遇到日期汇总）。
              </p>
              <p style={{ marginBottom: 0 }}>
                <strong>快捷流程</strong>：点击"确认并重算"可一键完成确认+重算+版本切换，适合大部分场景。
              </p>
            </div>
          }
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
          placeholder="确认/拒绝原因（必填，例如：客户交付临期/现场指令/工艺例外审批等）"
          maxLength={200}
          showCount
        />

        {recalcFailed && (
          <Alert
            type="warning"
            showIcon
            message="重算失败"
            description='确认已保存，但重算失败。请稍后在工作台手动执行"一键优化"或联系管理员。'
          />
        )}
      </Space>
    </Modal>
  );
};

export default PathOverridePendingCenterModal;

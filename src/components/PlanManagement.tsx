import React, { useMemo, useState, useEffect } from 'react';
import { useDebounce } from '../hooks/useDebounce';
import { Alert, Button, Card, Descriptions, Divider, Input, InputNumber, Modal, Select, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { ExclamationCircleOutlined } from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import type { EChartsOption } from 'echarts';
import { capacityApi, planApi } from '../api/tauri';
import { useCurrentUser, useGlobalActions } from '../stores/use-global-store';
import dayjs from 'dayjs';
import { formatDate } from '../utils/formatters';
import type { BackendVersionComparisonKpiResult, BackendVersionComparisonResult, PlanItemSnapshot, VersionDiff } from '../types/comparison';
import { exportCSV, exportJSON, exportHTML, exportMarkdown } from '../utils/exportUtils';

const LazyECharts = React.lazy(() => import('echarts-for-react'));

const Chart: React.FC<{ option: EChartsOption; height: number }> = ({ option, height }) => {
  return (
    <React.Suspense
      fallback={
        <div
          style={{
            height,
            width: '100%',
            background: '#fafafa',
            border: '1px dashed #d9d9d9',
            borderRadius: 6,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#8c8c8c',
            fontSize: 12,
          }}
        >
          图表加载中…
        </div>
      }
    >
      <LazyECharts option={option} style={{ height, width: '100%' }} notMerge lazyUpdate />
    </React.Suspense>
  );
};

interface Plan {
  plan_id: string;
  plan_name: string;
  created_by: string;
  created_at: string;
}

interface Version {
  version_id: string;
  version_no: number;
  status: string;
  recalc_window_days: number;
  created_at: string;
  config_snapshot_json?: string | null;
}

const RETROSPECTIVE_NOTE_KEY_PREFIX = 'aps_retrospective_note';

function makeRetrospectiveKey(versionIdA: string, versionIdB: string): string {
  const [a, b] = [String(versionIdA || ''), String(versionIdB || '')].sort();
  return `${RETROSPECTIVE_NOTE_KEY_PREFIX}__${a}__${b}`;
}

type LocalVersionDiffSummary = {
  totalChanges: number;
  addedCount: number;
  removedCount: number;
  modifiedCount: number;
  movedCount: number;
};

type LocalCapacityDeltaRow = {
  machine_code: string;
  date: string;
  used_a: number;
  used_b: number;
  delta: number;
  target_a: number | null;
  limit_a: number | null;
  target_b: number | null;
  limit_b: number | null;
};

function normalizeDateOnly(date: string): string {
  const trimmed = String(date || '').trim();
  if (!trimmed) return '';
  if (/^\d{4}-\d{2}-\d{2}/.test(trimmed)) return trimmed.slice(0, 10);
  return trimmed;
}

function extractVersionNameCn(version: any): string | null {
  const raw = version?.config_snapshot_json;
  if (raw == null) return null;
  const text = String(raw || '').trim();
  if (!text) return null;
  try {
    const obj = JSON.parse(text);
    const v = obj?.__meta_version_name_cn;
    if (typeof v === 'string' && v.trim()) return v.trim();
    return null;
  } catch {
    return null;
  }
}

function formatVersionLabel(version: Version): string {
  const nameCn = extractVersionNameCn(version);
  if (nameCn) return nameCn;
  const no = Number(version.version_no ?? 0);
  if (Number.isFinite(no) && no > 0) return `V${no}`;
  return version.version_id;
}

function normalizePlanItem(raw: any): PlanItemSnapshot | null {
  const id = String(raw?.material_id ?? '').trim();
  if (!id) return null;
  return {
    material_id: id,
    machine_code: String(raw?.machine_code ?? ''),
    plan_date: normalizeDateOnly(String(raw?.plan_date ?? '')),
    seq_no: Number(raw?.seq_no ?? 0),
    weight_t: raw?.weight_t == null ? undefined : Number(raw.weight_t),
    urgent_level: raw?.urgent_level == null ? undefined : String(raw.urgent_level),
    locked_in_plan: raw?.locked_in_plan == null ? undefined : !!raw.locked_in_plan,
    force_release_in_plan: raw?.force_release_in_plan == null ? undefined : !!raw.force_release_in_plan,
    sched_state: raw?.sched_state == null ? undefined : String(raw.sched_state),
    assign_reason: raw?.assign_reason == null ? undefined : String(raw.assign_reason),
  };
}

function computeVersionDiffs(itemsA: PlanItemSnapshot[], itemsB: PlanItemSnapshot[]): { diffs: VersionDiff[]; summary: LocalVersionDiffSummary } {
  const mapA = new Map<string, PlanItemSnapshot>();
  const mapB = new Map<string, PlanItemSnapshot>();
  itemsA.forEach((it) => mapA.set(it.material_id, it));
  itemsB.forEach((it) => mapB.set(it.material_id, it));

  const allIds = new Set<string>([...mapA.keys(), ...mapB.keys()]);
  const diffs: VersionDiff[] = [];

  const isDifferent = (a: unknown, b: unknown) => {
    if (a === b) return false;
    if (a == null && b == null) return false;
    return String(a ?? '') !== String(b ?? '');
  };

  const isWeightDifferent = (a: number | undefined, b: number | undefined) => {
    const na = a == null || !Number.isFinite(a) ? null : Number(a);
    const nb = b == null || !Number.isFinite(b) ? null : Number(b);
    if (na == null && nb == null) return false;
    if (na == null || nb == null) return true;
    return Math.abs(na - nb) > 1e-6;
  };

  Array.from(allIds)
    .sort()
    .forEach((id) => {
      const a = mapA.get(id) ?? null;
      const b = mapB.get(id) ?? null;

      if (!a && b) {
        diffs.push({
          materialId: id,
          changeType: 'ADDED',
          previousState: null,
          currentState: b,
        });
        return;
      }

      if (a && !b) {
        diffs.push({
          materialId: id,
          changeType: 'REMOVED',
          previousState: a,
          currentState: null,
        });
        return;
      }

      if (!a || !b) return;

      const moved =
        isDifferent(a.machine_code, b.machine_code) ||
        isDifferent(a.plan_date, b.plan_date) ||
        Number(a.seq_no ?? 0) !== Number(b.seq_no ?? 0);

      const modified =
        !moved &&
        (isWeightDifferent(a.weight_t, b.weight_t) ||
          isDifferent(a.urgent_level, b.urgent_level) ||
          isDifferent(a.locked_in_plan, b.locked_in_plan) ||
          isDifferent(a.force_release_in_plan, b.force_release_in_plan) ||
          isDifferent(a.sched_state, b.sched_state) ||
          isDifferent(a.assign_reason, b.assign_reason));

      if (!moved && !modified) return;

      diffs.push({
        materialId: id,
        changeType: moved ? 'MOVED' : 'MODIFIED',
        previousState: a,
        currentState: b,
      });
    });

  const summary: LocalVersionDiffSummary = diffs.reduce(
    (acc, d) => {
      acc.totalChanges += 1;
      if (d.changeType === 'ADDED') acc.addedCount += 1;
      else if (d.changeType === 'REMOVED') acc.removedCount += 1;
      else if (d.changeType === 'MOVED') acc.movedCount += 1;
      else acc.modifiedCount += 1;
      return acc;
    },
    { totalChanges: 0, addedCount: 0, removedCount: 0, modifiedCount: 0, movedCount: 0 }
  );

  return { diffs, summary };
}

function computeCapacityMap(items: PlanItemSnapshot[]): Map<string, number> {
  const map = new Map<string, number>();
  items.forEach((it) => {
    const machine = String(it.machine_code ?? '').trim();
    const date = normalizeDateOnly(String(it.plan_date ?? ''));
    if (!machine || !date) return;
    const weight = Number(it.weight_t ?? 0);
    if (!Number.isFinite(weight) || weight <= 0) return;
    const key = `${machine}__${date}`;
    map.set(key, (map.get(key) ?? 0) + weight);
  });
  return map;
}

function computeDailyTotals(items: PlanItemSnapshot[]): Map<string, number> {
  const map = new Map<string, number>();
  items.forEach((it) => {
    const date = normalizeDateOnly(String(it.plan_date ?? ''));
    if (!date) return;
    const weight = Number(it.weight_t ?? 0);
    if (!Number.isFinite(weight) || weight <= 0) return;
    map.set(date, (map.get(date) ?? 0) + weight);
  });
  return map;
}

const PlanManagement: React.FC = () => {
  const navigate = useNavigate();
  const [plans, setPlans] = useState<Plan[]>([]);
  const [filteredPlans, setFilteredPlans] = useState<Plan[]>([]);
  const [versions, setVersions] = useState<Version[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedPlanId, setSelectedPlanId] = useState<string | null>(null);
  const [createPlanVisible, setCreatePlanVisible] = useState(false);
  const [createVersionVisible, setCreateVersionVisible] = useState(false);
  const [planName, setPlanName] = useState('');
  const [windowDays, setWindowDays] = useState(30);
  const [compareModalVisible, setCompareModalVisible] = useState(false);
  const [loadLocalCompareDetail, setLoadLocalCompareDetail] = useState(false);
  const [selectedVersions, setSelectedVersions] = useState<string[]>([]);
  const [compareResult, setCompareResult] = useState<BackendVersionComparisonResult | null>(null);
  const [retrospectiveNote, setRetrospectiveNote] = useState('');
  const [retrospectiveSavedAt, setRetrospectiveSavedAt] = useState<string | null>(null);
  const [planSearchText, setPlanSearchText] = useState('');
  const currentUser = useCurrentUser();
  const { setRecalculating, setActiveVersion } = useGlobalActions();
  const [diffSearchText, setDiffSearchText] = useState('');
  const [diffTypeFilter, setDiffTypeFilter] = useState<'ALL' | 'ADDED' | 'REMOVED' | 'MOVED' | 'MODIFIED'>('ALL');
  const [showAllCapacityRows, setShowAllCapacityRows] = useState(false);

  // 防抖搜索文本（延迟 300ms）
  const debouncedPlanSearchText = useDebounce(planSearchText, 300);

  const planColumns: ColumnsType<Plan> = [
    {
      title: '方案名称',
      dataIndex: 'plan_name',
      key: 'plan_name',
    },
    {
      title: '创建人',
      dataIndex: 'created_by',
      key: 'created_by',
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
    },
    {
      title: '操作',
      key: 'action',
      render: (_, record) => (
        <Space>
          <Button size="small" onClick={() => loadVersions(record.plan_id)}>
            查看版本
          </Button>
          <Button size="small" onClick={() => handleCreateVersion(record.plan_id)}>
            创建版本
          </Button>
          <Button
            size="small"
            danger
            onClick={() => handleDeletePlan(record)}
          >
            删除
          </Button>
        </Space>
      ),
    },
  ];

  const versionColumns: ColumnsType<Version> = [
    {
      title: '版本',
      key: 'version',
      render: (_: any, record) => {
        const nameCn = extractVersionNameCn(record);
        return (
          <Space size={6}>
            <Tag color={record.status === 'ACTIVE' ? 'green' : 'default'}>V{record.version_no}</Tag>
            {nameCn ? <Typography.Text>{nameCn}</Typography.Text> : <Typography.Text type="secondary">—</Typography.Text>}
          </Space>
        );
      },
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
    },
    {
      title: '窗口天数',
      dataIndex: 'recalc_window_days',
      key: 'recalc_window_days',
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
    },
    {
      title: '操作',
      key: 'action',
      render: (_, record) => (
        <Space>
          <Button
            size="small"
            type="primary"
            disabled={record.status === 'ACTIVE'}
            onClick={() => handleActivateVersion(record.version_id)}
          >
            {record.status === 'ACTIVE' ? '已激活' : '回滚/激活'}
          </Button>
          {record.status === 'ACTIVE' && (
            <Button
              size="small"
              type="default"
              onClick={() => handleRecalc(record.version_id)}
            >
              一键重算
            </Button>
          )}
          {record.status !== 'ACTIVE' && (
            <Button
              size="small"
              danger
              onClick={() => handleDeleteVersion(record)}
            >
              删除
            </Button>
          )}
        </Space>
      ),
    },
  ];

  const loadPlans = async () => {
    setLoading(true);
    try {
      const result = await planApi.listPlans();
      setPlans(result);
      setFilteredPlans(result);
    } catch (error: any) {
      message.error(`加载失败: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  // 筛选排产方案
  const filterPlans = () => {
    let filtered = [...plans];

    // 按搜索文本筛选（方案名称或创建人）
    if (debouncedPlanSearchText) {
      const searchLower = debouncedPlanSearchText.toLowerCase();
      filtered = filtered.filter(
        (plan) =>
          plan.plan_name.toLowerCase().includes(searchLower) ||
          plan.created_by.toLowerCase().includes(searchLower)
      );
    }

    setFilteredPlans(filtered);
  };

  const loadVersions = async (planId: string) => {
    setSelectedPlanId(planId);
    setLoading(true);
    try {
      const result = await planApi.listVersions(planId);
      setVersions(result);
      const active = (result || []).find((v: Version) => v.status === 'ACTIVE');
      if (active) {
        setActiveVersion(active.version_id);
      }
    } catch (error: any) {
      message.error(`加载失败: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleCreatePlan = async () => {
    if (!planName.trim()) {
      message.warning('请输入方案名称');
      return;
    }

    setLoading(true);
    try {
      await planApi.createPlan(planName, currentUser);
      message.success('创建成功');
      setCreatePlanVisible(false);
      setPlanName('');
      await loadPlans();
    } catch (error: any) {
      message.error(`创建失败: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleCreateVersion = (planId: string) => {
    setSelectedPlanId(planId);
    setCreateVersionVisible(true);
  };

  const handleCreateVersionSubmit = async () => {
    if (!selectedPlanId) return;

    setLoading(true);
    try {
      await planApi.createVersion(selectedPlanId, windowDays, undefined, undefined, currentUser);
      message.success('创建版本成功');
      setCreateVersionVisible(false);
      setWindowDays(30);
      await loadVersions(selectedPlanId);
    } catch (error: any) {
      message.error(`创建失败: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleActivateVersion = async (versionId: string) => {
    if (!selectedPlanId) {
      message.warning('请先选择方案');
      return;
    }

    const target = versions.find((v) => v.version_id === versionId);
    const label = target ? formatVersionLabel(target) : versionId;

    let inputReason = '';
    Modal.confirm({
      title: `确认回滚并激活 ${label}？`,
      icon: <ExclamationCircleOutlined />,
      okText: '确认回滚',
      okButtonProps: { danger: true },
      cancelText: '取消',
      content: (
        <div>
          <p style={{ marginBottom: 8 }}>
            该操作将把当前方案的激活版本切换为 <strong>{label}</strong>（{versionId}）。
          </p>
          <p style={{ marginBottom: 8 }}>
            回滚会尝试<strong>恢复该版本的配置快照</strong>（覆盖当前全局配置），并触发决策数据刷新。
          </p>
          <p style={{ marginBottom: 8 }}>请填写回滚原因（将写入审计日志）：</p>
          <Input.TextArea
            rows={3}
            placeholder="例如：回滚到上周稳定版本，等待产能参数确认后再发布新方案"
            onChange={(e) => {
              inputReason = e.target.value;
            }}
          />
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            提示：回滚完成后，驾驶舱/风险等数据可能需要几十秒刷新。
          </Typography.Text>
        </div>
      ),
      onOk: async () => {
        const reason = String(inputReason || '').trim();
        if (!reason) {
          message.warning('请输入回滚原因');
          // 阻止确认框关闭
          return Promise.reject(new Error('MISSING_REASON'));
        }

        setLoading(true);
        try {
          const res = await planApi.rollbackVersion(selectedPlanId, versionId, currentUser, reason);
          setActiveVersion(versionId);
          message.success('回滚成功');
          if (res?.config_restore_skipped) {
            message.warning(String(res.config_restore_skipped));
          }
          await loadVersions(selectedPlanId);
        } catch (error: any) {
          message.error(`回滚失败: ${error?.message || error}`);
          throw error;
        } finally {
          setLoading(false);
        }
      },
    });
  };

  const handleDeletePlan = async (plan: Plan) => {
    Modal.confirm({
      title: '确认删除排产方案？',
      icon: <ExclamationCircleOutlined />,
      content: (
        <div>
          <p>
            将删除方案 <strong>{plan.plan_name}</strong>，并级联删除其所有版本与排产明细。
          </p>
          <p style={{ marginBottom: 0 }}>该操作不可恢复（建议先备份数据库文件）。</p>
        </div>
      ),
      okText: '删除',
      okButtonProps: { danger: true },
      cancelText: '取消',
      onOk: async () => {
        setLoading(true);
        try {
          await planApi.deletePlan(plan.plan_id, currentUser);
          message.success('删除成功');

          // 如果当前正在查看该方案，清空右侧版本区
          if (selectedPlanId === plan.plan_id) {
            setSelectedPlanId(null);
            setVersions([]);
            setSelectedVersions([]);
          }

          // 删除后尝试自动回填最新激活版本
          try {
            const latest = await planApi.getLatestActiveVersionId();
            setActiveVersion(latest || null);
          } catch {
            // 忽略：该错误已由 IpcClient 统一处理
          }

          await loadPlans();
        } catch (error: any) {
          message.error(`删除失败: ${error.message || error}`);
        } finally {
          setLoading(false);
        }
      },
    });
  };

  const handleDeleteVersion = async (version: Version) => {
    const label = formatVersionLabel(version);
    Modal.confirm({
      title: '确认删除版本？',
      icon: <ExclamationCircleOutlined />,
      content: (
        <div>
          <p>
            将删除版本 <strong>{label}</strong>（{version.version_id}）及其排产明细。
          </p>
          <p style={{ marginBottom: 0 }}>该操作不可恢复。</p>
        </div>
      ),
      okText: '删除',
      okButtonProps: { danger: true },
      cancelText: '取消',
      onOk: async () => {
        setLoading(true);
        try {
          await planApi.deleteVersion(version.version_id, currentUser);
          message.success('删除成功');
          setSelectedVersions((prev) => prev.filter((id) => id !== version.version_id));
          if (selectedPlanId) {
            await loadVersions(selectedPlanId);
          }
        } catch (error: any) {
          message.error(`删除失败: ${error.message || error}`);
        } finally {
          setLoading(false);
        }
      },
    });
  };

  const handleRecalc = async (versionId: string) => {
    setRecalculating(true);
    try {
      const baseDate = formatDate(dayjs());
      await planApi.recalcFull(versionId, baseDate, undefined, currentUser);
      message.success('重算完成');
      if (selectedPlanId) {
        await loadVersions(selectedPlanId);
      }
    } catch (error: any) {
      console.error('重算失败:', error);
    } finally {
      setRecalculating(false);
    }
  };

  const handleCompareVersions = async () => {
    if (selectedVersions.length !== 2) {
      message.warning('请选择两个版本进行对比');
      return;
    }

    setLoading(true);
    try {
      const result = (await planApi.compareVersions(
        selectedVersions[0],
        selectedVersions[1]
      )) as BackendVersionComparisonResult;
      setCompareResult(result);
      setCompareModalVisible(true);
      setDiffSearchText('');
      setDiffTypeFilter('ALL');
      setShowAllCapacityRows(false);
      setLoadLocalCompareDetail(false);
      message.success('版本对比完成');
    } catch (error: any) {
      console.error('版本对比失败:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadPlans();
  }, []);

  // 监听搜索文本变化（使用防抖后的文本）
  useEffect(() => {
    filterPlans();
  }, [debouncedPlanSearchText, plans]);

  const retrospectiveKey = useMemo(() => {
    if (!compareResult) return null;
    return makeRetrospectiveKey(compareResult.version_id_a, compareResult.version_id_b);
  }, [compareResult]);

  // 加载/回填复盘总结（本地保存，不依赖后端）
  useEffect(() => {
    if (!compareModalVisible || !compareResult || !retrospectiveKey) return;
    try {
      const raw = localStorage.getItem(retrospectiveKey);
      if (!raw) {
        setRetrospectiveNote('');
        setRetrospectiveSavedAt(null);
        return;
      }
      const parsed = JSON.parse(raw) as { note?: string; updated_at?: string } | null;
      setRetrospectiveNote(String(parsed?.note ?? ''));
      setRetrospectiveSavedAt(parsed?.updated_at ? String(parsed.updated_at) : null);
    } catch {
      setRetrospectiveNote('');
      setRetrospectiveSavedAt(null);
    }
  }, [compareModalVisible, compareResult, retrospectiveKey]);

  const saveRetrospectiveNote = () => {
    if (!compareResult || !retrospectiveKey) return;
    const note = retrospectiveNote.trim();
    const payload = {
      version_id_a: compareResult.version_id_a,
      version_id_b: compareResult.version_id_b,
      note,
      operator: currentUser,
      updated_at: dayjs().format('YYYY-MM-DD HH:mm:ss'),
    };
    try {
      localStorage.setItem(retrospectiveKey, JSON.stringify(payload));
      setRetrospectiveSavedAt(payload.updated_at);
      message.success('复盘总结已保存（本地）');
    } catch (e: any) {
      message.error(e?.message || '保存失败（localStorage 不可用）');
    }
  };

  const exportRetrospectiveReport = () => {
    if (!compareResult) return;
    const exportedAt = dayjs().format('YYYY-MM-DD HH:mm:ss');
    const report = {
      type: 'APS_VERSION_COMPARISON_REPORT',
      exported_at: exportedAt,
      operator: currentUser,
      comparison: compareResult,
      local_analysis: {
        diff_summary: localDiffResult?.summary ?? null,
        diff_sample: localDiffResult ? localDiffResult.diffs.slice(0, 500) : null,
        capacity_summary: localCapacityRows
          ? {
              date_from: localCapacityRows.dateFrom,
              date_to: localCapacityRows.dateTo,
              machines: localCapacityRows.machines,
              total_a_t: localCapacityRows.totalA,
              total_b_t: localCapacityRows.totalB,
              delta_t: localCapacityRows.totalB - localCapacityRows.totalA,
              overflow_rows_count: localCapacityRows.overflowRows.length,
              overflow_rows_sample: localCapacityRows.overflowRows.slice(0, 200),
            }
          : localCapacityRowsBase
          ? {
              date_from: localCapacityRowsBase.dateFrom,
              date_to: localCapacityRowsBase.dateTo,
              machines: localCapacityRowsBase.machines,
              total_a_t: localCapacityRowsBase.totalA,
              total_b_t: localCapacityRowsBase.totalB,
              delta_t: localCapacityRowsBase.totalB - localCapacityRowsBase.totalA,
              overflow_rows_count: null,
            }
          : null,
      },
      retrospective: {
        note: retrospectiveNote.trim(),
        saved_at: retrospectiveSavedAt,
      },
    };
    try {
      exportJSON([report], '版本对比报告');
      message.success('已导出（JSON）');
    } catch (e: any) {
      message.error(e?.message || '导出失败');
    }
  };

  const versionsInCompare = useMemo(() => {
    if (!compareResult) return null;
    return { versionIdA: compareResult.version_id_a, versionIdB: compareResult.version_id_b };
  }, [compareResult]);

  const compareKpiQuery = useQuery({
    queryKey: ['compareVersionsKpi', versionsInCompare?.versionIdA, versionsInCompare?.versionIdB],
    enabled: compareModalVisible && !!versionsInCompare?.versionIdA && !!versionsInCompare?.versionIdB,
    queryFn: async () => {
      if (!versionsInCompare?.versionIdA || !versionsInCompare?.versionIdB) return null;
      return (await planApi.compareVersionsKpi(
        versionsInCompare.versionIdA,
        versionsInCompare.versionIdB
      )) as BackendVersionComparisonKpiResult;
    },
    staleTime: 30 * 1000,
  });

  const compareKpiRows = useMemo(() => {
    const data = compareKpiQuery.data;
    if (!data) return null;
    const a = data.kpi_a;
    const b = data.kpi_b;

    const fmtInt = (v: number | null | undefined) => (v == null || !Number.isFinite(Number(v)) ? '-' : String(Math.trunc(Number(v))));
    const fmtNum = (v: number | null | undefined, digits = 1) =>
      v == null || !Number.isFinite(Number(v)) ? '-' : Number(v).toFixed(digits);

    const deltaInt = (va: number | null | undefined, vb: number | null | undefined) =>
      va == null || vb == null ? '-' : String(Math.trunc(Number(vb) - Number(va)));
    const deltaNum = (va: number | null | undefined, vb: number | null | undefined, digits = 1) =>
      va == null || vb == null ? '-' : (Number(vb) - Number(va)).toFixed(digits);

    return [
      { key: 'plan_items_count', metric: '排产项数', a: fmtInt(a.plan_items_count), b: fmtInt(b.plan_items_count), delta: deltaInt(a.plan_items_count, b.plan_items_count) },
      { key: 'total_weight_t', metric: '总吨位(t)', a: fmtNum(a.total_weight_t, 1), b: fmtNum(b.total_weight_t, 1), delta: deltaNum(a.total_weight_t, b.total_weight_t, 1) },
      { key: 'locked_in_plan_count', metric: '冻结项数', a: fmtInt(a.locked_in_plan_count), b: fmtInt(b.locked_in_plan_count), delta: deltaInt(a.locked_in_plan_count, b.locked_in_plan_count) },
      { key: 'force_release_in_plan_count', metric: '强放项数', a: fmtInt(a.force_release_in_plan_count), b: fmtInt(b.force_release_in_plan_count), delta: deltaInt(a.force_release_in_plan_count, b.force_release_in_plan_count) },
      { key: 'overflow_days', metric: '溢出天数(days)', a: fmtInt(a.overflow_days), b: fmtInt(b.overflow_days), delta: deltaInt(a.overflow_days, b.overflow_days) },
      { key: 'overflow_t', metric: '溢出吨位(t)', a: fmtNum(a.overflow_t, 1), b: fmtNum(b.overflow_t, 1), delta: deltaNum(a.overflow_t, b.overflow_t, 1) },
      { key: 'capacity_util_pct', metric: '产能利用率(%)', a: fmtNum(a.capacity_util_pct, 1), b: fmtNum(b.capacity_util_pct, 1), delta: deltaNum(a.capacity_util_pct, b.capacity_util_pct, 1) },
      { key: 'mature_backlog_t', metric: '适温待排(t)', a: fmtNum(a.mature_backlog_t, 1), b: fmtNum(b.mature_backlog_t, 1), delta: deltaNum(a.mature_backlog_t, b.mature_backlog_t, 1) },
      { key: 'immature_backlog_t', metric: '未适温待排(t)', a: fmtNum(a.immature_backlog_t, 1), b: fmtNum(b.immature_backlog_t, 1), delta: deltaNum(a.immature_backlog_t, b.immature_backlog_t, 1) },
      { key: 'urgent_total_t', metric: '紧急吨位(t)', a: fmtNum(a.urgent_total_t, 1), b: fmtNum(b.urgent_total_t, 1), delta: deltaNum(a.urgent_total_t, b.urgent_total_t, 1) },
    ];
  }, [compareKpiQuery.data]);

  const planItemsQueryA = useQuery({
    queryKey: ['planItems', versionsInCompare?.versionIdA],
    enabled: compareModalVisible && loadLocalCompareDetail && !!versionsInCompare?.versionIdA,
    queryFn: async () => {
      if (!versionsInCompare?.versionIdA) return [];
      const res = await planApi.listPlanItems(versionsInCompare.versionIdA);
      return Array.isArray(res) ? res : [];
    },
    staleTime: 30 * 1000,
  });

  const planItemsQueryB = useQuery({
    queryKey: ['planItems', versionsInCompare?.versionIdB],
    enabled: compareModalVisible && loadLocalCompareDetail && !!versionsInCompare?.versionIdB,
    queryFn: async () => {
      if (!versionsInCompare?.versionIdB) return [];
      const res = await planApi.listPlanItems(versionsInCompare.versionIdB);
      return Array.isArray(res) ? res : [];
    },
    staleTime: 30 * 1000,
  });

  const normalizedItemsA = useMemo<PlanItemSnapshot[]>(() => {
    const raw = Array.isArray(planItemsQueryA.data) ? planItemsQueryA.data : [];
    return raw.map(normalizePlanItem).filter((it): it is PlanItemSnapshot => it != null);
  }, [planItemsQueryA.data]);

  const normalizedItemsB = useMemo<PlanItemSnapshot[]>(() => {
    const raw = Array.isArray(planItemsQueryB.data) ? planItemsQueryB.data : [];
    return raw.map(normalizePlanItem).filter((it): it is PlanItemSnapshot => it != null);
  }, [planItemsQueryB.data]);

  const localDiffResult = useMemo(() => {
    if (!compareModalVisible || !versionsInCompare || !loadLocalCompareDetail) return null;
    if (planItemsQueryA.isLoading || planItemsQueryB.isLoading) return null;
    if (planItemsQueryA.error || planItemsQueryB.error) return null;
    return computeVersionDiffs(normalizedItemsA, normalizedItemsB);
  }, [
    compareModalVisible,
    loadLocalCompareDetail,
    normalizedItemsA,
    normalizedItemsB,
    planItemsQueryA.error,
    planItemsQueryA.isLoading,
    planItemsQueryB.error,
    planItemsQueryB.isLoading,
    versionsInCompare,
  ]);

  const filteredDiffs = useMemo(() => {
    const diffs = localDiffResult?.diffs ?? [];
    const q = diffSearchText.trim().toLowerCase();
    return diffs.filter((d) => {
      if (diffTypeFilter !== 'ALL' && d.changeType !== diffTypeFilter) return false;
      if (!q) return true;
      const idMatch = d.materialId.toLowerCase().includes(q);
      const from = d.previousState
        ? `${d.previousState.machine_code}/${d.previousState.plan_date}/${d.previousState.seq_no}`
        : '';
      const to = d.currentState ? `${d.currentState.machine_code}/${d.currentState.plan_date}/${d.currentState.seq_no}` : '';
      return idMatch || from.toLowerCase().includes(q) || to.toLowerCase().includes(q);
    });
  }, [diffSearchText, diffTypeFilter, localDiffResult?.diffs]);

  const diffSummaryChartOption = useMemo<EChartsOption | null>(() => {
    if (!localDiffResult) return null;
    const seriesData = [
      { name: '新增', value: localDiffResult.summary.addedCount, color: '#52c41a' },
      { name: '删除', value: localDiffResult.summary.removedCount, color: '#ff4d4f' },
      { name: '移动', value: localDiffResult.summary.movedCount, color: '#faad14' },
      { name: '修改', value: localDiffResult.summary.modifiedCount, color: '#722ed1' },
    ];
    const total = seriesData.reduce((sum, item) => sum + item.value, 0);
    if (total <= 0) return null;

    return {
      tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
      grid: { left: 44, right: 16, top: 16, bottom: 36 },
      xAxis: {
        type: 'category',
        data: seriesData.map((d) => d.name),
        axisTick: { alignWithLabel: true },
      },
      yAxis: { type: 'value', minInterval: 1 },
      series: [
        {
          name: '变更数量',
          type: 'bar',
          barMaxWidth: 44,
          label: { show: true, position: 'top' },
          data: seriesData.map((d) => ({
            value: d.value,
            itemStyle: { color: d.color },
          })),
        },
      ],
    };
  }, [localDiffResult]);

  const localCapacityRowsBase = useMemo(() => {
    if (!compareModalVisible || !versionsInCompare || !loadLocalCompareDetail) return null;
    const mapA = computeCapacityMap(normalizedItemsA);
    const mapB = computeCapacityMap(normalizedItemsB);
    const keys = new Set<string>([...mapA.keys(), ...mapB.keys()]);
    const rows: LocalCapacityDeltaRow[] = Array.from(keys)
      .map((key) => {
        const [machine, date] = key.split('__');
        const usedA = mapA.get(key) ?? 0;
        const usedB = mapB.get(key) ?? 0;
        return {
          machine_code: machine,
          date,
          used_a: usedA,
          used_b: usedB,
          delta: usedB - usedA,
          target_a: null,
          limit_a: null,
          target_b: null,
          limit_b: null,
        };
      })
      .filter((r) => showAllCapacityRows || Math.abs(r.delta) > 1e-9)
      .sort((a, b) => (a.date === b.date ? a.machine_code.localeCompare(b.machine_code) : a.date.localeCompare(b.date)));

    const dates = rows.map((r) => r.date).filter(Boolean).sort();
    const machines = Array.from(new Set(rows.map((r) => r.machine_code).filter(Boolean))).sort();
    return {
      rows,
      dateFrom: dates[0] || null,
      dateTo: dates[dates.length - 1] || null,
      machines,
      totalA: Array.from(mapA.values()).reduce((sum, v) => sum + v, 0),
      totalB: Array.from(mapB.values()).reduce((sum, v) => sum + v, 0),
    };
  }, [compareModalVisible, loadLocalCompareDetail, normalizedItemsA, normalizedItemsB, showAllCapacityRows, versionsInCompare]);

  const capacityPoolsQueryA = useQuery({
    queryKey: [
      'compareCapacityPools',
      versionsInCompare?.versionIdA,
      localCapacityRowsBase?.machines.join(',') || '',
      localCapacityRowsBase?.dateFrom || '',
      localCapacityRowsBase?.dateTo || '',
    ],
    enabled:
      compareModalVisible &&
      !!versionsInCompare?.versionIdA &&
      !!localCapacityRowsBase &&
      localCapacityRowsBase.machines.length > 0 &&
      !!localCapacityRowsBase.dateFrom &&
      !!localCapacityRowsBase.dateTo,
    queryFn: async () => {
      if (!versionsInCompare?.versionIdA || !localCapacityRowsBase?.dateFrom || !localCapacityRowsBase?.dateTo) return [];
      const res = await capacityApi.getCapacityPools(localCapacityRowsBase.machines, localCapacityRowsBase.dateFrom, localCapacityRowsBase.dateTo, versionsInCompare.versionIdA);
      return Array.isArray(res) ? res : [];
    },
    staleTime: 30 * 1000,
  });

  const capacityPoolsQueryB = useQuery({
    queryKey: [
      'compareCapacityPools',
      versionsInCompare?.versionIdB,
      localCapacityRowsBase?.machines.join(',') || '',
      localCapacityRowsBase?.dateFrom || '',
      localCapacityRowsBase?.dateTo || '',
    ],
    enabled:
      compareModalVisible &&
      !!versionsInCompare?.versionIdB &&
      !!localCapacityRowsBase &&
      localCapacityRowsBase.machines.length > 0 &&
      !!localCapacityRowsBase.dateFrom &&
      !!localCapacityRowsBase.dateTo,
    queryFn: async () => {
      if (!versionsInCompare?.versionIdB || !localCapacityRowsBase?.dateFrom || !localCapacityRowsBase?.dateTo) return [];
      const res = await capacityApi.getCapacityPools(localCapacityRowsBase.machines, localCapacityRowsBase.dateFrom, localCapacityRowsBase.dateTo, versionsInCompare.versionIdB);
      return Array.isArray(res) ? res : [];
    },
    staleTime: 30 * 1000,
  });

  const localCapacityRows = useMemo(() => {
    if (!localCapacityRowsBase) return null;
    const poolsA = Array.isArray(capacityPoolsQueryA.data) ? capacityPoolsQueryA.data : [];
    const poolsB = Array.isArray(capacityPoolsQueryB.data) ? capacityPoolsQueryB.data : [];
    const mapA = new Map<string, { target: number | null; limit: number | null }>();
    const mapB = new Map<string, { target: number | null; limit: number | null }>();

    poolsA.forEach((p: any) => {
      const machine = String(p?.machine_code ?? '').trim();
      const date = normalizeDateOnly(String(p?.plan_date ?? ''));
      if (!machine || !date) return;
      const target = Number(p?.target_capacity_t ?? 0);
      const limit = Number(p?.limit_capacity_t ?? 0);
      mapA.set(`${machine}__${date}`, {
        target: Number.isFinite(target) && target > 0 ? target : null,
        limit: Number.isFinite(limit) && limit > 0 ? limit : null,
      });
    });

    poolsB.forEach((p: any) => {
      const machine = String(p?.machine_code ?? '').trim();
      const date = normalizeDateOnly(String(p?.plan_date ?? ''));
      if (!machine || !date) return;
      const target = Number(p?.target_capacity_t ?? 0);
      const limit = Number(p?.limit_capacity_t ?? 0);
      mapB.set(`${machine}__${date}`, {
        target: Number.isFinite(target) && target > 0 ? target : null,
        limit: Number.isFinite(limit) && limit > 0 ? limit : null,
      });
    });

    const rows = localCapacityRowsBase.rows.map((r) => {
      const capA = mapA.get(`${r.machine_code}__${r.date}`);
      const capB = mapB.get(`${r.machine_code}__${r.date}`);
      return {
        ...r,
        target_a: capA?.target ?? null,
        limit_a: capA?.limit ?? capA?.target ?? null,
        target_b: capB?.target ?? null,
        limit_b: capB?.limit ?? capB?.target ?? null,
      };
    });

    const overflowRows = rows.filter((r) => {
      if (r.limit_b == null) return false;
      return r.used_b > r.limit_b + 1e-9;
    });

    return { ...localCapacityRowsBase, rows, overflowRows };
  }, [capacityPoolsQueryA.data, capacityPoolsQueryB.data, localCapacityRowsBase]);

  const capacityTrendOption = useMemo<EChartsOption | null>(() => {
    if (!compareModalVisible || !versionsInCompare) return null;
    if (planItemsQueryA.isLoading || planItemsQueryB.isLoading) return null;
    if (planItemsQueryA.error || planItemsQueryB.error) return null;

    const dailyA = computeDailyTotals(normalizedItemsA);
    const dailyB = computeDailyTotals(normalizedItemsB);
    const dates = Array.from(new Set([...dailyA.keys(), ...dailyB.keys()])).sort();
    if (dates.length === 0) return null;

    const aValues = dates.map((d) => Number(dailyA.get(d) ?? 0));
    const bValues = dates.map((d) => Number(dailyB.get(d) ?? 0));
    const deltas = dates.map((_, idx) => bValues[idx] - aValues[idx]);

    return {
      tooltip: { trigger: 'axis' },
      legend: { top: 0, left: 'left', data: ['版本A', '版本B', 'Δ(B-A)'] },
      grid: { left: 52, right: 52, top: 36, bottom: 44 },
      xAxis: {
        type: 'category',
        data: dates,
        axisLabel: { formatter: (value: string) => String(value).slice(5) },
      },
      yAxis: [
        { type: 'value', name: 't' },
        { type: 'value', name: 'Δt' },
      ],
      series: [
        { name: '版本A', type: 'line', data: aValues, smooth: true, showSymbol: false, lineStyle: { width: 2 } },
        { name: '版本B', type: 'line', data: bValues, smooth: true, showSymbol: false, lineStyle: { width: 2 } },
        {
          name: 'Δ(B-A)',
          type: 'bar',
          yAxisIndex: 1,
          barMaxWidth: 26,
          data: deltas.map((v) => ({
            value: v,
            itemStyle: { color: v >= 0 ? '#3f8600' : '#cf1322' },
          })),
        },
      ],
    };
  }, [
    compareModalVisible,
    normalizedItemsA,
    normalizedItemsB,
    planItemsQueryA.error,
    planItemsQueryA.isLoading,
    planItemsQueryB.error,
    planItemsQueryB.isLoading,
    versionsInCompare,
  ]);

  const riskTrendOption = useMemo<EChartsOption | null>(() => {
    const rows = Array.isArray(compareResult?.risk_delta) ? compareResult?.risk_delta : [];
    if (!rows || rows.length === 0) return null;
    const sorted = [...rows].sort((a, b) => String(a.date).localeCompare(String(b.date)));
    const dates = sorted.map((r) => String(r.date));
    const aValues = sorted.map((r) => (r.risk_score_a == null ? null : Number(r.risk_score_a)));
    const bValues = sorted.map((r) => (r.risk_score_b == null ? null : Number(r.risk_score_b)));
    const deltas = sorted.map((r) => Number(r.risk_score_delta ?? 0));

    return {
      tooltip: { trigger: 'axis' },
      legend: { top: 0, left: 'left', data: ['A风险', 'B风险', 'Δ'] },
      grid: { left: 52, right: 52, top: 36, bottom: 44 },
      xAxis: {
        type: 'category',
        data: dates,
        axisLabel: { formatter: (value: string) => String(value).slice(5) },
      },
      yAxis: [
        { type: 'value', name: '风险' },
        { type: 'value', name: 'Δ' },
      ],
      series: [
        { name: 'A风险', type: 'line', data: aValues, smooth: true, showSymbol: false, lineStyle: { width: 2 } },
        { name: 'B风险', type: 'line', data: bValues, smooth: true, showSymbol: false, lineStyle: { width: 2 } },
        {
          name: 'Δ',
          type: 'bar',
          yAxisIndex: 1,
          barMaxWidth: 26,
          data: deltas.map((v) => ({ value: v, itemStyle: { color: v >= 0 ? '#3f8600' : '#cf1322' } })),
        },
      ],
    };
  }, [compareResult?.risk_delta]);

  const exportCapacityDelta = (format: 'csv' | 'json') => {
    if (!compareResult || !localCapacityRows) return;
    const rows = localCapacityRows.rows.map((r) => ({
      date: r.date,
      machine_code: r.machine_code,
      used_a: r.used_a,
      used_b: r.used_b,
      delta: r.delta,
      target_a: r.target_a,
      limit_a: r.limit_a,
      target_b: r.target_b,
      limit_b: r.limit_b,
    }));
    const filename = `产能差异_${compareResult.version_id_a}_vs_${compareResult.version_id_b}`;
    if (format === 'csv') exportCSV(rows, filename);
    else exportJSON(rows, filename);
  };

  const exportDiffs = (format: 'csv' | 'json') => {
    if (!compareResult || !localDiffResult) return;
    const rows = localDiffResult.diffs.map((d) => ({
      change_type: d.changeType,
      material_id: d.materialId,
      from_machine: d.previousState?.machine_code ?? null,
      from_date: d.previousState?.plan_date ?? null,
      from_seq: d.previousState?.seq_no ?? null,
      to_machine: d.currentState?.machine_code ?? null,
      to_date: d.currentState?.plan_date ?? null,
      to_seq: d.currentState?.seq_no ?? null,
      weight_t: d.currentState?.weight_t ?? d.previousState?.weight_t ?? null,
      urgent_level: d.currentState?.urgent_level ?? d.previousState?.urgent_level ?? null,
      locked_in_plan: d.currentState?.locked_in_plan ?? d.previousState?.locked_in_plan ?? null,
      force_release_in_plan: d.currentState?.force_release_in_plan ?? d.previousState?.force_release_in_plan ?? null,
    }));

    const filename = `版本差异_${compareResult.version_id_a}_vs_${compareResult.version_id_b}`;
    if (format === 'csv') exportCSV(rows, filename);
    else exportJSON(rows, filename);
  };

  const exportReportMarkdown = () => {
    if (!compareResult) return;
    const header = `# 版本对比报告\n\n- 导出时间：${dayjs().format('YYYY-MM-DD HH:mm:ss')}\n- 操作人：${currentUser}\n- 版本A：${compareResult.version_id_a}\n- 版本B：${compareResult.version_id_b}\n\n`;
    const backendSummary = `## 后端摘要\n\n- moved_count: ${compareResult.moved_count}\n- added_count: ${compareResult.added_count}\n- removed_count: ${compareResult.removed_count}\n- squeezed_out_count: ${compareResult.squeezed_out_count}\n\n`;

    const localSummary = localDiffResult
      ? `## 本地差异摘要（由排产明细计算）\n\n- totalChanges: ${localDiffResult.summary.totalChanges}\n- movedCount: ${localDiffResult.summary.movedCount}\n- modifiedCount: ${localDiffResult.summary.modifiedCount}\n- addedCount: ${localDiffResult.summary.addedCount}\n- removedCount: ${localDiffResult.summary.removedCount}\n\n`
      : `## 本地差异摘要（由排产明细计算）\n\n> 本地差异明细未加载完成或加载失败。\n\n`;

    const configChanges = Array.isArray(compareResult.config_changes) ? compareResult.config_changes : [];
    const configSection =
      configChanges.length > 0
        ? `## 配置变化\n\n| Key | 版本A | 版本B |\n| --- | --- | --- |\n${configChanges
            .map((c) => `| ${String(c.key)} | ${c.value_a == null ? '-' : String(c.value_a)} | ${c.value_b == null ? '-' : String(c.value_b)} |`)
            .join('\n')}\n\n`
        : `## 配置变化\n\n- 无配置变化\n\n`;

    const diffSample = localDiffResult ? localDiffResult.diffs.slice(0, 200) : [];
    const diffsSection =
      diffSample.length > 0
        ? `## 物料变更明细（示例前200条）\n\n| 类型 | 物料 | From | To |\n| --- | --- | --- | --- |\n${diffSample
            .map((d) => {
              const from = d.previousState ? `${d.previousState.machine_code}/${d.previousState.plan_date}/序${d.previousState.seq_no}` : '-';
              const to = d.currentState ? `${d.currentState.machine_code}/${d.currentState.plan_date}/序${d.currentState.seq_no}` : '-';
              return `| ${d.changeType} | ${d.materialId} | ${from} | ${to} |`;
            })
            .join('\n')}\n\n`
        : `## 物料变更明细\n\n- 无变更或未加载。\n\n`;

    const capacitySection = localCapacityRows
      ? `## 产能变化（本地计算）\n\n- 总量A: ${localCapacityRows.totalA.toFixed(1)}t\n- 总量B: ${localCapacityRows.totalB.toFixed(1)}t\n- Δ: ${(localCapacityRows.totalB - localCapacityRows.totalA).toFixed(1)}t\n- 预计超上限行数（按版本B产能池）：${localCapacityRows.overflowRows.length}\n\n`
      : `## 产能变化（本地计算）\n\n- 未加载。\n\n`;

    const retrospectiveSection = `## 复盘总结（本地）\n\n${retrospectiveNote.trim() || '（空）'}\n\n`;

    try {
      exportMarkdown(header + backendSummary + localSummary + configSection + diffsSection + capacitySection + retrospectiveSection, '版本对比报告');
      message.success('已导出（Markdown）');
    } catch (e: any) {
      message.error(e?.message || '导出失败');
    }
  };

  const exportReportHTML = () => {
    if (!compareResult) return;
    const configChanges = Array.isArray(compareResult.config_changes) ? compareResult.config_changes : [];
    const diffSample = localDiffResult ? localDiffResult.diffs.slice(0, 200) : [];

    const escape = (v: unknown) =>
      String(v ?? '')
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/\"/g, '&quot;');

    const html = `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>版本对比报告</title>
    <style>
      body { font-family: -apple-system,BlinkMacSystemFont,Segoe UI,Roboto,Helvetica,Arial,"PingFang SC","Hiragino Sans GB","Microsoft YaHei",sans-serif; padding: 24px; }
      h1,h2 { margin: 16px 0 8px; }
      .meta { color: #666; font-size: 13px; margin-bottom: 16px; }
      table { border-collapse: collapse; width: 100%; margin: 8px 0 16px; }
      th, td { border: 1px solid #eee; padding: 8px; font-size: 13px; text-align: left; }
      th { background: #fafafa; }
      code { font-family: ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,"Liberation Mono","Courier New",monospace; }
      .badge { display: inline-block; padding: 2px 8px; border-radius: 999px; font-size: 12px; }
      .badge.red { background: rgba(255,77,79,0.15); color: #cf1322; }
      .badge.blue { background: rgba(22,119,255,0.12); color: #1677ff; }
    </style>
  </head>
  <body>
    <h1>版本对比报告</h1>
    <div class="meta">
      导出时间：${escape(dayjs().format('YYYY-MM-DD HH:mm:ss'))} · 操作人：${escape(currentUser)}<br/>
      版本A：<code>${escape(compareResult.version_id_a)}</code> · 版本B：<code>${escape(compareResult.version_id_b)}</code>
    </div>

    <h2>后端摘要</h2>
    <table>
      <tr><th>moved_count</th><td>${escape(compareResult.moved_count)}</td><th>added_count</th><td>${escape(compareResult.added_count)}</td></tr>
      <tr><th>removed_count</th><td>${escape(compareResult.removed_count)}</td><th>squeezed_out_count</th><td>${escape(compareResult.squeezed_out_count)}</td></tr>
    </table>

    <h2>本地差异摘要（由排产明细计算）</h2>
    ${
      localDiffResult
        ? `<table>
      <tr><th>totalChanges</th><td>${escape(localDiffResult.summary.totalChanges)}</td><th>movedCount</th><td>${escape(localDiffResult.summary.movedCount)}</td></tr>
      <tr><th>modifiedCount</th><td>${escape(localDiffResult.summary.modifiedCount)}</td><th>addedCount</th><td>${escape(localDiffResult.summary.addedCount)}</td></tr>
      <tr><th>removedCount</th><td>${escape(localDiffResult.summary.removedCount)}</td><th></th><td></td></tr>
    </table>`
        : `<div class="meta">本地差异明细未加载完成或加载失败。</div>`
    }

    <h2>配置变化</h2>
    ${
      configChanges.length > 0
        ? `<table>
      <thead><tr><th>Key</th><th>版本A</th><th>版本B</th></tr></thead>
      <tbody>
        ${configChanges
          .map((c) => `<tr><td>${escape(c.key)}</td><td>${escape(c.value_a ?? '-')}</td><td>${escape(c.value_b ?? '-')}</td></tr>`)
          .join('')}
      </tbody>
    </table>`
        : `<div class="meta">无配置变化</div>`
    }

    <h2>物料变更明细（示例前200条）</h2>
    ${
      diffSample.length > 0
        ? `<table>
      <thead><tr><th>类型</th><th>物料</th><th>From</th><th>To</th></tr></thead>
      <tbody>
        ${diffSample
          .map((d) => {
            const from = d.previousState ? `${d.previousState.machine_code}/${d.previousState.plan_date}/序${d.previousState.seq_no}` : '-';
            const to = d.currentState ? `${d.currentState.machine_code}/${d.currentState.plan_date}/序${d.currentState.seq_no}` : '-';
            return `<tr>
              <td><span class="badge ${d.changeType === 'REMOVED' ? 'red' : 'blue'}">${escape(d.changeType)}</span></td>
              <td><code>${escape(d.materialId)}</code></td>
              <td>${escape(from)}</td>
              <td>${escape(to)}</td>
            </tr>`;
          })
          .join('')}
      </tbody>
    </table>`
        : `<div class="meta">无变更或未加载。</div>`
    }

    <h2>产能变化（本地计算）</h2>
    ${
      localCapacityRows
        ? `<table>
      <tr><th>总量A</th><td>${escape(localCapacityRows.totalA.toFixed(1))}t</td><th>总量B</th><td>${escape(localCapacityRows.totalB.toFixed(1))}t</td></tr>
      <tr><th>Δ</th><td>${escape((localCapacityRows.totalB - localCapacityRows.totalA).toFixed(1))}t</td><th>预计超上限行数</th><td>${escape(localCapacityRows.overflowRows.length)}</td></tr>
    </table>`
        : `<div class="meta">未加载。</div>`
    }

    <h2>复盘总结（本地）</h2>
    <pre style="white-space: pre-wrap; border: 1px solid #eee; background: #fafafa; padding: 12px; border-radius: 6px;">${escape(
      retrospectiveNote.trim() || '（空）'
    )}</pre>
  </body>
</html>`;

    try {
      exportHTML(html, '版本对比报告');
      message.success('已导出（HTML）');
    } catch (e: any) {
      message.error(e?.message || '导出失败');
    }
  };

  return (
    <div>
      <Space style={{ marginBottom: 16 }} wrap>
        <Button onClick={() => navigate('/workbench')}>返回计划工作台</Button>
        <Button onClick={() => navigate('/overview')}>返回风险概览</Button>
        <Divider type="vertical" />
        <Button type="primary" onClick={() => setCreatePlanVisible(true)}>
          创建方案
        </Button>
        <Button onClick={loadPlans}>刷新</Button>
      </Space>

      <Space style={{ marginBottom: 16 }} wrap>
        <Input
          placeholder="搜索方案名称或创建人"
          value={planSearchText}
          onChange={(e) => setPlanSearchText(e.target.value)}
          style={{ width: 250 }}
          allowClear
        />
        <Button onClick={() => setPlanSearchText('')}>清除搜索</Button>
      </Space>

      <h3>排产方案列表</h3>
      <Table
        columns={planColumns}
        dataSource={filteredPlans}
        rowKey="plan_id"
        loading={loading}
        pagination={false}
      />

      {selectedPlanId && (
        <>
          <h3 style={{ marginTop: 24 }}>版本列表</h3>
          <Space style={{ marginBottom: 16 }}>
            <Button
              type="primary"
              disabled={selectedVersions.length !== 2}
              onClick={handleCompareVersions}
            >
              对比选中版本
            </Button>
            <Button onClick={() => setSelectedVersions([])}>清除选择</Button>
          </Space>
          <Table
            columns={versionColumns}
            dataSource={versions}
            rowKey="version_id"
            loading={loading}
            pagination={false}
            rowSelection={{
              type: 'checkbox',
              selectedRowKeys: selectedVersions,
              onChange: (selectedKeys) => {
                if (selectedKeys.length <= 2) {
                  setSelectedVersions(selectedKeys as string[]);
                } else {
                  message.warning('最多只能选择2个版本进行对比');
                }
              },
            }}
          />
        </>
      )}

      <Modal
        title="创建排产方案"
        open={createPlanVisible}
        onOk={handleCreatePlan}
        onCancel={() => {
          setCreatePlanVisible(false);
          setPlanName('');
        }}
        confirmLoading={loading}
      >
        <Input
          placeholder="请输入方案名称"
          value={planName}
          onChange={(e) => setPlanName(e.target.value)}
        />
      </Modal>

      <Modal
        title="创建新版本"
        open={createVersionVisible}
        onOk={handleCreateVersionSubmit}
        onCancel={() => {
          setCreateVersionVisible(false);
          setWindowDays(30);
        }}
        confirmLoading={loading}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <div>
            <label>窗口天数：</label>
            <InputNumber
              min={1}
              max={60}
              value={windowDays}
              onChange={(val) => setWindowDays(val || 30)}
            />
          </div>
        </Space>
      </Modal>

	      <Modal
	        title="版本对比结果"
	        open={compareModalVisible}
	        onCancel={() => {
	          setCompareModalVisible(false);
	          setCompareResult(null);
	        }}
	        footer={[
	          compareResult ? (
	            <Button
	              key="activateA"
	              onClick={() => handleActivateVersion(compareResult.version_id_a)}
	            >
	              回滚到版本A
	            </Button>
	          ) : null,
	          compareResult ? (
	            <Button
	              key="activateB"
	              type="primary"
	              onClick={() => handleActivateVersion(compareResult.version_id_b)}
	            >
	              回滚到版本B
	            </Button>
	          ) : null,
	          <Button key="close" onClick={() => setCompareModalVisible(false)}>
	            关闭
	          </Button>,
	        ]}
        width={1100}
        bodyStyle={{ maxHeight: 600, overflow: 'auto' }}
      >
        {compareResult && (
          <Space direction="vertical" style={{ width: '100%' }}>
            <Alert type="info" showIcon message={compareResult.message} />

	            <Card title="对比摘要" size="small">
	              <Descriptions size="small" column={2} bordered>
	                <Descriptions.Item label="版本A">{compareResult.version_id_a}</Descriptions.Item>
	                <Descriptions.Item label="版本B">{compareResult.version_id_b}</Descriptions.Item>
	                <Descriptions.Item label="移动数量">{compareResult.moved_count}</Descriptions.Item>
	                <Descriptions.Item label="新增数量">{compareResult.added_count}</Descriptions.Item>
	                <Descriptions.Item label="删除数量">{compareResult.removed_count}</Descriptions.Item>
	                <Descriptions.Item label="挤出数量">{compareResult.squeezed_out_count}</Descriptions.Item>
	              </Descriptions>
	            </Card>

	            <Card title="KPI 总览（后端聚合）" size="small">
	              {compareKpiQuery.isLoading ? (
	                <Alert type="info" showIcon message="正在计算 KPI…" />
	              ) : compareKpiQuery.error ? (
	                <Alert
	                  type="error"
	                  showIcon
	                  message="KPI 计算失败"
	                  description={String((compareKpiQuery.error as any)?.message || compareKpiQuery.error)}
	                />
	              ) : !compareKpiQuery.data ? (
	                <Alert type="info" showIcon message="暂无 KPI 数据" />
	              ) : (
	                <Space direction="vertical" style={{ width: '100%' }} size={10}>
	                  <Alert type="info" showIcon message={compareKpiQuery.data.message} />
	                  <Table
	                    size="small"
	                    pagination={false}
	                    rowKey={(r) => String((r as any).key)}
	                    dataSource={compareKpiRows ?? []}
	                    columns={[
	                      { title: '指标', dataIndex: 'metric', width: 180 },
	                      { title: '版本A', dataIndex: 'a', width: 160 },
	                      { title: '版本B', dataIndex: 'b', width: 160 },
	                      { title: 'Δ(B-A)', dataIndex: 'delta' },
	                    ]}
	                  />
	                </Space>
	              )}
	            </Card>

	            <Card
	              title="物料变更明细（本地计算）"
	              size="small"
	              extra={
	                <Space>
	                  <Button
	                    size="small"
	                    onClick={() => setLoadLocalCompareDetail(true)}
	                    disabled={loadLocalCompareDetail}
	                  >
	                    {loadLocalCompareDetail ? '已加载明细' : '加载明细'}
	                  </Button>
	                  <Button size="small" onClick={() => exportDiffs('csv')} disabled={!localDiffResult}>
	                    导出差异(CSV)
	                  </Button>
	                  <Button size="small" onClick={() => exportDiffs('json')} disabled={!localDiffResult}>
	                    导出差异(JSON)
	                  </Button>
	                </Space>
	              }
            >
	              {!loadLocalCompareDetail ? (
	                <Alert
	                  type="info"
	                  showIcon
	                  message="为提升性能，默认不加载全量排产明细"
	                  description="点击右上角「加载明细」后，将拉取两个版本的 plan_item 用于本地计算：变更明细/产能变化等。"
	                />
	              ) : planItemsQueryA.isLoading || planItemsQueryB.isLoading ? (
	                <Alert type="info" showIcon message="正在加载排产明细，用于计算差异…" />
	              ) : planItemsQueryA.error || planItemsQueryB.error ? (
	                <Alert
	                  type="error"
                  showIcon
                  message="排产明细加载失败，无法生成本地差异"
                  description={String((planItemsQueryA.error as any)?.message || planItemsQueryA.error || (planItemsQueryB.error as any)?.message || planItemsQueryB.error)}
                />
              ) : !localDiffResult ? (
                <Alert type="info" showIcon message="暂无差异数据" />
              ) : (
                <Space direction="vertical" style={{ width: '100%' }} size={10}>
                  <Space wrap>
                    <Tag color="blue">总变更 {localDiffResult.summary.totalChanges}</Tag>
                    <Tag color="green">新增 {localDiffResult.summary.addedCount}</Tag>
                    <Tag color="red">删除 {localDiffResult.summary.removedCount}</Tag>
                    <Tag color="gold">移动 {localDiffResult.summary.movedCount}</Tag>
                    <Tag color="purple">修改 {localDiffResult.summary.modifiedCount}</Tag>
                  </Space>

                  {diffSummaryChartOption ? (
                    <Chart option={diffSummaryChartOption} height={180} />
                  ) : (
                    <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                      暂无可视化统计
                    </Typography.Text>
                  )}

                  <Space wrap>
                    <Input
                      placeholder="搜索物料/From/To…"
                      value={diffSearchText}
                      onChange={(e) => setDiffSearchText(e.target.value)}
                      style={{ width: 260 }}
                      allowClear
                    />
                    <Select
                      value={diffTypeFilter}
                      style={{ width: 220 }}
                      onChange={(v) => setDiffTypeFilter(v)}
                      options={[
                        { value: 'ALL', label: '全部类型' },
                        { value: 'MOVED', label: '移动' },
                        { value: 'MODIFIED', label: '修改' },
                        { value: 'ADDED', label: '新增' },
                        { value: 'REMOVED', label: '删除' },
                      ]}
                    />
                    <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                      显示 {filteredDiffs.length} / {localDiffResult.diffs.length}
                    </Typography.Text>
                  </Space>

                  <Table<VersionDiff>
                    size="small"
                    rowKey={(r) => r.materialId}
                    pagination={false}
                    dataSource={filteredDiffs}
                    columns={[
                      {
                        title: '类型',
                        dataIndex: 'changeType',
                        width: 90,
                        render: (v) => {
                          const color = v === 'REMOVED' ? 'red' : v === 'ADDED' ? 'green' : v === 'MOVED' ? 'gold' : 'purple';
                          return <Tag color={color}>{v}</Tag>;
                        },
                      },
                      {
                        title: '物料',
                        dataIndex: 'materialId',
                        width: 200,
                        render: (v) => (
                          <Typography.Text code copyable>
                            {String(v)}
                          </Typography.Text>
                        ),
                      },
                      {
                        title: 'From',
                        key: 'from',
                        width: 260,
                        render: (_, r) => {
                          const s = r.previousState;
                          return s ? `${s.machine_code}/${s.plan_date}/序${s.seq_no}` : '-';
                        },
                      },
                      {
                        title: 'To',
                        key: 'to',
                        width: 260,
                        render: (_, r) => {
                          const s = r.currentState;
                          return s ? `${s.machine_code}/${s.plan_date}/序${s.seq_no}` : '-';
                        },
                      },
                      {
                        title: '紧急',
                        key: 'urgent',
                        width: 90,
                        render: (_, r) => {
                          const u = r.currentState?.urgent_level ?? r.previousState?.urgent_level ?? '';
                          return u ? <Tag>{u}</Tag> : '-';
                        },
                      },
                      {
                        title: '重量',
                        key: 'weight',
                        width: 90,
                        render: (_, r) => {
                          const w = r.currentState?.weight_t ?? r.previousState?.weight_t ?? null;
                          if (w == null || !Number.isFinite(Number(w))) return '-';
                          return `${Number(w).toFixed(3)}t`;
                        },
                      },
                    ]}
                    virtual
                    scroll={{ y: 320 }}
                  />
                </Space>
              )}
            </Card>

            <Card
              title="产能变化（本地计算）"
              size="small"
              extra={
                <Space>
                  <Button size="small" onClick={() => setShowAllCapacityRows((v) => !v)} disabled={!localCapacityRowsBase}>
                    {showAllCapacityRows ? '仅看变化' : '查看全量'}
                  </Button>
                  <Button size="small" onClick={() => exportCapacityDelta('csv')} disabled={!localCapacityRows}>
                    导出产能(CSV)
                  </Button>
                  <Button size="small" onClick={() => exportCapacityDelta('json')} disabled={!localCapacityRows}>
                    导出产能(JSON)
                  </Button>
                </Space>
              }
            >
	              {!loadLocalCompareDetail ? (
	                <Alert
	                  type="info"
	                  showIcon
	                  message="未加载排产明细"
	                  description="点击上方「物料变更明细」区域右上角的「加载明细」，即可生成本地产能变化分析。"
	                />
	              ) : !localCapacityRowsBase ? (
	                <Alert type="info" showIcon message="暂无产能差异数据" />
	              ) : (
	                <Space direction="vertical" style={{ width: '100%' }} size={10}>
	                  {capacityTrendOption ? (
	                    <Chart option={capacityTrendOption} height={220} />
	                  ) : (
                    <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                      暂无产能趋势图
                    </Typography.Text>
                  )}

                  <Space wrap>
                    <Tag color="blue">总量A {localCapacityRowsBase.totalA.toFixed(1)}t</Tag>
                    <Tag color="blue">总量B {localCapacityRowsBase.totalB.toFixed(1)}t</Tag>
                    <Tag color={localCapacityRowsBase.totalB - localCapacityRowsBase.totalA >= 0 ? 'green' : 'red'}>
                      Δ {(localCapacityRowsBase.totalB - localCapacityRowsBase.totalA).toFixed(1)}t
                    </Tag>
                    {localCapacityRows ? (
                      <Tag color={localCapacityRows.overflowRows.length > 0 ? 'red' : 'green'}>
                        预计超上限 {localCapacityRows.overflowRows.length}
                      </Tag>
                    ) : null}
                    <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                      {localCapacityRowsBase.dateFrom || '-'} ~ {localCapacityRowsBase.dateTo || '-'} · 机组 {localCapacityRowsBase.machines.length}
                    </Typography.Text>
                  </Space>

                  {capacityPoolsQueryA.error || capacityPoolsQueryB.error ? (
                    <Alert
                      type="warning"
                      showIcon
                      message="产能池加载失败（仍可查看吨位差异）"
                      description={String((capacityPoolsQueryA.error as any)?.message || capacityPoolsQueryA.error || (capacityPoolsQueryB.error as any)?.message || capacityPoolsQueryB.error)}
                    />
                  ) : null}

                  <Table<LocalCapacityDeltaRow>
                    size="small"
                    rowKey={(r) => `${r.machine_code}__${r.date}`}
                    pagination={false}
                    dataSource={localCapacityRows?.rows ?? localCapacityRowsBase.rows}
                    columns={[
                      { title: '日期', dataIndex: 'date', width: 120 },
                      { title: '机组', dataIndex: 'machine_code', width: 90 },
                      {
                        title: 'A已用(t)',
                        dataIndex: 'used_a',
                        width: 110,
                        render: (v: number) => v.toFixed(1),
                      },
                      {
                        title: 'B已用(t)',
                        dataIndex: 'used_b',
                        width: 110,
                        render: (v: number, r) => {
                          const threshold = r.limit_b ?? r.target_b ?? null;
                          const over = threshold != null && v > threshold + 1e-9;
                          return <span style={{ color: over ? '#cf1322' : undefined }}>{v.toFixed(1)}</span>;
                        },
                      },
                      {
                        title: 'Δ(t)',
                        dataIndex: 'delta',
                        width: 110,
                        render: (v: number) => (
                          <span style={{ color: v > 1e-9 ? '#3f8600' : v < -1e-9 ? '#cf1322' : undefined }}>
                            {v.toFixed(1)}
                          </span>
                        ),
                      },
                      {
                        title: 'B目标/上限',
                        key: 'capB',
                        width: 160,
                        render: (_, r) => {
                          const target = r.target_b;
                          const limit = r.limit_b;
                          if (target == null && limit == null) return '-';
                          const t = target == null ? '-' : target.toFixed(1);
                          const l = limit == null ? '-' : limit.toFixed(1);
                          return `${t} / ${l}`;
                        },
                      },
                    ]}
                    virtual
                    scroll={{ y: 320 }}
                  />
                </Space>
              )}
            </Card>

            <Card title="配置变化" size="small">
              {compareResult.config_changes && compareResult.config_changes.length > 0 ? (
                <Table
                  size="small"
                  pagination={false}
                  rowKey={(r) => r.key}
                  dataSource={compareResult.config_changes}
                  columns={[
                    { title: 'Key', dataIndex: 'key', width: 220 },
                    {
                      title: '版本A',
                      dataIndex: 'value_a',
                      render: (v) => (v == null ? '-' : String(v)),
                    },
                    {
                      title: '版本B',
                      dataIndex: 'value_b',
                      render: (v) => (v == null ? '-' : String(v)),
                    },
                  ]}
                  scroll={{ y: 240 }}
                />
              ) : (
                <Alert type="success" showIcon message="无配置变化" />
              )}
            </Card>

            <Card title="风险/产能变化" size="small">
              <Space direction="vertical" style={{ width: '100%' }} size={10}>
                {compareResult.risk_delta ? (
                  <Space direction="vertical" style={{ width: '100%' }} size={10}>
                    {riskTrendOption ? (
                      <Chart option={riskTrendOption} height={220} />
                    ) : null}
                    <Table
                      size="small"
                      pagination={false}
                      rowKey={(r) => `${r.date}`}
                      dataSource={compareResult.risk_delta}
                      columns={[
                        { title: '日期', dataIndex: 'date', width: 120 },
                        { title: 'A风险', dataIndex: 'risk_score_a', width: 120 },
                        { title: 'B风险', dataIndex: 'risk_score_b', width: 120 },
                        { title: 'Δ', dataIndex: 'risk_score_delta' },
                      ]}
                      scroll={{ y: 200 }}
                    />
                  </Space>
                ) : (
                  <Alert
                    type="info"
                    showIcon
                    message="风险变化对比暂不可用"
                    description="后端 compare_versions 当前未返回 risk_delta（待 RiskSnapshotRepository 支持）。"
                  />
                )}

                {compareResult.capacity_delta ? (
                  <Table
                    size="small"
                    pagination={false}
                    rowKey={(r) => `${r.machine_code}__${r.date}`}
                    dataSource={compareResult.capacity_delta}
                    columns={[
                      { title: '机组', dataIndex: 'machine_code', width: 90 },
                      { title: '日期', dataIndex: 'date', width: 120 },
                      { title: 'A已用', dataIndex: 'used_capacity_a', width: 120 },
                      { title: 'B已用', dataIndex: 'used_capacity_b', width: 120 },
                      { title: 'Δ', dataIndex: 'capacity_delta' },
                    ]}
                    scroll={{ y: 200 }}
                  />
                ) : (
                  <Alert
                    type="info"
                    showIcon
                    message="产能变化对比暂不可用"
                    description="后端 compare_versions 当前未返回 capacity_delta（待 CapacityPoolRepository 支持）。"
                  />
                )}
              </Space>
            </Card>

            <Card
              title="复盘总结"
              size="small"
              extra={
                <Space>
                  <Button size="small" onClick={saveRetrospectiveNote}>
                    保存总结
                  </Button>
                  <Button size="small" onClick={exportRetrospectiveReport}>
                    导出报告(JSON)
                  </Button>
                  <Button size="small" onClick={exportReportMarkdown} disabled={!compareResult}>
                    导出报告(MD)
                  </Button>
                  <Button size="small" onClick={exportReportHTML} disabled={!compareResult}>
                    导出报告(HTML)
                  </Button>
                </Space>
              }
            >
              <Space direction="vertical" style={{ width: '100%' }} size={8}>
                <Input.TextArea
                  rows={5}
                  value={retrospectiveNote}
                  onChange={(e) => setRetrospectiveNote(e.target.value)}
                  placeholder="记录本次决策要点、代价与后续关注项（本地保存，不会写入数据库）。"
                />
                <Alert
                  type="info"
                  showIcon
                  message={
                    retrospectiveSavedAt
                      ? `已保存（本地）：${retrospectiveSavedAt}`
                      : '未保存（本地）'
                  }
                />
              </Space>
            </Card>
          </Space>
        )}
      </Modal>
    </div>
  );
};

export default PlanManagement;

import React, { useMemo, useState, useEffect } from 'react';
import { useDebounce } from '../hooks/useDebounce';
import { Button, Divider, Input, InputNumber, Modal, Space, Table, Typography, message } from 'antd';
import { ExclamationCircleOutlined } from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import type { EChartsOption } from 'echarts';
import { capacityApi, planApi } from '../api/tauri';
import { useCurrentUser, useGlobalActions } from '../stores/use-global-store';
import dayjs from 'dayjs';
import { formatDate } from '../utils/formatters';
import type { BackendVersionComparisonKpiResult, BackendVersionComparisonResult, PlanItemSnapshot } from '../types/comparison';
import { VersionComparisonModal } from './comparison/VersionComparisonModal';
import { Plan, Version, LocalCapacityDeltaRow } from './comparison/types';
import {
  normalizeDateOnly,
  formatVersionLabel,
  normalizePlanItem,
  computeVersionDiffs,
  computeCapacityMap,
  computeDailyTotals,
  makeRetrospectiveKey,
} from './comparison/utils';
import { createPlanColumns, createVersionColumns } from './plan-management/columns';
import {
  exportCapacityDelta,
  exportDiffs,
  exportRetrospectiveReport,
  exportReportMarkdown,
  exportReportHTML,
  type ExportContext,
} from './plan-management/exportHelpers';

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

  const planColumns = useMemo(
    () => createPlanColumns(loadVersions, handleCreateVersion, handleDeletePlan),
    []
  );

  const versionColumns = useMemo(
    () => createVersionColumns(handleActivateVersion, handleRecalc, handleDeleteVersion),
    []
  );

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

  const diffSummaryChartOption = useMemo<EChartsOption | null>(() => {
    if (!localDiffResult) return null;
    const { addedCount, removedCount, movedCount, modifiedCount } = localDiffResult.summary;
    return {
      title: { text: '变更类型分布' },
      tooltip: { trigger: 'item' },
      legend: { orient: 'vertical', left: 'left' },
      series: [
        {
          name: '变更数量',
          type: 'pie',
          radius: '50%',
          data: [
            { value: addedCount, name: '新增' },
            { value: removedCount, name: '删除' },
            { value: movedCount, name: '移动' },
            { value: modifiedCount, name: '修改' },
          ],
          emphasis: {
            itemStyle: {
              shadowBlur: 10,
              shadowOffsetX: 0,
              shadowColor: 'rgba(0, 0, 0, 0.5)',
            },
          },
        },
      ],
    };
  }, [localDiffResult]);

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

  const handleExportCapacityDelta = async (format: 'csv' | 'json'): Promise<void> => {
    if (!compareResult || !localCapacityRows) return;
    const context: ExportContext = {
      compareResult,
      currentUser,
      localDiffResult,
      localCapacityRows,
      retrospectiveNote,
    };
    return exportCapacityDelta(format, context);
  };

  const handleExportDiffs = async (format: 'csv' | 'json'): Promise<void> => {
    if (!compareResult || !localDiffResult) return;
    const context: ExportContext = {
      compareResult,
      currentUser,
      localDiffResult,
      localCapacityRows,
      retrospectiveNote,
    };
    return exportDiffs(format, context);
  };

  const handleExportReport = async (format: 'json' | 'markdown' | 'html'): Promise<void> => {
    if (!compareResult) return;
    const context: ExportContext = {
      compareResult,
      currentUser,
      localDiffResult,
      localCapacityRows,
      retrospectiveNote,
    };
    if (format === 'json') return exportRetrospectiveReport(context);
    if (format === 'markdown') return exportReportMarkdown(context);
    if (format === 'html') return exportReportHTML(context);
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

      <VersionComparisonModal
        open={compareModalVisible}
        onClose={() => {
          setCompareModalVisible(false);
          setCompareResult(null);
        }}
        compareResult={compareResult}
        compareKpiRows={compareKpiRows ?? []}
        compareKpiLoading={compareKpiQuery.isLoading}
        compareKpiError={compareKpiQuery.error as Error | null}
        localDiffResult={localDiffResult}
        loadLocalCompareDetail={loadLocalCompareDetail}
        planItemsLoading={planItemsQueryA.isLoading || planItemsQueryB.isLoading}
        planItemsErrorA={planItemsQueryA.error as Error | null}
        planItemsErrorB={planItemsQueryB.error as Error | null}
        localCapacityRows={localCapacityRows}
        localCapacityRowsBase={localCapacityRowsBase}
        capacityPoolsErrorA={capacityPoolsQueryA.error as Error | null}
        capacityPoolsErrorB={capacityPoolsQueryB.error as Error | null}
        showAllCapacityRows={showAllCapacityRows}
        retrospectiveNote={retrospectiveNote}
        retrospectiveSavedAt={retrospectiveSavedAt}
        diffSearchText={diffSearchText}
        diffTypeFilter={diffTypeFilter}
        diffSummaryChartOption={diffSummaryChartOption}
        capacityTrendOption={capacityTrendOption}
        riskTrendOption={riskTrendOption}
        onActivateVersion={handleActivateVersion}
        onLoadLocalCompareDetail={() => setLoadLocalCompareDetail(true)}
        onToggleShowAllCapacityRows={() => setShowAllCapacityRows((v) => !v)}
        onRetrospectiveNoteChange={(note) => setRetrospectiveNote(note)}
        onRetrospectiveNoteSave={saveRetrospectiveNote}
        onDiffSearchChange={(text) => setDiffSearchText(text)}
        onDiffTypeFilterChange={(type) => setDiffTypeFilter(type)}
        onExportDiffs={handleExportDiffs}
        onExportCapacity={handleExportCapacityDelta}
        onExportReport={handleExportReport}
      />
    </div>
  );
};

export default PlanManagement;

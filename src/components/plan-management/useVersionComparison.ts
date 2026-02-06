import { useEffect, useMemo, useState } from 'react';
import { message } from 'antd';
import type { EChartsOption } from 'echarts';
import { useQuery } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { formatNumber } from '../../utils/formatters';

import { capacityApi, planApi } from '../../api/tauri';
import type {
  BackendVersionComparisonKpiResult,
  BackendVersionComparisonResult,
  PlanItemSnapshot,
} from '../../types/comparison';
import type { VersionComparisonModalProps } from '../comparison/VersionComparisonModal';
import type { LocalCapacityDeltaRow } from '../comparison/types';
import {
  computeCapacityMap,
  computeDailyTotals,
  computeVersionDiffs,
  makeRetrospectiveKey,
  normalizeDateOnly,
  normalizePlanItem,
} from '../comparison/utils';
import {
  exportCapacityDelta,
  exportDiffs,
  exportReportHTML,
  exportReportMarkdown,
  exportRetrospectiveReport,
  type ExportContext,
} from './exportHelpers';

type VersionsInCompare = { versionIdA: string; versionIdB: string };

export function useVersionComparison(params: {
  selectedVersions: string[];
  currentUser: string;
  setLoading: (loading: boolean) => void;
  onActivateVersion?: (versionId: string) => Promise<void>;
}): { handleCompareVersions: () => Promise<void>; modalProps: VersionComparisonModalProps } {
  const { selectedVersions, currentUser, setLoading, onActivateVersion } = params;

  const [compareModalVisible, setCompareModalVisible] = useState(false);
  const [loadLocalCompareDetail, setLoadLocalCompareDetail] = useState(false);
  const [compareResult, setCompareResult] = useState<BackendVersionComparisonResult | null>(null);
  const [retrospectiveNote, setRetrospectiveNote] = useState('');
  const [retrospectiveSavedAt, setRetrospectiveSavedAt] = useState<string | null>(null);
  const [diffSearchText, setDiffSearchText] = useState('');
  const [diffTypeFilter, setDiffTypeFilter] = useState<
    'ALL' | 'ADDED' | 'REMOVED' | 'MOVED' | 'MODIFIED'
  >('ALL');
  const [showAllCapacityRows, setShowAllCapacityRows] = useState(false);

  const versionsInCompare = useMemo<VersionsInCompare | null>(() => {
    if (!compareResult) return null;
    return { versionIdA: compareResult.version_id_a, versionIdB: compareResult.version_id_b };
  }, [compareResult]);

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
      )) as unknown as BackendVersionComparisonResult;
      setCompareResult(result);
      setCompareModalVisible(true);
      setDiffSearchText('');
      setDiffTypeFilter('ALL');
      setShowAllCapacityRows(false);
      setLoadLocalCompareDetail(false);
      message.success('版本对比完成');
    } catch (error: unknown) {
      console.error('版本对比失败:', error);
    } finally {
      setLoading(false);
    }
  };

  const retrospectiveKey = useMemo(() => {
    if (!compareResult) return null;
    return makeRetrospectiveKey(compareResult.version_id_a, compareResult.version_id_b);
  }, [compareResult]);

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
    } catch (error: unknown) {
      const errMsg = error instanceof Error ? error.message : String(error ?? '');
      message.error(errMsg || '保存失败（本地存储不可用）');
    }
  };

  const compareKpiQuery = useQuery({
    queryKey: ['compareVersionsKpi', versionsInCompare?.versionIdA, versionsInCompare?.versionIdB],
    enabled: compareModalVisible && !!versionsInCompare?.versionIdA && !!versionsInCompare?.versionIdB,
    queryFn: async () => {
      if (!versionsInCompare?.versionIdA || !versionsInCompare?.versionIdB) return null;
      return (await planApi.compareVersionsKpi(
        versionsInCompare.versionIdA,
        versionsInCompare.versionIdB
      )) as unknown as BackendVersionComparisonKpiResult;
    },
    staleTime: 30 * 1000,
  });

  const compareKpiRows = useMemo(() => {
    const data = compareKpiQuery.data;
    if (!data) return null;
    const a = data.kpi_a;
    const b = data.kpi_b;

    const fmtInt = (v: number | null | undefined) =>
      v == null || !Number.isFinite(Number(v)) ? '-' : String(Math.trunc(Number(v)));
    const fmtNum = (v: number | null | undefined, digits = 1) =>
      v == null || !Number.isFinite(Number(v)) ? '-' : formatNumber(Number(v), digits, { useGrouping: false });

    const deltaInt = (va: number | null | undefined, vb: number | null | undefined) =>
      va == null || vb == null ? '-' : String(Math.trunc(Number(vb) - Number(va)));
    const deltaNum = (
      va: number | null | undefined,
      vb: number | null | undefined,
      digits = 1
    ) => (va == null || vb == null ? '-' : formatNumber(Number(vb) - Number(va), digits, { useGrouping: false }));

    return [
      {
        key: 'plan_items_count',
        metric: '排产项数',
        a: fmtInt(a.plan_items_count),
        b: fmtInt(b.plan_items_count),
        delta: deltaInt(a.plan_items_count, b.plan_items_count),
      },
      {
        key: 'total_weight_t',
        metric: '总吨位（吨）',
        a: fmtNum(a.total_weight_t, 1),
        b: fmtNum(b.total_weight_t, 1),
        delta: deltaNum(a.total_weight_t, b.total_weight_t, 1),
      },
      {
        key: 'locked_in_plan_count',
        metric: '冻结项数',
        a: fmtInt(a.locked_in_plan_count),
        b: fmtInt(b.locked_in_plan_count),
        delta: deltaInt(a.locked_in_plan_count, b.locked_in_plan_count),
      },
      {
        key: 'force_release_in_plan_count',
        metric: '强放项数',
        a: fmtInt(a.force_release_in_plan_count),
        b: fmtInt(b.force_release_in_plan_count),
        delta: deltaInt(a.force_release_in_plan_count, b.force_release_in_plan_count),
      },
      {
        key: 'overflow_days',
        metric: '溢出天数（天）',
        a: fmtInt(a.overflow_days),
        b: fmtInt(b.overflow_days),
        delta: deltaInt(a.overflow_days, b.overflow_days),
      },
      {
        key: 'overflow_t',
        metric: '溢出吨位（吨）',
        a: fmtNum(a.overflow_t, 1),
        b: fmtNum(b.overflow_t, 1),
        delta: deltaNum(a.overflow_t, b.overflow_t, 1),
      },
      {
        key: 'capacity_util_pct',
        metric: '产能利用率（%）',
        a: fmtNum(a.capacity_util_pct, 1),
        b: fmtNum(b.capacity_util_pct, 1),
        delta: deltaNum(a.capacity_util_pct, b.capacity_util_pct, 1),
      },
      {
        key: 'mature_backlog_t',
        metric: '适温待排（吨）',
        a: fmtNum(a.mature_backlog_t, 1),
        b: fmtNum(b.mature_backlog_t, 1),
        delta: deltaNum(a.mature_backlog_t, b.mature_backlog_t, 1),
      },
      {
        key: 'immature_backlog_t',
        metric: '未适温待排（吨）',
        a: fmtNum(a.immature_backlog_t, 1),
        b: fmtNum(b.immature_backlog_t, 1),
        delta: deltaNum(a.immature_backlog_t, b.immature_backlog_t, 1),
      },
      {
        key: 'urgent_total_t',
        metric: '紧急吨位（吨）',
        a: fmtNum(a.urgent_total_t, 1),
        b: fmtNum(b.urgent_total_t, 1),
        delta: deltaNum(a.urgent_total_t, b.urgent_total_t, 1),
      },
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
      .sort((a, b) =>
        a.date === b.date ? a.machine_code.localeCompare(b.machine_code) : a.date.localeCompare(b.date)
      );

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
  }, [
    compareModalVisible,
    loadLocalCompareDetail,
    normalizedItemsA,
    normalizedItemsB,
    showAllCapacityRows,
    versionsInCompare,
  ]);

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
      if (
        !versionsInCompare?.versionIdA ||
        !localCapacityRowsBase?.dateFrom ||
        !localCapacityRowsBase?.dateTo
      )
        return [];
      const res = await capacityApi.getCapacityPools(
        localCapacityRowsBase.machines,
        localCapacityRowsBase.dateFrom,
        localCapacityRowsBase.dateTo,
        versionsInCompare.versionIdA
      );
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
      if (
        !versionsInCompare?.versionIdB ||
        !localCapacityRowsBase?.dateFrom ||
        !localCapacityRowsBase?.dateTo
      )
        return [];
      const res = await capacityApi.getCapacityPools(
        localCapacityRowsBase.machines,
        localCapacityRowsBase.dateFrom,
        localCapacityRowsBase.dateTo,
        versionsInCompare.versionIdB
      );
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

    poolsA.forEach((p) => {
      const machine = String(p.machine_code ?? '').trim();
      const date = normalizeDateOnly(String(p.plan_date ?? ''));
      if (!machine || !date) return;
      const target = Number(p.target_capacity_t ?? 0);
      const limit = Number(p.limit_capacity_t ?? 0);
      mapA.set(`${machine}__${date}`, {
        target: Number.isFinite(target) && target > 0 ? target : null,
        limit: Number.isFinite(limit) && limit > 0 ? limit : null,
      });
    });

    poolsB.forEach((p) => {
      const machine = String(p.machine_code ?? '').trim();
      const date = normalizeDateOnly(String(p.plan_date ?? ''));
      if (!machine || !date) return;
      const target = Number(p.target_capacity_t ?? 0);
      const limit = Number(p.limit_capacity_t ?? 0);
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
      legend: { top: 0, left: 'left', data: ['版本甲', '版本乙', '变化值（乙-甲）'] },
      grid: { left: 52, right: 52, top: 36, bottom: 44 },
      xAxis: {
        type: 'category',
        data: dates,
        axisLabel: { formatter: (value: string) => String(value).slice(5) },
      },
      yAxis: [{ type: 'value', name: '吨' }, { type: 'value', name: '变化吨位' }],
      series: [
        {
          name: '版本甲',
          type: 'line',
          data: aValues,
          smooth: true,
          showSymbol: false,
          lineStyle: { width: 2 },
        },
        {
          name: '版本乙',
          type: 'line',
          data: bValues,
          smooth: true,
          showSymbol: false,
          lineStyle: { width: 2 },
        },
        {
          name: '变化值（乙-甲）',
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
      legend: { top: 0, left: 'left', data: ['版本甲风险', '版本乙风险', '变化值'] },
      grid: { left: 52, right: 52, top: 36, bottom: 44 },
      xAxis: {
        type: 'category',
        data: dates,
        axisLabel: { formatter: (value: string) => String(value).slice(5) },
      },
      yAxis: [{ type: 'value', name: '风险' }, { type: 'value', name: '变化值' }],
      series: [
        { name: '版本甲风险', type: 'line', data: aValues, smooth: true, showSymbol: false, lineStyle: { width: 2 } },
        { name: '版本乙风险', type: 'line', data: bValues, smooth: true, showSymbol: false, lineStyle: { width: 2 } },
        {
          name: '变化值',
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

  const modalProps: VersionComparisonModalProps = {
    open: compareModalVisible,
    onClose: () => {
      setCompareModalVisible(false);
      setCompareResult(null);
    },
    compareResult,
    compareKpiRows: compareKpiRows ?? [],
    compareKpiLoading: compareKpiQuery.isLoading,
    compareKpiError: compareKpiQuery.error as Error | null,
    localDiffResult,
    loadLocalCompareDetail,
    planItemsLoading: planItemsQueryA.isLoading || planItemsQueryB.isLoading,
    planItemsErrorA: planItemsQueryA.error as Error | null,
    planItemsErrorB: planItemsQueryB.error as Error | null,
    localCapacityRows,
    localCapacityRowsBase,
    capacityPoolsErrorA: capacityPoolsQueryA.error as Error | null,
    capacityPoolsErrorB: capacityPoolsQueryB.error as Error | null,
    showAllCapacityRows,
    retrospectiveNote,
    retrospectiveSavedAt,
    diffSearchText,
    diffTypeFilter,
    diffSummaryChartOption,
    capacityTrendOption,
    riskTrendOption,
    onActivateVersion,
    onLoadLocalCompareDetail: () => setLoadLocalCompareDetail(true),
    onToggleShowAllCapacityRows: () => setShowAllCapacityRows((v) => !v),
    onRetrospectiveNoteChange: (note) => setRetrospectiveNote(note),
    onRetrospectiveNoteSave: saveRetrospectiveNote,
    onDiffSearchChange: (text) => setDiffSearchText(text),
    onDiffTypeFilterChange: (type) => setDiffTypeFilter(type),
    onExportDiffs: handleExportDiffs,
    onExportCapacity: handleExportCapacityDelta,
    onExportReport: handleExportReport,
  };

  return { handleCompareVersions, modalProps };
}

/**
 * 风险快照分析状态管理 Hook
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { message } from 'antd';
import dayjs, { Dayjs } from 'dayjs';
import { decisionService, planApi } from '../../api/tauri';
import { useEvent } from '../../api/eventBus';
import { useActiveVersionId } from '../../stores/use-global-store';
import type { DaySummary } from '../../types/decision';
import type { VersionOption } from './types';

export interface UseRiskSnapshotChartsReturn {
  // 加载状态
  loading: boolean;

  // 数据
  riskSnapshots: DaySummary[];
  mostRiskyDate: string | null;
  versionOptions: VersionOption[];

  // 筛选状态
  selectedVersion: string;
  setSelectedVersion: (version: string) => void;
  dateRange: [Dayjs, Dayjs] | null;
  setDateRange: (range: [Dayjs, Dayjs] | null) => void;

  // Tab 状态
  activeTab: string;
  setActiveTab: (tab: string) => void;

  // 操作
  loadRiskSnapshots: (versionId: string) => Promise<void>;
  refresh: () => void;

  // 版本
  activeVersionId: string | null;
}

export function useRiskSnapshotCharts(): UseRiskSnapshotChartsReturn {
  const activeVersionId = useActiveVersionId();
  const [loading, setLoading] = useState(false);
  const [selectedVersion, setSelectedVersion] = useState<string>('');
  const [versionOptions, setVersionOptions] = useState<VersionOption[]>([]);
  const [rawRiskSnapshots, setRawRiskSnapshots] = useState<DaySummary[]>([]);
  const [mostRiskyDate, setMostRiskyDate] = useState<string | null>(null);
  const [dateRange, setDateRange] = useState<[Dayjs, Dayjs] | null>(null);
  const [activeTab, setActiveTab] = useState<string>('trend');
  const prevActiveVersionIdRef = useRef<string | null>(null);

  // 加载风险快照数据
  const loadRiskSnapshots = useCallback(async (versionId: string) => {
    if (!versionId) {
      message.warning('请先激活一个版本');
      return;
    }

    setLoading(true);
    try {
      const result = await decisionService.getAllRiskSnapshots(versionId);
      const items = result?.items || [];
      setRawRiskSnapshots(items);
      // mostRiskyDate 可由已加载数据计算，避免额外 IPC 调用
      const most = items.reduce<DaySummary | null>((best, cur) => {
        if (!best) return cur;
        const bestScore = Number(best.riskScore || 0);
        const curScore = Number(cur.riskScore || 0);
        if (curScore !== bestScore) return curScore > bestScore ? cur : best;
        return String(cur.planDate || '') < String(best.planDate || '') ? cur : best;
      }, null);
      setMostRiskyDate(most?.planDate || null);
      message.success(`成功加载 ${items.length} 条风险摘要`);
    } catch (error: any) {
      console.error('加载风险快照失败:', error);
    } finally {
      setLoading(false);
    }
  }, []);

  // 刷新操作
  const refresh = useCallback(() => {
    if (selectedVersion) {
      loadRiskSnapshots(selectedVersion);
    }
  }, [selectedVersion, loadRiskSnapshots]);

  // 订阅risk_snapshot_updated事件,自动刷新
  useEvent('risk_snapshot_updated', () => {
    if (!activeVersionId) return;
    // risk_snapshot_updated 语义上通常指"当前激活版本"的快照刷新。
    // 若用户正在查看其他版本，避免把视图强制切回 activeVersionId。
    if (!selectedVersion || selectedVersion === activeVersionId) {
      const vid = selectedVersion || activeVersionId;
      loadRiskSnapshots(vid);
    }
  });

  // 过滤和排序后的风险快照
  const riskSnapshots = useMemo(() => {
    const base = Array.isArray(rawRiskSnapshots) ? rawRiskSnapshots : [];
    const filtered = dateRange
      ? base.filter((s: DaySummary) => {
          const d = dayjs(s.planDate);
          return (
            d.isAfter(dateRange[0].startOf('day').subtract(1, 'millisecond')) &&
            d.isBefore(dateRange[1].endOf('day').add(1, 'millisecond'))
          );
        })
      : base;

    // 确保按日期升序，避免不同后端排序导致图表抖动
    return [...filtered].sort((a, b) => a.planDate.localeCompare(b.planDate));
  }, [rawRiskSnapshots, dateRange]);

  // 默认跟随"当前激活版本"；如果用户手动切换到其他版本，则保持其选择不被覆盖。
  useEffect(() => {
    const prev = prevActiveVersionIdRef.current;
    prevActiveVersionIdRef.current = activeVersionId;

    if (!activeVersionId) return;
    if (!selectedVersion || selectedVersion === prev) {
      setSelectedVersion(activeVersionId);
    }
  }, [activeVersionId, selectedVersion]);

  // 版本切换时自动刷新（避免用户必须手动点击"刷新"）
  useEffect(() => {
    if (selectedVersion) {
      loadRiskSnapshots(selectedVersion);
    }
  }, [selectedVersion, loadRiskSnapshots]);

  // 加载版本下拉选项（跨方案汇总）
  useEffect(() => {
    if (!activeVersionId) return;

    (async () => {
      try {
        const plans = await planApi.listPlans();
        const options: VersionOption[] = [];

        for (const plan of plans || []) {
          const versions = await planApi.listVersions(plan.plan_id);
          for (const v of versions || []) {
            options.push({
              value: v.version_id,
              label: `${plan.plan_name} / V${v.version_no} (${v.status})`,
            });
          }
        }

        setVersionOptions(options);
      } catch (error: any) {
        console.error('加载版本列表失败:', error);
        setVersionOptions([]);
      }
    })();
  }, [activeVersionId]);

  return {
    loading,
    riskSnapshots,
    mostRiskyDate,
    versionOptions,
    selectedVersion,
    setSelectedVersion,
    dateRange,
    setDateRange,
    activeTab,
    setActiveTab,
    loadRiskSnapshots,
    refresh,
    activeVersionId,
  };
}

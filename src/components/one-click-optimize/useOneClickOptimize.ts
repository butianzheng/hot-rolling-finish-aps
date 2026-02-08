/**
 * OneClickOptimize 状态管理 Hook
 */

import { useState } from 'react';
import { message } from 'antd';
import type { Dayjs } from 'dayjs';
import dayjs from 'dayjs';
import { planApi } from '../../api/tauri';
import { useGlobalActions, useGlobalStore } from '../../stores/use-global-store';
import { formatDate } from '../../utils/formatters';
import type { OptimizeStrategy, SimulateResult } from './types';
import { DEFAULT_LATEST_RUN_TTL_MS } from '../../stores/latestRun';
import { createRunId } from '../../utils/runId';
import { getStrategyLabel } from './types';

interface UseOneClickOptimizeOptions {
  activeVersionId: string | null;
  operator: string;
  onBeforeExecute?: () => void;
  onAfterExecute?: () => void;
}

export function useOneClickOptimize({
  activeVersionId,
  operator,
  onBeforeExecute,
  onAfterExecute,
}: UseOneClickOptimizeOptions) {
  const {
    setActiveVersion,
    beginLatestRun,
    markLatestRunRunning,
    markLatestRunDone,
    markLatestRunFailed,
    expireLatestRunIfNeeded,
  } = useGlobalActions();

  const [previewOpen, setPreviewOpen] = useState(false);
  const [baseDate, setBaseDate] = useState<Dayjs>(dayjs());
  const [windowDaysOverride, setWindowDaysOverride] = useState<number | null>(null);
  const [simulateLoading, setSimulateLoading] = useState(false);
  const [executeLoading, setExecuteLoading] = useState(false);
  const [simulateResult, setSimulateResult] = useState<SimulateResult | null>(null);
  const [strategy, setStrategy] = useState<OptimizeStrategy>('balanced');
  const [postCreateOpen, setPostCreateOpen] = useState(false);
  const [createdVersionId, setCreatedVersionId] = useState<string | null>(null);
  const [postActionLoading, setPostActionLoading] = useState<'switch' | 'activate' | null>(null);

  const strategyLabel = getStrategyLabel(strategy);

  const changeBaseDate = (date: Dayjs) => {
    setBaseDate(date);
    setSimulateResult(null);
  };

  const changeWindowDaysOverride = (v: number | null) => {
    setWindowDaysOverride(v);
    setSimulateResult(null);
  };

  const runSimulate = async () => {
    if (!activeVersionId) {
      message.warning('请先激活一个版本');
      return;
    }

    setSimulateLoading(true);
    try {
      const res = await planApi.simulateRecalc(
        activeVersionId,
        formatDate(baseDate),
        undefined,
        operator,
        strategy,
        windowDaysOverride ?? undefined
      );
      setSimulateResult(res);
      message.success('试算完成');
    } catch (e: any) {
      console.error('【一键优化】模拟执行失败：', e);
      message.error(e?.message || '试算失败');
      setSimulateResult(null);
    } finally {
      setSimulateLoading(false);
    }
  };

  const runExecute = async () => {
    if (!activeVersionId) {
      message.warning('请先激活一个版本');
      return;
    }

    expireLatestRunIfNeeded();
    const localRunId = createRunId('recalc');
    const beginResult = beginLatestRun({
      runId: localRunId,
      versionId: activeVersionId,
      ttlMs: DEFAULT_LATEST_RUN_TTL_MS,
    });

    if (!beginResult.accepted) {
      message.info('已存在更新的重算触发，本次请求已忽略');
      return;
    }

    setExecuteLoading(true);
    onBeforeExecute?.();
    markLatestRunRunning(localRunId);
    try {
      const res: any = await planApi.recalcFull(
        activeVersionId,
        formatDate(baseDate),
        undefined,
        operator,
        strategy,
        windowDaysOverride ?? undefined,
        localRunId,
      );

      const responseRunId = String(res?.run_id ?? localRunId).trim() || localRunId;
      const responsePlanRev = Number(res?.plan_rev);
      const newVersionId = String(res?.version_id ?? '').trim();

      markLatestRunDone(responseRunId, {
        versionId: newVersionId || activeVersionId,
        planRev: Number.isFinite(responsePlanRev) ? responsePlanRev : undefined,
      });

      const latestRunId = useGlobalStore.getState().latestRun.runId;
      if (latestRunId !== responseRunId) {
        return;
      }

      message.success(String(res?.message || '重算完成'));
      if (newVersionId) {
        setCreatedVersionId(newVersionId);
        setPostCreateOpen(true);
      }
      setPreviewOpen(false);
      setSimulateResult(null);
    } catch (e: any) {
      console.error('【一键优化】正式执行失败：', e);
      markLatestRunFailed(localRunId, e?.message || '重算失败');
      message.error(e?.message || '重算失败');
    } finally {
      setExecuteLoading(false);
      onAfterExecute?.();
    }
  };

  const handleSwitch = async () => {
    if (!createdVersionId) return;
    setPostActionLoading('switch');
    try {
      setActiveVersion(createdVersionId);
      message.success('已切换到新版本');
      setPostCreateOpen(false);
      setCreatedVersionId(null);
    } finally {
      setPostActionLoading(null);
    }
  };

  const handleActivate = async () => {
    if (!createdVersionId) return;
    setPostActionLoading('activate');
    try {
      await planApi.activateVersion(createdVersionId, operator || 'admin');
      setActiveVersion(createdVersionId);
      message.success('已激活并切换到新版本');
      setPostCreateOpen(false);
      setCreatedVersionId(null);
    } finally {
      setPostActionLoading(null);
    }
  };

  const closePreview = () => {
    setPreviewOpen(false);
    setSimulateResult(null);
  };

  const closePostCreate = () => {
    setPostCreateOpen(false);
    setCreatedVersionId(null);
    setPostActionLoading(null);
  };

  const changeStrategy = (newStrategy: OptimizeStrategy) => {
    setStrategy(newStrategy);
    setSimulateResult(null);
  };

  const openPreview = () => {
    setPreviewOpen(true);
  };

  const openPreviewWithStrategy = (newStrategy: OptimizeStrategy) => {
    setStrategy(newStrategy);
    setPreviewOpen(true);
    setSimulateResult(null);
  };

  return {
    // 状态
    previewOpen,
    baseDate,
    changeBaseDate,
    windowDaysOverride,
    simulateLoading,
    executeLoading,
    simulateResult,
    strategy,
    strategyLabel,
    postCreateOpen,
    createdVersionId,
    postActionLoading,

    // 操作
    runSimulate,
    runExecute,
    handleSwitch,
    handleActivate,
    closePreview,
    closePostCreate,
    changeStrategy,
    changeWindowDaysOverride,
    openPreview,
    openPreviewWithStrategy,
  };
}

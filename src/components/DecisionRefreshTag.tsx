import React, { useMemo, useState } from 'react';
import { Tag, Tooltip, message } from 'antd';
import { SyncOutlined } from '@ant-design/icons';
import { useDecisionRefreshStatus } from '../hooks/useDecisionRefreshStatus';
import { useEvent } from '../api/eventBus';
import { dashboardApi } from '../api/tauri';
import { useCurrentUser } from '../stores/use-global-store';

function isRecent(ts?: string | null, windowMs: number = 15_000): boolean {
  if (!ts) return false;
  const t = new Date(ts).getTime();
  if (!Number.isFinite(t)) return false;
  return Date.now() - t <= windowMs;
}

export const DecisionRefreshTag: React.FC<{ versionId: string | null }> = ({ versionId }) => {
  const query = useDecisionRefreshStatus(versionId);
  const currentUser = useCurrentUser();
  const [retrying, setRetrying] = useState(false);

  // 关键事件触发时主动拉一次状态，避免等 refetchInterval。
  useEvent('plan_updated', () => query.refetch());
  useEvent('risk_snapshot_updated', () => query.refetch());
  useEvent('material_state_changed', () => query.refetch());

  const data = query.data;

  const hint = useMemo(() => {
    if (!data) return '决策刷新状态未知';
    const counts = data.queue_counts || ({} as any);
    const running = Number(counts.running || 0) + Number(counts.pending || 0);
    const lastCompletedAt = data.latest_log?.completed_at ?? data.latest_task?.completed_at ?? null;

    if (data.is_refreshing) {
      return `刷新中：${running} 个任务`;
    }
    if (data.last_error) {
      return `刷新失败：${data.last_error}`;
    }
    if (isRecent(lastCompletedAt, 30_000)) {
      return `已刷新：${lastCompletedAt}`;
    }
    return 'OK';
  }, [data]);

  if (!versionId) return null;

  if (query.isFetching && !data) {
    return (
      <Tooltip title="正在获取决策刷新状态…">
        <Tag color="default" style={{ margin: 0 }}>
          <SyncOutlined spin style={{ marginRight: 6 }} />
          决策
        </Tag>
      </Tooltip>
    );
  }

  if (!data) return null;

  const lastCompletedAt = data.latest_log?.completed_at ?? data.latest_task?.completed_at ?? null;

  if (data.is_refreshing) {
    const inflight = Number(data.queue_counts?.pending || 0) + Number(data.queue_counts?.running || 0);
    return (
      <Tooltip title={hint}>
        <Tag color="processing" style={{ margin: 0 }}>
          <SyncOutlined spin style={{ marginRight: 6 }} />
          决策刷新中{inflight > 0 ? `(${inflight})` : ''}
        </Tag>
      </Tooltip>
    );
  }

  if (data.last_error) {
    const handleRetry = async () => {
      if (!versionId) return;
      if (retrying) return;

      setRetrying(true);
      try {
        const operator = currentUser || 'admin';
        const resp = await dashboardApi.manualRefreshDecision(versionId, operator);
        message.success(resp?.message || '已触发决策刷新');
        query.refetch();
      } catch (e: any) {
        message.error(e?.message || '触发决策刷新失败');
      } finally {
        setRetrying(false);
      }
    };

    return (
      <Tooltip title={`${hint}（点击重试）`}>
        <Tag color="error" style={{ margin: 0, cursor: 'pointer' }} onClick={handleRetry}>
          决策刷新失败
        </Tag>
      </Tooltip>
    );
  }

  if (isRecent(lastCompletedAt, 15_000)) {
    return (
      <Tooltip title={hint}>
        <Tag color="success" style={{ margin: 0 }}>
          决策已更新
        </Tag>
      </Tooltip>
    );
  }

  return null;
};

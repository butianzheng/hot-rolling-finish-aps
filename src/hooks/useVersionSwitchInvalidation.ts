/**
 * 版本切换时自动失效 React Query 缓存
 *
 * 问题背景：
 * - 当用户切换版本时（通过版本管理激活不同版本），前端 React Query 可能缓存旧版本的决策数据
 * - 虽然 queryKey 包含 versionId，理论上会重新获取，但存在缓存延迟和用户感知不同步的问题
 * - 需要在版本切换时立即失效所有决策相关的查询缓存，强制重新获取最新数据
 *
 * 解决方案：
 * - 监听全局 activeVersionId 的变化
 * - 当检测到版本切换时，立即失效所有决策相关的 React Query 缓存
 * - 配合 useDecisionRefreshStatus 的刷新完成后失效逻辑，确保数据一致性
 */

import { useEffect, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useActiveVersionId } from '../stores/use-global-store';
import { decisionQueryKeys } from './queries/use-decision-queries';

/**
 * 版本切换缓存失效 Hook
 *
 * 使用方式：
 * - 在顶层组件（如 App.tsx 或 Layout.tsx）中调用
 * - 确保在版本切换时立即失效所有决策数据缓存
 */
export function useVersionSwitchInvalidation() {
  const queryClient = useQueryClient();
  const activeVersionId = useActiveVersionId();
  const prevVersionIdRef = useRef<string | null>(null);

  useEffect(() => {
    // 首次挂载时，初始化 prevVersionIdRef，不触发失效
    if (prevVersionIdRef.current === undefined) {
      prevVersionIdRef.current = activeVersionId;
      return;
    }

    // 检测版本切换（activeVersionId 发生变化，且不为 null）
    const versionChanged =
      activeVersionId !== null &&
      prevVersionIdRef.current !== activeVersionId;

    if (versionChanged) {
      console.log('[VersionSwitch] 检测到版本切换:', {
        from: prevVersionIdRef.current,
        to: activeVersionId,
      });

      // 立即失效所有决策相关的查询
      queryClient.invalidateQueries({ queryKey: decisionQueryKeys.all });

      // 失效全局 KPI 查询（按新版本重新获取）
      queryClient.invalidateQueries({ queryKey: ['globalKpi', activeVersionId] });

      console.log('[VersionSwitch] 已失效所有决策数据缓存，将重新获取');
    }

    // 更新 prevVersionIdRef 为当前值
    prevVersionIdRef.current = activeVersionId;
  }, [activeVersionId, queryClient]);
}

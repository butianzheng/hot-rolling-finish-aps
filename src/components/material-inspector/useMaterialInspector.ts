/**
 * MaterialInspector 状态管理 Hook
 */

import { useState, useEffect, useCallback } from 'react';
import { dashboardApi } from '../../api/tauri';
import { formatDateTime } from '../../utils/formatters';
import type { ActionLog, Material } from './types';

export interface UseMaterialInspectorReturn {
  actionLogs: ActionLog[];
  loadingLogs: boolean;
  loadActionLogs: () => Promise<void>;
}

export function useMaterialInspector(
  visible: boolean,
  material: Material | null
): UseMaterialInspectorReturn {
  const [actionLogs, setActionLogs] = useState<ActionLog[]>([]);
  const [loadingLogs, setLoadingLogs] = useState(false);

  const loadActionLogs = useCallback(async () => {
    if (!material) return;

    setLoadingLogs(true);
    try {
      // 获取最近30天的操作日志
      const endTime = formatDateTime(new Date());
      const startTime = formatDateTime(new Date(Date.now() - 30 * 24 * 60 * 60 * 1000));
      const logs = await dashboardApi.listActionLogsByMaterial(material.material_id, startTime, endTime, 10);
      setActionLogs(Array.isArray(logs) ? (logs as ActionLog[]) : []);
    } catch (error) {
      console.error('加载操作历史失败:', error);
      setActionLogs([]);
    } finally {
      setLoadingLogs(false);
    }
  }, [material]);

  // 加载操作历史
  useEffect(() => {
    if (visible && material) {
      loadActionLogs();
    }
  }, [visible, material, loadActionLogs]);

  return {
    actionLogs,
    loadingLogs,
    loadActionLogs,
  };
}

export default useMaterialInspector;

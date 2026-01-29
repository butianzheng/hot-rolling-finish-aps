/**
 * 操作日志状态管理 Hook
 */

import { useCallback, useEffect, useMemo, useState } from 'react';
import { message } from 'antd';
import dayjs, { Dayjs } from 'dayjs';
import { dashboardApi } from '../../api/tauri';
import { formatDateTime } from '../../utils/formatters';
import { useDebounce } from '../../hooks/useDebounce';
import type { ActionLog } from './types';

export interface UseActionLogQueryReturn {
  // 加载状态
  loading: boolean;
  loadError: string | null;

  // 数据
  actionLogs: ActionLog[];
  filteredLogs: ActionLog[];

  // 筛选状态
  timeRange: [Dayjs, Dayjs] | null;
  setTimeRange: (range: [Dayjs, Dayjs] | null) => void;
  selectedActionType: string;
  setSelectedActionType: (type: string) => void;
  selectedActor: string;
  setSelectedActor: (actor: string) => void;
  selectedVersion: string;
  setSelectedVersion: (version: string) => void;
  searchText: string;
  setSearchText: (text: string) => void;

  // 详情模态框
  selectedLog: ActionLog | null;
  showDetailModal: boolean;
  handleViewDetail: (log: ActionLog) => void;
  closeDetailModal: () => void;

  // 操作
  loadActionLogs: (limit?: number) => Promise<void>;
  clearFilters: () => void;

  // 唯一值列表
  uniqueActors: string[];
  uniqueVersions: string[];
}

export function useActionLogQuery(): UseActionLogQueryReturn {
  const [loading, setLoading] = useState(false);
  const [actionLogs, setActionLogs] = useState<ActionLog[]>([]);
  const [filteredLogs, setFilteredLogs] = useState<ActionLog[]>([]);
  const [selectedLog, setSelectedLog] = useState<ActionLog | null>(null);
  const [showDetailModal, setShowDetailModal] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  // 筛选条件
  const [timeRange, setTimeRange] = useState<[Dayjs, Dayjs] | null>(null);
  const [selectedActionType, setSelectedActionType] = useState<string>('all');
  const [selectedActor, setSelectedActor] = useState<string>('all');
  const [selectedVersion, setSelectedVersion] = useState<string>('all');
  const [searchText, setSearchText] = useState('');

  // 防抖搜索文本（延迟 300ms）
  const debouncedSearchText = useDebounce(searchText, 300);

  // 加载操作日志
  const loadActionLogs = useCallback(async (limit: number = 100) => {
    setLoading(true);
    setLoadError(null);
    try {
      let result;
      if (timeRange) {
        const [start, end] = timeRange;
        result = await dashboardApi.listActionLogs(
          formatDateTime(start),
          formatDateTime(end)
        );
      } else {
        result = await dashboardApi.getRecentActions(limit);
      }

      setActionLogs(result);
      setFilteredLogs(result);
      message.success(`成功加载 ${result.length} 条操作日志`);
    } catch (error: any) {
      console.error('加载操作日志失败:', error);
      const msg = String(error?.message || error || '加载失败');
      setLoadError(msg);
      message.error(`加载失败: ${msg}`);
    } finally {
      setLoading(false);
    }
  }, [timeRange]);

  // 筛选数据
  const filterData = useCallback(() => {
    let filtered = [...actionLogs];

    // 按时间范围筛选
    if (timeRange) {
      const [start, end] = timeRange;
      filtered = filtered.filter((log) => {
        const logTime = dayjs(log.action_ts);
        return logTime.isAfter(start.subtract(1, 'second')) && logTime.isBefore(end.add(1, 'second'));
      });
    }

    // 按操作类型筛选
    if (selectedActionType !== 'all') {
      filtered = filtered.filter((log) => log.action_type === selectedActionType);
    }

    // 按操作人筛选
    if (selectedActor !== 'all') {
      filtered = filtered.filter((log) => log.actor === selectedActor);
    }

    // 按版本筛选
    if (selectedVersion !== 'all') {
      filtered = filtered.filter((log) => log.version_id === selectedVersion);
    }

    // 按搜索文本筛选（使用防抖后的搜索文本）
    if (debouncedSearchText) {
      const searchLower = debouncedSearchText.toLowerCase();
      filtered = filtered.filter(
        (log) =>
          log.action_id.toLowerCase().includes(searchLower) ||
          log.detail?.toLowerCase().includes(searchLower)
      );
    }

    setFilteredLogs(filtered);
  }, [actionLogs, timeRange, selectedActionType, selectedActor, selectedVersion, debouncedSearchText]);

  // 查看详情
  const handleViewDetail = useCallback((log: ActionLog) => {
    setSelectedLog(log);
    setShowDetailModal(true);
  }, []);

  const closeDetailModal = useCallback(() => {
    setShowDetailModal(false);
  }, []);

  // 清除筛选
  const clearFilters = useCallback(() => {
    setTimeRange(null);
    setSelectedActionType('all');
    setSelectedActor('all');
    setSelectedVersion('all');
    setSearchText('');
  }, []);

  // 获取唯一的操作人列表
  const uniqueActors = useMemo(() => {
    const actors = new Set(actionLogs.map((log) => log.actor));
    return Array.from(actors);
  }, [actionLogs]);

  // 获取唯一的版本列表
  const uniqueVersions = useMemo(() => {
    const versions = new Set(actionLogs.map((log) => log.version_id));
    return Array.from(versions);
  }, [actionLogs]);

  // 初始加载
  useEffect(() => {
    loadActionLogs();
  }, []);

  // 筛选条件变化时重新筛选（使用防抖后的搜索文本）
  useEffect(() => {
    filterData();
  }, [filterData]);

  return {
    loading,
    loadError,
    actionLogs,
    filteredLogs,
    timeRange,
    setTimeRange,
    selectedActionType,
    setSelectedActionType,
    selectedActor,
    setSelectedActor,
    selectedVersion,
    setSelectedVersion,
    searchText,
    setSearchText,
    selectedLog,
    showDetailModal,
    handleViewDetail,
    closeDetailModal,
    loadActionLogs,
    clearFilters,
    uniqueActors,
    uniqueVersions,
  };
}
